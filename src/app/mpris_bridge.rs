use std::borrow::Cow;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::process::{ExitCode, Stdio};
use std::time::{Duration, Instant, SystemTime};
use std::{env, fmt};

use eyre::{Context, Result, bail};
use html_escape::decode_html_entities;
use module::Merge;
use module::types::Overridable;
use regex::Regex;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::config::Config;
use crate::discord::*;
use crate::util::{OneshotTimer, SystemTimeExt, capitalize_words};

const CLIENT_ID: &str = "1485616471035088896";

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
pub struct File {
    playerctl: Option<Overridable<PathBuf>>,

    #[merge(rename = "client-id")]
    client_id: Option<Overridable<String>>,

    filter: MetadataFilters,

    #[merge(rename = "auto-stop-delay")]
    auto_stop_delay: Option<Overridable<u64>>,
}

#[derive(Debug, Default, Clone, Deserialize, Merge)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MetadataFilters {
    player: Option<Overridable<MetadataFilter>>,
    status: Option<Overridable<MetadataFilter>>,
    track_id: Option<Overridable<MetadataFilter>>,
    title: Option<Overridable<MetadataFilter>>,
    album: Option<Overridable<MetadataFilter>>,
    artist: Option<Overridable<MetadataFilter>>,
    url: Option<Overridable<MetadataFilter>>,
    art_url: Option<Overridable<MetadataFilter>>,
}

