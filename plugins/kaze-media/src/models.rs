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

/// Whether the app may read music in shared storage.
#[derive(Debug, Clone, Deserialize)]
pub struct AudioAccess {
    pub granted: bool,
}

#[derive(Serialize)]
pub(crate) struct ShareArgs<'a> {
    pub title: Option<&'a str>,
    pub text: &'a str,
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

/// Transport action raised by the media session (notification, lock screen,
/// headset or Bluetooth buttons).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCommandEvent {
    pub command: String,
    pub position_ms: Option<u64>,
}

#[cfg(target_os = "android")]
#[derive(Serialize)]
pub(crate) struct ListenerArgs<T> {
    pub event: &'static str,
    pub handler: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn snapshot_serializes_camel_case_with_optional_fields() {
        let snapshot = PlaybackSnapshot {
            playing: true,
            title: Some("Song".into()),
            duration_ms: Some(1500),
            ..Default::default()
        };
        let value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(value["playing"], json!(true));
        assert_eq!(value["durationMs"], json!(1500));
        assert_eq!(value["title"], json!("Song"));
        assert!(value["artist"].is_null());
    }

    #[test]
    fn snapshot_round_trips() {
        let snapshot = PlaybackSnapshot {
            playing: false,
            title: Some("A".into()),
            artist: Some("B".into()),
            album: Some("C".into()),
            duration_ms: Some(9),
            position_ms: Some(4),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let back: PlaybackSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, snapshot.title);
        assert_eq!(back.position_ms, Some(4));
        assert!(!back.playing);
    }

    #[test]
    fn snapshot_default_is_idle_and_parses_minimal_payload() {
        let back: PlaybackSnapshot = serde_json::from_value(json!({ "playing": false })).unwrap();
        assert!(!back.playing && back.title.is_none() && back.position_ms.is_none());
    }

    #[test]
    fn picked_folder_handles_cancel_and_selection() {
        let cancelled: PickedFolder = serde_json::from_value(json!({ "uri": null })).unwrap();
        assert!(cancelled.uri.is_none());
        let picked: PickedFolder = serde_json::from_value(json!({ "uri": "content://x" })).unwrap();
        assert_eq!(picked.uri.as_deref(), Some("content://x"));
    }

    #[test]
    fn secret_args_serialize_value_key_pair() {
        let value = serde_json::to_value(SecretArgs {
            key: "k",
            value: Some("v"),
        })
        .unwrap();
        assert_eq!(value, json!({ "key": "k", "value": "v" }));
        let parsed: SecretValue = serde_json::from_value(json!({ "value": null })).unwrap();
        assert!(parsed.value.is_none());
    }

    #[test]
    fn media_command_event_parses_with_and_without_position() {
        let seek: MediaCommandEvent =
            serde_json::from_value(json!({ "command": "seek", "positionMs": 1200 })).unwrap();
        assert_eq!(
            seek,
            MediaCommandEvent {
                command: "seek".into(),
                position_ms: Some(1200)
            }
        );
        let next: MediaCommandEvent = serde_json::from_value(json!({ "command": "next" })).unwrap();
        assert_eq!(next.position_ms, None);
    }
}

#[derive(Serialize)]
pub(crate) struct InstallArgs<'a> {
    pub path: &'a str,
}
