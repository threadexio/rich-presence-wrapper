use std::{ops::ControlFlow, time::Duration};

use eyre::Result;
use serde::Deserialize;

use crate::util::OneshotTimer;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    after_pause: Option<f32>,
    after_inactivity: Option<f32>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    pipeline.stage(AutoStopBuilder {
        after_pause: config.after_pause.map(Duration::from_secs_f32),
        after_inactivity: config.after_inactivity.map(Duration::from_secs_f32),
    });

    Ok(())
}

///////////////////////////////////////////////////////////////////////////////

struct AutoStopBuilder {
    after_pause: Option<Duration>,
    after_inactivity: Option<Duration>,
}

impl StageBuilder<Record> for AutoStopBuilder {
    type Stage = AutoStopStage;

    fn build(self, sink: Sink<Record>, source: Source<Record>) -> Self::Stage {
        fn timer(duration: Option<Duration>) -> OneshotTimer {
            let mut timer = OneshotTimer::new(duration.unwrap_or_default());
            if duration.is_some() {
                timer.restart();
            }

            timer
        }

        let Self {
            after_pause,
            after_inactivity,
        } = self;

        AutoStopStage {
            pause: timer(after_pause),
            inactivity: timer(after_inactivity),
            playing_track: None,

            source,
            sink,
        }
    }
}

struct AutoStopStage {
    pause: OneshotTimer,
    inactivity: OneshotTimer,
    playing_track: Option<TrackInfo>,

    source: Source<Record>,
    sink: Sink<Record>,
}

struct TrackInfo {
    player: String,
    id: String,
}

impl Stage<Record> for AutoStopStage {
    async fn run(&mut self) -> Result<()> {
        loop {
            let r = tokio::select! {
                r = self.source.pull() => {
                    let Some(record) = r else { return Ok(()); };
                    self.process_record(record)
                }

                () = self.pause.wait() => self.process_timer_expiration(),
                () = self.inactivity.wait() => self.process_timer_expiration(),
            };

            if r.is_break() {
                return Ok(());
            }
        }
    }
}

impl AutoStopStage {
    fn process_record(&mut self, record: Record) -> ControlFlow<()> {
        self.inactivity.restart();

        if record.status == TrackStatus::Stopped {
            self.playing_track = None;
            return self.emit(record);
        }

        if self
            .playing_track
            .as_ref()
            .is_none_or(|track| track.id != record.track_id)
        {
            self.playing_track = Some(TrackInfo {
                player: record.player.clone(),
                id: record.track_id.clone(),
            });
        }

        self.pause.restart();

        self.emit(record)
    }

    fn process_timer_expiration(&mut self) -> ControlFlow<()> {
        let Some(TrackInfo { player, id }) = self.playing_track.take() else {
            return ControlFlow::Continue(());
        };

        trace!("stopping...");

        self.emit(Record {
            player,
            track_id: id,
            status: TrackStatus::Stopped,
            title: None,
            album: None,
            artist: None,
            url: None,
            art_url: None,
            position: None,
            length: None,
        })
    }

    fn emit(&mut self, record: Record) -> ControlFlow<()> {
        if self.sink.push(record) {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}
