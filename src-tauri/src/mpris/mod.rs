use crate::core::command::RepeatMode;
use std::collections::HashMap;
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

pub mod service;
pub use service::*;

/// Converts playback state flags to MPRIS PlaybackStatus string.
pub fn playback_status_from_state(
    is_playing: bool,
    is_paused: bool,
    is_buffering: bool,
) -> &'static str {
    if is_playing {
        "Playing"
    } else if is_paused || is_buffering {
        "Paused"
    } else {
        "Stopped"
    }
}

/// Converts Kaze RepeatMode to MPRIS LoopStatus string.
pub fn repeat_mode_to_loop_status(mode: RepeatMode) -> &'static str {
    match mode {
        RepeatMode::Off => "None",
        RepeatMode::One => "Track",
        RepeatMode::All => "Playlist",
    }
}

/// Converts MPRIS LoopStatus string to Kaze RepeatMode.
pub fn loop_status_to_repeat_mode(status: &str) -> RepeatMode {
    match status.trim().to_lowercase().as_str() {
        "track" => RepeatMode::One,
        "playlist" => RepeatMode::All,
        _ => RepeatMode::Off,
    }
}

/// Converts duration/position in seconds (f64) to microseconds (i64).
pub fn seconds_to_microseconds(secs: f64) -> i64 {
    if secs.is_nan() || secs.is_infinite() || secs <= 0.0 {
        0
    } else {
        (secs * 1_000_000.0).round() as i64
    }
}

/// Converts duration/position in microseconds (i64) to seconds (f64).
pub fn microseconds_to_seconds(micros: i64) -> f64 {
    if micros <= 0 {
        0.0
    } else {
        micros as f64 / 1_000_000.0
    }
}

/// Sanitizes a track ID into a valid D-Bus ObjectPath element and returns full track path.
/// If track_id is empty, returns the standard MPRIS NoTrack sentinel.
pub fn track_id_to_object_path(track_id: &str) -> String {
    let trimmed = track_id.trim();
    if trimmed.is_empty() {
        return "/org/mpris/MediaPlayer2/TrackList/NoTrack".to_string();
    }

    let sanitized: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let element = if sanitized.is_empty() {
        "default".to_string()
    } else {
        sanitized
    };

    format!("/org/mpris/MediaPlayer2/track/{}", element)
}

/// Holds metadata about a track for MPRIS exposure.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MprisTrackInfo {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub duration_secs: f64,
    pub cover_art: Option<String>,
    pub track_number: Option<i32>,
    pub url: Option<String>,
}

