use eyre::{Context, Result};
use serde::Deserialize;

mod rewrite;

use super::metadata::Record;
use super::pipeline;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Config {
    Rewrite(rewrite::Config),
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    match config {
        Config::Rewrite(x) => rewrite::setup(pipeline, x).await.context("rewrite"),
    }
}
