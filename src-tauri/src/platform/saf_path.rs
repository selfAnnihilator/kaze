//! Turns a folder picked through Android's system picker (a Storage Access
//! Framework tree URI) into the file path the library scanner reads.

/// Decodes `%XX` escapes; `None` for malformed input.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// `content://com.android.externalstorage.documents/tree/primary%3AMusic`
/// → `/storage/emulated/0/Music`; an SD card volume `1A2B-3C4D:Songs` →
/// `/storage/1A2B-3C4D/Songs`; a `raw:` id is already a path. Other
/// providers (cloud drives, Downloads by id) have no file path: `None`.
pub fn tree_uri_to_path(uri: &str) -> Option<String> {
    let tree = uri.split("/tree/").nth(1)?;
    let id = percent_decode(tree.split('/').next()?)?;
    if let Some(raw) = id.strip_prefix("raw:") {
        return Some(raw.to_string());
    }
    if !uri.starts_with("content://com.android.externalstorage.documents/") {
        return None;
    }
    let (volume, rest) = id.split_once(':')?;
    let root = if volume == "primary" { "/storage/emulated/0".to_string() } else { format!("/storage/{volume}") };
    let rest = rest.trim_matches('/');
    Some(if rest.is_empty() { root } else { format!("{root}/{rest}") })
}

#[cfg(test)]
mod tests {
    use super::tree_uri_to_path;

    #[test]
    fn maps_internal_storage_folders() {
        let uri = "content://com.android.externalstorage.documents/tree/primary%3AMusic%2FKaze";
        assert_eq!(tree_uri_to_path(uri).as_deref(), Some("/storage/emulated/0/Music/Kaze"));
        let root = "content://com.android.externalstorage.documents/tree/primary%3A";
        assert_eq!(tree_uri_to_path(root).as_deref(), Some("/storage/emulated/0"));
    }

    #[test]
    fn maps_sd_cards_and_raw_paths() {
        let sd = "content://com.android.externalstorage.documents/tree/1A2B-3C4D%3ASongs%20Old";
        assert_eq!(tree_uri_to_path(sd).as_deref(), Some("/storage/1A2B-3C4D/Songs Old"));
        let raw = "content://com.android.providers.downloads.documents/tree/raw%3A%2Fstorage%2Femulated%2F0%2FDownload";
        assert_eq!(tree_uri_to_path(raw).as_deref(), Some("/storage/emulated/0/Download"));
    }

    #[test]
    fn rejects_providers_without_a_path() {
        assert_eq!(tree_uri_to_path("content://com.google.android.apps.docs.storage/tree/acc%3D1"), None);
        assert_eq!(tree_uri_to_path("content://com.android.externalstorage.documents/document/primary%3AMusic"), None);
        assert_eq!(tree_uri_to_path("content://com.android.externalstorage.documents/tree/bad%ZZ"), None);
    }
}
