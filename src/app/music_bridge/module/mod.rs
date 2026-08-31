use eyre::Result;
use serde::Deserialize;

use super::metadata::Metadata;
use super::pipeline::{Sink, Source};

///////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "music-bridge.module.auto-stop")]
pub mod auto_stop;

#[cfg(feature = "music-bridge.module.external")]
pub mod external;

#[cfg(feature = "music-bridge.module.filter")]
pub mod filter;

#[cfg(feature = "music-bridge.module.fixup-id")]
pub mod fixup_id;

#[cfg(feature = "music-bridge.module.track-position")]
pub mod track_position;

#[allow(unused_imports)]
mod prelude {
    pub(super) use super::super::metadata::Metadata;
    pub(super) use super::super::pipeline::{Sink, Source};
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Config {
    #[cfg(feature = "music-bridge.module.auto-stop")]
    AutoStop(auto_stop::Config),

    #[cfg(feature = "music-bridge.module.external")]
    External(external::Config),

    #[cfg(feature = "music-bridge.module.filter")]
    Filter(filter::Config),

    #[cfg(feature = "music-bridge.module.fixup-id")]
    FixupId(fixup_id::Config),

    #[cfg(feature = "music-bridge.module.track-position")]
    TrackPosition(track_position::Config),
}

///////////////////////////////////////////////////////////////////////////////

#[allow(unused_imports, unused_variables)]
pub async fn run(config: &Config, source: Source<Metadata>, sink: Sink<Metadata>) -> Result<()> {
    use eyre::Context;

    match config {
        #[cfg(feature = "music-bridge.module.auto-stop")]
        Config::AutoStop(x) => auto_stop::run(x, source, sink).await.context("auto-stop"),

        #[cfg(feature = "music-bridge.module.external")]
        Config::External(x) => external::run(x, source, sink).await.context("external"),

        #[cfg(feature = "music-bridge.module.filter")]
        Config::Filter(x) => filter::run(x, source, sink).await.context("filter"),

        #[cfg(feature = "music-bridge.module.fixup-id")]
        Config::FixupId(x) => fixup_id::run(x, source, sink).await.context("fixup-id"),

        #[cfg(feature = "music-bridge.module.track-position")]
        Config::TrackPosition(x) => track_position::run(x, source, sink)
            .await
            .context("track-position"),

        #[allow(unreachable_patterns)]
        _ => unreachable!(),
    }
}
