use std::path::Path;

use eyre::Result;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::sync::mpsc;

use super::model;

///////////////////////////////////////////////////////////////////////////////

pub async fn run(tx: mpsc::Sender<model::Action>, path: &Path) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.read(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;

        let metadata = fs::metadata(path).await?;
        if metadata.file_type().is_fifo() {
            options.write(true);
        }
    }

    let reader = options
        .open(path)
        .await
        .map(BufReader::new)
        .map(AsyncBufReadExt::lines)?;

    File { reader, tx }.run().await
}

struct File {
    reader: Lines<BufReader<fs::File>>,
    tx: mpsc::Sender<model::Action>,
}

impl File {
    async fn run(&mut self) -> Result<()> {
        while let Some(line) = self.reader.next_line().await? {
            debug!("line={line:?}");

            let action = match self.parse_action(&line) {
                Ok(x) => x,
                Err(e) => {
                    warn!("{e:#}");
                    continue;
                }
            };

            if self.tx.send(action).await.is_err() {
                return Ok(());
            }
        }

        trace!("eof");
        Ok(())
    }

    fn parse_action(&mut self, line: &str) -> Result<model::Action> {
        let line = line.trim();
        let x = serde_json::from_str(line)?;
        Ok(x)
    }
}
