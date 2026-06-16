use std::cmp::min;
use std::time::{Duration, SystemTime};

use eyre::Result;
use serde::Deserialize;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub struct Config {}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, _config: &Config) -> Result<()> {
    pipeline.stage(TrackPositionBuilder {});

    Ok(())
}

///////////////////////////////////////////////////////////////////////////////

struct TrackPositionBuilder {}

impl StageBuilder<Record> for TrackPositionBuilder {
    type Stage = TrackPosition;

    fn build(self, sink: Sink<Record>, source: Source<Record>) -> Self::Stage {
        TrackPosition {
            current_track: None,
            source,
            sink,
        }
    }
}

struct TrackPosition {
    current_track: Option<TrackInfo>,
    source: Source<Record>,
    sink: Sink<Record>,
}

struct TrackInfo {
    id: Box<str>,
    playing: Option<Playing>,
    position: Duration,
}

struct Playing {
    since: SystemTime,
}

impl Stage<Record> for TrackPosition {
    async fn run(&mut self) -> Result<()> {
        loop {
            let Some(mut record) = self.source.pull().await else {
                return Ok(());
            };

            if self
                .current_track
                .as_ref()
                .is_none_or(|track| track.id.as_ref() != record.track_id.as_str())
            {
                self.current_track = Some(TrackInfo {
                    id: record.track_id.as_str().into(),
                    playing: None,
                    position: Duration::ZERO,
                });
            }

            let current_track = self
                .current_track
                .as_mut()
                .expect("should have been set above");

            match (record.status, &current_track.playing) {
                (TrackStatus::Playing, Some(Playing { since })) => {
                    let now = SystemTime::now();
                    current_track.position += now.duration_since(*since).unwrap_or_default();
                    current_track.playing = Some(Playing { since: now });
                }
                (TrackStatus::Playing, None) => {
                    let now = SystemTime::now();
                    current_track.playing = Some(Playing { since: now });
                }

                (TrackStatus::Paused | TrackStatus::Stopped, Some(Playing { since })) => {
                    let now = SystemTime::now();
                    current_track.position += now.duration_since(*since).unwrap_or_default();
                    current_track.playing = None;
                }
                (TrackStatus::Paused | TrackStatus::Stopped, None) => {}
            }

            if let Some(length) = record.length {
                record.position = Some(min(current_track.position, length));
            }

            if !self.sink.push(record) {
                return Ok(());
            }
        }
    }
}
