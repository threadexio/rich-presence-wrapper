use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::process::Stdio;
use std::time::SystemTime;

use eyre::Context;
use eyre::Result;
use module::Merge;
use module::types::Overridable;
use serde::Deserialize;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

use crate::config::Config;
use crate::config::cli;
use crate::discord::*;
use crate::util::SystemTimeExt;
use crate::util::exit_status_to_code;

const CLIENT_ID: &str = "1485616471035088896";

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
}

pub async fn run(config: Config) -> Result<ExitCode> {
    let cli::Command::MprisBridge(ref command) = config.command else {
        unreachable!()
    };

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
                eprintln!("warn: {e:#}");
                continue;
            }
        };

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
        r#""title":"{{markup_escape(title)}}","#,
        r#""album":"{{markup_escape(album)}}","#,
        r#""artist":"{{markup_escape(artist)}}","#,
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
        let s = <&'de str>::deserialize(deserializer)?;
        let s = decode_html_entities(s);

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
        let s = <&'de str>::deserialize(deserializer)?;
        let s = decode_html_entities(s);

        if s.is_empty() {
            return Ok(None);
        }

        let us = s.parse().map_err(<D::Error as serde::de::Error>::custom)?;
        Ok(Some(Duration::from_micros(us)))
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
