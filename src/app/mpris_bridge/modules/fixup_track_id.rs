use eyre::Result;
use serde::Deserialize;

use super::super::metadata::Record;
use super::super::pipeline::{self, Sink, Source, Stage, StageBuilder};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub sensitivity: Sensitivity,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct Sensitivity {
    pub track_id: bool,
    pub title: bool,
    pub album: bool,
    pub artist: bool,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    pipeline.stage(FixupTrackIdBuilder {
        sensitivity: config.sensitivity.clone(),
    });

    Ok(())
}

///////////////////////////////////////////////////////////////////////////////

struct FixupTrackIdBuilder {
    sensitivity: Sensitivity,
}

impl StageBuilder<Record> for FixupTrackIdBuilder {
    type Stage = FixupTrackId;

    fn build(self, sink: Sink<Record>, source: Source<Record>) -> Self::Stage {
        let FixupTrackIdBuilder { sensitivity } = self;

        FixupTrackId {
            sensitivity,
            source,
            sink,
        }
    }
}

struct FixupTrackId {
    sensitivity: Sensitivity,
    source: Source<Record>,
    sink: Sink<Record>,
}

impl Stage<Record> for FixupTrackId {
    async fn run(&mut self) -> Result<()> {
        loop {
            let Some(mut record) = self.source.pull().await else {
                return Ok(());
            };

            let hash = fxhash::hash64(&(
                if self.sensitivity.track_id {
                    record.track_id.as_str()
                } else {
                    ""
                },
                if self.sensitivity.title {
                    record.title.as_deref().unwrap_or_default()
                } else {
                    ""
                },
                if self.sensitivity.album {
                    record.album.as_deref().unwrap_or_default()
                } else {
                    ""
                },
                if self.sensitivity.artist {
                    record.artist.as_deref().unwrap_or_default()
                } else {
                    ""
                },
            ));

            record.track_id = format!("{hash:016x}");

            if !self.sink.push(record) {
                return Ok(());
            }
        }
    }
}
