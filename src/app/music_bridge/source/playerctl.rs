use std::collections::HashMap;
use std::env;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use eyre::{Context, ContextCompat, Result, bail};
use module::{Merge, types::Overridable};
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::super::metadata::TrackStatus;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Clone, Deserialize, Merge)]
pub struct Config {
    pub command: Option<Overridable<PathBuf>>,
    pub player: Option<Overridable<String>>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config, mut sink: Sink<Metadata>) -> Result<()> {
    let mut playerctl = Command::new(
        None.or_else(|| env::var_os("_playerctl").map(PathBuf::from))
            .or_else(|| config.command.as_deref().cloned())
            .unwrap_or_else(|| PathBuf::from("playerctl")),
    );

    if let Some(player) = config.player.as_deref() {
        playerctl.args(["--player", player]);
    }

    let mut playerctl = playerctl
        .args([
            "metadata",
            "--follow",
            "--format",
            PLAYERCTL_METADATA_FORMAT,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn playerctl")?;

    let mut reader = playerctl
        .stdout
        .take()
        .map(BufReader::new)
        .expect("stdout was captured")
        .lines();

    while let Some(line) = reader
        .next_line()
        .await
        .context("failed to read metadata")?
    {
        if line.is_empty() {
            continue;
        }

        debug!("playerctl metadata line: {line:?}");

        let metadata = parse_metadata_line(&line).context("failed to parse metadata")?;
        debug!("playerctl parsed metadata: {metadata:#?}");

        if !sink.push(metadata) {
            break;
        }
    }

    Ok(())
}

const PLAYERCTL_METADATA_FORMAT: &str = concat!(
    "\x02",
    "{{playerName}}",
    "\t",
    "{{mpris:trackid}}",
    "\t",
    "{{lc(status)}}",
    "\t",
    "{{xesam:title}}",
    "\t",
    "{{xesam:album}}",
    "\t",
    "{{xesam:artist}}",
    "\t",
    "{{xesam:url}}",
    "\t",
    "{{mpris:artUrl}}",
    "\t",
    "{{position}}",
    "\t",
    "{{mpris:length}}",
    "\x03"
);

fn parse_metadata_line(line: &str) -> Result<Metadata> {
    let Some(line) = line.strip_prefix("\x02") else {
        bail!("invalid format")
    };

    let Some(line) = line.strip_suffix("\x03") else {
        bail!("invalid format")
    };

    fn none_if_empty_str(s: &str) -> Option<&str> {
        (!s.is_empty()).then_some(s)
    }

    let mut fields = line.split('\t');

    let player = fields
        .next()
        .map(ToOwned::to_owned)
        .context("missing 'player' field")?;

    let trackid = fields
        .next()
        .map(none_if_empty_str)
        .context("missing 'trackid' field")?;

    let status =
        fields
            .next()
            .context("missing 'status' field")
            .and_then(|status| match status {
                "playing" => Ok(TrackStatus::Playing),
                "paused" => Ok(TrackStatus::Paused),
                "stopped" => Ok(TrackStatus::Stopped),
                _ => bail!("invalid 'status': '{status}'"),
            })?;

    let title = fields
        .next()
        .map(|s| none_if_empty_str(s).map(ToOwned::to_owned))
        .context("missing 'title' field")?;

    let album = fields
        .next()
        .map(|s| none_if_empty_str(s).map(ToOwned::to_owned))
        .context("missing 'album' field")?;

    let artist = fields
        .next()
        .map(|s| none_if_empty_str(s).map(ToOwned::to_owned))
        .context("missing 'artist' field")?;

    let url = fields
        .next()
        .map(|s| none_if_empty_str(s).map(ToOwned::to_owned))
        .context("missing 'url' field")?;

    let art_url = fields
        .next()
        .map(|s| none_if_empty_str(s).map(ToOwned::to_owned))
        .context("missing 'artUrl' field")?;

    let position = fields
        .next()
        .context("missing 'position' field")
        .and_then(|s| {
            none_if_empty_str(s)
                .map(|s| {
                    s.parse()
                        .map(Duration::from_micros)
                        .context("failed to parse 'position' field")
                })
                .transpose()
        })?;

    let length = fields
        .next()
        .context("missing 'length' field")
        .and_then(|s| {
            none_if_empty_str(s)
                .map(|s| {
                    s.parse()
                        .map(Duration::from_micros)
                        .context("failed to parse 'length' field")
                })
                .transpose()
        })?;

    let id = {
        let mut hasher = DefaultHasher::new();
        (&trackid, &title, &album, &artist).hash(&mut hasher);
        let hash = hasher.finish();
        format!("{hash:016x}")
    };

    Ok(Metadata {
        player,

        id,
        status,

        title,
        album,
        artist,

        url,
        art_url,

        position,
        length,

        extra: HashMap::new(),
    })
}
