use std::env;
use std::path::PathBuf;
use std::process::{ExitCode, Stdio};
use std::time::{Duration, SystemTime};

use eyre::{Context, Result, bail};
use module::Merge;
use module::types::{Ordered, Overridable};
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::discord::*;
use crate::util::{SystemTimeExt, capitalize_words};

use self::metadata::{Record, TrackStatus};
use self::pipeline::{Sink, Source};

const CLIENT_ID: &str = "1485616471035088896";

mod metadata;
mod modules;
mod pipeline;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, clap::Parser)]
#[command(name = "mpris-bridge")]
pub struct Command {
    #[clap(
        long,
        help = "Specify which MPRIS players to bridge. Will be passed to `playerctl`'s `--player`."
    )]
    player: Option<String>,
}

#[derive(Debug, Default, Deserialize, Merge)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    playerctl: Option<Overridable<PathBuf>>,

    #[merge(rename = "client-id")]
    client_id: Option<Overridable<String>>,

    #[serde(rename = "module")]
    #[merge(rename = "module")]
    modules: Option<Ordered<Vec<modules::Config>>>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, command: &Command) -> Result<ExitCode> {
    let mut pipeline = pipeline::builder();

    if let Some(modules) = config.modules.as_deref() {
        for module in modules {
            modules::setup(&mut pipeline, module)
                .await
                .context("failed to setup module")?;
        }
    }

    let (mut tasks, input, output) = pipeline.build();

    tasks.spawn_local({
        let playerctl = env::var_os("_playerctl")
            .map(PathBuf::from)
            .or_else(|| config.playerctl.as_deref().cloned())
            .unwrap_or_else(|| PathBuf::from("playerctl"));
        let player = command.player.clone();

        async move { RecordReader::new(playerctl, player, input).run().await }
    });

    tasks.spawn_local({
        let discord = Discord::builder()
            .client_id(
                config
                    .client_id
                    .as_deref()
                    .map(String::as_str)
                    .unwrap_or(CLIENT_ID),
            )
            .finish();

        async move { RpcTask::new(discord, output).run().await }
    });

    let mut errored = false;
    while let Some(r) = tasks.join_next().await {
        let r = match r {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(e).context("failed to join task"),
        };

        if let Err(e) = r {
            errored = true;
            error!("{e:#}");
        }
    }

    Ok(if errored {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

///////////////////////////////////////////////////////////////////////////////

mod record_reader {
    use super::*;

    pub struct RecordReader {
        playerctl: PathBuf,
        player: Option<String>,

        sink: Sink<Record>,
    }

    impl RecordReader {
        pub fn new(playerctl: PathBuf, player: Option<String>, sink: Sink<Record>) -> Self {
            Self {
                playerctl,
                player,
                sink,
            }
        }

        pub async fn run(&mut self) -> Result<()> {
            debug!("using playerctl binary: '{}'", self.playerctl.display());

            let mut playerctl = tokio::process::Command::new(&self.playerctl);

            if let Some(player) = self.player.as_deref() {
                playerctl.args(["--player", player]);
            }

            let mut playerctl = playerctl
                .args(["metadata", "--follow", "--format", Record::PLAYERCTL_FORMAT])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .context("failed to spawn playerctl")?;

            let mut reader =
                BufReader::new(playerctl.stdout.take().expect("stdout was captured")).lines();

            trace!("waiting for output...");

            while let Some(line) = reader
                .next_line()
                .await
                .context("failed to read output of playerctl")?
            {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let record: Record = match serde_json::from_str(line)
                    .context("failed to parse a metadata record from playerctl")
                {
                    Ok(x) => x,
                    Err(e) => {
                        warn!("{e:#}");
                        continue;
                    }
                };

                trace!("-> {record:#?}");

                if !self.sink.push(record) {
                    break;
                }
            }

            bail!("playerctl died")
        }
    }
}
use self::record_reader::RecordReader;

///////////////////////////////////////////////////////////////////////////////

mod fixup_track_id {
    use super::*;

    pub struct FixupTrackId {
        source: Source<Record>,
        sink: Sink<Record>,
    }

    impl FixupTrackId {
        pub fn new(source: Source<Record>, sink: Sink<Record>) -> Self {
            Self { source, sink }
        }

        pub async fn run(&mut self) -> Result<()> {
            loop {
                let Some(mut record) = self.source.pull().await else {
                    return Ok(());
                };

                let id = fxhash::hash64(&(
                    &record.track_id,
                    &record.title,
                    &record.album,
                    &record.artist,
                ));

                record.track_id = format!("{id:016x}");

                if !self.sink.push(record) {
                    return Ok(());
                }
            }
        }
    }
}
use self::fixup_track_id::FixupTrackId;

///////////////////////////////////////////////////////////////////////////////

mod keep_track_position {
    use super::*;

    pub struct KeepTrackPosition {
        track: Option<TrackInfo>,

        source: Source<Record>,
        sink: Sink<Record>,
    }

    struct TrackInfo {
        id: String,
        playing_since: Option<SystemTime>,
        position: Duration,
    }

    impl KeepTrackPosition {
        pub fn new(source: Source<Record>, sink: Sink<Record>) -> Self {
            Self {
                track: None,

                source,
                sink,
            }
        }

        pub async fn run(&mut self) -> Result<()> {
            loop {
                let Some(mut record) = self.source.pull().await else {
                    return Ok(());
                };

                if self
                    .track
                    .as_ref()
                    .is_none_or(|track| track.id != record.track_id)
                {
                    self.track = Some(TrackInfo {
                        id: record.track_id.clone(),
                        playing_since: None,
                        position: Duration::ZERO,
                    });
                }

                let track = self.track.as_mut().expect("should have been set above");

                match (record.status, track.playing_since) {
                    (TrackStatus::Playing, None) => track.playing_since = Some(SystemTime::now()),
                    (TrackStatus::Playing, Some(since)) => {
                        let now = SystemTime::now();
                        track.position += now.duration_since(since).unwrap_or(Duration::ZERO);
                        track.playing_since = Some(now);
                    }

                    (_, Some(since)) => {
                        let now = SystemTime::now();
                        track.position += now.duration_since(since).unwrap_or(Duration::ZERO);
                        track.playing_since = None;
                    }
                    (_, None) => {}
                }

                if record.length.is_some() {
                    record.position = Some(track.position);
                }

                if !self.sink.push(record) {
                    return Ok(());
                }
            }
        }
    }
}
use self::keep_track_position::KeepTrackPosition;

///////////////////////////////////////////////////////////////////////////////

mod rpc_task {
    use super::*;

    pub struct RpcTask {
        discord: Discord,
        source: Source<Record>,
    }

    impl RpcTask {
        pub fn new(discord: Discord, source: Source<Record>) -> Self {
            Self { discord, source }
        }

        pub async fn run(&mut self) -> Result<()> {
            loop {
                let Some(record) = self.source.pull().await else {
                    return Ok(());
                };

                trace!("<- {record:#?}");

                match self.build_activity(record) {
                    Some(activity) => self
                        .discord
                        .set_activity(activity)
                        .await
                        .context("failed to set activity")?,

                    None => self
                        .discord
                        .clear_activity()
                        .await
                        .context("faled to clear activity")?,
                }
            }
        }

        fn build_activity(&self, record: Record) -> Option<Activity<'static>> {
            if record.status == TrackStatus::Stopped {
                return None;
            }

            let mut activity = Activity::new()
                .activity_type(ActivityType::Listening)
                .status_display_type(StatusDisplayType::Details)
                .name(capitalize_words(&record.player));

            if let Some(title) = record.title {
                activity = activity.details(title);
            }

            match (record.artist, record.album) {
                (Some(artist), Some(album)) => {
                    activity = activity.state(format!("{artist} • {album}"))
                }
                (Some(artist), None) => activity = activity.state(artist),
                (None, Some(album)) => activity = activity.state(album),
                (None, None) => {}
            }

            if let Some(position) = record.position
                && let Some(length) = record.length
            {
                let start = SystemTime::now() - position;
                let end = start + length;

                activity = activity.timestamps(
                    Timestamps::new()
                        .start(start.duration_since_epoch().as_secs() as i64)
                        .end(end.duration_since_epoch().as_secs() as i64),
                );
            }

            if let Some(url) = record.url {
                activity = activity.buttons(vec![Button::new("Listen", url)]);
            }

            if let Some(art_url) = record.art_url {
                activity = activity.assets(Assets::new().large_image(art_url));
            }

            Some(activity)
        }
    }
}
use self::rpc_task::RpcTask;
