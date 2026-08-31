use eyre::Result;
use module::Merge;
use serde::Deserialize;

use super::metadata::Metadata;
use super::pipeline::Sink;

///////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "music-bridge.source.external")]
pub mod external;

#[cfg(feature = "music-bridge.source.file")]
pub mod file;

#[cfg(feature = "music-bridge.source.playerctl")]
pub mod playerctl;

#[allow(unused_imports)]
mod prelude {
    pub(super) use super::super::metadata::Metadata;
    pub(super) use super::super::pipeline::Sink;
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Config {
    #[cfg(feature = "music-bridge.source.external")]
    External(external::Config),

    #[cfg(feature = "music-bridge.source.file")]
    File(file::Config),

    #[cfg(feature = "music-bridge.source.playerctl")]
    Playerctl(playerctl::Config),
}

impl Config {
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
            #[cfg(feature = "music-bridge.source.external")]
            (Self::External(a), Self::External(b)) => a.merge_ref(b),
            #[cfg(feature = "music-bridge.source.external")]
            (Self::External(_), _) => Err(module::Error::collision()),

            #[cfg(feature = "music-bridge.source.file")]
            (Self::File(a), Self::File(b)) => a.merge_ref(b),
            #[cfg(feature = "music-bridge.source.file")]
            (Self::File(_), _) => Err(module::Error::collision()),

            #[cfg(feature = "music-bridge.source.playerctl")]
            (Self::Playerctl(a), Self::Playerctl(b)) => a.merge_ref(b),
            #[cfg(feature = "music-bridge.source.playerctl")]
            (Self::Playerctl(_), _) => Err(module::Error::collision()),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[allow(unused_imports, unused_variables)]
pub async fn run(config: &Config, sink: Sink<Metadata>) -> Result<()> {
    use eyre::Context;

    match config {
        #[cfg(feature = "music-bridge.source.external")]
        Config::External(x) => external::run(x, sink).await.context("external"),

        #[cfg(feature = "music-bridge.source.file")]
        Config::File(x) => file::run(x, sink).await.context("file"),

        #[cfg(feature = "music-bridge.source.playerctl")]
        Config::Playerctl(x) => playerctl::run(x, sink).await.context("playerctl"),

        #[allow(unreachable_patterns)]
        _ => unreachable!(),
    }
}
