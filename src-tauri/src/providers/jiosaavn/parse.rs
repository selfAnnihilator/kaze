//! Reading JioSaavn JSON: tracks, credits, chart feeds and media URLs.
use super::*;

pub(super) fn full_track_source(value: &Value) -> Result<(String, f64)> {
    let info = value.get("more_info").unwrap_or(value);
    if text(value, "type").is_some_and(|kind| kind != "song") {
        return Err(JioSaavnError::Unavailable("not a song".into()));
    }
    let metadata = parse_track(value)?;
    let duration = metadata
        .duration_secs
        .filter(|d| *d > 0.0)
        .ok_or_else(|| JioSaavnError::Malformed("missing full-song duration".into()))?;
    // Decode only the ordinary URL envelope from the reference adapter.
    // Never use encrypted_drm_media_url, licenses, keys, or preview substitutions.
    if value
        .get("rights")
        .or_else(|| info.get("rights"))
        .and_then(|r| number(r, "code"))
        .is_some_and(|n| n != 0.0)
    {
        return Err(JioSaavnError::Unavailable(
            "territory or content restriction".into(),
        ));
    }
    let encrypted = text(value, "encrypted_media_url")
        .or_else(|| text(info, "encrypted_media_url"))
        .ok_or_else(|| JioSaavnError::Unavailable("ordinary full-track media absent".into()))?;
    let url = decode_media_url(
        &encrypted,
        truthy(value.get("320kbps").unwrap_or(&info["320kbps"])),
    )?;
    Ok((url, duration))
}

pub(super) fn album_credits(value: &Value) -> Vec<String> {
    if let Some(artists) =
        text(value, "primary_artists").or_else(|| text(&value["more_info"], "primary_artists"))
    {
        return artists
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect();
    }
    value
        .pointer("/more_info/artistMap/primary_artists")
        .and_then(Value::as_array)
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| text(artist, "name"))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn merge_charts(feeds: &[Value]) -> Vec<Value> {
    let mut seen = HashSet::new();
    let mut charts: Vec<_> = feeds
        .iter()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|item| {
            let id = text(item, "id").or_else(|| text(item, "listid"))?;
            let title = text(item, "title").or_else(|| text(item, "listname"))?;
            if !id.chars().all(|c| c.is_ascii_digit()) || !seen.insert(id.clone()) {
                return None;
            }
            let image = text(item, "image")
                .filter(|image| image.starts_with("https://"))
                .map(|image| {
                    image
                        .replace("150x150", "500x500")
                        .replace("50x50", "500x500")
                });
            Some(serde_json::json!({"id":id,"title":title,"image":image,"perma_url":text(item,"perma_url")}))
        })
        .take(90)
        .collect();
    charts.sort_by_key(|chart| {
        let title = chart["title"].as_str().unwrap_or("").to_lowercase();
        !["global", "english", "international", "worldwide"]
            .iter()
            .any(|word| title.contains(word))
    });
    charts.truncate(30);
    charts
}

pub(super) fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .filter(|s| !s.trim().is_empty())
        .map(|s| html_escape::decode_html_entities(&s).trim().to_string())
}
pub(super) fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key)
        .and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()))
        .filter(|n| n.is_finite())
}
pub(super) fn truthy(v: &Value) -> bool {
    v == true || v == 1 || v == "1" || v == "true"
}

pub fn parse_track(v: &Value) -> Result<ExternalTrackMetadata> {
    let id = text(v, "id").ok_or_else(|| JioSaavnError::Malformed("missing song ID".into()))?;
    let title = text(v, "song")
        .or_else(|| text(v, "title"))
        .ok_or_else(|| JioSaavnError::Malformed("missing title".into()))?;
    let info = v.get("more_info").unwrap_or(v);
    let artist = text(v, "primary_artists")
        .or_else(|| text(info, "primary_artists"))
        .or_else(|| text(v, "singers"))
        .or_else(|| {
            info.pointer("/artistMap/primary_artists")
                .and_then(Value::as_array)
                .map(|artists| {
                    artists
                        .iter()
                        .filter_map(|artist| text(artist, "name"))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_default();
    let cover = text(v, "image")
        .filter(|u| u.starts_with("https://"))
        .map(|u| u.replace("50x50", "500x500").replace("150x150", "500x500"));
    Ok(ExternalTrackMetadata {
        provider: "jiosaavn".into(),
        provider_track_id: id,
        title,
        artist_name: artist,
        artist_id: text(v, "primary_artists_id"),
        album_title: text(v, "album").or_else(|| text(info, "album")),
        album_id: text(v, "albumid").or_else(|| text(info, "album_id")),
        duration_secs: number(v, "duration").or_else(|| number(info, "duration")),
        year: number(v, "year").filter(|n| *n > 0.0).map(|n| n as i64),
        track_number: number(v, "track_number")
            .filter(|n| *n > 0.0)
            .map(|n| n as u32),
        cover_art_url: cover,
        language: text(v, "language").or_else(|| text(info, "language")),
    })
}

pub(super) fn decode_media_url(encrypted: &str, high_quality: bool) -> Result<String> {
    let mut bytes = STANDARD
        .decode(encrypted.trim())
        .map_err(|_| JioSaavnError::Malformed("invalid media envelope".into()))?;
    if bytes.is_empty() || bytes.len() % 8 != 0 {
        return Err(JioSaavnError::Malformed(
            "invalid media block length".into(),
        ));
    }
    let cipher = Des::new_from_slice(b"38346591").expect("DES key length");
    for chunk in bytes.as_chunks_mut::<8>().0 {
        cipher.decrypt_block(chunk.into());
    }
    let padding = *bytes.last().unwrap() as usize;
    if padding == 0
        || padding > 8
        || !bytes[bytes.len() - padding..]
            .iter()
            .all(|b| *b as usize == padding)
    {
        return Err(JioSaavnError::Malformed("invalid media padding".into()));
    }
    bytes.truncate(bytes.len() - padding);
    let url = String::from_utf8(bytes)
        .map_err(|_| JioSaavnError::Malformed("non-UTF8 media URL".into()))?
        .replace(
            "_96.mp4",
            if high_quality { "_320.mp4" } else { "_160.mp4" },
        );
    let parsed = reqwest::Url::parse(&url)
        .map_err(|_| JioSaavnError::Malformed("invalid media URL".into()))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("aac.saavncdn.com")
        || parsed.path().contains("_p.")
    {
        return Err(JioSaavnError::Unavailable(
            "not an ordinary full-track Saavn CDN URL".into(),
        ));
    }
    Ok(url)
}
