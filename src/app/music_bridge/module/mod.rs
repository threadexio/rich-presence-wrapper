use eyre::{Context, Result};
use serde::Deserialize;

use super::metadata::Metadata;
use super::pipeline::{Sink, Source};

///////////////////////////////////////////////////////////////////////////////

pub mod auto_stop;
pub mod external;
pub mod fixup_id;
pub mod track_position;

mod prelude {
    pub(super) use super::super::metadata::Metadata;
    pub(super) use super::super::pipeline::{Sink, Source};
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Config {
    AutoStop(auto_stop::Config),
    External(external::Config),
    FixupId(fixup_id::Config),
    TrackPosition(track_position::Config),
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, source: Source<Metadata>, sink: Sink<Metadata>) -> Result<()> {
    match config {
        Config::AutoStop(x) => auto_stop::run(x, source, sink).await.context("auto-stop"),

        Config::External(x) => external::run(x, source, sink).await.context("external"),

        Config::FixupId(x) => fixup_id::run(x, source, sink).await.context("fixup-id"),

        Config::TrackPosition(x) => track_position::run(x, source, sink)
            .await
            .context("track-position"),
    }
}
