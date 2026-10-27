//! Release ordering changes selection, never an already-bound recording's artwork.
use super::{strict_match::normalized, types::ExternalTrackMetadata};
use std::collections::HashMap;

/// Rank candidate editions without replacing an already displayed recording.
pub fn release_preference(title: &str, album: Option<&str>) -> u8 {
    let Some(album) = album.filter(|album| !album.trim().is_empty()) else {
        return 0;
    };
    let normalized = super::strict_match::normalized;
    if normalized(title) == normalized(album) {
        return 3;
    }
    let album = album.to_lowercase();
    if [
        "compilation",
        "greatest hits",
        "best of",
        "top hits",
        "pop 20",
        "workout",
        "viral hits",
        "party hits",
        "summer hits",
    ]
    .iter()
    .any(|marker| album.contains(marker))
    {
        0
    } else {
        2
    }
}

pub fn release_rank(title: &str, album: Option<&str>, year: Option<i64>) -> (u8, i64) {
    (
        release_preference(title, album),
        -year.filter(|year| *year > 0).unwrap_or(9999),
    )
}

/// Preserve provider relevance between songs, ordering only editions of the same song.
pub fn prefer_artist_editions(tracks: &mut [ExternalTrackMetadata], query: &str) {
    let mut groups = HashMap::new();
    for track in tracks.iter() {
        let key = (normalized(&track.title), normalized(&track.artist_name));
        let next = groups.len();
        groups.entry(key).or_insert(next);
    }
    let query = normalized(query);
    tracks.sort_by(|a, b| {
        let group = |track: &ExternalTrackMetadata| {
            groups[&(normalized(&track.title), normalized(&track.artist_name))]
        };
        let explicit_album = |track: &ExternalTrackMetadata| {
            track.album_title.as_ref().is_some_and(|album| {
                let album = normalized(album);
                !album.is_empty() && album != normalized(&track.title) && query.contains(&album)
            })
        };
        group(a)
            .cmp(&group(b))
            .then_with(|| explicit_album(b).cmp(&explicit_album(a)))
            .then_with(|| {
                release_rank(&b.title, b.album_title.as_deref(), b.year).cmp(&release_rank(
                    &a.title,
                    a.album_title.as_deref(),
                    a.year,
                ))
            })
    });
}
