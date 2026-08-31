use std::path::PathBuf;
use std::process::Stdio;

use eyre::{Context, ContextCompat, Result};
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    command: Option<PathBuf>,

    #[serde(default)]
    args: Vec<String>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(
    config: &Config,
    source: Mut<Source<Metadata>>,
    sink: Mut<Sink<Metadata>>,
) -> Result<()> {
    let command = config.command.as_deref().context("missing 'command'")?;

    let mut child = Command::new(command)
        .args(config.args.iter())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn program")?;

    let stdin = child.stdin.take().expect("stdin was captured");

    let stdout = child
        .stdout
        .take()
        .map(BufReader::new)
        .expect("stdout was captured");

    External {
        child,
        stdin,
        stdout,

        source: source.into_inner(),
        sink: sink.into_inner(),
    }
    .run()
    .await
}

struct External {
    #[expect(dead_code)]
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,

    source: Source<Metadata>,
    sink: Sink<Metadata>,
}

impl External {
    async fn run(&mut self) -> Result<()> {
        loop {
            let Some(metadata) = self.source.pull().await else {
                return Ok(());
            };

            let metadata = self.process_metadata(metadata).await?;

            if !self.sink.push(metadata) {
                return Ok(());
            }
        }
    }

    async fn process_metadata(&mut self, metadata: Metadata) -> Result<Metadata> {
        let mut buf = Vec::new();

        serde_json::to_writer(&mut buf, &metadata)?;
        buf.push(b'\n');
        self.stdin.write_all(&buf).await?;
        self.stdin.flush().await?;

        loop {
            buf.clear();
            self.stdout.read_until(b'\n', &mut buf).await?;

            match try2!({
                let s = str::from_utf8(&buf)?;
                let x = serde_json::from_str(s.trim())?;
                Result::<_>::Ok(x)
            })
            .context("failed to parse metadata")
            {
                Ok(metadata) => {
                    return Ok(metadata);
                }

                Err(e) => {
                    warn!("{e:#}");
                    continue;
                }
            }
        }
    }
}
