//! The Android release manifest (`android.json`, published next to each
//! GitHub release) and the pure checks the in-app updater relies on.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Where the phone looks for the newest release.
pub const MANIFEST_URL: &str = "https://github.com/selfAnnihilator/kaze/releases/latest/download/android.json";

/// One Android release: its version, the APK's address and SHA-256.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AndroidRelease {
    pub version: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// `major.minor.patch` of a version like "v2.1.10" (a pre-release suffix is ignored).
fn parts(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.trim().trim_start_matches('v').split(['-', '+']).next()?;
    let mut it = core.split('.').map(|p| p.parse::<u64>().ok());
    let v = (it.next()??, it.next()??, it.next()??);
    it.next().is_none().then_some(v)
}

/// Whether `candidate` is a newer release than `current`. Unreadable versions never are.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    matches!((parts(candidate), parts(current)), (Some(a), Some(b)) if a > b)
}

/// Reads a manifest, refusing one that isn't a usable release. Only a local
/// test manifest (debug builds) may point at a plain-http APK.
pub fn parse_manifest(json: &str, allow_http: bool) -> Result<AndroidRelease, String> {
    let release: AndroidRelease = serde_json::from_str(json).map_err(|e| format!("Bad update manifest: {e}"))?;
    let secure = release.url.starts_with("https://") || (allow_http && release.url.starts_with("http://"));
    if parts(&release.version).is_none() || !secure || release.sha256.len() != 64 {
        return Err("The update manifest is incomplete".into());
    }
    Ok(release)
}

/// Whether `bytes` are exactly the APK the manifest describes.
#[cfg_attr(not(test), allow(dead_code))]
pub fn sha256_matches(bytes: &[u8], expected: &str) -> bool {
    format!("{:x}", Sha256::digest(bytes)).eq_ignore_ascii_case(expected.trim())
}

/// A release without `android.json` (older releases, PC-only fixes) has no
/// Android update: that is "up to date", not an error.
pub fn means_no_release(status: u16) -> bool {
    status == 404
}

/// What the phone shows when the manifest can't be fetched; details go to the log.
pub const UNREACHABLE: &str = "Couldn’t reach the update server. Check your connection and try again.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_manifest_is_no_update() {
        assert!(means_no_release(404));
        assert!(!means_no_release(500) && !means_no_release(403));
    }

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("2.1.2", "2.1.1"));
        assert!(is_newer("v2.10.0", "2.9.9"));
        assert!(is_newer("3.0.0", "2.99.99"));
        assert!(!is_newer("2.1.1", "2.1.1"), "the same version is not an update");
        assert!(!is_newer("2.1.0", "2.1.1"));
        assert!(!is_newer("latest", "2.1.1"), "junk is never newer");
        assert!(!is_newer("2.1", "2.1.1"));
        assert!(is_newer("2.2.0-beta", "2.1.9"));
    }

    #[test]
    fn reads_only_complete_manifests() {
        let sha = "a".repeat(64);
        let ok = format!(r#"{{"version":"2.1.2","url":"https://github.com/x/Kaze.apk","sha256":"{sha}"}}"#);
        assert_eq!(parse_manifest(&ok, false).unwrap().version, "2.1.2");
        assert!(parse_manifest(&ok.replace("https://", "http://"), false).is_err(), "https only");
        assert!(parse_manifest(&ok.replace("https://", "http://"), true).is_ok(), "a local test manifest");
        assert!(parse_manifest(&ok.replace(&sha, "abc"), false).is_err(), "a real sha256");
        assert!(parse_manifest("{}", false).is_err());
    }

    #[test]
    fn refuses_a_tampered_apk() {
        let apk = b"kaze apk bytes";
        let sha = format!("{:x}", Sha256::digest(apk));
        assert!(sha256_matches(apk, &sha.to_uppercase()));
        assert!(!sha256_matches(b"kaze apk bytez", &sha));
    }
}
