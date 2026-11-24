//! Uploads only what changed since this device's last successful upload.
//!
//! Every live sync (every two minutes on each signed-in device) used to push
//! the whole library: all songs, stats, playlists and settings. The worker
//! upserts each row, and Cloudflare D1 counts every upsert (plus its index
//! updates) as rows written, so a few hundred songs ran the free 100k/day
//! limit out within hours. The worker only upserts and deletes nothing that
//! isn't a tombstone, so leaving unchanged rows out is safe.
//!
//! The ledger is in memory: after a restart the first sync uploads
//! everything once, then only changes.

use crate::cloud::models::SyncPayload;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};

type Fingerprints = HashMap<&'static str, HashSet<u64>>;

/// What was last uploaded, per account and worker.
static LEDGER: LazyLock<Mutex<HashMap<String, Fingerprints>>> = LazyLock::new(Default::default);

/// Fingerprints to record once the upload succeeds.
#[derive(Debug, Default)]
pub struct Pending {
    account: String,
    fingerprints: Fingerprints,
}

/// A row's content, ignoring `updated_at` (stamped with the sync or archive time).
fn fingerprint<T: Serialize>(row: &T) -> u64 {
    let mut value = serde_json::to_value(row).unwrap_or_default();
    // A list (the yearly stats summary) carries one stamp per entry.
    let objects: Vec<&mut serde_json::Value> = match &mut value {
        serde_json::Value::Array(items) => items.iter_mut().collect(),
        other => vec![other],
    };
    for object in objects.into_iter().filter_map(|v| v.as_object_mut()) {
        object.remove("updated_at");
    }
    let mut hasher = DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    hasher.finish()
}

fn keep_changed<T: Serialize>(kind: &'static str, rows: Vec<T>, sent: &Fingerprints, next: &mut Fingerprints) -> Vec<T> {
    let before = sent.get(kind);
    let seen = next.entry(kind).or_default();
    rows.into_iter()
        .filter(|row| {
            let print = fingerprint(row);
            seen.insert(print);
            !before.is_some_and(|set| set.contains(&print))
        })
        .collect()
}

fn keep_changed_value(kind: &'static str, value: Option<serde_json::Value>, sent: &Fingerprints, next: &mut Fingerprints) -> Option<serde_json::Value> {
    value.and_then(|v| keep_changed(kind, vec![v], sent, next).pop())
}

/// The part of `payload` this device hasn't uploaded yet (deletions always go).
pub fn changed_only(account: &str, payload: SyncPayload) -> (SyncPayload, Pending) {
    let ledger = LEDGER.lock().unwrap_or_else(|e| e.into_inner());
    let empty = Fingerprints::new();
    let sent = ledger.get(account).unwrap_or(&empty);
    let mut next = Fingerprints::new();
    let delta = SyncPayload {
        songs: keep_changed("songs", payload.songs, sent, &mut next),
        playlists: keep_changed("playlists", payload.playlists, sent, &mut next),
        playlist_songs: keep_changed("playlist_songs", payload.playlist_songs, sent, &mut next),
        song_stats: keep_changed("song_stats", payload.song_stats, sent, &mut next),
        daily_stats: keep_changed("daily_stats", payload.daily_stats, sent, &mut next),
        track_device_stats: keep_changed("track_device_stats", payload.track_device_stats, sent, &mut next),
        user_stats: keep_changed_value("user_stats", payload.user_stats, sent, &mut next),
        user_settings: keep_changed_value("user_settings", payload.user_settings, sent, &mut next),
        deleted_playlists: payload.deleted_playlists,
        deleted_playlist_songs: payload.deleted_playlist_songs,
    };
    (delta, Pending { account: account.to_string(), fingerprints: next })
}

/// Nothing to upload.
pub fn is_empty(p: &SyncPayload) -> bool {
    p.songs.is_empty()
        && p.playlists.is_empty()
        && p.playlist_songs.is_empty()
        && p.song_stats.is_empty()
        && p.daily_stats.is_empty()
        && p.track_device_stats.is_empty()
        && p.user_stats.is_none()
        && p.user_settings.is_none()
        && p.deleted_playlists.is_empty()
        && p.deleted_playlist_songs.is_empty()
}

/// The upload succeeded: what was current is now what the cloud has.
pub fn commit(pending: Pending) {
    let mut ledger = LEDGER.lock().unwrap_or_else(|e| e.into_inner());
    ledger.insert(pending.account, pending.fingerprints);
}

#[cfg(test)]
#[path = "delta_tests.rs"]
mod tests;
