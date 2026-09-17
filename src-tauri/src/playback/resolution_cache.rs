use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Conservative fallback TTL for resolved audio URLs when the provider does not
/// advertise an explicit expiry timestamp.  Chosen to be well inside YouTube's
/// ~6-hour signed-URL window with a safety margin.
const FALLBACK_TTL_SECS: u64 = 4 * 3600; // 4 hours

/// Hard cap on the number of entries stored in the cache.
pub const RESOLUTION_CACHE_MAX: usize = 100;

/// A single resolved entry.
#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    /// The direct, streamable audio URL.
    pub url: String,
    /// Track duration in seconds from the provider.
    pub duration: f64,
    /// Wall-clock time at which this entry was inserted.
    resolved_at: Instant,
    /// Absolute expiry time.  Computed either from a provider-supplied UNIX
    /// timestamp (e.g., the `expire` query parameter in a YouTube signed URL)
    /// or from `resolved_at + FALLBACK_TTL_SECS`.
    expires_at: Instant,
}

impl ResolvedEntry {
    /// Create a new entry.
    ///
    /// * `url`              – Direct streamable URL.
    /// * `duration`         – Provider-reported duration in seconds.
    /// * `provider_expiry`  – Optional UNIX timestamp (seconds) supplied by the
    ///                         provider.  When `Some`, the entry TTL is clamped
    ///                         to `min(provider_expiry, now + FALLBACK_TTL_SECS)`
    ///                         to avoid trusting arbitrarily long windows.
    pub fn new(url: String, duration: f64, provider_expiry: Option<u64>) -> Self {
        let now = Instant::now();
        let fallback_expiry = now + Duration::from_secs(FALLBACK_TTL_SECS);

        let expires_at = if let Some(unix_exp) = provider_expiry {
            // Convert UNIX timestamp to a Duration from now.
            let unix_now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if unix_exp > unix_now {
                let secs_remaining = unix_exp.saturating_sub(unix_now);
                // Apply a 10-minute safety margin so we never hand out a URL
                // that is about to expire before playback starts.
                let capped = secs_remaining.saturating_sub(600);
                let provider_instant = now + Duration::from_secs(capped);
                // Never trust the provider for longer than our fallback window.
                provider_instant.min(fallback_expiry)
            } else {
                // Provider expiry is in the past — treat as immediately expired.
                now
            }
        } else {
            fallback_expiry
        };

        Self {
            url,
            duration,
            resolved_at: now,
            expires_at,
        }
    }

    /// Returns `true` if this entry is still valid for use right now.
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.expires_at
    }
}

/// Thread-safe, bounded in-memory cache of resolved audio source URLs.
///
/// Lookup priority (from `StreamPlaybackManager::resolve_and_prepare_playback`):
///   disk audio cache → **resolution cache** → provider resolution
///
/// Eviction policy (on `insert` when at capacity):
///   1. Remove all expired entries.
///   2. If still at capacity, remove the oldest `resolved_at` entry.
pub struct ResolutionCache {
    /// `key → entry`.  Key is the same `full:v2:…` string used for the disk
    /// cache so lookups are a direct map hit.
    inner: Mutex<HashMap<String, ResolvedEntry>>,
}

impl ResolutionCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Look up a cached resolved URL.  Returns `None` when the entry is absent
    /// or has expired (expired entries are evicted on the spot).
    pub fn get(&self, key: &str) -> Option<ResolvedEntry> {
        let mut map = self.inner.lock().unwrap();
        match map.get(key) {
            Some(entry) if entry.is_valid() => Some(entry.clone()),
            Some(_) => {
                // Entry expired — remove it now.
                map.remove(key);
                None
            }
            None => None,
        }
    }

    /// Insert or replace an entry.  Enforces `RESOLUTION_CACHE_MAX`:
    ///   1. All expired entries are removed first.
    ///   2. If the cache is still at capacity, the oldest entry by
    ///      `resolved_at` is evicted.
    pub fn insert(&self, key: String, entry: ResolvedEntry) {
        let mut map = self.inner.lock().unwrap();

        // Step 1: evict all expired entries.
        map.retain(|_, v| v.is_valid());

        // Step 2: if still at or above the hard cap, remove the single oldest entry.
        if map.len() >= RESOLUTION_CACHE_MAX {
            if let Some(oldest_key) = map
                .iter()
                .min_by_key(|(_, v)| v.resolved_at)
                .map(|(k, _)| k.clone())
            {
                map.remove(&oldest_key);
            }
        }

        map.insert(key, entry);
    }

    /// Remove a specific entry (e.g., after a stream error reveals the URL is
    /// stale despite being within the TTL window).
    pub fn invalidate(&self, key: &str) {
        self.inner.lock().unwrap().remove(key);
    }

    /// Number of currently held entries (valid + expired-not-yet-evicted).
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// Number of currently valid (non-expired) entries.
    pub fn valid_len(&self) -> usize {
        self.inner
            .lock()
            .unwrap()
            .values()
            .filter(|e| e.is_valid())
            .count()
    }
}

