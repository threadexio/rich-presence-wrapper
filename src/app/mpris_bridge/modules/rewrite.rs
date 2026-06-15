use std::borrow::Cow;

use eyre::Result;
use regex::Regex;
use serde::Deserialize;

use super::super::metadata::Record;
use super::super::pipeline::{self, Sink, Source, Stage, StageBuilder};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub rules: RewriteRuleSet,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RewriteRuleSet {
    pub player: Option<RewriteRule>,
    pub track_id: Option<RewriteRule>,
    pub title: Option<RewriteRule>,
    pub album: Option<RewriteRule>,
    pub artist: Option<RewriteRule>,
    pub url: Option<RewriteRule>,
    pub art_url: Option<RewriteRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RewriteRule {
    #[serde(deserialize_with = "deserialize_regex")]
    pub pattern: Regex,
    pub rewrite: String,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    pipeline.stage(RewriteBuilder {
        rules: config.rules.clone(),
    });

    Ok(())
}

///////////////////////////////////////////////////////////////////////////////

struct RewriteBuilder {
    rules: RewriteRuleSet,
}

impl StageBuilder<Record> for RewriteBuilder {
    type Stage = RewriteStage;

    fn build(self, sink: Sink<Record>, source: Source<Record>) -> Self::Stage {
        let Self { rules } = self;

        RewriteStage {
            rules,
            source,
            sink,
        }
    }
}

struct RewriteStage {
    rules: RewriteRuleSet,
    source: Source<Record>,
    sink: Sink<Record>,
}

impl Stage<Record> for RewriteStage {
    async fn run(&mut self) -> Result<()> {
        loop {
            let Some(record) = self.source.pull().await else {
                return Ok(());
            };

            let record = self.rewrite_record(record);

            if !self.sink.push(record) {
                return Ok(());
            }
        }
    }
}

impl RewriteStage {
    fn rewrite_record(&self, record: Record) -> Record {
        trait Rewrite {
            fn rewrite(&self, s: &str) -> String;
        }

        impl Rewrite for RewriteRule {
            fn rewrite(&self, s: &str) -> String {
                let Some(captures) = self.pattern.captures(s) else {
                    return s.to_owned();
                };

                let mut out = String::new();
                captures.expand(&self.rewrite, &mut out);
                out
            }
        }

        impl Rewrite for Option<RewriteRule> {
            fn rewrite(&self, s: &str) -> String {
                match self {
                    Some(rule) => rule.rewrite(s),
                    None => s.to_owned(),
                }
            }
        }

        Record {
            player: self.rules.player.rewrite(&record.player),
            status: record.status,
            track_id: self.rules.track_id.rewrite(&record.track_id),
            title: record.title.map(|x| self.rules.title.rewrite(&x)),
            album: record.album.map(|x| self.rules.album.rewrite(&x)),
            artist: record.artist.map(|x| self.rules.artist.rewrite(&x)),
            url: record.url.map(|x| self.rules.url.rewrite(&x)),
            art_url: record.art_url.map(|x| self.rules.art_url.rewrite(&x)),
            position: record.position,
            length: record.length,
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let pattern = Cow::<'de, str>::deserialize(deserializer)?;
    Regex::new(&pattern).map_err(serde::de::Error::custom)
}
