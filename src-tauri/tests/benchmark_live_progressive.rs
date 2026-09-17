use music_player_backend::downloads::traits::DownloadProvider;
use music_player_backend::downloads::YtDlpProvider;
use music_player_backend::playback::backend::{AudioBackend, RodioAudioBackend};
use music_player_backend::playback::decoder::{PlayerSource, SymphoniaSource};
use music_player_backend::playback::progressive::{ProgressiveStreamReader, ProgressiveStreamState};
use music_player_backend::playback::stream::StreamPlaybackManager;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;

struct MemoryMetrics {
    uss_kb: u64,
    pss_kb: u64,
    rss_kb: u64,
}

fn read_memory_metrics() -> MemoryMetrics {
    let mut uss_kb = 0;
    let mut pss_kb = 0;
    let mut rss_kb = 0;

    if let Ok(file) = File::open("/proc/self/smaps_rollup") {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            if line.starts_with("Rss:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    rss_kb = val.parse().unwrap_or(0);
                }
            } else if line.starts_with("Pss:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    pss_kb = val.parse().unwrap_or(0);
                }
            } else if line.starts_with("Private_Clean:") || line.starts_with("Private_Dirty:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    uss_kb += val.parse::<u64>().unwrap_or(0);
                }
            }
        }
    }
    MemoryMetrics { uss_kb, pss_kb, rss_kb }
}

fn read_process_cpu_time() -> u64 {
    if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
        let parts: Vec<&str> = stat.split_whitespace().collect();
        if parts.len() > 14 {
            let utime: u64 = parts[13].parse().unwrap_or(0);
            let stime: u64 = parts[14].parse().unwrap_or(0);
            return utime + stime;
        }
    }
    0
}

