//! Conservative recording matching, intentionally independent of the discovery fuzzy matcher.
use super::types::ExternalTrackMetadata;

pub fn normalized(s: &str) -> String {
    html_escape::decode_html_entities(s)
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn unique_match<'a>(
    title: &str,
    artist: Option<&str>,
    album: Option<&str>,
    duration: f64,
    candidates: &'a [ExternalTrackMetadata],
) -> Option<&'a ExternalTrackMetadata> {
    let title = normalized(title);
    let artist = artist
        .map(normalized)
        .filter(|a| !a.is_empty() && a != "unknown artist" && a != "unknown");
    let album = album
        .map(normalized)
        .filter(|a| !a.is_empty() && a != "unknown album");
    if title.is_empty() || !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    if artist.is_none() && album.is_none() {
        return None;
    }
    let mut matches = candidates.iter().filter(|c| {
        normalized(&c.title) == title
            && c.duration_secs
                .is_some_and(|d| d.is_finite() && (d - duration).abs() <= 3.0)
            && if let Some(ref a) = artist {
                normalized(&c.artist_name) == *a
            } else {
                c.album_title.as_deref().map(normalized).as_ref() == album.as_ref()
            }
    });
    let first = matches.next()?;
    if matches.any(|c| c.provider_track_id != first.provider_track_id) {
        None
    } else {
        Some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(id: &str, title: &str) -> ExternalTrackMetadata {
        ExternalTrackMetadata {
            language: None,
            provider: "jiosaavn".into(),
            provider_track_id: id.into(),
            title: title.into(),
            artist_name: "Artist".into(),
            artist_id: None,
            album_title: Some("Album".into()),
            album_id: None,
            duration_secs: Some(200.0),
            year: None,
            track_number: None,
            cover_art_url: None,
        }
    }
    #[test]
    fn requires_unique_evidence_and_preserves_recordings() {
        let c = [track("1", "Song"), track("2", "Song (Live)")];
        assert!(unique_match("Song", Some("Artist"), None, 202.9, &c).is_some());
        assert!(unique_match("Song (Remix)", Some("Artist"), None, 200.0, &c).is_none());
        assert!(unique_match("Song", Some("Artist"), None, 204.0, &c).is_none());
        assert!(unique_match("Song", None, None, 200.0, &c).is_none());
        assert!(unique_match("Song", None, Some("Album"), 200.0, &c).is_some());
        assert!(unique_match("Song", None, Some("Other"), 200.0, &c).is_none());
        assert!(unique_match(
            "Song",
            Some("Artist"),
            None,
            200.0,
            &[track("1", "Song"), track("2", "Song")]
        )
        .is_none());
        for version in ["Live", "Remix", "Acoustic", "Cover", "Remastered"] {
            assert!(unique_match(
                "Song",
                Some("Artist"),
                None,
                200.0,
                &[track("1", &format!("Song ({version})"))]
            )
            .is_none());
        }
    }
}
