use eyre::{Context, Result};
use module::Merge;
use serde::Deserialize;

use super::metadata::Metadata;
use super::pipeline::Sink;

///////////////////////////////////////////////////////////////////////////////

pub mod playerctl;

mod prelude {
    pub(super) use super::super::metadata::Metadata;
    pub(super) use super::super::pipeline::Sink;
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Config {
    Playerctl(playerctl::Config),
}

impl Config {
    pub fn default_for_platform() -> Option<Self> {
        cfg_select! {
            target_os = "linux" => {
                Some(Self::Playerctl(playerctl::Config { command: None, player: None }))
            },

            // TODO: add macos platform

            _ => {
                None
            },
        }
    }
}

impl Merge for Config {
    fn merge_ref(&mut self, other: Self) -> Result<(), module::Error> {
        match (self, other) {
            (Self::Playerctl(a), Self::Playerctl(b)) => a.merge_ref(b),
            (Self::Playerctl(_), _) => Err(module::Error::collision()),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, sink: Sink<Metadata>) -> Result<()> {
    match config {
        Config::Playerctl(x) => playerctl::run(x, sink).await.context("playerctl"),
    }
}
