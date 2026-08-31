use std::collections::HashMap;

use eyre::Result;
use regex::Regex;
use serde::Deserialize;

use super::super::metadata::TrackStatus;
use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(flatten)]
    filters: HashMap<String, Filter>,
}

#[derive(Debug, Clone, Deserialize)]
struct Filter {
    #[serde(rename = "match")]
    #[serde(deserialize_with = "x::deserialize_regex")]
    pattern: Regex,

    #[serde(default = "crate::util::r#false")]
    invert: bool,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(
    config: &Config,
    mut source: Source<Metadata>,
    mut sink: Sink<Metadata>,
) -> Result<()> {
    loop {
        let Some(metadata) = source.pull().await else {
            return Ok(());
        };

        if !config.filters.matches(&metadata) {
            continue;
        }

        if !sink.push(metadata) {
            return Ok(());
        }
    }
}

trait Matches<T>
where
    T: ?Sized,
{
    fn matches(&self, x: &T) -> bool;
}

impl<T, U> Matches<&T> for U
where
    T: ?Sized,
    U: Matches<T>,
{
    fn matches(&self, x: &&T) -> bool {
        self.matches(*x)
    }
}

impl Matches<str> for Filter {
    fn matches(&self, x: &str) -> bool {
        self.pattern.is_match(x) ^ self.invert
    }
}

impl Matches<String> for Filter {
    fn matches(&self, x: &String) -> bool {
        self.matches(x.as_str())
    }
}

impl Matches<TrackStatus> for Filter {
    fn matches(&self, x: &TrackStatus) -> bool {
        let s = match x {
            TrackStatus::Playing => "playing",
            TrackStatus::Paused => "paused",
            TrackStatus::Stopped => "stopped",
        };

        self.matches(s)
    }
}

impl<T> Matches<Option<T>> for Filter
where
    Filter: Matches<T>,
{
    fn matches(&self, x: &Option<T>) -> bool {
        match x.as_ref() {
            Some(x) => self.matches(x),
            None => false,
        }
    }
}

impl Matches<Metadata> for HashMap<String, Filter> {
    fn matches(&self, x: &Metadata) -> bool {
        let Metadata {
            player,
            id,
            status,
            title,
            album,
            artist,
            url,
            art_url,
            position: _,
            length: _,
            extra,
        } = x;

        self.iter().fold(true, |r, (key, filter)| {
            r & match key.as_str() {
                "player" => filter.matches(player),
                "id" => filter.matches(id),
                "status" => filter.matches(status),
                "title" => filter.matches(title),
                "album" => filter.matches(album),
                "artist" => filter.matches(artist),
                "url" => filter.matches(url),
                "art_url" => filter.matches(art_url),
                _ => filter.matches(&extra.get(key)),
            }
        })
    }
}

mod x {
    use super::*;
    use std::borrow::Cow;

    pub fn deserialize_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let pattern = Cow::<'de, str>::deserialize(deserializer)?;
        Regex::new(&pattern).map_err(serde::de::Error::custom)
    }
}
