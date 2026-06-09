use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::{ExitCode, Stdio};
use std::time::SystemTime;

use eyre::{Context, Result};
use module::Merge;
use module::types::Overridable;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::config::Config;
use crate::discord::*;
use crate::util::SystemTimeExt;
use crate::util::exit_status_to_code;

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

    filter: Option<HashMap<String, Overridable<MetadataFilter>>>,
}

#[derive(Debug, Clone, Deserialize, Merge)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MetadataFilter {
    #[serde(deserialize_with = "deserialize_overridable_regex")]
    r#match: Overridable<Regex>,

    #[serde(default)]
    invert: Overridable<bool>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, command: &Command) -> Result<ExitCode> {
    let filters: HashMap<String, MetadataFilter> = config
        .mpris_bridge
        .filter
        .as_ref()
        .map(|x| x.iter().map(|(k, v)| (k.clone(), (**v).clone())).collect())
        .unwrap_or_default();

    let mut playerctl = tokio::process::Command::new(
        env::var_os("_playerctl")
            .map(PathBuf::from)
            .or_else(|| config.mpris_bridge.playerctl.as_deref().cloned())
            .unwrap_or_else(|| PathBuf::from("playerctl")),
    );

    if let Some(player) = command.player.as_deref() {
        playerctl.args(["--player", player]);
    }

    let mut playerctl = playerctl
        .args([
            "metadata",
            "--follow",
            "--format",
            metadata::PLAYERCTL_FORMAT,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn playerctl")?;

    let mut reader = BufReader::new(playerctl.stdout.take().expect("stdout was captured")).lines();

    let mut discord = Discord::builder()
        .client_id(
            config
                .mpris_bridge
                .client_id
                .as_deref()
                .map(String::as_str)
                .unwrap_or(CLIENT_ID),
        )
        .finish();

    let mut last_track_id = None;
    let mut track_change = None;

    while let Some(line) = reader
        .next_line()
        .await
        .context("failed to read output of playerctl")?
    {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let record: metadata::Record = match serde_json::from_str(line)
            .context("failed to parse a metadata record from playerctl")
        {
            Ok(x) => x,
            Err(e) => {
                warn!("{e:#}");
                continue;
            }
        };

        if !record_matches(&record, &filters) {
            last_track_id = None;
            track_change = None;

            discord.clear_activity().await?;
            continue;
        }

        if last_track_id
            .as_deref()
            .is_none_or(|x| x != record.track_id)
        {
            last_track_id = Some(record.track_id);
            track_change = Some(SystemTime::now());
        }

        let mut activity = Activity::new()
            .activity_type(ActivityType::Listening)
            .status_display_type(StatusDisplayType::Details)
            .name(capitalize_words(&record.player));

        if let Some(title) = record.title {
            activity = activity.details(title);
        }

        match (record.artist, record.album) {
            (Some(artist), Some(album)) => activity = activity.state(format!("{artist} • {album}")),
            (Some(artist), None) => activity = activity.state(artist),
            (None, Some(album)) => activity = activity.state(album),
            (None, None) => {}
        }

        if let Some(length) = record.length
            && record.status == metadata::Status::Playing
        {
            let now = SystemTime::now().duration_since_epoch();

            let start = match record.position {
                Some(position) => now.saturating_sub(position),
                None => track_change
                    .expect("track_change should have been set")
                    .duration_since_epoch(),
            };

            let end = start.saturating_add(length);

            activity = activity.timestamps(
                Timestamps::new()
                    .start(start.as_secs() as i64)
                    .end(end.as_secs_f64().ceil() as i64),
            );
        }

        if let Some(art_url) = record.art_url {
            activity = activity.assets(Assets::new().large_image(art_url));
        }

        discord.set_activity(activity).await?;
    }

    Ok(exit_status_to_code(playerctl.wait().await?))
}

mod metadata {
    use std::time::Duration;

    use html_escape::decode_html_entities;

    use super::*;

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

    #[derive(Debug, Deserialize)]
    pub struct Record {
        pub player: String,
        pub status: Status,
        pub track_id: String,
        #[serde(deserialize_with = "none_if_empty")]
        pub title: Option<String>,
        #[serde(deserialize_with = "none_if_empty")]
        pub album: Option<String>,
        #[serde(deserialize_with = "none_if_empty")]
        pub artist: Option<String>,
        #[serde(deserialize_with = "none_if_empty")]
        pub url: Option<String>,
        #[serde(deserialize_with = "none_if_empty")]
        pub art_url: Option<String>,
        #[serde(deserialize_with = "none_if_empty_str_us")]
        pub position: Option<Duration>,
        #[serde(deserialize_with = "none_if_empty_str_us")]
        pub length: Option<Duration>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Status {
        Playing,
        Paused,
        Stopped,
    }

    fn none_if_empty<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
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

    fn none_if_empty_str_us<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
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

fn record_matches(record: &metadata::Record, filters: &HashMap<String, MetadataFilter>) -> bool {
    let metadata::Record {
        player,
        status,
        track_id,
        title,
        album,
        artist,
        url,
        art_url,
        position,
        length,
    } = record;

    // We can't match these.
    let _ = (position, length);

    let match_str = |name: &str, value: &str| -> bool {
        filters
            .get(name)
            .map(|filter| metadata_filter_matches(filter, value))
            .unwrap_or(true)
    };

    let match_optional_str = |name: &str, value: Option<&str>| -> bool {
        match (value, filters.get(name)) {
            (Some(value), Some(filter)) => metadata_filter_matches(filter, value),
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true,
        }
    };

    match_str("player", player)
        && match_str(
            "status",
            match status {
                metadata::Status::Playing => "playing",
                metadata::Status::Paused => "paused",
                metadata::Status::Stopped => "stopped",
            },
        )
        && match_str("track_id", track_id)
        && match_optional_str("title", title.as_deref())
        && match_optional_str("album", album.as_deref())
        && match_optional_str("artist", artist.as_deref())
        && match_optional_str("url", url.as_deref())
        && match_optional_str("art_url", art_url.as_deref())
}

fn metadata_filter_matches(filter: &MetadataFilter, value: &str) -> bool {
    let MetadataFilter {
        ref r#match,
        invert,
    } = *filter;

    let matches = r#match.is_match(value);

    match invert.into_value() {
        false => matches,
        true => !matches,
    }
}

fn capitalize_words(s: &str) -> String {
    let s = s.trim();

    let mut out = String::with_capacity(s.len());

    let mut last: Option<char> = None;

    for c in s.chars() {
        match (last, c) {
            (Some(last), c) => match (last.is_whitespace(), c.is_whitespace()) {
                (false, _) => out.push(c),
                (true, false) => out.extend(c.to_uppercase()),
                (true, true) => {}
            },

            (None, c) => out.extend(c.to_uppercase()),
        }

        last = Some(c);
    }

    out
}

fn deserialize_overridable_regex<'de, D>(deserializer: D) -> Result<Overridable<Regex>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Overridable::<Cow<'de, str>>::deserialize(deserializer)?;

    Regex::new(&value)
        .map(|x| Overridable::with_priority(x, value.priority()))
        .map_err(serde::de::Error::custom)
}
