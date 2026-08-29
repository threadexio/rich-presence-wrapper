use eyre::{Context, Result};
use module::Merge;
use serde::Deserialize;

use super::metadata::Metadata;
use super::pipeline::Sink;

///////////////////////////////////////////////////////////////////////////////

pub mod external;
pub mod file;
pub mod playerctl;

mod prelude {
    pub(super) use super::super::metadata::Metadata;
    pub(super) use super::super::pipeline::Sink;
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Config {
    External(external::Config),
    File(file::Config),
    Playerctl(playerctl::Config),
}

impl Config {
    pub fn default_for_platform() -> Option<Self> {
        cfg_select! {
            target_os = "linux" => {
                Some(Self::Playerctl(playerctl::Config::default()))
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
            (Self::External(a), Self::External(b)) => a.merge_ref(b),
            (Self::External(_), _) => Err(module::Error::collision()),

            (Self::File(a), Self::File(b)) => a.merge_ref(b),
            (Self::File(_), _) => Err(module::Error::collision()),

            (Self::Playerctl(a), Self::Playerctl(b)) => a.merge_ref(b),
            (Self::Playerctl(_), _) => Err(module::Error::collision()),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, sink: Sink<Metadata>) -> Result<()> {
    match config {
        Config::External(x) => external::run(x, sink).await.context("external"),
        Config::File(x) => file::run(x, sink).await.context("file"),
        Config::Playerctl(x) => playerctl::run(x, sink).await.context("playerctl"),
    }
}