#[derive(Debug, Clone, Deserialize, Merge)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MetadataFilter {
    r#match: Overridable<String>,

    #[serde(default)]
    invert: Overridable<bool>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, command: &Command) -> Result<ExitCode> {
    let filters = try2!({
        fn f(x: &Option<Overridable<MetadataFilter>>) -> Result<Option<transformer::Filter>> {
            match x {
                None => Ok(None),
                Some(x) => Ok(Some(transformer::Filter {
                    pattern: Regex::new(&x.r#match)?,
                    invert: *x.invert,
                })),
            }
        }

        let MetadataFilters {
            player,
            status,
            track_id,
            title,
            album,
            artist,
            url,
            art_url,
        } = &config.mpris_bridge.filter;

        Result::<_>::Ok(transformer::FilterSet {
            player: f(player).context("player")?,
            status: f(status).context("status")?,
            track_id: f(track_id).context("track_id")?,
            title: f(title).context("title")?,
            album: f(album).context("album")?,
            artist: f(artist).context("artist")?,
            url: f(url).context("url")?,
            art_url: f(art_url).context("art_url")?,
        })
    })
    .context("failed to compile filter")?;

    let (metadata_tx, metadata_rx) = watch::channel(None);
    let (rpc_tx, rpc_rx) = watch::channel(None);

    let mut tasks = JoinSet::new();

    tasks.spawn_local({
        let playerctl = env::var_os("_playerctl")
            .map(PathBuf::from)
            .or_else(|| config.mpris_bridge.playerctl.as_deref().cloned())
            .unwrap_or_else(|| PathBuf::from("playerctl"));
        let player = command.player.clone();

        async move {
            metadata::Task::new(metadata_tx, playerctl, player)
                .run()
                .await
        }
    });

    tasks.spawn_local({
        let auto_stop_delay = config
            .mpris_bridge
            .auto_stop_delay
            .as_deref()
            .copied()
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(10));

        async move {
            transformer::Task::new(metadata_rx, rpc_tx, filters, auto_stop_delay)
                .run()
                .await
        }
    });

    tasks.spawn_local({
        let discord = Discord::builder()
            .client_id(
                config
                    .mpris_bridge
                    .client_id
                    .as_deref()
                    .map(String::as_str)
                    .unwrap_or(CLIENT_ID),
            )
            .finish();

        async move { rpc::Task::new(rpc_rx, discord).run().await }
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

#[derive(Debug, Clone, Deserialize)]
struct MetadataRecord {
    player: String,
    status: TrackStatus,
    track_id: String,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    title: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    album: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    artist: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    url: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    art_url: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty_us")]
    position: Option<Duration>,
    #[serde(deserialize_with = "deserialize::none_if_empty_us")]
    length: Option<Duration>,
}

impl MetadataRecord {
    pub const PLAYERCTL_FORMAT: &str = concat!(
        r#"{"#,
        r#""player":"{{markup_escape(playerName)}}","#,
        r#""status":"{{lc(status)}}","#,
        r#""track_id":"{{markup_escape(mpris:trackid)}}","#,
        r#""title":"{{markup_escape(xesam:title)}}","#,
        r#""album":"{{markup_escape(xesam:album)}}","#,
        r#""artist":"{{markup_escape(xesam:artist)}}","#,
        r#""url":"{{markup_escape(xesam:url)}}","#,
        r#""art_url":"{{markup_escape(mpris:artUrl)}}","#,
        r#""position":"{{position}}","#,
        r#""length":"{{mpris:length}}""#,
        r#"}"#,
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TrackStatus {
    Playing,
    Paused,
    Stopped,
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Clone, PartialEq, Eq)]
struct TrackFingerprint(u64);

impl fmt::Display for TrackFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl TrackFingerprint {
    fn take(record: &MetadataRecord) -> Self {
        Self(fxhash::hash64(&(
            &record.track_id,
            &record.title,
            &record.album,
            &record.artist,
        )))
    }
}

///////////////////////////////////////////////////////////////////////////////

mod metadata {
    use super::*;

    pub struct Task {
        tx: watch::Sender<Option<MetadataRecord>>,
        playerctl: PathBuf,
        player: Option<String>,
    }

    impl Task {
        pub fn new(
            tx: watch::Sender<Option<MetadataRecord>>,
            playerctl: PathBuf,
            player: Option<String>,
        ) -> Self {
            Self {
                tx,
                playerctl,
                player,
            }
        }

        pub async fn run(&mut self) -> Result<()> {
            debug!("using playerctl binary: '{}'", self.playerctl.display());

            let mut playerctl = tokio::process::Command::new(&self.playerctl);

            if let Some(player) = self.player.as_deref() {
                playerctl.args(["--player", player]);
            }

            let mut playerctl = playerctl
                .args([
                    "metadata",
                    "--follow",
                    "--format",
                    MetadataRecord::PLAYERCTL_FORMAT,
                ])
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

                let record: MetadataRecord = match serde_json::from_str(line)
                    .context("failed to parse a metadata record from playerctl")
                {
                    Ok(x) => x,
                    Err(e) => {
                        warn!("{e:#}");
                        continue;
                    }
                };

                trace!("-> {record:#?}");

                if self.tx.send(Some(record)).is_err() {
                    break;
                }
            }

            bail!("playerctl died")
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

mod transformer {
    use super::*;

    pub struct Task {
        source: watch::Receiver<Option<MetadataRecord>>,
        sink: watch::Sender<Option<MetadataRecord>>,
        current_track: Option<TrackInfo>,
        filters: FilterSet,
        auto_stop_delay: Duration,
    }

    struct TrackInfo {
        fingerprint: TrackFingerprint,
        state: TrackState,
        position: Duration,

        id: String,
        player: String,
        title: Option<String>,
        album: Option<String>,
        artist: Option<String>,
    }

    enum TrackState {
        Playing { since: Instant },
        Paused,
        Stopped,
    }

    impl Task {
        pub fn new(
            source: watch::Receiver<Option<MetadataRecord>>,
            sink: watch::Sender<Option<MetadataRecord>>,
            filters: FilterSet,
            auto_stop_delay: Duration,
        ) -> Self {
            Self {
                source,
                sink,
                current_track: None,
                filters,
                auto_stop_delay,
            }
        }

        pub async fn run(&mut self) -> Result<()> {
            let mut stop_timer = OneshotTimer::new(self.auto_stop_delay);
            stop_timer.restart();

            loop {
                tokio::select! {
                    _ = self.source.changed() => {
                        let Some(record) = self.source.borrow_and_update().clone() else {
                            continue;
                        };

                        if record.status == TrackStatus::Playing {
                            stop_timer.restart();
                        }

                        if !self.filters.matches(&record) {
                            continue;
                        }

                        if self.handle_record(record).is_break() {
                            return Ok(());
                        }
                    }

                    _ = stop_timer.wait() => {
                        if self.handle_stop_timer().is_break() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        fn handle_record(&mut self, mut record: MetadataRecord) -> ControlFlow<()> {
            let now = Instant::now();
            let fingerprint = TrackFingerprint::take(&record);

            if self
                .current_track
                .as_ref()
                .is_none_or(|track| track.fingerprint != fingerprint)
            {
                self.current_track = Some(TrackInfo {
                    fingerprint: fingerprint.clone(),
                    state: match record.status {
                        TrackStatus::Playing => TrackState::Playing { since: now },
                        TrackStatus::Paused => TrackState::Paused,
                        TrackStatus::Stopped => TrackState::Stopped,
                    },
                    position: Duration::ZERO,

                    id: record.track_id.clone(),
                    player: record.player.clone(),
                    title: record.title.clone(),
                    album: record.album.clone(),
                    artist: record.artist.clone(),
                });

                debug!("new track");
            }

            let Some(current_track) = self.current_track.as_mut() else {
                panic!("should have been set above")
            };

            current_track.player = record.player.clone();

            if let TrackState::Playing { since } = &current_track.state {
                current_track.position += now - *since;
                current_track.state = TrackState::Playing { since: now };
            }

            match (&current_track.state, record.status) {
                (TrackState::Playing { .. }, TrackStatus::Playing) => {}
                (TrackState::Playing { .. }, TrackStatus::Paused) => {
                    current_track.state = TrackState::Paused
                }
                (TrackState::Playing { .. }, TrackStatus::Stopped) => {
                    current_track.state = TrackState::Stopped;
                }

                (TrackState::Paused | TrackState::Stopped, TrackStatus::Playing) => {
                    current_track.state = TrackState::Playing { since: now };
                }

                (TrackState::Paused, TrackStatus::Paused) => {}
                (TrackState::Paused, TrackStatus::Stopped) => {
                    current_track.state = TrackState::Stopped;
                }

                (TrackState::Stopped, TrackStatus::Paused) => return ControlFlow::Continue(()),
                (TrackState::Stopped, TrackStatus::Stopped) => return ControlFlow::Continue(()),
            }

            // Some buggy implementations (`plasma-browser-integration`) return
            // a static `track_id`. We rewrite it here to make sure it is
            // actually unique to the track playing.
            record.track_id = format!("{fingerprint}");

            record.position = Some(current_track.position);

            match self.sink.send(Some(record)) {
                Ok(()) => ControlFlow::Continue(()),
                Err(_) => ControlFlow::Break(()),
            }
        }

        fn handle_stop_timer(&mut self) -> ControlFlow<()> {
            let Some(track) = self.current_track.as_ref() else {
                return ControlFlow::Continue(());
            };

            debug!("auto-pausing...");

            let player = track.player.clone();
            let track_id = track.id.clone();
            let title = track.title.clone();
            let album = track.album.clone();
            let artist = track.artist.clone();

            self.handle_record(MetadataRecord {
                player,
                status: TrackStatus::Stopped,
                track_id,
                title,
                album,
                artist,
                url: None,
                art_url: None,
                position: None,
                length: None,
            })
        }
    }

    trait Matches<T: ?Sized> {
        fn matches(&self, value: &T) -> bool;
    }

    pub struct Filter {
        pub pattern: Regex,
        pub invert: bool,
    }

    impl Matches<str> for Filter {
        fn matches(&self, value: &str) -> bool {
            let r = self.pattern.is_match(value);
            if self.invert { !r } else { r }
        }
    }

    impl Matches<String> for Filter {
        fn matches(&self, value: &String) -> bool {
            Matches::<str>::matches(self, value.as_str())
        }
    }

    impl Matches<TrackStatus> for Filter {
        fn matches(&self, value: &TrackStatus) -> bool {
            match value {
                TrackStatus::Playing => self.matches("playing"),
                TrackStatus::Paused => self.matches("paused"),
                TrackStatus::Stopped => self.matches("stopped"),
            }
        }
    }

    impl<T> Matches<Option<T>> for Filter
    where
        Filter: Matches<T>,
    {
        fn matches(&self, value: &Option<T>) -> bool {
            match value {
                Some(x) => self.matches(x),
                None => false,
            }
        }
    }

    impl<T: ?Sized> Matches<T> for Option<Filter>
    where
        Filter: Matches<T>,
    {
        fn matches(&self, value: &T) -> bool {
            match self {
                Some(x) => x.matches(value),
                None => true,
            }
        }
    }

    pub struct FilterSet {
        pub player: Option<Filter>,
        pub status: Option<Filter>,
        pub track_id: Option<Filter>,
        pub title: Option<Filter>,
        pub album: Option<Filter>,
        pub artist: Option<Filter>,
        pub url: Option<Filter>,
        pub art_url: Option<Filter>,
    }

    impl Matches<MetadataRecord> for FilterSet {
        fn matches(&self, record: &MetadataRecord) -> bool {
            let Self {
                player,
                status,
                track_id,
                title,
                album,
                artist,
                url,
                art_url,
            } = self;

            player.matches(&record.player)
                && status.matches(&record.status)
                && track_id.matches(&record.track_id)
                && title.matches(&record.title)
                && album.matches(&record.album)
                && artist.matches(&record.album)
                && url.matches(&record.url)
                && art_url.matches(&record.art_url)
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

mod rpc {
    use super::*;

    pub struct Task {
        metadata: watch::Receiver<Option<MetadataRecord>>,
        discord: Discord,
    }

    impl Task {
        pub fn new(metadata: watch::Receiver<Option<MetadataRecord>>, discord: Discord) -> Self {
            Self { metadata, discord }
        }

        pub async fn run(&mut self) -> Result<()> {
            loop {
                if self.metadata.changed().await.is_err() {
                    break;
                }

                let record = self.metadata.borrow_and_update();
                let record = match &*record {
                    Some(x) => x,
                    None => continue,
                };

                trace!("<- {record:#?}");

                match build_activity(record) {
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

            Ok(())
        }
    }

    fn build_activity(record: &MetadataRecord) -> Option<Activity<'static>> {
        if record.status == TrackStatus::Stopped {
            return None;
        }

        let mut activity = Activity::new()
            .activity_type(ActivityType::Listening)
            .status_display_type(StatusDisplayType::Details)
            .name(capitalize_words(&record.player));

        if let Some(title) = record.title.clone() {
            activity = activity.details(title);
        }

        match (record.artist.as_ref(), record.album.as_ref()) {
            (Some(artist), Some(album)) => activity = activity.state(format!("{artist} • {album}")),
            (Some(artist), None) => activity = activity.state(artist.clone()),
            (None, Some(album)) => activity = activity.state(album.clone()),
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

        if let Some(url) = record.url.clone() {
            activity = activity.buttons(vec![Button::new("Listen", url)]);
        }

        if let Some(art_url) = record.art_url.clone() {
            activity = activity.assets(Assets::new().large_image(art_url));
        }

        Some(activity)
    }
}

///////////////////////////////////////////////////////////////////////////////

mod deserialize {
    use super::*;

    pub fn none_if_empty<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = Cow::<'de, str>::deserialize(deserializer)?;
        let s = decode_html_entities(&s);

        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s.into_owned()))
        }
    }

    pub fn none_if_empty_us<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = Cow::<'de, str>::deserialize(deserializer)?;
        let s = decode_html_entities(&s);

        if s.is_empty() {
            return Ok(None);
        }

        let us = s.parse().map_err(<D::Error as serde::de::Error>::custom)?;
        Ok(Some(Duration::from_micros(us)))
    }
}
