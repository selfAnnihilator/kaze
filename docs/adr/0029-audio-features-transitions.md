# Audio features and smooth Flow transitions

Flow picked songs only by taste score and artist spacing. A slow ballad could follow a fast dance track. Spotify orders radio queues by audio attributes. Kaze had no audio descriptors at all.

Decision:

| Area | Rule |
| --- | --- |
| Analyzer | `src-tauri/src/audio_features.rs`. Symphonia decodes 60 s from the middle of the track to mono and decimates it to about 11 kHz. Energy = RMS dB mapped from −40..0 to 0..1. Brightness = zero-crossing rate / 0.25. Tempo = autocorrelation of positive onset flux over 60–180 BPM, with a log-Gaussian prior around 120 and parabolic refinement. Tempo is NULL when confidence is below 0.15. No new dependencies. |
| Storage | `track_audio_features(track_id, tempo_bpm, energy, brightness, analyzer_version, analyzed_at)`. A decode failure stores NULL energy so the track is not retried. Bumping `ANALYZER_VERSION` queues every track again. |
| Worker | `AudioFeaturesService` is spawned by the core processor. It starts after 45 s and analyzes one local track per tick, with a 2 s gap between ticks. It pauses for 20 s on buffering or an online request and for 5 s on start or error. It sleeps up to 30 min when idle and wakes on `LibraryScanCompleted`. Online tracks (`://` paths) are never analyzed. |
| Distance | Octave-tolerant tempo term `min(|log2 r|, |log2 r ± 1|) / 0.2`, capped at 1. `0.5·tempo + 0.35·energy + 0.15·brightness`; without tempo, `0.7·energy + 0.3·brightness`. |
| Diversifier | `DiversityItem.features` and `DiversityRules.transition_weight`, default 0. `diversify_from(seed)` subtracts `weight × cost(prev, cand)`. A candidate without features costs a neutral 0.5. With no previous sound the cost is 0. |
| Flow | Weight 0.10, used only by `generate_playback_tracks_seeded`. A refill seeds from the last queued track. Rerank, steering and the Flow start seed from the current track. Mixes, shelves and Discovery are unchanged. |

Alternatives:
- Spectral features through an FFT crate: better brightness, but a new dependency and more CPU for a small ordering nudge.
- Analyzing whole tracks: about 4× the decode cost for little gain.
- Treating a missing feature as zero cost (the original plan): this would make online tracks look like perfect transitions, so a neutral cost is used instead.

Consequences:
- The first full library pass takes a while (about 2–3 s per track on a laptop). Smoothing improves as coverage grows.
- Tempo octave errors (detecting half or double tempo) are absorbed by the octave-tolerant distance.
- Online songs never get features, so heavily online Flows see little smoothing.

Tests: `src-tauri/tests/audio_features_tests.rs`
- The fixture MP3 gives finite descriptors.
- A synthetic 120 BPM click track measures within ±3 BPM.
- Silence has no tempo.
- The distance is octave-tolerant.
- Weight 0 gives the same order as `diversify`, and weight > 0 picks the near-tempo neighbour.
- The worker analyzes local files, records failures and skips online tracks.
