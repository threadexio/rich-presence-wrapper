use eyre::Result;
use magic_args::{Extend, Mut, apply};
use serde::Deserialize;

use super::metadata::Metadata;
use super::pipeline::{Sink, Source};

///////////////////////////////////////////////////////////////////////////////

macro_rules! module {
    ($mod:ident if $cfg:meta) => {
        #[cfg($cfg)]
        mod $mod;

        #[cfg(not($cfg))]
        mod $mod {
            use eyre::{Result, bail};
            use serde::Deserialize;

            #[derive(Debug, Clone, Deserialize)]
            #[serde(rename_all = "kebab-case")]
            pub struct Config {}

            pub async fn run() -> Result<()> {
                bail!(
                    "module is not compiled-in for this build of {}. see: `--version`",
                    env!("CARGO_BIN_NAME")
                )
            }
        }
    };
}

module!(auto_stop if feature = "music-bridge.module.auto-stop");
module!(external if feature = "music-bridge.module.external");
module!(filter if feature = "music-bridge.module.filter");
module!(fixup_id if feature = "music-bridge.module.fixup-id");
module!(rewrite if feature = "music-bridge.module.rewrite");
module!(track_position if feature = "music-bridge.module.track-position");

#[allow(unused_imports)]
mod prelude {
    pub(super) use super::super::metadata::Metadata;
    pub(super) use super::super::pipeline::{Sink, Source};
    pub(super) use magic_args::Mut;
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Config {
    AutoStop(auto_stop::Config),
    External(external::Config),
    Filter(filter::Config),
    FixupId(fixup_id::Config),
    Rewrite(rewrite::Config),
    TrackPosition(track_position::Config),
}

impl Config {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::AutoStop(_) => "auto-stop",
            Self::External(_) => "external",
            Self::Filter(_) => "filter",
            Self::FixupId(_) => "fixup-id",
            Self::Rewrite(_) => "rewrite",
            Self::TrackPosition(_) => "track-position",
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, source: Source<Metadata>, sink: Sink<Metadata>) -> Result<()> {
    let all = (config, Mut::from(source), Mut::from(sink));

    match config {
        Config::AutoStop(x) => apply(auto_stop::run, all.extend(x)).await,
        Config::External(x) => apply(external::run, all.extend(x)).await,
        Config::Filter(x) => apply(filter::run, all.extend(x)).await,
        Config::FixupId(x) => apply(fixup_id::run, all.extend(x)).await,
        Config::Rewrite(x) => apply(rewrite::run, all.extend(x)).await,
        Config::TrackPosition(x) => apply(track_position::run, all.extend(x)).await,
    }
}
