//! The taste curves: saturation, per-song damping, library priors and the
//! constants that tune them.
use super::*;

/// Saturation constants: raw evidence needed to reach ~63% affinity.
pub(super) const ARTIST_K: f64 = 3.0;
pub(super) const GENRE_K: f64 = 4.0;
pub(super) const STYLE_K: f64 = 4.0;
pub(super) const PLAYLIST_TRACK_BONUS: f64 = 0.5;

/// Absolute, confidence-aware curve: a couple of listens stay modest, broad
/// sustained engagement approaches 1.0. Negative evidence maps to 0.
pub(crate) fn saturate(raw: f64, k: f64) -> f64 {
    if raw <= 0.0 {
        0.0
    } else {
        (1.0 - (-raw / k).exp()).clamp(0.0, 1.0)
    }
}

/// Per-song contribution to the artist: log-damped when positive, so replays
/// matter less than variety; skips stay linear. One song alone can never
/// carry an artist past ~0.5 affinity, however often it is replayed.
pub(super) const MAX_TRACK_ARTIST_RAW: f64 = 2.0;
pub(super) fn damp_track(net: f64) -> f64 {
    if net > 0.0 {
        net.ln_1p().min(MAX_TRACK_ARTIST_RAW)
    } else {
        net
    }
}

/// Adds shared-library priors only to keys that already carry positive
/// user-specific evidence (listening, likes, playlists).
pub(super) fn amplify_known(
    short: &mut HashMap<String, f64>,
    long: &mut HashMap<String, f64>,
    priors: HashMap<String, f64>,
) {
    for (key, prior) in priors {
        let known = short.get(&key).is_some_and(|v| *v > 0.0)
            || long.get(&key).is_some_and(|v| *v > 0.0);
        if known {
            *short.entry(key.clone()).or_default() += prior;
            *long.entry(key).or_default() += prior;
        }
    }
}

/// Small, log-saturating prior for owned / saved songs.
pub(super) fn library_prior(count: f64) -> f64 {
    (0.6 * count.ln_1p()).min(1.5)
}

pub(super) fn decade_of(year: Option<i64>) -> Option<String> {
    year.filter(|year| *year > 1900 && *year <= 2030)
        .map(|year| format!("{}0s", year / 10))
}
