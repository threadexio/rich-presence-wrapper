use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Deserialize)]
pub struct Metadata {
    pub player: String,

    pub id: String,
    pub status: TrackStatus,

    pub title: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,

    pub url: Option<String>,
    pub art_url: Option<String>,

    pub position: Option<Duration>,
    pub length: Option<Duration>,

    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackStatus {
    Playing,
    Paused,
    Stopped,
}
