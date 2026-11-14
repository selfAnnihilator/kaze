use super::*;
use serde_json::json;
#[test]
fn parses_entities_numbers_versions_and_missing_fields() {
    let t = parse_track(&json!({"id":"abc","song":"Rock &amp; Roll (Live)","primary_artists":"A &#039;B&#039;","duration":"123","year":"2024","image":"https://c.saavncdn.com/a-150x150.jpg"})).unwrap();
    assert_eq!(t.title, "Rock & Roll (Live)");
    assert_eq!(t.artist_name, "A 'B'");
    assert_eq!(t.duration_secs, Some(123.0));
    assert_eq!(t.year, Some(2024));
    assert!(t.cover_art_url.unwrap().ends_with("500x500.jpg"));
    assert_eq!(t.track_number, None);
    assert!(parse_track(&json!({"id":"abc"})).is_err());
    assert!(decode_media_url("invalid", true).is_err());
}
#[test]
fn decodes_reference_envelope_without_using_drm_field() {
    let s = "ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq";
    assert!(decode_media_url(s, true).unwrap().ends_with("_320.mp4"));
    assert!(decode_media_url(s, false).unwrap().ends_with("_160.mp4"));
}
