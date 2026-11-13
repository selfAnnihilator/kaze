use music_player_backend::audio_features::{
    analyze_file, analyze_samples, load_vectors, store, transition_distance, AudioFeatures,
    AudioFeaturesService, ANALYZER_VERSION,
};
use music_player_backend::core::event_bus::EventBus;
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::{
    ScannedMetadata, SqliteTrackRepository, TrackRepository,
};
use music_player_backend::recommendations::diversity::{
    diversify, diversify_from, DiversityItem, DiversityRules,
};
use std::path::PathBuf;
use std::sync::Arc;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// 10 ms decaying 1 kHz bursts every `60 / bpm` seconds.
fn click_track(bpm: f64, secs: f64, rate: u32) -> Vec<f32> {
    let total = (secs * rate as f64) as usize;
    let period = (60.0 / bpm * rate as f64) as usize;
    let burst = (0.010 * rate as f64) as usize;
    (0..total)
        .map(|i| {
            let k = i % period;
            if k < burst {
                let t = k as f64 / rate as f64;
                let decay = 1.0 - k as f64 / burst as f64;
                (0.8 * decay * (2.0 * std::f64::consts::PI * 1000.0 * t).sin()) as f32
            } else {
                0.0
            }
        })
        .collect()
}

#[test]
fn analyzer_reads_fixture_with_finite_descriptors() {
    let features = analyze_file(&fixture("seek-tone.mp3")).expect("analyze fixture");
    assert!(features.energy.is_finite() && (0.0..=1.0).contains(&features.energy));
    assert!(features.brightness.is_finite() && (0.0..=1.0).contains(&features.brightness));
    if let Some(bpm) = features.tempo_bpm {
        assert!((60.0..=180.0).contains(&bpm));
    }
}

#[test]
fn analyzer_rejects_missing_file() {
    assert!(analyze_file(&fixture("does-not-exist.mp3")).is_err());
}

#[test]
fn click_track_tempo_is_detected() {
    let samples = click_track(120.0, 60.0, 44_100);
    let features = analyze_samples(&samples, 44_100).expect("long enough");
    let bpm = features.tempo_bpm.expect("confident tempo");
    assert!((bpm - 120.0).abs() <= 3.0, "measured {bpm}");
    // Silence has no energy and no confident tempo.
    let silence = analyze_samples(&vec![0.0; 44_100 * 10], 44_100).unwrap();
    assert!(silence.energy < 0.05);
    assert!(silence.tempo_bpm.is_none());
    // Under five seconds is not analyzed.
    assert!(analyze_samples(&vec![0.0; 44_100 * 2], 44_100).is_none());
}

#[test]
fn transition_distance_is_octave_tolerant() {
    let a = AudioFeatures { tempo_bpm: Some(120.0), energy: 0.5, brightness: 0.3 }.vector();
    let double = AudioFeatures { tempo_bpm: Some(240.0), energy: 0.5, brightness: 0.3 }.vector();
    let far = AudioFeatures { tempo_bpm: Some(85.0), energy: 0.5, brightness: 0.3 }.vector();
    let no_tempo = AudioFeatures { tempo_bpm: None, energy: 0.5, brightness: 0.3 }.vector();
    assert!(transition_distance(&a, &a) < 1e-9);
    assert!(transition_distance(&a, &double) < 1e-6);
    assert!(transition_distance(&a, &far) > 0.3);
    assert!(transition_distance(&a, &no_tempo) < 1e-9);
}

fn item(artist: &str, score: f64, features: Option<[f32; 3]>) -> DiversityItem {
    DiversityItem {
        score,
        artist: artist.to_string(),
        album: None,
        is_online: false,
        max_per_window: None,
        features,
    }
}

#[test]
fn diversifier_unchanged_at_zero_weight_and_smooths_above_it() {
    let seed = [120.0_f32, 0.5, 0.3];
    let items = vec![
        item("A", 0.80, Some([90.0, 0.9, 0.7])),
        item("B", 0.78, Some([121.0, 0.5, 0.3])),
        item("C", 0.60, None),
    ];
    let rules = DiversityRules::queue(0.0);
    let plain = diversify(&items, 3, &rules, &[]);
    let seeded_off = diversify_from(&items, 3, &rules, &[], Some(seed));
    assert_eq!(plain, seeded_off);
    assert_eq!(plain[0], 0);

    let mut smooth = rules;
    smooth.transition_weight = 0.10;
    let order = diversify_from(&items, 3, &smooth, &[], Some(seed));
    assert_eq!(order[0], 1, "near-tempo neighbour should lead: {order:?}");
    // No seed: first pick is unaffected by transitions.
    let unseeded = diversify_from(&items, 3, &smooth, &[], None);
    assert_eq!(unseeded[0], 0);
}

async fn save(repo: &SqliteTrackRepository, title: &str, path: String) -> String {
    repo.save_scanned_track(ScannedMetadata {
        title_is_fallback: false,
        file_path: path,
        file_size: 2000,
        modified_timestamp: 100,
        file_hash: None,
        title: title.to_string(),
        artist: Some("Artist".to_string()),
        album: Some("Album".to_string()),
        album_artist: None,
        genre: Some("Rock".to_string()),
        track_number: Some(1),
        disc_number: Some(1),
        year: Some(2020),
        duration_secs: 10.0,
        bitrate: Some(128),
        sample_rate: Some(44100),
        format: "mp3".to_string(),
        has_cover_art: false,
        musicbrainz_track_id: None,
    })
    .await
    .expect("save track")
}

#[tokio::test]
async fn worker_analyzes_local_tracks_once_and_skips_online() {
    let pool = create_in_memory_pool().await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let real = save(&repo, "Tone", fixture("seek-tone.mp3").display().to_string()).await;
    let missing = save(&repo, "Gone", "/nowhere/gone.mp3".to_string()).await;
    let online = save(&repo, "Stream", "online://jiosaavn/abc".to_string()).await;

    let service = AudioFeaturesService::new(pool.clone(), Arc::new(EventBus::new(8)));
    while service.tick().await.unwrap() {}

    let rows: Vec<(String, Option<f64>, i64)> = sqlx::query_as(
        "SELECT track_id, energy, analyzer_version FROM track_audio_features ORDER BY track_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2, "online track must be skipped");
    assert!(rows.iter().all(|r| r.2 == ANALYZER_VERSION));
    let energy = |id: &str| rows.iter().find(|r| r.0 == id).unwrap().1;
    assert!(energy(&real).is_some());
    assert!(energy(&missing).is_none(), "failure stored as unanalyzable marker");
    assert!(rows.iter().all(|r| r.0 != online));

    // Nothing pending: the worker idles.
    assert!(!service.tick().await.unwrap());

    let vectors = load_vectors(&pool).await.unwrap();
    assert!(vectors.contains_key(&real));
    assert!(!vectors.contains_key(&missing));

    // Store overwrites in place.
    let manual = AudioFeatures { tempo_bpm: Some(100.0), energy: 0.4, brightness: 0.2 };
    store(&pool, &missing, Some(&manual)).await.unwrap();
    let vectors = load_vectors(&pool).await.unwrap();
    assert_eq!(vectors[&missing][0], 100.0);
}
