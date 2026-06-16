use eyre::Result;
use regex::Regex;
use serde::Deserialize;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    filters: FilterSet,
}

#[derive(Debug, Clone, Deserialize)]
struct FilterSet {
    player: Option<Filter>,
    track_id: Option<Filter>,
    title: Option<Filter>,
    album: Option<Filter>,
    artist: Option<Filter>,
    url: Option<Filter>,
    art_url: Option<Filter>,
}

#[derive(Debug, Clone, Deserialize)]
struct Filter {
    #[serde(deserialize_with = "crate::util::deserialize_regex")]
    r#match: Regex,

    #[serde(default)]
    invert: bool,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    pipeline.stage(FilterBuilder {
        filters: config.filters.clone(),
    });

    Ok(())
}

///////////////////////////////////////////////////////////////////////////////

struct FilterBuilder {
    filters: FilterSet,
}

impl StageBuilder<Record> for FilterBuilder {
    type Stage = FilterStage;

    fn build(self, sink: Sink<Record>, source: Source<Record>) -> Self::Stage {
        let Self { filters } = self;

        FilterStage {
            filters,
            source,
            sink,
        }
    }
}

struct FilterStage {
    filters: FilterSet,
    source: Source<Record>,
    sink: Sink<Record>,
}

impl Stage<Record> for FilterStage {
    async fn run(&mut self) -> Result<()> {
        loop {
            let Some(record) = self.source.pull().await else {
                return Ok(());
            };

            if !self.filters.matches(&record) {
                continue;
            }

            if !self.sink.push(record) {
                return Ok(());
            }
        }
    }
}

trait Matches<T: ?Sized> {
    fn matches(&self, item: &T) -> bool;
}

impl Matches<str> for Filter {
    fn matches(&self, s: &str) -> bool {
        let r = self.r#match.is_match(s);
        if self.invert { !r } else { r }
    }
}

impl<T> Matches<Option<T>> for Filter
where
    T: AsRef<str>,
{
    fn matches(&self, item: &Option<T>) -> bool {
        match item {
            Some(item) => self.matches(item.as_ref()),
            None => false,
        }
    }
}

impl<T: ?Sized> Matches<T> for Option<Filter>
where
    Filter: Matches<T>,
{
    fn matches(&self, item: &T) -> bool {
        match self {
            Some(filter) => filter.matches(item),
            None => true,
        }
    }
}

impl Matches<Record> for FilterSet {
    fn matches(&self, record: &Record) -> bool {
        let Self {
            player,
            track_id,
            title,
            album,
            artist,
            url,
            art_url,
        } = self;

        player.matches(record.player.as_str())
            && track_id.matches(record.track_id.as_str())
            && title.matches(&record.title)
            && album.matches(&record.album)
            && artist.matches(&record.artist)
            && url.matches(&record.url)
            && art_url.matches(&record.art_url)
    }
}
