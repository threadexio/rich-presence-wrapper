use eyre::{Context, Result};
use serde::Deserialize;

mod fixup_track_id;
mod rewrite;

use super::metadata::Record;
use super::pipeline;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Config {
    Rewrite(Box<rewrite::Config>),
    #[serde(rename = "fixup-track-id")]
    FixupTrackId(fixup_track_id::Config),
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    match config {
        Config::Rewrite(x) => rewrite::setup(pipeline, x).await.context("rewrite"),
        Config::FixupTrackId(x) => fixup_track_id::setup(pipeline, x)
            .await
            .context("fixup-track-id"),
    }
}