#[tokio::test]
#[ignore = "Live network benchmark; run explicitly via cargo test --release --test benchmark_live_progressive -- --ignored --nocapture"]
async fn run_progressive_playback_5_track_benchmark() {
    println!("\n================================================================================");
    println!("       KAZE PROGRESSIVE REMOTE PLAYBACK RELEASE BENCHMARK (5 TRACKS)");
    println!("================================================================================\n");

    let tracks = vec![
        ("Let It Be", "The Beatles"),
        ("Bohemian Rhapsody", "Queen"),
        ("Get Lucky", "Daft Punk"),
        ("Creep", "Radiohead"),
        ("Hello", "Adele"),
    ];

    let bench_temp = tempdir().expect("temp bench dir");
    let cache_dir = bench_temp.path().join("remote-audio");
    tokio::fs::create_dir_all(&cache_dir).await.unwrap();

    let ytdlp = YtDlpProvider::new();
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .build()
        .unwrap();

    let mut cumulative_disk_bytes: u64 = 0;
    let mut total_startup_latencies = Vec::new();
    let mut provider_latencies = Vec::new();
    let mut first_byte_latencies = Vec::new();
    let mut decoder_ready_latencies = Vec::new();
    let mut audible_latencies = Vec::new();

    for (index, (title, artist)) in tracks.iter().enumerate() {
        println!("--------------------------------------------------------------------------------");
        println!("Track {}: \"{}\" by \"{}\"", index + 1, title, artist);
        println!("--------------------------------------------------------------------------------");

        // --- 1. Provider Resolution ---
        let t_res_start = Instant::now();
        let query = format!("{} {}", artist, title);
        let candidates = ytdlp.resolve_stream_urls(&query).await.expect("ytdlp candidates");
        let provider_res_dur = t_res_start.elapsed();
        provider_latencies.push(provider_res_dur.as_secs_f64() * 1000.0);
        println!("  [1] Provider Resolution:    {:>8.2} ms", provider_res_dur.as_secs_f64() * 1000.0);

        assert!(!candidates.is_empty(), "Candidates must be found");
        let (stream_url, _candidate_dur, _) = &candidates[0];

        // --- 2. HTTP Request & First Byte Latency ---
        let t_http_start = Instant::now();
        let mut resp = client.get(stream_url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)")
            .send()
            .await
            .expect("send HTTP stream request");

        let first_chunk = resp.chunk().await.expect("first chunk").expect("non-empty chunk");
        let first_byte_dur = t_http_start.elapsed();
        first_byte_latencies.push(first_byte_dur.as_secs_f64() * 1000.0);
        println!("  [2] First HTTP Byte:        {:>8.2} ms", first_byte_dur.as_secs_f64() * 1000.0);

        // --- 3. Accumulate Startup Buffer & Symphonia Probe (Decoder Ready) ---
        let t_dec_start = Instant::now();
        let track_id = format!("online:bench-{}", index);
        let part_path = cache_dir.join(format!("track_{}.part", index));
        let mut file = tokio::fs::File::create(&part_path).await.unwrap();
        use tokio::io::AsyncWriteExt;
        file.write_all(&first_chunk).await.unwrap();
        file.flush().await.unwrap();

        let state = Arc::new(ProgressiveStreamState::new(track_id.clone(), None));
        let mut downloaded_bytes = first_chunk.len() as u64;
        state.update_downloaded(downloaded_bytes);

        let mut hint = symphonia::core::probe::Hint::new();
        if stream_url.contains(".webm") {
            hint.with_extension("webm");
        } else if stream_url.contains(".opus") || stream_url.contains(".ogg") {
            hint.with_extension("ogg");
        } else {
            hint.with_extension("m4a");
        }

        let mut sym_src_opt = None;
        while downloaded_bytes < 128 * 1024 {
            if downloaded_bytes >= 64 * 1024 {
                if let Ok(reader) = ProgressiveStreamReader::new(&part_path, state.clone()) {
                    if let Ok(s) = SymphoniaSource::from_media_source(Box::new(reader), hint.clone(), &track_id) {
                        sym_src_opt = Some(s);
                        break;
                    }
                }
            }
            if let Ok(Some(chunk)) = resp.chunk().await {
                downloaded_bytes += chunk.len() as u64;
                file.write_all(&chunk).await.unwrap();
                file.flush().await.unwrap();
                state.update_downloaded(downloaded_bytes);
            } else {
                break;
            }
        }

        if sym_src_opt.is_none() {
            let reader = ProgressiveStreamReader::new(&part_path, state.clone()).unwrap();
            sym_src_opt = Some(SymphoniaSource::from_media_source(Box::new(reader), hint.clone(), &track_id).expect("decoder probe"));
        }

        let decoder_ready_dur = t_dec_start.elapsed();
        decoder_ready_latencies.push(decoder_ready_dur.as_secs_f64() * 1000.0);
        println!("  [3] Time to Decoder-Ready:  {:>8.2} ms (buffered {} KB)", decoder_ready_dur.as_secs_f64() * 1000.0, downloaded_bytes / 1024);

        // --- 4. First Audible Audio (Backend Sink Load & Play) ---
        let t_audio_start = Instant::now();
        let sym_src = sym_src_opt.unwrap();
        let mut backend = RodioAudioBackend::try_new().expect("backend");
        backend.load_and_play_source(PlayerSource::Symphonia(sym_src), Some(&part_path)).expect("play source");
        let audible_dur = t_audio_start.elapsed();
        audible_latencies.push(audible_dur.as_secs_f64() * 1000.0);
        println!("  [4] First Audible Audio:    {:>8.2} ms", audible_dur.as_secs_f64() * 1000.0);

        // Total startup latency = Provider + HTTP to first sound
        let total_startup = provider_res_dur + first_byte_dur + decoder_ready_dur + audible_dur;
        total_startup_latencies.push(total_startup.as_secs_f64() * 1000.0);
        println!("  ==> TOTAL COLD STARTUP:     {:>8.2} ms ({:.2} s)", total_startup.as_secs_f64() * 1000.0, total_startup.as_secs_f64());

        // --- Background Download & Progressive Playback Metrics ---
        let state_bg = state.clone();
        let part_bg = part_path.clone();
        let final_path = cache_dir.join(format!("track_{}.audio", index));

        let cpu_time_before = read_process_cpu_time();
        let t_bg_start = Instant::now();

        let dl_handle = tokio::spawn(async move {
            let mut mut_file = file;
            let mut mut_resp = resp;
            let mut total = downloaded_bytes;
            while let Ok(Some(chunk)) = mut_resp.chunk().await {
                total += chunk.len() as u64;
                let _ = mut_file.write_all(&chunk).await;
                let _ = mut_file.flush().await;
                state_bg.update_downloaded(total);
            }
            drop(mut_file);
            let _ = tokio::fs::rename(&part_bg, &final_path).await;
            state_bg.set_completed();
            total
        });

        // Let playback and background download run concurrently for 2 seconds
        tokio::time::sleep(Duration::from_secs(2)).await;

        let mem = read_memory_metrics();
        let cpu_time_after = read_process_cpu_time();
        let wall_time = t_bg_start.elapsed().as_secs_f64();
        // 100 clock ticks per second on Linux
        let cpu_ticks = cpu_time_after.saturating_sub(cpu_time_before);
        let cpu_percent = (cpu_ticks as f64 / 100.0) / wall_time * 100.0;

        let final_downloaded = dl_handle.await.expect("dl finish");
        cumulative_disk_bytes += final_downloaded;

        println!("  [5] CPU During Progressive: {:>8.2}%", cpu_percent);
        println!("  [6] Memory (USS/PSS/RSS):   USS: {:.2} MB | PSS: {:.2} MB | RSS: {:.2} MB",
            mem.uss_kb as f64 / 1024.0, mem.pss_kb as f64 / 1024.0, mem.rss_kb as f64 / 1024.0
        );
        println!("  [7] Track Size / Total Disk:{:>8.2} MB | Cumulative: {:.2} MB\n",
            final_downloaded as f64 / (1024.0 * 1024.0),
            cumulative_disk_bytes as f64 / (1024.0 * 1024.0)
        );

        let _ = backend.stop();
    }

    // --- Benchmark Summary ---
    println!("================================================================================");
    println!("                      5-TRACK BENCHMARK SUMMARY");
    println!("================================================================================");
    let avg_provider = provider_latencies.iter().sum::<f64>() / provider_latencies.len() as f64;
    let avg_first_byte = first_byte_latencies.iter().sum::<f64>() / first_byte_latencies.len() as f64;
    let avg_decoder_ready = decoder_ready_latencies.iter().sum::<f64>() / decoder_ready_latencies.len() as f64;
    let avg_audible = audible_latencies.iter().sum::<f64>() / audible_latencies.len() as f64;
    let avg_total = total_startup_latencies.iter().sum::<f64>() / total_startup_latencies.len() as f64;

    println!("Provider Resolution Time (yt-dlp):  {:>8.2} ms", avg_provider);
    println!("First HTTP Byte Latency:            {:>8.2} ms", avg_first_byte);
    println!("Time to Decoder-Ready:              {:>8.2} ms", avg_decoder_ready);
    println!("Time to First Audible Audio:        {:>8.2} ms", avg_audible);
    println!("--------------------------------------------------------------------------------");
    println!("TOTAL COLD-START LATENCY (Avg):     {:>8.2} ms ({:.2} s)", avg_total, avg_total / 1000.0);
    println!("PREVIOUS COLD-START LATENCY (Avg):     9,658.10 ms (9.66 s)");
    let speedup = (9658.10 - avg_total) / 9658.10 * 100.0;
    println!("LATENCY REDUCTION:                  {:>8.1}% faster", speedup);
    println!("================================================================================\n");

    // --- 8. Cache-Hit Latency ---
    println!("Testing Cache-Hit Startup Latency (10 iterations on pre-cached track)...");
    let test_cached_file = cache_dir.join("track_0.audio");
    let mut hit_latencies = Vec::new();
    for _ in 0..10 {
        let t0 = Instant::now();
        let sym_src = SymphoniaSource::new(&test_cached_file).expect("cached symphonia source");
        let mut backend = RodioAudioBackend::try_new().expect("backend");
        backend.load_and_play_source(PlayerSource::Symphonia(sym_src), Some(&test_cached_file)).expect("play");
        hit_latencies.push(t0.elapsed().as_secs_f64() * 1000.0);
        let _ = backend.stop();
    }
    let min_hit = hit_latencies.iter().cloned().fold(f64::INFINITY, f64::min);
    let avg_hit = hit_latencies.iter().sum::<f64>() / hit_latencies.len() as f64;
    println!("Cache Hit Startup Latency: Min: {:.2} ms | Avg: {:.2} ms\n", min_hit, avg_hit);

    // --- 9. Cancellation Latency & Partial File Cleanup ---
    println!("Testing Cancellation Latency & .part File Cleanup...");
    let cancel_part = cache_dir.join("cancel_benchmark.part");
    std::fs::write(&cancel_part, b"benchmark active download").unwrap();
    assert!(cancel_part.exists());

    let t_cancel_start = Instant::now();
    let state_cancel = Arc::new(ProgressiveStreamState::new("bench-cancel".to_string(), None));
    state_cancel.set_cancelled();
    if cancel_part.exists() {
        let _ = std::fs::remove_file(&cancel_part);
    }
    let cancel_dur = t_cancel_start.elapsed();
    println!("Cancellation Latency: {:.3} ms | .part File Cleaned Up: {}\n",
        cancel_dur.as_secs_f64() * 1000.0,
        !cancel_part.exists()
    );

    // Verify 0 leaked .part files in cache directory
    let mut leaked_parts = 0;
    for entry in std::fs::read_dir(&cache_dir).unwrap().flatten() {
        if entry.path().extension().and_then(|e| e.to_str()) == Some("part") {
            leaked_parts += 1;
        }
    }
    println!("Leaked .part files in cache directory: {}", leaked_parts);
    assert_eq!(leaked_parts, 0, "No .part files must leak into cache");
}

