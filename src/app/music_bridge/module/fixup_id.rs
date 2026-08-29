use std::hash::{DefaultHasher, Hash, Hasher};

use eyre::Result;
use serde::Deserialize;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default = "default_sensitivity_list")]
    sensitivity: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
enum Field {
    Player,
    Id,
    Title,
    Album,
    Artist,
    Url,
    ArtUrl,
    Extra(Box<str>),
}
fn default_sensitivity_list() -> Vec<Field> {
    vec![Field::Id, Field::Title, Field::Album, Field::Artist]
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

fn hash_metadata(metadata: &Metadata, sensivity: &[Field]) -> u64 {
    let mut hasher = DefaultHasher::new();

    for field in sensivity {
        match field {
            Field::Player => metadata.player.hash(&mut hasher),
            Field::Id => metadata.id.hash(&mut hasher),
            Field::Title => metadata.title.hash(&mut hasher),
            Field::Album => metadata.album.hash(&mut hasher),
            Field::Artist => metadata.artist.hash(&mut hasher),
            Field::Url => metadata.url.hash(&mut hasher),
            Field::ArtUrl => metadata.art_url.hash(&mut hasher),
            Field::Extra(name) => metadata.extra.get(name.as_ref()).hash(&mut hasher),
        }
    }

    hasher.finish()
}
