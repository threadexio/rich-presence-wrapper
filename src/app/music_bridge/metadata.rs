use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub player: String,

    pub id: String,
    pub status: TrackStatus,

    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,

    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub art_url: Option<String>,

    #[serde(default, with = "x::optional_duration_as_secs_f32")]
    pub position: Option<Duration>,
    #[serde(default, with = "x::optional_duration_as_secs_f32")]
    pub length: Option<Duration>,

    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackStatus {
    Playing,
    Paused,
    Stopped,
}

///////////////////////////////////////////////////////////////////////////////

mod x {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub mod optional_duration_as_secs_f32 {
        use super::*;

        pub fn serialize<S>(x: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            x.as_ref().map(|x| x.as_secs_f32()).serialize(serializer)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let x = Option::<f32>::deserialize(deserializer)?;
            Ok(x.map(Duration::from_secs_f32))
        }
    }
}
