use std::path::PathBuf;
use std::process::ExitCode;

use eyre::{Context, Result, bail};
use module::Merge;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::Instrument;

use crate::discord::*;

mod file;
mod model;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, clap::Parser)]
pub struct Command {
    #[arg(long)]
    client_id: String,

    #[arg(
        long,
        help = "Read activity updates from this file.",
        value_name = "path"
    )]
    file: Vec<PathBuf>,
}

#[derive(Debug, Default, Deserialize, Merge)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(command: &Command) -> Result<ExitCode> {
    let (tx, rx) = mpsc::channel(16);

    for file in &command.file {
        tokio::spawn({
            let file = file.clone();
            let tx = tx.clone();

            async move {
                if let Err(e) = file::run(tx, &file)
                    .instrument(error_span!("file", path = file.display().to_string()))
                    .await
                {
                    error!("{e:#}")
                }
            }
        });
    }

    drop(tx);
    if rx.is_closed() {
        bail!("you must specify at least one source")
    }

    External {
        action: rx,
        discord: Discord::builder().client_id(&command.client_id).finish(),
    }
    .run()
    .await
    .map(|_| ExitCode::SUCCESS)
}

struct External {
    action: mpsc::Receiver<model::Action>,
    discord: Discord,
}

impl External {
    async fn run(&mut self) -> Result<()> {
        while let Some(action) = self.action.recv().await {
            debug!("-> {action:#?}");

            match action {
                model::Action::Update(activity) => {
                    self.discord
                        .set_activity(Activity::from(*activity))
                        .await
                        .context("failed to set activity")?;
                }

                model::Action::Clear => {
                    self.discord
                        .clear_activity()
                        .await
                        .context("failed to clear activity")?;
                }
            }
        }

        Ok(())
    }
}
