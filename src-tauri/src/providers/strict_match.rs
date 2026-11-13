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

/// Words that mark a different recording; both sides must agree on them.
const VARIANT_MARKERS: &[&str] = &[
    "live", "remix", "acoustic", "unplugged", "reprise", "instrumental", "lofi", "slowed",
    "reverb", "karaoke", "cover",
];

/// Strip decorations Spotify/JioSaavn add to titles: ` - …`, ` | …`, `(…)`, `[…]`.
pub fn clean_title(raw: &str) -> String {
    let decoded = html_escape::decode_html_entities(raw).to_string();
    let mut head = decoded.as_str();
    for sep in [" - ", " | ", " – "] {
        if let Some(i) = head.find(sep) {
            if i > 0 {
                head = &head[..i];
            }
        }
    }
    let mut out = String::with_capacity(head.len());
    let mut depth = 0usize;
    for c in head.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.is_empty() { decoded.trim().to_string() } else { out }
}

fn markers(raw: &str) -> Vec<&'static str> {
    let n = normalized(raw);
    let words: Vec<&str> = n.split(' ').collect();
    VARIANT_MARKERS
        .iter()
        .copied()
        .filter(|m| words.contains(m))
        .collect()
}

fn contains_words(haystack: &str, needle: &str) -> bool {
    !needle.is_empty() && format!(" {haystack} ").contains(&format!(" {needle} "))
}

/// Looser matcher for importing an external (Spotify) track list onto JioSaavn.
/// Title (after decoration stripping), one shared artist and duration ±5s are all required;
/// variant markers (live/remix/…) must agree. Closest duration wins.
pub fn import_match<'a>(
    title: &str,
    artists: &[String],
    duration: f64,
    candidates: &'a [ExternalTrackMetadata],
) -> Option<&'a ExternalTrackMetadata> {
    let core = normalized(&clean_title(title));
    let artists: Vec<String> = artists
        .iter()
        .map(|a| normalized(a))
        .filter(|a| !a.is_empty())
        .collect();
    if core.is_empty() || artists.is_empty() || !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let want_markers = markers(title);
    candidates
        .iter()
        .filter(|c| {
            let other = normalized(&clean_title(&c.title));
            let title_ok = other == core || {
                let (short, long) = if other.len() < core.len() { (&other, &core) } else { (&core, &other) };
                short.split(' ').count() >= 2 && long.starts_with(&format!("{short} "))
            };
            let cand_artist = normalized(&c.artist_name);
            let artist_ok = artists.iter().any(|a| contains_words(&cand_artist, a))
                || c.artist_name
                    .split(',')
                    .map(normalized)
                    .any(|ca| artists.iter().any(|a| contains_words(a, &ca)));
            title_ok
                && artist_ok
                && markers(&c.title) == want_markers
                && c.duration_secs
                    .is_some_and(|d| d.is_finite() && (d - duration).abs() <= 5.0)
        })
        .min_by(|a, b| {
            let da = (a.duration_secs.unwrap_or(f64::MAX) - duration).abs();
            let db = (b.duration_secs.unwrap_or(f64::MAX) - duration).abs();
            da.total_cmp(&db)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: &str, title: &str, artist: &str, secs: f64) -> ExternalTrackMetadata {
        ExternalTrackMetadata {
            artist_name: artist.into(),
            duration_secs: Some(secs),
            ..track(id, title)
        }
    }

    #[test]
    fn clean_title_strips_decorations() {
        assert_eq!(clean_title("Arz Kiya Hai | Coke Studio Bharat"), "Arz Kiya Hai");
        assert_eq!(clean_title("Mann Mera - Original Version"), "Mann Mera");
        assert_eq!(clean_title("Kesariya (From &quot;Brahmastra&quot;)"), "Kesariya");
        assert_eq!(clean_title("Song [feat. X] (Remastered)"), "Song");
        assert_eq!(clean_title("(Intro)"), "(Intro)");
    }

    #[test]
    fn import_match_accepts_decorated_titles_and_shared_artist() {
        let artists = vec!["Anuv Jain".to_string(), "Lost Stories".to_string()];
        let c = [
            cand("far", "Arz Kiya Hai", "Anuv Jain", 280.0),
            cand("near", "Arz Kiya Hai", "Anuv Jain, Lost Stories", 295.0),
            cand("other", "Arz Kiya Hai", "Someone Else", 294.0),
        ];
        let m = import_match("Arz Kiya Hai | Coke Studio Bharat", &artists, 294.5, &c).unwrap();
        assert_eq!(m.provider_track_id, "near");
    }

    #[test]
    fn import_match_rejects_duration_and_variant_mismatch() {
        let a = vec!["Artist".to_string()];
        let c = [cand("1", "Song", "Artist", 200.0), cand("2", "Song (Live)", "Artist", 201.0)];
        assert!(import_match("Song", &a, 206.0, &c).is_none());
        assert_eq!(import_match("Song - Live", &a, 201.0, &c).unwrap().provider_track_id, "2");
        assert_eq!(import_match("Song", &a, 201.0, &c).unwrap().provider_track_id, "1");
        assert!(import_match("Song (Remix)", &a, 200.0, &c).is_none());
        // single-word prefix is not enough
        let c = [cand("3", "Tum Hi Ho", "Artist", 200.0)];
        assert!(import_match("Tum", &a, 200.0, &c).is_none());
        assert!(import_match("Tum Hi", &a, 200.0, &c).is_some());
    }
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
