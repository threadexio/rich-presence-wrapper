use std::collections::HashMap;
use std::future::pending;
use std::{ops::ControlFlow, time::Duration};

use eyre::Result;
use serde::Deserialize;

use crate::util::OneshotTimer;

use super::super::metadata::TrackStatus;
use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    after_pause: Option<f32>,
    after_inactivity: Option<f32>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, source: Source<Metadata>, sink: Sink<Metadata>) -> Result<()> {
    AutoStop {
        pause: config
            .after_pause
            .map(Duration::from_secs_f32)
            .map(OneshotTimer::new),

        inactivity: config
            .after_inactivity
            .map(Duration::from_secs_f32)
            .map(OneshotTimer::new),

        playing_track: None,

        source,
        sink,
    }
    .run()
    .await
}

struct AutoStop {
    pause: Option<OneshotTimer>,
    inactivity: Option<OneshotTimer>,
    playing_track: Option<TrackInfo>,

    source: Source<Metadata>,
    sink: Sink<Metadata>,
}

struct TrackInfo {
    player: String,
    id: String,
}

impl AutoStop {
    async fn run(&mut self) -> Result<()> {
        if let Some(timer) = self.inactivity.as_mut() {
            timer.restart();
        }

        loop {
            let r = tokio::select! {
                r = self.source.pull() => {
                    let Some(metadata) = r else { return Ok(()); };
                    self.process_metadata(metadata)
                }

                () = wait_if_some(&mut self.pause) => {
                    trace!("pause timer expired. stopping...");
                    self.do_stop()
                }

                () = wait_if_some(&mut self.inactivity) => {
                    trace!("inactivity timer expired. stopping...");
                    self.do_stop()
                },
            };

            if r.is_break() {
                return Ok(());
            }
        }
    }

    fn process_metadata(&mut self, metadata: Metadata) -> ControlFlow<()> {
        if let Some(timer) = self.inactivity.as_mut() {
            timer.restart();
        }

        match metadata.status {
            TrackStatus::Stopped => {
                self.playing_track = None;
                self.emit(metadata)
            }

            status => {
                if self
                    .playing_track
                    .as_ref()
                    .is_none_or(|track| track.id != metadata.id)
                {
                    self.playing_track = Some(TrackInfo {
                        player: metadata.player.clone(),
                        id: metadata.id.clone(),
                    });
                }

                if let Some(timer) = self.pause.as_mut() {
                    if matches!(status, TrackStatus::Paused) {
                        timer.restart();
                    } else {
                        timer.stop();
                    }
                }

                self.emit(metadata)
            }
        }
    }

    fn do_stop(&mut self) -> ControlFlow<()> {
        let Some(TrackInfo { player, id }) = self.playing_track.take() else {
            return ControlFlow::Continue(());
        };

        self.emit(Metadata {
            player,
            id,
            status: TrackStatus::Stopped,
            title: None,
            album: None,
            artist: None,
            url: None,
            art_url: None,
            position: None,
            length: None,
            extra: HashMap::new(),
        })
    }

    fn emit(&mut self, metadata: Metadata) -> ControlFlow<()> {
        if self.sink.push(metadata) {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}

async fn wait_if_some(timer: &mut Option<OneshotTimer>) {
    match timer.as_mut() {
        Some(x) => x.wait().await,
        None => pending().await,
    }
}
