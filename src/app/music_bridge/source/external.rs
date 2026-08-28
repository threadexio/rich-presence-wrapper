use std::path::PathBuf;
use std::process::Stdio;

use eyre::{Context, ContextCompat, Result};
use module::Merge;
use module::types::{Ordered, Overridable};
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, Merge)]
pub struct Config {
    pub command: Option<Overridable<PathBuf>>,
    pub args: Option<Overridable<Ordered<Vec<String>>>>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, mut sink: Sink<Metadata>) -> Result<()> {
    let command = config.command.as_deref().context("missing 'command'")?;

    let args = config
        .args
        .as_ref()
        .map(|x| &***x)
        .context("missing 'args'")?;

    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn program")?;

    let mut reader = child
        .stdout
        .take()
        .map(BufReader::new)
        .expect("stdout was captured")
        .lines();

    while let Some(line) = reader
        .next_line()
        .await
        .context("failed to read metadata")?
    {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        debug!("metadata line: {line:?}");

        let metadata = serde_json::from_str(line).context("failed to parse metadata")?;
        debug!("parsed metadata: {metadata:#?}");

        if !sink.push(metadata).await {
            break;
        }
    }

    Ok(())
}
