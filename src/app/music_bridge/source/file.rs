use std::path::PathBuf;

use eyre::{Context, ContextCompat, Result};
use module::Merge;
use module::types::Overridable;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize, Merge)]
pub struct Config {
    path: Option<Overridable<PathBuf>>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, sink: Mut<Sink<Metadata>>) -> Result<()> {
    let mut sink = sink.into_inner();

    let path = config.path.as_deref().context("missing 'path'")?;

    loop {
        let mut reader = File::options()
            .read(true)
            .open(path)
            .await
            .map(BufReader::new)
            .map(BufReader::lines)
            .with_context(|| format!("failed to open '{}'", path.display()))?;

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

            let metadata = match serde_json::from_str(line).context("failed to parse metadata") {
                Ok(x) => x,
                Err(e) => {
                    warn!("{e:#}");
                    continue;
                }
            };

            debug!("parsed metadata: {metadata:#?}");

            if !sink.push(metadata) {
                return Ok(());
            }
        }
    }
}