/// Builds the MPRIS Metadata dictionary (a{sv}).
pub fn build_mpris_metadata(info: Option<&MprisTrackInfo>) -> HashMap<String, OwnedValue> {
    let mut map = HashMap::new();

    match info {
        Some(info) if !info.track_id.trim().is_empty() => {
            let path_str = track_id_to_object_path(&info.track_id);
            if let Ok(obj_path) = ObjectPath::try_from(path_str.as_str()) {
                if let Ok(val) = Value::from(obj_path).try_into() {
                    map.insert("mpris:trackid".to_string(), val);
                }
            }

            let length_us = seconds_to_microseconds(info.duration_secs);
            if let Ok(val) = Value::from(length_us).try_into() {
                map.insert("mpris:length".to_string(), val);
            }

            if let Ok(val) = Value::from(info.title.as_str()).try_into() {
                map.insert("xesam:title".to_string(), val);
            }

            if !info.artist.is_empty() {
                if let Ok(val) = Value::from(vec![info.artist.as_str()]).try_into() {
                    map.insert("xesam:artist".to_string(), val);
                }
            }

            if let Some(ref album) = info.album {
                if let Ok(val) = Value::from(album.as_str()).try_into() {
                    map.insert("xesam:album".to_string(), val);
                }
            }

            if let Some(ref album_artist) = info.album_artist {
                if let Ok(val) = Value::from(vec![album_artist.as_str()]).try_into() {
                    map.insert("xesam:albumArtist".to_string(), val);
                }
            }

            if let Some(ref art) = info.cover_art {
                let art_uri = if art.starts_with("http://")
                    || art.starts_with("https://")
                    || art.starts_with("file://")
                {
                    art.clone()
                } else {
                    format!("file://{}", art)
                };
                if let Ok(val) = Value::from(art_uri.as_str()).try_into() {
                    map.insert("mpris:artUrl".to_string(), val);
                }
            }

            if let Some(num) = info.track_number {
                if let Ok(val) = Value::from(num).try_into() {
                    map.insert("xesam:trackNumber".to_string(), val);
                }
            }

            if let Some(ref url) = info.url {
                let track_uri = if url.starts_with("http://")
                    || url.starts_with("https://")
                    || url.starts_with("file://")
                {
                    url.clone()
                } else {
                    format!("file://{}", url)
                };
                if let Ok(val) = Value::from(track_uri.as_str()).try_into() {
                    map.insert("xesam:url".to_string(), val);
                }
            }
        }
        _ => {
            if let Ok(no_track) = ObjectPath::try_from("/org/mpris/MediaPlayer2/TrackList/NoTrack")
            {
                if let Ok(val) = Value::from(no_track).try_into() {
                    map.insert("mpris:trackid".to_string(), val);
                }
            }
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_status_from_state() {
        assert_eq!(playback_status_from_state(true, false, false), "Playing");
        assert_eq!(playback_status_from_state(false, true, false), "Paused");
        assert_eq!(playback_status_from_state(false, false, true), "Paused");
        assert_eq!(playback_status_from_state(false, false, false), "Stopped");
        assert_eq!(playback_status_from_state(true, true, false), "Playing");
        assert_eq!(playback_status_from_state(true, false, true), "Playing");
    }

    #[test]
    fn test_repeat_mode_loop_status_roundtrip() {
        assert_eq!(repeat_mode_to_loop_status(RepeatMode::Off), "None");
        assert_eq!(repeat_mode_to_loop_status(RepeatMode::One), "Track");
        assert_eq!(repeat_mode_to_loop_status(RepeatMode::All), "Playlist");

        assert_eq!(loop_status_to_repeat_mode("None"), RepeatMode::Off);
        assert_eq!(loop_status_to_repeat_mode("none"), RepeatMode::Off);
        assert_eq!(loop_status_to_repeat_mode("Track"), RepeatMode::One);
        assert_eq!(loop_status_to_repeat_mode("track"), RepeatMode::One);
        assert_eq!(loop_status_to_repeat_mode("Playlist"), RepeatMode::All);
        assert_eq!(loop_status_to_repeat_mode("playlist"), RepeatMode::All);
        assert_eq!(loop_status_to_repeat_mode("invalid"), RepeatMode::Off);
    }

    #[test]
    fn test_seconds_and_microseconds_conversion() {
        assert_eq!(seconds_to_microseconds(0.0), 0);
        assert_eq!(seconds_to_microseconds(-5.0), 0);
        assert_eq!(seconds_to_microseconds(f64::NAN), 0);
        assert_eq!(seconds_to_microseconds(1.5), 1_500_000);
        assert_eq!(seconds_to_microseconds(180.25), 180_250_000);

        assert_eq!(microseconds_to_seconds(0), 0.0);
        assert_eq!(microseconds_to_seconds(-100), 0.0);
        assert_eq!(microseconds_to_seconds(1_500_000), 1.5);
    }

    #[test]
    fn test_track_id_to_object_path_sanitization() {
        assert_eq!(
            track_id_to_object_path(""),
            "/org/mpris/MediaPlayer2/TrackList/NoTrack"
        );
        assert_eq!(
            track_id_to_object_path("   "),
            "/org/mpris/MediaPlayer2/TrackList/NoTrack"
        );
        assert_eq!(
            track_id_to_object_path("e3b0c442-98fc-1c14-9afb-4c7220130e38"),
            "/org/mpris/MediaPlayer2/track/e3b0c442_98fc_1c14_9afb_4c7220130e38"
        );
        assert_eq!(
            track_id_to_object_path("track:123/song.mp3"),
            "/org/mpris/MediaPlayer2/track/track_123_song_mp3"
        );
        // Ensure path starts with valid character and only valid chars
        let path = track_id_to_object_path("simple_123");
        assert!(ObjectPath::try_from(path.as_str()).is_ok());
    }

    #[test]
    fn test_build_mpris_metadata_none() {
        let meta = build_mpris_metadata(None);
        assert_eq!(meta.len(), 1);
        assert!(meta.contains_key("mpris:trackid"));
    }

    #[test]
    fn test_build_mpris_metadata_with_track() {
        let info = MprisTrackInfo {
            track_id: "test-track-123".to_string(),
            title: "Midnight City".to_string(),
            artist: "M83".to_string(),
            album: Some("Hurry Up, We're Dreaming".to_string()),
            album_artist: Some("M83".to_string()),
            duration_secs: 243.0,
            cover_art: Some("/path/to/cover.jpg".to_string()),
            track_number: Some(2),
            url: Some("/path/to/song.mp3".to_string()),
        };

        let meta = build_mpris_metadata(Some(&info));
        assert!(meta.contains_key("mpris:trackid"));
        assert!(meta.contains_key("mpris:length"));
        assert!(meta.contains_key("xesam:title"));
        assert!(meta.contains_key("xesam:artist"));
        assert!(meta.contains_key("xesam:album"));
        assert!(meta.contains_key("xesam:albumArtist"));
        assert!(meta.contains_key("mpris:artUrl"));
        assert!(meta.contains_key("xesam:trackNumber"));
        assert!(meta.contains_key("xesam:url"));
    }
}
