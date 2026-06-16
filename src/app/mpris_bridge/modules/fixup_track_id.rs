use std::collections::HashSet;

use eyre::Result;
use serde::Deserialize;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    sensitivity: HashSet<Sensitivity>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sensitivity: HashSet::from_iter([
                Sensitivity::TrackId,
                Sensitivity::Title,
                Sensitivity::Album,
                Sensitivity::Artist,
            ]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Sensitivity {
    TrackId,
    Title,
    Album,
    Artist,
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
    sensitivity: HashSet<Sensitivity>,
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
    sensitivity: HashSet<Sensitivity>,
    source: Source<Record>,
    sink: Sink<Record>,
}

impl Stage<Record> for FixupTrackId {
    async fn run(&mut self) -> Result<()> {
        loop {
            let Some(mut record) = self.source.pull().await else {
                return Ok(());
            };

            record.track_id = format!("{:016x}", self.hash_record(&record));

            if !self.sink.push(record) {
                return Ok(());
            }
        }
    }
}

impl FixupTrackId {
    fn hash_record(&self, record: &Record) -> u64 {
        fxhash::hash64(&(
            if self.sensitivity.contains(&Sensitivity::TrackId) {
                record.track_id.as_str()
            } else {
                ""
            },
            if self.sensitivity.contains(&Sensitivity::Title)
                && let Some(x) = record.title.as_deref()
            {
                x
            } else {
                ""
            },
            if self.sensitivity.contains(&Sensitivity::Album)
                && let Some(x) = record.album.as_deref()
            {
                x
            } else {
                ""
            },
            if self.sensitivity.contains(&Sensitivity::Artist)
                && let Some(x) = record.artist.as_deref()
            {
                x
            } else {
                ""
            },
        ))
    }
}
