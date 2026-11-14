//! Walking provider JSON: finding track identities and writing the resolved
//! ids, local copies and download state back into the response.
use super::*;

pub(super) fn identity(value: &Value) -> Option<&str> {
    // A source pointer also has track_id. It is a reference, not another track.
    value.get("title")?;
    value
        .get("external_track_id")
        .or_else(|| value.get("track_id"))
        .or_else(|| {
            if value.get("file_path").is_some() {
                value.get("id")
            } else {
                None
            }
        })
        .and_then(Value::as_str)
}
pub(super) fn collect(value: &Value, descriptors: &mut HashMap<String, Value>) {
    if let Some(id) = identity(value) {
        descriptors.insert(id.into(), value.clone());
    }
    match value {
        Value::Array(items) => {
            for item in items {
                collect(item, descriptors);
            }
        }
        Value::Object(fields) => {
            for value in fields.values() {
                collect(value, descriptors);
            }
        }
        _ => (),
    }
}
pub(super) fn apply(
    value: &mut Value,
    online: &HashSet<String>,
    linked: &HashMap<String, ExternalTrackMetadata>,
    owned: &HashSet<String>,
) -> bool {
    if let Some(id) = identity(value).map(str::to_string) {
        if online.contains(&id) {
            let Some(track) = linked.get(&id) else {
                return false;
            };
            if let Some(object) = value.as_object_mut() {
                object.insert("title".into(), json!(track.title));
                for (key, data) in [
                    ("artist", json!(track.artist_name)),
                    ("artist_name", json!(track.artist_name)),
                    ("album", json!(track.album_title)),
                    ("album_title", json!(track.album_title)),
                    ("duration_secs", json!(track.duration_secs)),
                    ("cover_art_url", json!(track.cover_art_url)),
                    ("provider", json!("jiosaavn")),
                    ("provider_id", json!(track.provider_track_id)),
                    ("preview_url", Value::Null),
                    ("matched_local_track_id", Value::Null),
                    ("local_copy", json!(owned.contains(&id))),
                    (
                        "playback_source",
                        json!({"provider":"jiosaavn","track_id":track.provider_track_id}),
                    ),
                ] {
                    object.insert(key.into(), data);
                }
            }
        }
    }
    match value {
        Value::Array(items) => items.retain_mut(|item| apply(item, online, linked, owned)),
        Value::Object(fields) => {
            for child in fields.values_mut() {
                if !apply(child, online, linked, owned) {
                    *child = Value::Null;
                }
            }
        }
        _ => (),
    }
    true
}
