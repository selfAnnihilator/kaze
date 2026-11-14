//! Local cover art, served to the webview as images sized for where they show.
//!
//! Covers used to cross IPC as full-resolution base64 data URLs. The webview
//! then decoded a 1400px bitmap (about 8 MB) even for a 48px row thumbnail,
//! and the backend kept every data URL in memory. Now the backend returns a
//! short `kaze-art:` URL; the webview loads it like any image, asking for a
//! size bucket, and the backend answers with a resized JPEG from a disk cache
//! keyed by the image's content (album tracks share one file).

use image::{imageops::FilterType, ImageFormat, ImageReader};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub const SCHEME: &str = "kaze-art";
/// Sizes the webview may ask for (CSS pixels x device pixel ratio, rounded up).
pub const SIZES: [u32; 4] = [128, 256, 512, 1024];
pub const DEFAULT_SIZE: u32 = 512;
/// The disk cache is trimmed back to `PRUNE_TO` once it grows past `PRUNE_AT`.
const PRUNE_AT: u64 = 96 * 1024 * 1024;
const PRUNE_TO: u64 = 64 * 1024 * 1024;

const FOLDER_IMAGES: [&str; 7] = [
    "cover.jpg",
    "folder.jpg",
    "album.jpg",
    "cover.png",
    "folder.png",
    "front.jpg",
    "front.png",
];

/// Where a local track's cover comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalArt {
    /// A picture tag inside the audio file.
    Embedded(PathBuf),
    /// An image file (folder cover or enrichment download).
    File(PathBuf),
}

impl LocalArt {
    /// Finds the cover of a local audio file: its own picture tag first, then
    /// a cover image in its folder.
    pub fn find(audio: &Path) -> Option<Self> {
        if embedded_picture(audio).is_some() {
            return Some(Self::Embedded(audio.to_path_buf()));
        }
        let folder = audio.parent()?;
        FOLDER_IMAGES
            .iter()
            .map(|name| folder.join(name))
            .find(|path| path.is_file())
            .map(Self::File)
    }

    /// The original image bytes.
    pub fn read(&self) -> Option<Vec<u8>> {
        match self {
            Self::Embedded(path) => embedded_picture(path),
            Self::File(path) => std::fs::read(path).ok(),
        }
    }
}

fn embedded_picture(audio: &Path) -> Option<Vec<u8>> {
    use lofty::file::TaggedFileExt;
    let tagged = lofty::probe::Probe::open(audio).ok()?.guess_file_type().ok()?.read().ok()?;
    let picture = tagged
        .tags()
        .iter()
        .find_map(|tag| tag.pictures().first())
        .or_else(|| tagged.primary_tag().and_then(|tag| tag.pictures().first()))
        .or_else(|| tagged.first_tag().and_then(|tag| tag.pictures().first()))?;
    Some(picture.data().to_vec())
}

static VERSION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Changes whenever library metadata changes; part of every cover URL.
pub fn version() -> u64 {
    VERSION.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn bump_version() {
    VERSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// The URL the webview loads for a track's cover. `version` changes when the
/// library metadata changes, so the webview's image cache never shows a stale
/// cover.
pub fn track_url(track_id: &str, version: u64) -> String {
    let hex: String = track_id.bytes().map(|b| format!("{b:02x}")).collect();
    // WebView2 reaches custom schemes through http://<scheme>.localhost.
    if cfg!(windows) {
        format!("http://{SCHEME}.localhost/{hex}?v={version}")
    } else {
        format!("{SCHEME}://localhost/{hex}?v={version}")
    }
}

/// Parses `kaze-art://localhost/<hex track id>?s=<size>&v=<version>`.
pub fn parse_request(path: &str, query: Option<&str>) -> Option<(String, u32)> {
    let hex = path.trim_start_matches('/').as_bytes();
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return None;
    }
    let bytes = hex
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    let id = String::from_utf8(bytes).ok()?;
    let size = query
        .into_iter()
        .flat_map(|q| q.split('&'))
        .find_map(|pair| pair.strip_prefix("s="))
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SIZE);
    Some((id, bucket(size)))
}

/// The smallest supported size at least as large as `size`.
pub fn bucket(size: u32) -> u32 {
    SIZES
        .iter()
        .copied()
        .find(|&s| s >= size)
        .unwrap_or(SIZES[SIZES.len() - 1])
}

/// Resized images on disk, keyed by the original's content and the size.
pub struct ThumbnailCache {
    dir: PathBuf,
}

impl ThumbnailCache {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// An image no larger than `size` on its longest side, and its MIME type.
    /// Small originals are returned as they are; images that cannot be
    /// decoded are too, so the webview can still try.
    pub fn get(&self, original: Vec<u8>, size: u32) -> (Vec<u8>, &'static str) {
        let key = format!("{:x}", Sha256::digest(&original));
        let path = self.dir.join(format!("{}-{size}.jpg", &key[..32]));
        if let Ok(bytes) = std::fs::read(&path) {
            return (bytes, "image/jpeg");
        }
        match resize(&original, size) {
            Some(bytes) => {
                let _ = std::fs::create_dir_all(&self.dir);
                let temp = path.with_extension("tmp");
                if std::fs::write(&temp, &bytes).is_ok() {
                    let _ = std::fs::rename(&temp, &path);
                }
                (bytes, "image/jpeg")
            }
            None => {
                let mime = image::guess_format(&original)
                    .map(|f| f.to_mime_type())
                    .unwrap_or("image/jpeg");
                (original, mime)
            }
        }
    }

    /// Deletes the oldest files once the cache passes its size limit.
    pub fn prune(&self) {
        prune_dir(&self.dir, PRUNE_AT, PRUNE_TO);
    }
}

/// A JPEG no larger than `size`, or `None` when the original is already small
/// enough or cannot be decoded.
pub fn resize(original: &[u8], size: u32) -> Option<Vec<u8>> {
    let reader = ImageReader::new(Cursor::new(original)).with_guessed_format().ok()?;
    let (width, height) = reader.into_dimensions().ok()?;
    if width.max(height) <= size {
        return None;
    }
    let image = image::load_from_memory(original).ok()?;
    let scaled = image.resize(size, size, FilterType::CatmullRom).into_rgb8();
    let mut out = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    scaled.write_with_encoder(encoder).ok()?;
    Some(out.into_inner())
}

fn prune_dir(dir: &Path, at: u64, to: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok().filter(|meta| meta.is_file())?;
            Some((meta.modified().ok()?, meta.len(), entry.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|(_, len, _)| len).sum();
    if total <= at {
        return;
    }
    files.sort();
    for (_, len, path) in files {
        if total <= to {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= len;
        }
    }
}

/// Guards the format list against an `image` feature change.
pub fn can_decode_common_formats() -> bool {
    [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::WebP]
        .iter()
        .all(|f| f.reading_enabled())
}

#[cfg(test)]
mod tests;