impl Default for ResolutionCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Attempt to parse the `expire` query-parameter from a YouTube/googlevideo CDN
/// URL and return it as a UNIX timestamp (seconds).
///
/// YouTube signed URLs look like:
///   `https://rr*.googlevideo.com/…?…&expire=1747123456&…`
///
/// Returns `None` for non-YouTube URLs or when the parameter is absent/invalid.
pub fn parse_url_expiry(url: &str) -> Option<u64> {
    // Fast reject for non-YouTube CDN URLs.
    if !url.contains("googlevideo.com") && !url.contains("youtube.com") {
        return None;
    }
    // Parse the query string without pulling in a URL crate — we only need one
    // specific parameter.
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        if let Some(val) = pair.strip_prefix("expire=") {
            return val.parse::<u64>().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get_valid_entry() {
        let cache = ResolutionCache::new();
        let entry = ResolvedEntry::new("https://example.com/audio.m4a".to_string(), 210.0, None);
        cache.insert("key1".to_string(), entry);
        assert!(cache.get("key1").is_some());
    }

    #[test]
    fn expired_entry_is_evicted_on_get() {
        let cache = ResolutionCache::new();
        // Manually build an entry that is already expired.
        let entry = ResolvedEntry {
            url: "https://example.com/audio.m4a".to_string(),
            duration: 210.0,
            resolved_at: Instant::now(),
            // expiry in the past
            expires_at: Instant::now() - Duration::from_secs(1),
        };
        cache.inner.lock().unwrap().insert("key2".to_string(), entry);
        assert!(cache.get("key2").is_none());
        // Should have been removed.
        assert!(cache.inner.lock().unwrap().get("key2").is_none());
    }

    #[test]
    fn cap_is_enforced_by_evicting_oldest() {
        let cache = ResolutionCache::new();
        for i in 0..RESOLUTION_CACHE_MAX {
            let entry = ResolvedEntry::new(
                format!("https://example.com/{}.m4a", i),
                210.0,
                None,
            );
            cache.insert(format!("key{}", i), entry);
        }
        assert_eq!(cache.valid_len(), RESOLUTION_CACHE_MAX);

        // One more insert must evict the oldest.
        let extra = ResolvedEntry::new("https://example.com/extra.m4a".to_string(), 210.0, None);
        cache.insert("extra".to_string(), extra);
        assert_eq!(cache.valid_len(), RESOLUTION_CACHE_MAX);
        assert!(cache.get("extra").is_some());
    }

    #[test]
    fn parse_url_expiry_extracts_from_youtube_url() {
        let url = "https://rr3.googlevideo.com/path?itag=140&expire=1747123456&other=val";
        assert_eq!(parse_url_expiry(url), Some(1747123456));
    }

    #[test]
    fn parse_url_expiry_returns_none_for_non_youtube() {
        let url = "https://download.audius.co/tracks/abc/stream";
        assert_eq!(parse_url_expiry(url), None);
    }

    #[test]
    fn provider_expiry_clamps_to_fallback() {
        // Expiry far in the future (100 years) must be clamped to FALLBACK_TTL.
        let far_future_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 100 * 365 * 24 * 3600;
        let entry = ResolvedEntry::new("https://rr.googlevideo.com/x".to_string(), 180.0, Some(far_future_unix));
        // expires_at must be <= now + FALLBACK_TTL_SECS + tiny epsilon.
        let max_allowed = Instant::now() + Duration::from_secs(FALLBACK_TTL_SECS + 5);
        assert!(entry.expires_at <= max_allowed);
    }
}
