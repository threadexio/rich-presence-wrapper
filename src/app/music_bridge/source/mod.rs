use eyre::{Context, Result};
use magic_args::{Extend, Mut, apply};
use module::Merge;
use serde::Deserialize;

use super::metadata::Metadata;
use super::pipeline::Sink;

///////////////////////////////////////////////////////////////////////////////

macro_rules! source {
    ($mod:ident if $cfg:meta) => {
        #[cfg($cfg)]
        mod $mod;

        #[cfg(not($cfg))]
        mod $mod {
            use eyre::{Result, bail};
            use module::Merge;
            use serde::Deserialize;

            #[derive(Debug, Clone, Deserialize, Merge)]
            #[serde(rename_all = "kebab-case")]
            pub struct Config {}

            pub async fn run() -> Result<()> {
                bail!(
                    "source is not compiled-in for this build of {}. see: `--version`",
                    env!("CARGO_BIN_NAME")
                )
            }
        }
    };
}

source!(external if feature = "music-bridge.source.external");
source!(file if feature = "music-bridge.source.file");
source!(playerctl if feature = "music-bridge.source.playerctl");

#[allow(unused_imports)]
mod prelude {
    pub(super) use super::super::metadata::Metadata;
    pub(super) use super::super::pipeline::Sink;
    pub(super) use magic_args::Mut;
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
    pub fn name(&self) -> &'static str {
        match self {
            Self::External(_) => "external",
            Self::File(_) => "file",
            Self::Playerctl(_) => "playerctl",
        }
    }

    pub fn default_for_platform() -> Option<Self> {
        cfg_select! {
            all(target_os = "linux", feature = "music-bridge.source.playerctl")  => {
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
    let all = (config, Mut::from(sink));

    match config {
        Config::External(x) => apply(external::run, all.extend(x)).await,
        Config::File(x) => apply(file::run, all.extend(x)).await,
        Config::Playerctl(x) => apply(playerctl::run, all.extend(x)).await,
    }
    .context(config.name())
}