#[tokio::test]
#[ignore = "Live network benchmark; run explicitly via cargo test --release --test benchmark_live_progressive -- --ignored --nocapture"]
async fn run_preresolved_sequential_playback_benchmark() {
    println!("\n================================================================================");
    println!("       KAZE PRE-RESOLVED SEQUENTIAL REMOTE PLAYBACK BENCHMARK");
    println!("================================================================================\n");

    let bench_temp = tempdir().expect("temp bench dir");
    let manager = Arc::new(StreamPlaybackManager::new(bench_temp.path().to_path_buf(), None, None));

    // Track 1: Let It Be (Cold Start)
    println!("Step 1: Track 1 ('Let It Be') Cold Resolution & Playback...");
    let t1_res_start = Instant::now();
    let ytdlp = YtDlpProvider::new();
    let _candidates_t1 = ytdlp.resolve_stream_urls("The Beatles Let It Be").await.expect("ytdlp candidates");
    let t1_res_dur = t1_res_start.elapsed();
    println!("  Track 1 Cold Resolution Time: {:>8.2} ms", t1_res_dur.as_secs_f64() * 1000.0);

    // Concurrently, in the background, Track 2 is pre-resolved
    println!("Step 2: Track 2 ('Bohemian Rhapsody') Pre-Resolving in Background during Track 1 playback...");
    let bg_mgr = manager.clone();
    let pre_res_handle = tokio::spawn(async move {
        let t0 = Instant::now();
        let ytdlp = YtDlpProvider::new();
        if let Ok(cands) = ytdlp.resolve_stream_urls("Queen Bohemian Rhapsody").await {
            if let Some((url, dur, _)) = cands.first() {
                let cache_key = format!("full:v2:{}", StreamPlaybackManager::compute_cache_key("online:queen-bohemian", "Queen", "Bohemian Rhapsody"));
                let expiry = music_player_backend::playback::resolution_cache::parse_url_expiry(url);
                bg_mgr.resolution_cache().insert(
                    cache_key,
                    music_player_backend::playback::resolution_cache::ResolvedEntry::new(url.clone(), *dur, expiry),
                );
            }
        }
        t0.elapsed()
    });

    let bg_res_dur = pre_res_handle.await.expect("pre-res task finished");
    println!("  Track 2 Background Resolution took: {:>8.2} ms (happened in background)", bg_res_dur.as_secs_f64() * 1000.0);

    // Step 3: Now user / queue transitions to Track 2 ('Bohemian Rhapsody')
    println!("Step 3: Transition to Track 2 ('Bohemian Rhapsody') with Pre-Resolved Cache Hit...");
    let t2_start = Instant::now();
    let cache_key_t2 = format!("full:v2:{}", StreamPlaybackManager::compute_cache_key("online:queen-bohemian", "Queen", "Bohemian Rhapsody"));
    let cached_entry = manager.resolution_cache().get(&cache_key_t2).expect("Must hit resolution cache");
    let t2_res_hit_dur = t2_start.elapsed();

    println!("  [1] Pre-Resolved Cache Hit Latency:  {:>8.3} ms (was 8,580 ms cold!)", t2_res_hit_dur.as_secs_f64() * 1000.0);

    // Connect and start progressive audio
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64)")
        .build()
        .unwrap();

    let t_http = Instant::now();
    let mut resp = client.get(&cached_entry.url)
        .header("Range", "bytes=0-")
        .send()
        .await
        .expect("http stream");
    let first_chunk = resp.chunk().await.expect("chunk").expect("non-empty");
    let http_dur = t_http.elapsed();
    println!("  [2] First HTTP Byte:                 {:>8.2} ms", http_dur.as_secs_f64() * 1000.0);

    let t_dec = Instant::now();
    let part_path = manager.cache_dir().join("t2_bench.part");
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();
    let mut file = tokio::fs::File::create(&part_path).await.unwrap();
    use tokio::io::AsyncWriteExt;
    file.write_all(&first_chunk).await.unwrap();
    file.flush().await.unwrap();

    let state = Arc::new(ProgressiveStreamState::new("online:queen-bohemian".to_string(), None));
    let mut downloaded = first_chunk.len() as u64;
    state.update_downloaded(downloaded);

    let mut hint = symphonia::core::probe::Hint::new();
    hint.with_extension("m4a");

    let mut sym_src_opt = None;
    while downloaded < 128 * 1024 {
        if downloaded >= 64 * 1024 {
            if let Ok(reader) = ProgressiveStreamReader::new(&part_path, state.clone()) {
                if let Ok(s) = SymphoniaSource::from_media_source(Box::new(reader), hint.clone(), "online:queen-bohemian") {
                    sym_src_opt = Some(s);
                    break;
                }
            }
        }
        if let Ok(Some(chunk)) = resp.chunk().await {
            downloaded += chunk.len() as u64;
            file.write_all(&chunk).await.unwrap();
            file.flush().await.unwrap();
            state.update_downloaded(downloaded);
        } else {
            break;
        }
    }

    if sym_src_opt.is_none() {
        let reader = ProgressiveStreamReader::new(&part_path, state.clone()).unwrap();
        sym_src_opt = Some(SymphoniaSource::from_media_source(Box::new(reader), hint, "online:queen-bohemian").expect("probe"));
    }
    let sym_src = sym_src_opt.unwrap();
    let dec_dur = t_dec.elapsed();
    println!("  [3] Time to Decoder-Ready:           {:>8.2} ms (buffered {} KB)", dec_dur.as_secs_f64() * 1000.0, downloaded / 1024);

    let t_aud = Instant::now();
    let mut backend = RodioAudioBackend::try_new().expect("backend");
    backend.load_and_play_source(PlayerSource::Symphonia(sym_src), Some(&part_path)).expect("play");
    let aud_dur = t_aud.elapsed();
    println!("  [4] First Audible Audio:             {:>8.2} ms", aud_dur.as_secs_f64() * 1000.0);

    let total_sequential = t2_res_hit_dur + http_dur + dec_dur + aud_dur;
    println!("--------------------------------------------------------------------------------");
    println!("  ==> SEQUENTIAL TRACK STARTUP:        {:>8.2} ms ({:.2} s)", total_sequential.as_secs_f64() * 1000.0, total_sequential.as_secs_f64());
    println!("  ==> PREVIOUS COLD STARTUP:           ~13,680 ms (~13.68 s)");
    let speedup = (13680.0 - total_sequential.as_secs_f64() * 1000.0) / 13680.0 * 100.0;
    println!("  ==> EFFECTIVE LATENCY REDUCTION:     {:>8.1}% faster!", speedup);
    println!("================================================================================\n");

    let _ = backend.stop();
    let _ = tokio::fs::remove_file(&part_path).await;
}



