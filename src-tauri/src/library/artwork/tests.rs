use super::*;

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 90]));
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

fn dimensions(bytes: &[u8]) -> (u32, u32) {
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .unwrap()
        .into_dimensions()
        .unwrap()
}

#[test]
fn urls_round_trip_any_track_id_and_size() {
    for id in ["4f0c-uuid", "online:jiosaavn:Ab_9", "weird / id?&=é"] {
        let url = track_url(id, 3);
        let rest = url.strip_prefix("kaze-art://localhost").unwrap();
        let (path, query) = rest.split_once('?').unwrap();
        assert_eq!(parse_request(path, Some(&format!("{query}&s=200"))), Some((id.to_string(), 256)));
        assert_eq!(parse_request(path, None), Some((id.to_string(), DEFAULT_SIZE)));
    }
    assert_eq!(parse_request("/", None), None);
    assert_eq!(parse_request("/zz", None), None);
}

#[test]
fn sizes_round_up_to_a_bucket_and_cap_at_the_largest() {
    assert_eq!(bucket(1), 128);
    assert_eq!(bucket(128), 128);
    assert_eq!(bucket(129), 256);
    assert_eq!(bucket(5000), 1024);
}

#[test]
fn large_covers_are_resized_and_cached_by_content() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ThumbnailCache::new(dir.path().join("art"));
    let (bytes, mime) = cache.get(png(1400, 1000), 256);
    assert_eq!(mime, "image/jpeg");
    assert_eq!(dimensions(&bytes), (256, 183));
    let files: Vec<_> = std::fs::read_dir(dir.path().join("art")).unwrap().collect();
    assert_eq!(files.len(), 1, "one cached file per image and size");
    let (again, _) = cache.get(png(1400, 1000), 256);
    assert_eq!(again, bytes, "the same image is served from the cache");
}

#[test]
fn small_or_undecodable_covers_are_served_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ThumbnailCache::new(dir.path().to_path_buf());
    let small = png(100, 100);
    assert_eq!(cache.get(small.clone(), 512), (small, "image/png"));
    let junk = b"not an image".to_vec();
    assert_eq!(cache.get(junk.clone(), 512).0, junk);
    assert!(can_decode_common_formats());
}

#[test]
fn folder_cover_is_found_next_to_the_audio_file() {
    let dir = tempfile::tempdir().unwrap();
    let audio = dir.path().join("song.flac");
    std::fs::write(&audio, b"not audio").unwrap();
    assert_eq!(LocalArt::find(&audio), None);
    std::fs::write(dir.path().join("folder.jpg"), b"x").unwrap();
    assert_eq!(LocalArt::find(&audio), Some(LocalArt::File(dir.path().join("folder.jpg"))));
}

#[test]
fn pruning_removes_the_oldest_files_down_to_the_target() {
    let dir = tempfile::tempdir().unwrap();
    for (i, name) in ["a", "b", "c", "d"].iter().enumerate() {
        let path = dir.path().join(name);
        std::fs::write(&path, vec![0u8; 100]).unwrap();
        let time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000 + i as u64);
        std::fs::File::options().write(true).open(&path).unwrap().set_modified(time).unwrap();
    }
    prune_dir(dir.path(), 300, 200);
    let mut left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(left, ["c", "d"]);
}
