use std::{collections::HashMap, mem::take};

use eyre::{Context, Result};
use regex::Regex;
use serde::Deserialize;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(flatten)]
    rules: HashMap<String, Rule>,
}

#[derive(Debug, Clone, Deserialize)]
struct Rule {
    #[serde(rename = "match")]
    #[serde(deserialize_with = "x::deserialize_regex")]
    pattern: Regex,

    rewrite: Box<str>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(
    config: &Config,
    source: Mut<Source<Metadata>>,
    sink: Mut<Sink<Metadata>>,
) -> Result<()> {
    let mut source = source.into_inner();
    let mut sink = sink.into_inner();

    loop {
        let Some(mut metadata) = source.pull().await else {
            return Ok(());
        };

        do_rewrite(&config.rules, &mut metadata);

        if !sink.push(metadata) {
            return Ok(());
        }
    }
}

fn do_rewrite(rules: &HashMap<String, Rule>, metadata: &mut Metadata) {
    let Metadata {
        player,
        id,
        status: _,
        title,
        album,
        artist,
        url,
        art_url,
        position: _,
        length: _,
        extra,
    } = metadata;

    for (field, rule) in rules {
        let r = match field.as_str() {
            "player" => player.rewrite(rule),
            "id" => id.rewrite(rule),
            "title" => title.rewrite(rule),
            "album" => album.rewrite(rule),
            "artist" => artist.rewrite(rule),
            "url" => url.rewrite(rule),
            "art_url" => art_url.rewrite(rule),
            _ => extra.get_mut(field).rewrite(rule),
        };

        if let Err(e) = r.with_context(|| format!("failed to rewrite '{field}'")) {
            warn!("{e:#}");
        }
    }
}

trait Rewrite {
    fn rewrite(&mut self, rule: &Rule) -> Result<()>;
}

impl<T> Rewrite for &mut T
where
    T: Rewrite,
{
    fn rewrite(&mut self, rule: &Rule) -> Result<()> {
        T::rewrite(self, rule)
    }
}

impl Rewrite for String {
    fn rewrite(&mut self, rule: &Rule) -> Result<()> {
        let me = take(self);

        if let Some(captures) = rule.pattern.captures(&me) {
            self.reserve(me.len());
            captures.expand(&rule.rewrite, self);
        } else {
            *self = me;
        }

        Ok(())
    }
}

impl<T> Rewrite for Option<T>
where
    T: Rewrite,
{
    fn rewrite(&mut self, rule: &Rule) -> Result<()> {
        let Some(me) = self.as_mut() else {
            return Ok(());
        };

        me.rewrite(rule)
    }
}

mod x {
    use super::*;
    use serde::de::Error;
    use std::borrow::Cow;

    pub fn deserialize_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Cow::<'_, str>::deserialize(deserializer)
            .and_then(|s| Regex::new(&s).map_err(Error::custom))
    }
}
