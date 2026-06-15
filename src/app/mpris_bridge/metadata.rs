use std::borrow::Cow;
use std::time::Duration;

use html_escape::decode_html_entities;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Record {
    pub player: String,
    pub status: TrackStatus,
    pub track_id: String,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    pub title: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    pub album: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    pub artist: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    pub url: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty")]
    pub art_url: Option<String>,
    #[serde(deserialize_with = "deserialize::none_if_empty_us")]
    pub position: Option<Duration>,
    #[serde(deserialize_with = "deserialize::none_if_empty_us")]
    pub length: Option<Duration>,
}

impl Record {
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
pub enum TrackStatus {
    Playing,
    Paused,
    Stopped,
}

impl TrackStatus {
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
        }
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
