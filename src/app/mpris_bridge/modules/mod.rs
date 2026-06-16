use eyre::{Context, Result};
use serde::Deserialize;

mod fixup_track_id;
mod rewrite;

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
    Rewrite(Box<rewrite::Config>),
    FixupTrackId(fixup_track_id::Config),
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    if !config.enable {
        return Ok(());
    }

    use Module::*;
    match &config.module {
        Rewrite(x) => rewrite::setup(pipeline, x).await.context("rewrite"),
        FixupTrackId(x) => fixup_track_id::setup(pipeline, x)
            .await
            .context("fixup-track-id"),
    }
}
