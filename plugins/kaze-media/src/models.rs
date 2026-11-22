use serde::{Deserialize, Serialize};

/// Playback state mirrored into the Android media session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshot {
    pub playing: bool,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub position_ms: Option<u64>,
}

/// A music folder chosen through the system picker (SAF tree URI).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedFolder {
    pub uri: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct SecretArgs<'a> {
    pub key: &'a str,
    pub value: Option<&'a str>,
}

#[derive(Deserialize)]
pub(crate) struct SecretValue {
    pub value: Option<String>,
}
