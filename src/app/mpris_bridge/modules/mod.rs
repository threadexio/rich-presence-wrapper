use eyre::{Context, Result};
use serde::Deserialize;

mod auto_stop;
mod filter;
mod fixup_track_id;
mod rewrite;
mod script;
mod track_position;

mod prelude {
    pub(super) use super::super::metadata::*;
    pub(super) use super::super::pipeline::{self, Sink, Source, Stage, StageBuilder};
}

use super::metadata::Record;
use super::pipeline;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default = "crate::util::r#true")]
    enable: bool,

    #[serde(flatten)]
    module: Module,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
enum Module {
    AutoStop(auto_stop::Config),
    Filter(Box<filter::Config>),
    FixupTrackId(fixup_track_id::Config),
    Rewrite(Box<rewrite::Config>),
    Script(script::Config),
    TrackPosition(track_position::Config),
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    if !config.enable {
        return Ok(());
    }

    use Module::*;
    match &config.module {
        AutoStop(x) => auto_stop::setup(pipeline, x).await.context("auto-stop"),

        Filter(x) => filter::setup(pipeline, x).await.context("filter"),

        FixupTrackId(x) => fixup_track_id::setup(pipeline, x)
            .await
            .context("fixup-track-id"),

        Rewrite(x) => rewrite::setup(pipeline, x).await.context("rewrite"),

        Script(x) => script::setup(pipeline, x).await.context("script"),

        TrackPosition(x) => track_position::setup(pipeline, x)
            .await
            .context("track-position"),
    }
}
