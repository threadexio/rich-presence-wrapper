use std::hash::{DefaultHasher, Hash, Hasher};

use eyre::Result;
use serde::Deserialize;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default = "default_sensitivity_list")]
    sensitivity: Vec<Box<str>>,
}

fn default_sensitivity_list() -> Vec<Box<str>> {
    vec!["id".into(), "title".into(), "album".into(), "artist".into()]
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(
    config: &Config,
    mut source: Source<Metadata>,
    mut sink: Sink<Metadata>,
) -> Result<()> {
    loop {
        let Some(mut metadata) = source.pull().await else {
            return Ok(());
        };

        let hash = hash_metadata(&metadata, &config.sensitivity);
        metadata.id = format!("<{hash:016x}>");

        if !sink.push(metadata) {
            return Ok(());
        }
    }
}

fn hash_metadata(metadata: &Metadata, sensivity: &[Box<str>]) -> u64 {
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

    let mut hasher = DefaultHasher::new();

    for field in sensivity {
        match field.as_ref() {
            "player" => player.hash(&mut hasher),
            "id" => id.hash(&mut hasher),
            "title" => title.hash(&mut hasher),
            "album" => album.hash(&mut hasher),
            "artist" => artist.hash(&mut hasher),
            "url" => url.hash(&mut hasher),
            "art_url" => art_url.hash(&mut hasher),
            field => extra.get(field).hash(&mut hasher),
        }
    }

    hasher.finish()
}
