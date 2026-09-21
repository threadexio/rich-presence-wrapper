use std::cmp::min;
use std::time::{Duration, SystemTime};

use eyre::Result;
use serde::Deserialize;

use super::super::metadata::TrackStatus;
use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
pub struct Config {}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(source: Mut<Source>, sink: Mut<Sink>) -> Result<()> {
    TrackPosition {
        current_track: None,

        source: source.into_inner(),
        sink: sink.into_inner(),
    }
    .run()
    .await
}

struct TrackPosition {
    current_track: Option<TrackInfo>,

    source: Source,
    sink: Sink,
}

struct TrackInfo {
    id: Box<str>,
    playing: Option<Playing>,
    position: Duration,
}

struct Playing {
    since: SystemTime,
}

impl TrackPosition {
    async fn run(&mut self) -> Result<()> {
        loop {
            let Some(mut metadata) = self.source.recv().await else {
                return Ok(());
            };

            if self
                .current_track
                .as_ref()
                .is_none_or(|track| track.id.as_ref() != metadata.id.as_str())
            {
                self.current_track = Some(TrackInfo {
                    id: metadata.id.as_str().into(),
                    playing: None,
                    position: Duration::ZERO,
                });
            }

            let current_track = self
                .current_track
                .as_mut()
                .expect("should have been set above");

            match (metadata.status, &current_track.playing) {
                (TrackStatus::Playing, Some(Playing { since })) => {
                    let now = SystemTime::now();
                    current_track.position += now.duration_since(*since).unwrap_or_default();
                    current_track.playing = Some(Playing { since: now });
                }

                (TrackStatus::Playing, None) => {
                    let now = SystemTime::now();
                    current_track.playing = Some(Playing { since: now });
                }

                (_, Some(Playing { since })) => {
                    let now = SystemTime::now();
                    current_track.position += now.duration_since(*since).unwrap_or_default();
                    current_track.playing = None;
                }

                (_, None) => {}
            }

            metadata.position = Some(match metadata.length {
                Some(length) => min(current_track.position, length),
                None => current_track.position,
            });

            if self.sink.send(metadata).is_err() {
                return Ok(());
            }
        }
    }
}
