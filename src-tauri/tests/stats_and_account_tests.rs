fn local_midnight(date: &str) -> i64 {
    use chrono::TimeZone;
    let midnight = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap().and_hms_opt(0, 0, 0).unwrap();
    chrono::Local.from_local_datetime(&midnight).earliest().unwrap().timestamp()
}

use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::playback::backend::MockAudioBackend;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn spawn_mock_auth_worker() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock server");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };
                let req_str = String::from_utf8_lossy(&buf[..n]);
                let first_line = req_str.lines().next().unwrap_or_default();

                let now = chrono::Utc::now().timestamp();
                let (status, body) = if first_line.contains("/api/auth/register") {
                    if req_str.contains("sessionuser") {
                        (200, serde_json::json!({
                            "success": true,
                            "user": {
                                "id": "user_cf_session_test",
                                "username": "sessionuser",
                                "created_at": now
                            },
                            "token": "bearer_secret_token_session",
                            "session": {
                                "id": "sess_cf_session_1",
                                "device_id": "test_device_uuid",
                                "device_name": "Test Host",
                                "client_version": "0.1.0",
                                "created_at": now,
                                "last_used_at": now,
                                "idle_expires_at": now + 30 * 86400,
                                "absolute_expires_at": now + 90 * 86400
                            }
                        }))
                    } else if req_str.contains("johndoe") {
                        (200, serde_json::json!({
                            "success": true,
                            "user": {
                                "id": "user_cf_123",
                                "username": "johndoe",
                                "created_at": now
                            },
                            "token": "bearer_secret_token_12345",
                            "session": {
                                "id": "sess_cf_1",
                                "device_id": "test_device_uuid",
                                "device_name": "Test Host",
                                "client_version": "0.1.0",
                                "created_at": now,
                                "last_used_at": now,
                                "idle_expires_at": now + 30 * 86400,
                                "absolute_expires_at": now + 90 * 86400
                            }
                        }))
                    } else {
                        (400, serde_json::json!({
                            "success": false,
                            "error": "Username must be between 3 and 50 characters"
                        }))
                    }
                } else if first_line.contains("/api/auth/login") {
                    if req_str.contains("sessionsecret123") {
                        (200, serde_json::json!({
                            "success": true,
                            "user": {
                                "id": "user_cf_session_test",
                                "username": "sessionuser",
                                "created_at": now
                            },
                            "token": "bearer_secret_token_session",
                            "session": {
                                "id": "sess_cf_session_1",
                                "device_id": "test_device_uuid",
                                "device_name": "Test Host",
                                "client_version": "0.1.0",
                                "created_at": now,
                                "last_used_at": now,
                                "idle_expires_at": now + 30 * 86400,
                                "absolute_expires_at": now + 90 * 86400
                            }
                        }))
                    } else if req_str.contains("secretpassword123") {
                        (200, serde_json::json!({
                            "success": true,
                            "user": {
                                "id": "user_cf_123",
                                "username": "johndoe",
                                "created_at": now
                            },
                            "token": "bearer_secret_token_12345",
                            "session": {
                                "id": "sess_cf_1",
                                "device_id": "test_device_uuid",
                                "device_name": "Test Host",
                                "client_version": "0.1.0",
                                "created_at": now,
                                "last_used_at": now,
                                "idle_expires_at": now + 30 * 86400,
                                "absolute_expires_at": now + 90 * 86400
                            }
                        }))
                    } else if req_str.contains("ratelimited") {
                        (429, serde_json::json!({
                            "success": false,
                            "error": "Too many requests. Please try again later."
                        }))
                    } else {
                        (401, serde_json::json!({
                            "success": false,
                            "error": "Invalid username or password"
                        }))
                    }
                } else if first_line.contains("/api/auth/logout-all") {
                    (200, serde_json::json!({
                        "success": true,
                        "revoked_count": 2
                    }))
                } else if first_line.contains("/api/auth/sessions/") && first_line.starts_with("DELETE") {
                    (200, serde_json::json!({
                        "success": true
                    }))
                } else if first_line.contains("/api/auth/sessions") && first_line.starts_with("GET") {
                    let sess_id = if req_str.contains("bearer_secret_token_session") {
                        "sess_cf_session_1"
                    } else {
                        "sess_cf_1"
                    };
                    (200, serde_json::json!({
                        "success": true,
                        "sessions": [
                            {
                                "id": sess_id,
                                "device_id": "test_device_uuid",
                                "device_name": "Test Host",
                                "client_version": "0.1.0",
                                "created_at": now,
                                "last_used_at": now,
                                "idle_expires_at": now + 30 * 86400,
                                "absolute_expires_at": now + 90 * 86400,
                                "is_current": true
                            }
                        ]
                    }))
                } else if first_line.contains("/api/auth/logout") {
                    (200, serde_json::json!({
                        "success": true
                    }))
                } else if first_line.contains("/api/auth/me") {
                    if req_str.contains("bearer_secret_token_session") {
                        (200, serde_json::json!({
                            "success": true,
                            "user": {
                                "id": "user_cf_session_test",
                                "username": "sessionuser",
                                "created_at": now
                            }
                        }))
                    } else if req_str.contains("bearer_secret_token_12345") {
                        (200, serde_json::json!({
                            "success": true,
                            "user": {
                                "id": "user_cf_123",
                                "username": "johndoe",
                                "created_at": now
                            }
                        }))
                    } else {
                        (401, serde_json::json!({
                            "success": false,
                            "error": "Unauthorized"
                        }))
                    }
                } else {
                    (404, serde_json::json!({
                        "success": false,
                        "error": "Not found"
                    }))
                };

                let body_str = body.to_string();
                let status_line = match status {
                    200 => "HTTP/1.1 200 OK",
                    400 => "HTTP/1.1 400 Bad Request",
                    401 => "HTTP/1.1 401 Unauthorized",
                    429 => "HTTP/1.1 429 Too Many Requests",
                    _ => "HTTP/1.1 404 Not Found",
                };
                let response = format!(
                    "{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status_line,
                    body_str.len(),
                    body_str
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });

    (base_url, handle)
}

#[tokio::test]
async fn test_user_signup_login_and_claim_data() {
    let (mock_url, _server_handle) = spawn_mock_auth_worker().await;

    let pool = create_in_memory_pool().await.expect("create db pool");
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), config.clone(), backend);

    // Point processor to mock worker
    processor
        .dispatch_command(Command::SetCloudServerUrl {
            url: mock_url.clone(),
        })
        .await
        .expect("set server url");

    // Initial state: no user logged in
    let user_res = processor.execute_query(Query::GetCurrentUser).await.expect("query current user");
    match user_res {
        QueryResponse::CurrentUser(val) => assert!(val.is_none()),
        _ => panic!("Unexpected response type"),
    }

    // 1. Sign up against authoritative cloud worker
    let signup_cmd = Command::SignUp {
        username: "johndoe".to_string(),
        password: "secretpassword123".to_string(),
    };
    let signup_res = processor.dispatch_command(signup_cmd).await.expect("signup ok");
    match signup_res {
        CommandResponse::UserProfile(profile) => {
            assert_eq!(profile["username"], "johndoe");
            assert_eq!(profile["id"], "user_cf_123");
        }
        _ => panic!("Expected UserProfile"),
    }

    // Verify token was stored in credentials and metadata only in SQLite
    let cred_token = music_player_backend::cloud::credentials::get_session_token("user_cf_123");
    assert_eq!(cred_token.as_deref(), Some("bearer_secret_token_12345"));

    // Verify local SQLite user record does NOT store password hash
    let (stored_hash,): (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE id = 'user_cf_123'")
        .fetch_one(&pool)
        .await
        .expect("fetch local user");
    assert_eq!(stored_hash, "", "Password hash must NOT be stored in local SQLite");

    // Verify cloud_sessions in SQLite does NOT store the raw bearer token
    let session_meta = music_player_backend::cloud::SyncManager::get_active_session(&pool)
        .await
        .expect("get session")
        .expect("session exists");
    assert_eq!(session_meta.user_id, "user_cf_123");
    assert_eq!(session_meta.username, "johndoe");

    // 2. Offline Support Test:
    // Create a new processor on the same database pool simulating app restart without network
    let offline_backend = Box::new(MockAudioBackend::new());
    let offline_processor = CoreProcessor::new_with_backend(pool.clone(), config.clone(), offline_backend);

    // Allow async startup session restoration task to finish
    let mut offline_user = None;
    for _ in 0..20 {
        if let Ok(QueryResponse::CurrentUser(Some(u))) = offline_processor.execute_query(Query::GetCurrentUser).await {
            offline_user = Some(u);
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
    }

    let u = offline_user.expect("Expected authenticated user session restored offline");
    assert_eq!(u["username"], "johndoe");
    assert_eq!(u["id"], "user_cf_123");

    // 3. Logout
    let logout_res = processor.dispatch_command(Command::Logout).await.expect("logout ok");
    assert!(matches!(logout_res, CommandResponse::Ok));

    // Token should now be purged from credentials
    let cleared_token = music_player_backend::cloud::credentials::get_session_token("user_cf_123");
    assert!(cleared_token.is_none(), "Token should be deleted from secure storage on logout");

    // Session metadata in SQLite should be cleared
    let cleared_meta = music_player_backend::cloud::SyncManager::get_active_session(&pool)
        .await
        .expect("get session");
    assert!(cleared_meta.is_none(), "Session metadata must be deleted on logout");

    // 4. Login with wrong password should fail with generic error
    let wrong_login = Command::Login {
        username: "johndoe".to_string(),
        password: "wrongpassword".to_string(),
    };
    let err = processor.dispatch_command(wrong_login).await.unwrap_err();
    assert!(
        err.to_string().contains("Invalid username or password"),
        "Expected generic error: {}",
        err
    );

    // 5. Rate limiting error handling
    let rate_limited_login = Command::Login {
        username: "ratelimited".to_string(),
        password: "wrongpassword".to_string(),
    };
    let rate_err = processor.dispatch_command(rate_limited_login).await.unwrap_err();
    assert!(
        rate_err.to_string().contains("Too many requests"),
        "Expected rate limit error: {}",
        rate_err
    );

    // 6. Login with correct password
    let login_cmd = Command::Login {
        username: "johndoe".to_string(),
        password: "secretpassword123".to_string(),
    };
    let login_res = processor.dispatch_command(login_cmd).await.expect("login ok");
    assert!(matches!(login_res, CommandResponse::UserProfile(_)));

    // Verify token re-stored in credentials
    let restored_token = music_player_backend::cloud::credentials::get_session_token("user_cf_123");
    assert_eq!(restored_token.as_deref(), Some("bearer_secret_token_12345"));
}

#[tokio::test]
async fn test_unreachable_cloud_worker_does_not_create_local_password_authority() {
    let pool = create_in_memory_pool().await.expect("create db pool");
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), config, backend);

    // Point to a non-existent port where no server is listening
    processor
        .dispatch_command(Command::SetCloudServerUrl {
            url: "http://127.0.0.1:59999".to_string(),
        })
        .await
        .expect("set server url");

    let signup_cmd = Command::SignUp {
        username: "solouser".to_string(),
        password: "password12345".to_string(),
    };

    // Must fail because cloud is sole authority and unreachable; must NOT silently create local password account
    let res = processor.dispatch_command(signup_cmd).await;
    assert!(res.is_err(), "Must fail when cloud authority is unreachable");
}

#[tokio::test]
async fn test_stats_overview_and_top_days() {
    let pool = create_in_memory_pool().await.expect("create db pool");
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), config, backend);

    // Set up test tracks first so foreign key constraints are satisfied
    let track_repo = processor.library_service().track_repo();
    use music_player_backend::database::repositories::ScannedMetadata;
    let track1_id = track_repo.save_scanned_track(ScannedMetadata {
        file_path: "/music/track1.flac".into(),
        file_size: 1000,
        modified_timestamp: 100,
        file_hash: None,
        title: "All of the Lights".into(),
        artist: Some("Kanye West".into()),
        album: Some("My Beautiful Dark Twisted Fantasy".into()),
        album_artist: None,
        genre: Some("Hip-Hop".into()),
        track_number: Some(1),
        disc_number: Some(1),
        year: Some(2010),
        duration_secs: 300.0,
        bitrate: Some(1000),
        sample_rate: Some(44100),
        format: "flac".into(),
        has_cover_art: false,
        musicbrainz_track_id: None,
    }).await.expect("save track 1");

    let track2_id = track_repo.save_scanned_track(ScannedMetadata {
        file_path: "/music/track2.flac".into(),
        file_size: 1000,
        modified_timestamp: 100,
        file_hash: None,
        title: "Heartless".into(),
        artist: Some("Kanye West".into()),
        album: Some("808s & Heartbreak".into()),
        album_artist: None,
        genre: Some("Hip-Hop".into()),
        track_number: Some(2),
        disc_number: Some(1),
        year: Some(2008),
        duration_secs: 210.0,
        bitrate: Some(1000),
        sample_rate: Some(44100),
        format: "flac".into(),
        has_cover_art: false,
        musicbrainz_track_id: None,
    }).await.expect("save track 2");

    // Record playback sessions directly via repository
    {
        use music_player_backend::database::models::PlaybackHistoryRecord;
        use music_player_backend::database::repositories::{HistoryRepository, SqliteHistoryRepository};
        let history_repo = SqliteHistoryRepository::new(pool.clone());
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let now = chrono::Utc::now().timestamp();

        let rec1 = PlaybackHistoryRecord {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: "default".to_string(),
            track_id: track1_id,
            started_at: now - 240,
            ended_at: now,
            seconds_listened: 240.0,
            percentage_listened: 0.8,
            completed: 1,
            skipped: 0,
            source: "discover".to_string(),
            playlist_id: None,
            recommendation_session_id: None,
        };
        history_repo.record_session(&rec1, &today, true).await.expect("record 1 ok");

        let rec2 = PlaybackHistoryRecord {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: "default".to_string(),
            track_id: track2_id,
            started_at: now - 180,
            ended_at: now,
            seconds_listened: 180.0,
            percentage_listened: 0.85,
            completed: 1,
            skipped: 0,
            source: "discover".to_string(),
            playlist_id: None,
            recommendation_session_id: None,
        };
        history_repo.record_session(&rec2, &today, true).await.expect("record 2 ok");
    }

    // Query stats
    let stats_res = processor.execute_query(Query::GetStatsOverview { year: None, month: None }).await.expect("stats query");
    match stats_res {
        QueryResponse::StatsOverview(val) => {
            assert!(val["daily_seconds"].as_f64().unwrap() >= 420.0);
            assert!(val["weekly_seconds"].as_f64().unwrap() >= 420.0);
            assert!(val["monthly_seconds"].as_f64().unwrap() >= 420.0);
            assert!(val["total_year_seconds"].as_f64().unwrap() >= 420.0);
            assert!(val["selected_month"].as_u64().unwrap() >= 1);

            let graph = val["monthly_graph"].as_array().expect("monthly graph array");
            assert!(graph.len() >= 28 && graph.len() <= 31);

            let songs = val["top_songs"].as_array().expect("top songs array");
            assert_eq!(songs.len(), 2);
            assert_eq!(songs[0]["artist_name"], "Kanye West");

            let artists = val["top_artists"].as_array().expect("top artists array");
            assert_eq!(artists.len(), 1);
            assert_eq!(artists[0]["name"], "Kanye West");

            let days = val["top_days"].as_array().expect("top days array");
            assert_eq!(days.len(), 1);
            assert!(days[0]["total_seconds"].as_f64().unwrap() >= 420.0);
        }
        _ => panic!("Expected StatsOverview response"),
    }
}

#[test]
fn test_query_serde_deserialization() {
    let s2 = r#"{"query":"GetStatsOverview","payload":{}}"#;
    let res2: Result<Query, _> = serde_json::from_str(s2);
    assert!(matches!(res2, Ok(Query::GetStatsOverview { year: None, month: None })));
}

#[tokio::test]
async fn test_session_management_lifecycle_and_hybrid_expiry() {
    let (mock_url, _server_handle) = spawn_mock_auth_worker().await;
    let pool = create_in_memory_pool().await.expect("create db pool");
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), config.clone(), backend);

    processor
        .dispatch_command(Command::SetCloudServerUrl {
            url: mock_url.clone(),
        })
        .await
        .expect("set server url");

    // 1. Initial session state should be signed_out
    let state_res = processor.execute_query(Query::GetSessionState).await.expect("query session state");
    match state_res {
        QueryResponse::SessionState(val) => {
            assert_eq!(val["state"], "signed_out");
        }
        _ => panic!("Expected SessionState"),
    }

    // 2. Login
    let login_cmd = Command::Login {
        username: "sessionuser".to_string(),
        password: "sessionsecret123".to_string(),
    };
    processor.dispatch_command(login_cmd).await.expect("login ok");

    // 3. State should now be online_authenticated
    let state_res = processor.execute_query(Query::GetSessionState).await.expect("query session state");
    match state_res {
        QueryResponse::SessionState(val) => {
            assert_eq!(val["state"], "online_authenticated");
            assert_eq!(val["payload"]["user"]["username"], "sessionuser");
            assert_eq!(val["payload"]["session_id"], "sess_cf_session_1");
        }
        _ => panic!("Expected SessionState"),
    }

    // 4. Cloud sync status should reflect session metadata
    let sync_res = processor.execute_query(Query::GetCloudSyncStatus).await.expect("sync status query");
    match sync_res {
        QueryResponse::CloudSyncStatus(val) => {
            assert_eq!(val["connected"], true);
            assert_eq!(val["username"], "sessionuser");
            assert_eq!(val["session_id"], "sess_cf_session_1");
            assert!(val["idle_expires_at"].as_i64().unwrap() > 0);
            assert!(val["absolute_expires_at"].as_i64().unwrap() > 0);
        }
        _ => panic!("Expected CloudSyncStatus"),
    }

    // 5. ListSessions query returns active session list
    let list_res = processor.execute_query(Query::ListSessions).await.expect("list sessions query");
    match list_res {
        QueryResponse::Sessions(sessions) => {
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0]["id"], "sess_cf_session_1");
            assert_eq!(sessions[0]["is_current"], true);
        }
        _ => panic!("Expected Sessions"),
    }

    // 6. Data preservation test: create local playlist, then logout all devices
    let create_pl = Command::CreatePlaylist {
        name: "Preserved Favorites".to_string(),
        description: Some("Should survive logout".to_string()),
    };
    processor.dispatch_command(create_pl).await.expect("create playlist ok");

    // Verify playlist exists
    let pl_res = processor.execute_query(Query::GetPlaylists).await.expect("get playlists");
    match pl_res {
        QueryResponse::Playlists(pls) => {
            assert_eq!(pls.len(), 2);
            assert!(pls.iter().any(|p| p["name"] == "Preserved Favorites"));
            assert!(pls.iter().any(|p| p["name"] == "Liked Songs"));
        }
        _ => panic!("Expected Playlists"),
    }

    // 7. LogoutAll
    let logout_all_res = processor.dispatch_command(Command::LogoutAll).await.expect("logout-all ok");
    assert!(matches!(logout_all_res, CommandResponse::Ok));

    // Session state should now be signed_out
    let state_after = processor.execute_query(Query::GetSessionState).await.expect("query session state");
    match state_after {
        QueryResponse::SessionState(val) => {
            assert_eq!(val["state"], "signed_out");
        }
        _ => panic!("Expected SessionState"),
    }

    // Secure token must be purged
    assert!(music_player_backend::cloud::credentials::get_session_token("user_cf_session_test").is_none());

    // User custom playlists must be cleared locally on logout so they are not visible to signed-out or subsequent users
    let pl_after = processor.execute_query(Query::GetPlaylists).await.expect("get playlists after logout");
    match pl_after {
        QueryResponse::Playlists(pls) => {
            assert!(pls.iter().all(|p| p["is_smart_mix"] == 1), "Custom user playlists must be cleared on logout");
        }
        _ => panic!("Expected Playlists"),
    }
}

#[tokio::test]
async fn test_cloudinary_avatar_metadata_persistence_and_profile() {
    use music_player_backend::database::repositories::user_repo::{SqliteUserRepository, UserRepository};

    let pool = create_in_memory_pool().await.expect("in memory pool");
    let user_repo = SqliteUserRepository::new(pool.clone());

    // 1. Create user
    let user = user_repo.create_user("cloudinary_user", "password123").await.expect("create user");
    assert_eq!(user.username, "cloudinary_user");
    assert!(user.avatar_public_id.is_none());
    assert!(user.avatar_url.is_none());
    assert!(user.avatar_version.is_none());

    // 2. Update avatar metadata with Cloudinary fields
    let now = chrono::Utc::now().timestamp();
    user_repo.update_avatar_metadata(
        &user.id,
        Some("avatars/test.webp"),
        Some("music-player/avatars/user_test_123"),
        Some("https://res.cloudinary.com/test-cloud/image/upload/v1710000000/music-player/avatars/user_test_123.webp"),
        Some(1710000000),
        Some(now),
    ).await.expect("update avatar metadata");

    // 3. Verify get_user_by_id returns all Cloudinary metadata
    let fetched = user_repo.get_user_by_id(&user.id).await.expect("get user").expect("user exists");
    assert_eq!(fetched.avatar_key.as_deref(), Some("avatars/test.webp"));
    assert_eq!(fetched.avatar_public_id.as_deref(), Some("music-player/avatars/user_test_123"));
    assert_eq!(fetched.avatar_url.as_deref(), Some("https://res.cloudinary.com/test-cloud/image/upload/v1710000000/music-player/avatars/user_test_123.webp"));
    assert_eq!(fetched.avatar_version, Some(1710000000));
    assert_eq!(fetched.avatar_updated_at, Some(now));

    // 4. Remove avatar metadata
    user_repo.update_avatar_metadata(
        &user.id,
        None,
        None,
        None,
        None,
        None,
    ).await.expect("clear avatar metadata");

    let cleared = user_repo.get_user_by_id(&user.id).await.expect("get user").expect("user exists");
    assert!(cleared.avatar_key.is_none());
    assert!(cleared.avatar_public_id.is_none());
    assert!(cleared.avatar_url.is_none());
    assert!(cleared.avatar_version.is_none());
    assert!(cleared.avatar_updated_at.is_none());
}

async fn setup_test_track_for_playback(processor: &CoreProcessor, title: &str) -> String {
    use music_player_backend::database::repositories::ScannedMetadata;
    let track_repo = processor.library_service().track_repo();
    track_repo
        .save_scanned_track(ScannedMetadata {
            file_path: format!("/music/{}.flac", title),
            file_size: 1500,
            modified_timestamp: 100,
            file_hash: None,
            title: title.into(),
            artist: Some("Test Artist".into()),
            album: Some("Test Album".into()),
            album_artist: None,
            genre: Some("Electronic".into()),
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(2026),
            duration_secs: 240.0,
            bitrate: Some(1000),
            sample_rate: Some(44100),
            format: "flac".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save test track")
}

#[tokio::test]
async fn test_authenticated_playback_writes_account_user_id() {
    use music_player_backend::core::event::Event;
    use music_player_backend::database::repositories::UserProfile;
    use std::time::Duration;

    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), AppConfig::default_with_dirs(), mock_backend);

    let track_id = setup_test_track_for_playback(&processor, "auth_song_1").await;
    let auth_user_id = "103cc229-6f41-4da2-abb9-beba57ef0367";

    // Set authenticated user
    *processor.current_user.write().await = Some(UserProfile::new(
        auth_user_id.to_string(),
        "zanken".to_string(),
        chrono::Utc::now().timestamp(),
    ));

    // Play track
    let sid_auth = "session-auth-test".to_string();
    processor.event_bus().publish(Event::PlaybackStarted {
        session_id: sid_auth.clone(),
        track_id: track_id.clone(),
        title: "Test Track".into(),
        artist: "Test Artist".into(),
        duration_secs: 240.0,
        source: "library".into(),
    }).unwrap();

    processor.event_bus().publish(Event::PlaybackPositionChanged {
        session_id: sid_auth.clone(),
        position_secs: 60.0,
        duration_secs: 240.0,
    }).unwrap();

    processor.event_bus().publish(Event::TrackFinished {
        session_id: sid_auth.clone(),
        track_id: track_id.clone(),
        seconds_listened: 60.0,
        completed: false,
    }).unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify playback_history
    let history_rows: Vec<(String, String, f64)> = sqlx::query_as(
        "SELECT id, user_id, seconds_listened FROM playback_history WHERE track_id = ?"
    )
    .bind(&track_id)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(history_rows.len(), 1, "Should have exactly 1 playback_history record");
    assert_eq!(history_rows[0].1, auth_user_id, "playback_history.user_id must match authenticated account ID");

    // Verify track_statistics
    let stats_rows: Vec<(String, i64, f64)> = sqlx::query_as(
        "SELECT user_id, play_count, total_time_listened FROM track_statistics WHERE track_id = ?"
    )
    .bind(&track_id)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(stats_rows.len(), 1, "Should have exactly 1 track_statistics record");
    assert_eq!(stats_rows[0].0, auth_user_id, "track_statistics.user_id must match authenticated account ID");
    assert_eq!(stats_rows[0].1, 1, "play_count must be 1");
    assert!((stats_rows[0].2 - 60.0).abs() < 0.01, "total_time_listened must be 60.0");

    // Confirm NO default records were written
    let default_history: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM playback_history WHERE user_id = 'default'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(default_history, 0, "No records should have been written to 'default'");

    let default_stats: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM track_statistics WHERE user_id = 'default'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(default_stats, 0, "No track_statistics should have been written to 'default'");
}

#[tokio::test]
async fn test_guest_playback_remains_guest_default() {
    use music_player_backend::core::event::Event;
    use std::time::Duration;

    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), AppConfig::default_with_dirs(), mock_backend);

    let track_id = setup_test_track_for_playback(&processor, "guest_song_1").await;

    // Explicitly unauthenticated / guest
    *processor.current_user.write().await = None;

    let sid_guest = "session-guest-test".to_string();
    processor.event_bus().publish(Event::PlaybackStarted {
        session_id: sid_guest.clone(),
        track_id: track_id.clone(),
        title: "Test Track".into(),
        artist: "Test Artist".into(),
        duration_secs: 240.0,
        source: "library".into(),
    }).unwrap();

    processor.event_bus().publish(Event::PlaybackPositionChanged {
        session_id: sid_guest.clone(),
        position_secs: 45.0,
        duration_secs: 240.0,
    }).unwrap();

    processor.event_bus().publish(Event::TrackFinished {
        session_id: sid_guest.clone(),
        track_id: track_id.clone(),
        seconds_listened: 45.0,
        completed: false,
    }).unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify playback_history wrote to "default"
    let history_user_id: String = sqlx::query_scalar(
        "SELECT user_id FROM playback_history WHERE track_id = ?"
    )
    .bind(&track_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(history_user_id, "default", "Guest playback must write user_id = 'default'");

    // Verify track_statistics wrote to "default"
    let stats_user_id: String = sqlx::query_scalar(
        "SELECT user_id FROM track_statistics WHERE track_id = ?"
    )
    .bind(&track_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stats_user_id, "default", "Guest track_statistics must write user_id = 'default'");
}

#[tokio::test]
async fn test_user_switch_during_active_session_retains_session_owner() {
    use music_player_backend::core::event::Event;
    use music_player_backend::database::repositories::UserProfile;
    use std::time::Duration;

    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), AppConfig::default_with_dirs(), mock_backend);

    let track_id = setup_test_track_for_playback(&processor, "switch_song_1").await;
    let user_a = "user_initial_aaa_111";
    let user_b = "user_switched_bbb_222";

    // Playback begins under User A
    *processor.current_user.write().await = Some(UserProfile::new(
        user_a.to_string(),
        "UserA".to_string(),
        chrono::Utc::now().timestamp(),
    ));

    let sid_switch = "session-switch-test".to_string();
    processor.event_bus().publish(Event::PlaybackStarted {
        session_id: sid_switch.clone(),
        track_id: track_id.clone(),
        title: "Test Track".into(),
        artist: "Test Artist".into(),
        duration_secs: 240.0,
        source: "library".into(),
    }).unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    processor.event_bus().publish(Event::PlaybackPositionChanged {
        session_id: sid_switch.clone(),
        position_secs: 30.0,
        duration_secs: 240.0,
    }).unwrap();

    // User switches to User B in the middle of playback
    *processor.current_user.write().await = Some(UserProfile::new(
        user_b.to_string(),
        "UserB".to_string(),
        chrono::Utc::now().timestamp(),
    ));

    processor.event_bus().publish(Event::PlaybackPositionChanged {
        session_id: sid_switch.clone(),
        position_secs: 50.0,
        duration_secs: 240.0,
    }).unwrap();

    // Track completes
    processor.event_bus().publish(Event::TrackFinished {
        session_id: sid_switch.clone(),
        track_id: track_id.clone(),
        seconds_listened: 50.0,
        completed: false,
    }).unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // The session must be attributed to User A who started the playback
    let history_owner: String = sqlx::query_scalar(
        "SELECT user_id FROM playback_history WHERE track_id = ?"
    )
    .bind(&track_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(history_owner, user_a, "ActiveSession owner captured at start must be retained despite user switch");

    // track_statistics should be attributed to User A
    let stats_owner: String = sqlx::query_scalar(
        "SELECT user_id FROM track_statistics WHERE track_id = ?"
    )
    .bind(&track_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stats_owner, user_a, "Track statistics must be attributed to session owner User A");

    // Confirm 0 records for User B
    let user_b_records: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM playback_history WHERE user_id = ?"
    )
    .bind(user_b)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(user_b_records, 0, "No records should be attributed to switched User B");
}

#[tokio::test]
async fn test_legacy_user_stats_migration_and_stats_overview() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let test_db_path = temp_dir.path().join("test_migration.db");

    // Copy live backup to test_db_path
    let backup_path = std::path::Path::new("/home/abhi/.local/share/music-player/music_player.db.backup_20260917_125601");
    if !backup_path.exists() {
        println!("Live backup file not found, skipping backup-based test");
        return;
    }
    std::fs::copy(backup_path, &test_db_path).expect("copy backup to test path");

    let canonical_user = "103cc229-6f41-4da2-abb9-beba57ef0367";

    // Run init_db_pool which executes sqlx::migrate!("./migrations")
    let pool = music_player_backend::database::init_db_pool(&test_db_path)
        .await
        .expect("init database and run migration");

    let applied: Vec<(i64, String)> = sqlx::query_as("SELECT version, description FROM _sqlx_migrations ORDER BY version")
        .fetch_all(&pool)
        .await
        .unwrap();
    println!("Applied migrations count: {}", applied.len());
    for (v, d) in &applied {
        println!("Migration: {} - {}", v, d);
    }

    // 1. Verify playback_history reassignment
    let default_history: i64 = sqlx::query_scalar("SELECT count(*) FROM playback_history WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_history: i64 = sqlx::query_scalar("SELECT count(*) FROM playback_history WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_history, 0, "All default playback history must be reassigned");
    assert_eq!(user_history, 44, "Canonical user must have exactly 44 playback history records");

    // 2. Verify track_statistics collisions and reassignment
    let default_tracks: i64 = sqlx::query_scalar("SELECT count(*) FROM track_statistics WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_tracks: i64 = sqlx::query_scalar("SELECT count(*) FROM track_statistics WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_tracks, 0, "All default track_statistics must be merged/reassigned");
    assert_eq!(user_tracks, 65, "Canonical user must have exactly 65 distinct tracks");

    // Check collision 1: ANUBIS (7f4c5f7b-a597-4af9-9f94-a7a74137e119)
    let anubis_row: (i64, f64, i64, i64, i64) = sqlx::query_as(
        "SELECT play_count, total_time_listened, completion_count, skip_count, last_played_at FROM track_statistics WHERE user_id = ? AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'"
    )
    .bind(canonical_user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(anubis_row.0, 14, "ANUBIS play_count should be 14 + 0 = 14");
    assert!((anubis_row.1 - 1817.11).abs() < 0.01, "ANUBIS seconds should be 1814.73 + 2.38 = 1817.11");
    assert_eq!(anubis_row.2, 14, "ANUBIS completion_count should be 14 + 0 = 14");
    assert_eq!(anubis_row.3, 2, "ANUBIS skip_count should be 1 + 1 = 2");
    assert_eq!(anubis_row.4, 1789626159, "ANUBIS last_played_at should be MAX (1789626159)");

    // Check collision 2: HEAVENLY JUMPSTYLE (dca9bb9a-4e0c-4437-a54b-2f793280e698)
    let hj_row: (i64, f64, i64, i64, i64) = sqlx::query_as(
        "SELECT play_count, total_time_listened, completion_count, skip_count, last_played_at FROM track_statistics WHERE user_id = ? AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'"
    )
    .bind(canonical_user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(hj_row.0, 63, "HJ play_count should be 63 + 0 = 63");
    assert!((hj_row.1 - 7174.49).abs() < 0.01, "HJ seconds should be 7165.26 + 9.23 = 7174.49");
    assert_eq!(hj_row.2, 63, "HJ completion_count should be 63 + 0 = 63");
    assert_eq!(hj_row.3, 1, "HJ skip_count should be 0 + 1 = 1");
    assert_eq!(hj_row.4, 1789626169, "HJ last_played_at should be MAX (1789626169)");

    // Check aggregate totals
    let agg: (i64, f64, i64, i64) = sqlx::query_as(
        "SELECT SUM(play_count), SUM(total_time_listened), SUM(completion_count), SUM(skip_count) FROM track_statistics WHERE user_id = ?"
    )
    .bind(canonical_user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(agg.0, 765, "Total plays should be 765");
    assert!((agg.1 - 85087.68).abs() < 0.05, "Total seconds should be ~85087.68");
    assert_eq!(agg.2, 751, "Total completions should be 751");
    assert_eq!(agg.3, 72, "Total skips should be 72");

    // 3. Verify artist, genre, preferences
    let default_artists: i64 = sqlx::query_scalar("SELECT count(*) FROM artist_statistics WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_artists: i64 = sqlx::query_scalar("SELECT count(*) FROM artist_statistics WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_artists, 0);
    assert_eq!(user_artists, 2);

    let default_genres: i64 = sqlx::query_scalar("SELECT count(*) FROM genre_statistics WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_genres: i64 = sqlx::query_scalar("SELECT count(*) FROM genre_statistics WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_genres, 0);
    assert_eq!(user_genres, 1);

    let default_prefs: i64 = sqlx::query_scalar("SELECT count(*) FROM user_preferences WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_prefs: i64 = sqlx::query_scalar("SELECT count(*) FROM user_preferences WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_prefs, 0);
    assert_eq!(user_prefs, 24);

    // 4. Verify yearly_stats_archive row preserved
    let archive_count: i64 = sqlx::query_scalar("SELECT count(*) FROM yearly_stats_archive WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(archive_count, 1, "Yearly stats archive must remain untouched");

    // 5. Test idempotency: re-run migration or re-init database
    drop(pool);
    let pool2 = music_player_backend::database::init_db_pool(&test_db_path)
        .await
        .expect("re-init database (idempotency check)");

    let user_history2: i64 = sqlx::query_scalar("SELECT count(*) FROM playback_history WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool2)
        .await
        .unwrap();
    assert_eq!(user_history2, 44, "Idempotency check: playback_history unchanged");

    let user_tracks2: i64 = sqlx::query_scalar("SELECT count(*) FROM track_statistics WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool2)
        .await
        .unwrap();
    assert_eq!(user_tracks2, 65, "Idempotency check: track_statistics unchanged");

    // 6. Test StatsRepository overview query for canonical user
    use music_player_backend::database::repositories::stats_repo::{SqliteStatsRepository, StatsRepository};
    let stats_repo = SqliteStatsRepository::new(pool2.clone());
    let weights = music_player_backend::config::RankingWeightsConfig::default();
    let overview = stats_repo.get_stats_overview(
        canonical_user,
        1700000000, // joined date
        1700000000, // app start date
        None,
        None,
        &weights,
    )
    .await
    .expect("get stats overview");

    println!("Migrated Stats Overview: daily={}, weekly={}, monthly={}, yearly={}, lifetime={}",
        overview.daily_seconds,
        overview.weekly_seconds,
        overview.monthly_seconds,
        overview.total_year_seconds,
        overview.lifetime_seconds,
    );
    println!("History started at: {:?}", overview.history_started_at);
    println!("Top tracks count: {}", overview.top_songs.len());
    println!("Top artists count: {}", overview.top_artists.len());

    assert!(overview.top_songs.len() > 0, "Top songs must not be empty");
    assert!(overview.top_artists.len() > 0, "Top artists must not be empty");
    assert!((overview.lifetime_seconds - 85087.68).abs() < 0.1, "Lifetime seconds must match track_statistics total ~85087.68");
    assert!((overview.total_year_seconds - 230.60).abs() < 0.1, "Yearly seconds must strictly match 2026 playback_history total (230.6s)");
    assert_eq!(overview.history_started_at, Some(local_midnight("2026-09-16")), "History started at must match earliest day (Sep 16, 2026 midnight)");
    assert_eq!(overview.top_songs[0].title, "PHONKY TOWN", "Top song #1 must be PHONKY TOWN from track_statistics");
    assert_eq!(overview.top_artists[0].name, "Playaphonk", "Top artist #1 must be Playaphonk from track_statistics");
}

#[tokio::test]
#[ignore = "manual production migration with stale fixture totals; never run against live user data in the test suite"]
async fn run_migration_on_live_database() {
    let live_db_path = std::path::Path::new("/home/abhi/.local/share/music-player/music_player.db");
    assert!(live_db_path.exists(), "Live database must exist");

    let canonical_user = "103cc229-6f41-4da2-abb9-beba57ef0367";

    let pool = music_player_backend::database::init_db_pool(live_db_path)
        .await
        .expect("apply migration to live database");

    // 1. Verify playback_history
    let default_history: i64 = sqlx::query_scalar("SELECT count(*) FROM playback_history WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_history: i64 = sqlx::query_scalar("SELECT count(*) FROM playback_history WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_history, 0, "All default playback history must be reassigned");
    assert_eq!(user_history, 44, "Canonical user must have exactly 44 playback history records");

    // 2. Verify track_statistics
    let default_tracks: i64 = sqlx::query_scalar("SELECT count(*) FROM track_statistics WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_tracks: i64 = sqlx::query_scalar("SELECT count(*) FROM track_statistics WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_tracks, 0, "All default track_statistics must be merged/reassigned");
    assert_eq!(user_tracks, 65, "Canonical user must have exactly 65 distinct tracks");

    // Check collisions
    let anubis_row: (i64, f64, i64, i64, i64) = sqlx::query_as(
        "SELECT play_count, total_time_listened, completion_count, skip_count, last_played_at FROM track_statistics WHERE user_id = ? AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'"
    )
    .bind(canonical_user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(anubis_row.0, 14);
    assert!((anubis_row.1 - 1817.11).abs() < 0.01);
    assert_eq!(anubis_row.2, 14);
    assert_eq!(anubis_row.3, 2);
    assert_eq!(anubis_row.4, 1789626159);

    let hj_row: (i64, f64, i64, i64, i64) = sqlx::query_as(
        "SELECT play_count, total_time_listened, completion_count, skip_count, last_played_at FROM track_statistics WHERE user_id = ? AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'"
    )
    .bind(canonical_user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(hj_row.0, 63);
    assert!((hj_row.1 - 7174.49).abs() < 0.01);
    assert_eq!(hj_row.2, 63);
    assert_eq!(hj_row.3, 1);
    assert_eq!(hj_row.4, 1789626169);

    // 3. Verify artist, genre, preferences
    let default_artists: i64 = sqlx::query_scalar("SELECT count(*) FROM artist_statistics WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_artists: i64 = sqlx::query_scalar("SELECT count(*) FROM artist_statistics WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_artists, 0);
    assert_eq!(user_artists, 2);

    let default_genres: i64 = sqlx::query_scalar("SELECT count(*) FROM genre_statistics WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_genres: i64 = sqlx::query_scalar("SELECT count(*) FROM genre_statistics WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_genres, 0);
    assert_eq!(user_genres, 1);

    let default_prefs: i64 = sqlx::query_scalar("SELECT count(*) FROM user_preferences WHERE user_id = 'default'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let user_prefs: i64 = sqlx::query_scalar("SELECT count(*) FROM user_preferences WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(default_prefs, 0);
    assert_eq!(user_prefs, 24);

    // 4. Verify yearly_stats_archive row preserved
    let archive_count: i64 = sqlx::query_scalar("SELECT count(*) FROM yearly_stats_archive WHERE user_id = ?")
        .bind(canonical_user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(archive_count, 1);

    // 5. Query StatsRepository overview for live user
    use music_player_backend::database::repositories::stats_repo::{SqliteStatsRepository, StatsRepository};
    let stats_repo = SqliteStatsRepository::new(pool.clone());
    let weights = music_player_backend::config::RankingWeightsConfig::default();
    let overview = stats_repo.get_stats_overview(
        canonical_user,
        1700000000,
        1700000000,
        None,
        None,
        &weights,
    )
    .await
    .expect("get stats overview");

    println!("Live DB Stats Overview:");
    println!("  Daily Seconds: {}", overview.daily_seconds);
    println!("  Weekly Seconds: {}", overview.weekly_seconds);
    println!("  Monthly Seconds: {}", overview.monthly_seconds);
    println!("  Yearly Seconds: {}", overview.total_year_seconds);
    println!("  Lifetime Seconds: {}", overview.lifetime_seconds);
    println!("  History Started At: {:?}", overview.history_started_at);
    println!("  Top Tracks Count: {}", overview.top_songs.len());
    println!("  Top Artists Count: {}", overview.top_artists.len());

    assert!(overview.daily_seconds > 0.0);
    assert!(overview.weekly_seconds > 0.0);
    assert!(overview.monthly_seconds > 0.0);
    assert!(overview.total_year_seconds > 0.0);
    assert!((overview.lifetime_seconds - 85087.68).abs() < 0.1);
    assert_eq!(overview.history_started_at, Some(local_midnight("2026-09-16")));
    assert_eq!(overview.top_songs[0].title, "PHONKY TOWN");
    assert_eq!(overview.top_artists[0].name, "Playaphonk");
    assert!(overview.top_songs.len() > 0);
    assert!(overview.top_artists.len() > 0);
}

#[tokio::test]
async fn test_stats_lifetime_and_all_time_rankings_architecture() {
    use music_player_backend::config::RankingWeightsConfig;
    use music_player_backend::database::repositories::stats_repo::{SqliteStatsRepository, StatsRepository};

    let pool = create_in_memory_pool().await.expect("DB pool");
    let stats_repo = SqliteStatsRepository::new(pool.clone());
    let weights = RankingWeightsConfig::default();

    let now = chrono::Utc::now().timestamp();
    let user_a = "user_audit_test_a";
    let user_b = "user_audit_test_b";

    // 1. Insert artists and tracks
    sqlx::query(
        "INSERT INTO artists (id, name, normalized_name, created_at)
         VALUES
         ('artist_1', 'Artist Alpha', 'artist alpha', ?),
         ('artist_2', 'Artist Beta', 'artist beta', ?),
         ('artist_3', 'Artist Gamma', 'artist gamma', ?)"
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert test artists");

    sqlx::query(
        "INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title, artist_id, duration_secs, format, created_at, updated_at)
         VALUES
         ('track_cloud_only', '/music/cloud.mp3', 1000, 100, 'Cloud Phonk', 'cloud phonk', 'artist_1', 200.0, 'mp3', ?, ?),
         ('track_recent_session', '/music/recent.mp3', 1000, 100, 'Recent Beat', 'recent beat', 'artist_2', 120.0, 'mp3', ?, ?),
         ('track_user_b', '/music/user_b.mp3', 1000, 100, 'User B Track', 'user b track', 'artist_3', 300.0, 'mp3', ?, ?)"
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert test tracks");

    // 2. User A:
    // - track_cloud_only in track_statistics (50 plays, 10,000.0s) -> 0 playback_history records
    // - track_recent_session in track_statistics (2 plays, 200.0s) AND 1 playback_history record (200.0s)
    sqlx::query(
        "INSERT INTO track_statistics (user_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, manual_like)
         VALUES
         (?, 'track_cloud_only', 50, 10000.0, 50, 0, ?, 1),
         (?, 'track_recent_session', 2, 200.0, 2, 0, ?, 0)"
    )
    .bind(user_a)
    .bind(now - 86400 * 30)
    .bind(user_a)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert user A track_statistics");

    let history_id_a = uuid::Uuid::new_v4().to_string();
    let history_session_start = now - 200;
    sqlx::query(
        "INSERT INTO playback_history (id, track_id, started_at, ended_at, seconds_listened, percentage_listened, completed, skipped, source, user_id)
         VALUES (?, 'track_recent_session', ?, ?, 200.0, 1.0, 1, 0, 'library', ?)"
    )
    .bind(&history_id_a)
    .bind(history_session_start)
    .bind(now)
    .bind(user_a)
    .execute(&pool)
    .await
    .expect("insert user A playback_history");

    let stat_date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let expected_history_start = local_midnight(&stat_date);
    stats_repo
        .record_daily_playback(user_a, None, &stat_date, 200.0, true, true, false)
        .await
        .expect("record daily playback");

    // 3. User B (Multi-tenant isolation):
    // - track_user_b in track_statistics (100 plays, 30,000.0s)
    sqlx::query(
        "INSERT INTO track_statistics (user_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, manual_like)
         VALUES (?, 'track_user_b', 100, 30000.0, 100, 0, ?, 0)"
    )
    .bind(user_b)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert user B track_statistics");

    // 4. Test User A Stats Overview
    let overview_a = stats_repo.get_stats_overview(
        user_a,
        now - 86400 * 365,
        now - 86400 * 365,
        None,
        None,
        &weights,
    )
    .await
    .expect("get stats overview for user A");

    // A. Verify lifetime_seconds computed strictly from track_statistics (10,000 + 200 = 10,200)
    assert_eq!(overview_a.lifetime_seconds, 10200.0, "Lifetime seconds must equal SUM(track_statistics.total_time_listened)");

    // B. Verify lifetime does NOT double-count playback_history or daily_user_stats (must NOT be 10,200 + 200 = 10,400)
    assert_ne!(overview_a.lifetime_seconds, 10400.0, "Lifetime seconds must not double count playback");

    // C. Verify time-bucketed metrics remain strictly based on daily_user_stats aggregates
    assert_eq!(overview_a.total_year_seconds, 200.0, "Yearly seconds must be based on daily_user_stats");
    assert_eq!(overview_a.daily_seconds, 200.0, "Daily seconds must be based on daily_user_stats");
    assert_eq!(overview_a.weekly_seconds, 200.0, "Weekly seconds must be based on daily_user_stats");
    assert_eq!(overview_a.monthly_seconds, 200.0, "Monthly seconds must be based on daily_user_stats");

    // D. Verify history_started_at reflects the earliest stat_date timestamp
    assert_eq!(overview_a.history_started_at, Some(expected_history_start), "history_started_at must match earliest stat_date timestamp");

    // E. Verify all-time top songs include cloud-restored tracks with 0 playback_history
    assert_eq!(overview_a.top_songs.len(), 2, "Top songs must include both track_statistics tracks");
    assert_eq!(overview_a.top_songs[0].track_id, "track_cloud_only");
    assert_eq!(overview_a.top_songs[0].title, "Cloud Phonk");
    assert_eq!(overview_a.top_songs[0].artist_name.as_deref(), Some("Artist Alpha"));
    assert_eq!(overview_a.top_songs[0].play_count, 50);
    assert_eq!(overview_a.top_songs[0].total_seconds, 10000.0);

    assert_eq!(overview_a.top_songs[1].track_id, "track_recent_session");
    assert_eq!(overview_a.top_songs[1].title, "Recent Beat");
    assert_eq!(overview_a.top_songs[1].play_count, 2);

    // F. Verify all-time top artists aggregate correctly from track_statistics
    assert_eq!(overview_a.top_artists.len(), 2);
    assert_eq!(overview_a.top_artists[0].name, "Artist Alpha");
    assert_eq!(overview_a.top_artists[0].play_count, 50);
    assert_eq!(overview_a.top_artists[0].total_seconds, 10000.0);

    assert_eq!(overview_a.top_artists[1].name, "Artist Beta");
    assert_eq!(overview_a.top_artists[1].play_count, 2);
    assert_eq!(overview_a.top_artists[1].total_seconds, 200.0);

    // 5. Test Multi-tenant User Isolation
    let overview_b = stats_repo.get_stats_overview(
        user_b,
        now,
        now,
        None,
        None,
        &weights,
    )
    .await
    .expect("get stats overview for user B");

    assert_eq!(overview_b.lifetime_seconds, 30000.0, "User B lifetime must be 30,000s");
    assert_eq!(overview_b.total_year_seconds, 0.0, "User B has no playback history");
    assert_eq!(overview_b.history_started_at, None, "User B has no playback history, so history_started_at must be None");
    assert_eq!(overview_b.top_songs.len(), 1);
    assert_eq!(overview_b.top_songs[0].track_id, "track_user_b");
    assert_eq!(overview_b.top_artists.len(), 1);
    assert_eq!(overview_b.top_artists[0].name, "Artist Gamma");

    // 6. Test Scoped Ranked Queries (All-Time vs Time-Windowed)
    // All-time (window_end is None) -> delegates to get_all_time_ranked_tracks (includes track_cloud_only)
    let all_time_ranked = stats_repo.get_ranked_tracks_scoped(Some(user_a), None, None, &weights, 10).await.expect("scoped all-time");
    assert_eq!(all_time_ranked.len(), 2);
    assert_eq!(all_time_ranked[0].track_id, "track_cloud_only");

    // Windowed (window_end is Some) -> queries playback_history only (excludes track_cloud_only)
    let windowed_ranked = stats_repo.get_ranked_tracks_scoped(Some(user_a), Some(now - 1000), Some(now + 1000), &weights, 10).await.expect("scoped windowed");
    assert_eq!(windowed_ranked.len(), 1);
    assert_eq!(windowed_ranked[0].track_id, "track_recent_session");
}

#[tokio::test]
async fn test_daily_aggregate_recording() {
    use music_player_backend::config::RankingWeightsConfig;
    use music_player_backend::database::repositories::stats_repo::{SqliteStatsRepository, StatsRepository};

    let pool = create_in_memory_pool().await.expect("DB pool");
    let stats_repo = SqliteStatsRepository::new(pool.clone());
    let weights = RankingWeightsConfig::default();

    let guest_user = "default";
    let auth_user = "user_canonical_abc";
    let other_user = "user_isolated_xyz";
    let dev_1 = "device_alpha_1";

    // 1. Playback session increments daily aggregate for guest
    let today = "2026-09-17";
    stats_repo
        .record_daily_playback(guest_user, Some(dev_1), today, 120.0, true, true, false)
        .await
        .expect("record guest playback");

    let guest_rows: Vec<(String, String, String, f64, i64, i64, i64)> = sqlx::query_as(
        "SELECT user_id, device_id, stat_date, listening_seconds, play_count, completion_count, skip_count
         FROM daily_user_stats WHERE user_id = ?"
    )
    .bind(guest_user)
    .fetch_all(&pool)
    .await
    .expect("fetch guest rows");

    assert_eq!(guest_rows.len(), 1);
    assert_eq!(guest_rows[0].0, "default");
    assert_eq!(guest_rows[0].1, dev_1);
    assert_eq!(guest_rows[0].2, today);
    assert_eq!(guest_rows[0].3, 120.0);
    assert_eq!(guest_rows[0].4, 1);
    assert_eq!(guest_rows[0].5, 1);
    assert_eq!(guest_rows[0].6, 0);

    // 2. Playback session increments daily aggregate for authenticated user using canonical ID
    stats_repo
        .record_daily_playback(auth_user, Some(dev_1), today, 100.0, true, true, false)
        .await
        .expect("record auth playback 1");

    // Same-day sessions accumulate
    stats_repo
        .record_daily_playback(auth_user, Some(dev_1), today, 50.5, true, false, true)
        .await
        .expect("record auth playback 2");

    let auth_rows: Vec<(String, String, String, f64, i64, i64, i64)> = sqlx::query_as(
        "SELECT user_id, device_id, stat_date, listening_seconds, play_count, completion_count, skip_count
         FROM daily_user_stats WHERE user_id = ? AND stat_date = ?"
    )
    .bind(auth_user)
    .bind(today)
    .fetch_all(&pool)
    .await
    .expect("fetch auth rows");

    assert_eq!(auth_rows.len(), 1);
    assert_eq!(auth_rows[0].0, auth_user);
    assert_eq!(auth_rows[0].1, dev_1);
    assert_eq!(auth_rows[0].2, today);
    assert_eq!(auth_rows[0].3, 150.5);
    assert_eq!(auth_rows[0].4, 2);
    assert_eq!(auth_rows[0].5, 1);
    assert_eq!(auth_rows[0].6, 1);

    // 3. Next-day creates a new row
    let tomorrow = "2026-09-18";
    stats_repo
        .record_daily_playback(auth_user, Some(dev_1), tomorrow, 300.0, true, true, false)
        .await
        .expect("record auth playback next day");

    let total_auth_rows: Vec<(String,)> = sqlx::query_as(
        "SELECT stat_date FROM daily_user_stats WHERE user_id = ? ORDER BY stat_date ASC"
    )
    .bind(auth_user)
    .fetch_all(&pool)
    .await
    .expect("fetch auth stat dates");

    assert_eq!(total_auth_rows.len(), 2);
    assert_eq!(total_auth_rows[0].0, today);
    assert_eq!(total_auth_rows[1].0, tomorrow);

    // 4. Record playback for other_user to test isolation
    stats_repo
        .record_daily_playback(other_user, Some(dev_1), today, 999.0, true, true, false)
        .await
        .expect("record other user");

    // User A cannot read User B
    let overview_a = stats_repo
        .get_stats_overview(auth_user, 0, 0, Some(2026), Some(9), &weights)
        .await
        .expect("get stats overview auth user");

    let overview_other = stats_repo
        .get_stats_overview(other_user, 0, 0, Some(2026), Some(9), &weights)
        .await
        .expect("get stats overview other user");

    // Auth user total year seconds = 150.5 + 300.0 = 450.5, strictly isolated from other user's 999.0
    assert_eq!(overview_a.total_year_seconds, 450.5);
    assert_eq!(overview_other.total_year_seconds, 999.0);
}

#[tokio::test]
async fn test_multi_device_daily_sync() {
    use music_player_backend::cloud::{CloudDailyStat, SyncManager, SyncPayload};
    use music_player_backend::config::RankingWeightsConfig;
    use music_player_backend::database::repositories::stats_repo::{SqliteStatsRepository, StatsRepository};

    let pool = create_in_memory_pool().await.expect("DB pool");
    let stats_repo = SqliteStatsRepository::new(pool.clone());
    let weights = RankingWeightsConfig::default();

    let user_id = "user_multidevice_test";
    let dev_a = "device_desktop";
    let dev_b = "device_laptop";
    let today = "2026-09-17";
    let now = chrono::Utc::now().timestamp();

    // 1. Device A records 3600.0s, Device B records 1000.0s on the same day
    stats_repo
        .record_daily_playback(user_id, Some(dev_a), today, 3600.0, true, true, false)
        .await
        .expect("record dev A");

    stats_repo
        .record_daily_playback(user_id, Some(dev_b), today, 1000.0, true, true, false)
        .await
        .expect("record dev B");

    // Overview sums across devices: 3600 + 1000 = 4600.0s
    let overview = stats_repo
        .get_stats_overview(user_id, 0, 0, Some(2026), Some(9), &weights)
        .await
        .expect("overview");
    assert_eq!(overview.total_year_seconds, 4600.0, "Same-day multi-device sessions must sum correctly");
    assert_eq!(overview.daily_seconds, 4600.0);

    // 2. Prepare local sync payload on Device A
    let payload = SyncManager::prepare_local_sync_payload(&pool, user_id)
        .await
        .expect("prepare sync payload");

    assert_eq!(payload.daily_stats.len(), 2);
    let dev_a_stat = payload.daily_stats.iter().find(|d| d.device_id == dev_a).expect("dev a in payload");
    assert_eq!(dev_a_stat.listening_seconds, 3600.0);
    let dev_b_stat = payload.daily_stats.iter().find(|d| d.device_id == dev_b).expect("dev b in payload");
    assert_eq!(dev_b_stat.listening_seconds, 1000.0);

    // 3. Pushing updates snapshot without duplicate sums:
    // Device A plays another 600.0s (total becomes 4200.0s)
    stats_repo
        .record_daily_playback(user_id, Some(dev_a), today, 600.0, true, true, false)
        .await
        .expect("record dev A more playback");

    let overview_updated = stats_repo
        .get_stats_overview(user_id, 0, 0, Some(2026), Some(9), &weights)
        .await
        .expect("overview updated");
    // Device A snapshot is now 4200.0s. Total is 4200 + 1000 = 5200.0s
    assert_eq!(overview_updated.total_year_seconds, 5200.0);

    // 4. Applying remote sync payload with existing/updated snapshot is idempotent
    let remote_payload = SyncPayload {
        daily_stats: vec![
            CloudDailyStat {
                user_id: user_id.to_string(),
                device_id: dev_b.to_string(),
                stat_date: today.to_string(),
                listening_seconds: 1000.0,
                play_count: 1,
                completion_count: 1,
                skip_count: 0,
                updated_at: now + 5,
            }
        ],
        ..Default::default()
    };

    // Apply remote payload twice
    SyncManager::apply_remote_sync_payload(&pool, user_id, &remote_payload)
        .await
        .expect("apply remote 1");
    SyncManager::apply_remote_sync_payload(&pool, user_id, &remote_payload)
        .await
        .expect("apply remote 2 (idempotent)");

    let overview_after_sync = stats_repo
        .get_stats_overview(user_id, 0, 0, Some(2026), Some(9), &weights)
        .await
        .expect("overview after pull");
    // Should still be exactly 5200.0s, not 6200.0s or 7200.0s!
    assert_eq!(overview_after_sync.total_year_seconds, 5200.0, "Pulling twice must be idempotent");

    // 5. Fresh/Reinstalled Device Scenario:
    // Empty local DB does NOT require playback_history to show full timeline stats!
    let fresh_pool = create_in_memory_pool().await.expect("fresh DB pool");
    let fresh_stats_repo = SqliteStatsRepository::new(fresh_pool.clone());

    // Verify playback_history is completely empty
    let history_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history")
        .fetch_one(&fresh_pool)
        .await
        .expect("count history");
    assert_eq!(history_count, 0);

    // Restore from remote sync payload
    let full_restore_payload = SyncPayload {
        daily_stats: vec![
            CloudDailyStat {
                user_id: user_id.to_string(),
                device_id: dev_a.to_string(),
                stat_date: today.to_string(),
                listening_seconds: 4200.0,
                play_count: 5,
                completion_count: 4,
                skip_count: 1,
                updated_at: now + 10,
            },
            CloudDailyStat {
                user_id: user_id.to_string(),
                device_id: dev_b.to_string(),
                stat_date: today.to_string(),
                listening_seconds: 1000.0,
                play_count: 2,
                completion_count: 2,
                skip_count: 0,
                updated_at: now + 5,
            },
            CloudDailyStat {
                user_id: user_id.to_string(),
                device_id: dev_a.to_string(),
                stat_date: "2026-09-16".to_string(),
                listening_seconds: 1800.0,
                play_count: 3,
                completion_count: 3,
                skip_count: 0,
                updated_at: now,
            }
        ],
        ..Default::default()
    };

    SyncManager::apply_remote_sync_payload(&fresh_pool, user_id, &full_restore_payload)
        .await
        .expect("apply full restore payload");

    // Verify playback_history is STILL 0 rows (pure compact sync, no raw history synced)
    let history_count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history")
        .fetch_one(&fresh_pool)
        .await
        .expect("count history after sync");
    assert_eq!(history_count_after, 0);

    // Verify timeline stats are fully restored on the new device!
    let restored_overview = fresh_stats_repo
        .get_stats_overview(user_id, 0, 0, Some(2026), Some(9), &weights)
        .await
        .expect("restored overview");

    assert_eq!(restored_overview.daily_seconds, 5200.0, "Today's daily total restored from daily_user_stats");
    assert_eq!(restored_overview.total_year_seconds, 7000.0, "Yearly total (5200 + 1800) restored from daily_user_stats");
    assert_eq!(restored_overview.monthly_seconds, 7000.0, "Monthly total restored from daily_user_stats");
    assert_eq!(restored_overview.history_started_at, Some(local_midnight("2026-09-16")), "Earliest stat_date (2026-09-16) restored as local midnight");
}

#[tokio::test]
async fn test_daily_user_stats_backfill() {
    use music_player_backend::database::repositories::stats_repo::{SqliteStatsRepository, StatsRepository};

    let pool = create_in_memory_pool().await.expect("DB pool");
    let stats_repo = SqliteStatsRepository::new(pool.clone());

    // 1. Backfill on completely empty DB succeeds with 0 rows
    let rows_empty = stats_repo
        .backfill_daily_stats_from_history(Some("test_device_uuid"))
        .await
        .expect("backfill empty");
    assert_eq!(rows_empty, 0);

    // 2. Insert test artist and tracks
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO artists (id, name, normalized_name, created_at)
         VALUES ('artist_bf', 'Backfill Artist', 'backfill artist', ?)"
    )
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert test artist");

    sqlx::query(
        "INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title, artist_id, duration_secs, format, created_at, updated_at)
         VALUES
         ('trk_1', '/music/trk1.mp3', 1000, 100, 'Track 1', 'track 1', 'artist_bf', 300.0, 'mp3', ?, ?),
         ('trk_2', '/music/trk2.mp3', 1000, 100, 'Track 2', 'track 2', 'artist_bf', 600.0, 'mp3', ?, ?)"
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert test tracks");

    // 3. Insert multi-session days into playback_history
    // Day 1: 2026-09-10 (timestamp: 1788998400) -> 3 sessions (100.0s, 200.0s, 50.0s = 350.0s)
    // Day 2: 2026-09-11 (timestamp: 1789084800) -> 2 sessions (500.0s, 300.0s = 800.0s)
    let user_id = "user_backfill_test";
    let day1_ts = 1788998400; // 2026-09-10 00:00:00 UTC
    let day2_ts = 1789084800; // 2026-09-11 00:00:00 UTC

    // Day 1 sessions
    for (i, dur) in [100.0, 200.0, 50.0].iter().enumerate() {
        let id = format!("h_d1_{}", i);
        sqlx::query(
            "INSERT INTO playback_history (id, track_id, started_at, ended_at, seconds_listened, percentage_listened, completed, skipped, source, user_id)
             VALUES (?, 'trk_1', ?, ?, ?, 1.0, 1, 0, 'library', ?)"
        )
        .bind(&id)
        .bind(day1_ts + (i as i64 * 300))
        .bind(day1_ts + (i as i64 * 300) + (*dur as i64))
        .bind(dur)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("insert day 1 session");
    }

    // Day 2 sessions
    for (i, dur) in [500.0, 300.0].iter().enumerate() {
        let id = format!("h_d2_{}", i);
        sqlx::query(
            "INSERT INTO playback_history (id, track_id, started_at, ended_at, seconds_listened, percentage_listened, completed, skipped, source, user_id)
             VALUES (?, 'trk_2', ?, ?, ?, 1.0, 1, 0, 'library', ?)"
        )
        .bind(&id)
        .bind(day2_ts + (i as i64 * 600))
        .bind(day2_ts + (i as i64 * 600) + (*dur as i64))
        .bind(dur)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("insert day 2 session");
    }

    // 3. Execute backfill
    let rows_backfilled = stats_repo
        .backfill_daily_stats_from_history(Some("test_device_uuid"))
        .await
        .expect("execute backfill");
    assert_eq!(rows_backfilled, 2, "Must aggregate into exactly 2 daily rows");

    let daily_rows: Vec<(String, f64, i64, i64)> = sqlx::query_as(
        "SELECT stat_date, listening_seconds, play_count, completion_count
         FROM daily_user_stats WHERE user_id = ? ORDER BY stat_date ASC"
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await
    .expect("fetch backfilled rows");

    assert_eq!(daily_rows.len(), 2);
    assert_eq!(daily_rows[0].1, 350.0, "Day 1 must sum all 3 sessions to 350.0s");
    assert_eq!(daily_rows[0].2, 3, "Day 1 must count 3 plays");
    assert_eq!(daily_rows[0].3, 3, "Day 1 must count 3 completions");

    assert_eq!(daily_rows[1].1, 800.0, "Day 2 must sum both sessions to 800.0s");
    assert_eq!(daily_rows[1].2, 2, "Day 2 must count 2 plays");
    assert_eq!(daily_rows[1].3, 2, "Day 2 must count 2 completions");

    // 4. Backfill is idempotent
    let rows_again = stats_repo
        .backfill_daily_stats_from_history(Some("test_device_uuid"))
        .await
        .expect("second backfill");
    assert_eq!(rows_again, 0, "Second backfill must affect 0 rows (INSERT OR IGNORE)");

    let count_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM daily_user_stats WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("count daily stats after second backfill");
    assert_eq!(count_after, 2);
}






#[tokio::test]
async fn test_phase22_correctness_suite() {
    use music_player_backend::core::event::Event;
    use music_player_backend::database::repositories::UserProfile;
    use std::time::Duration;

    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), AppConfig::default_with_dirs(), mock_backend);
    let track_id = setup_test_track_for_playback(&processor, "phase22_song").await;
    let user_id = "user_phase22_test";

    *processor.current_user.write().await = Some(UserProfile::new(
        user_id.to_string(),
        "phase22_user".to_string(),
        chrono::Utc::now().timestamp(),
    ));

    // 1. Session identity prevents stale TrackFinished from finalizing an active session
    let sid_active = "session-active-123".to_string();
    let sid_stale = "session-stale-999".to_string();

    processor.event_bus().publish(Event::PlaybackStarted {
        session_id: sid_active.clone(),
        track_id: track_id.clone(),
        title: "Phase 22 Track".into(),
        artist: "Phase 22 Artist".into(),
        duration_secs: 200.0,
        source: "library".into(),
    }).unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Stale completion event arrives with a mismatched session_id
    processor.event_bus().publish(Event::TrackFinished {
        session_id: sid_stale,
        track_id: track_id.clone(),
        seconds_listened: 200.0,
        completed: true,
    }).unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Stale event must NOT finalize the session
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "Stale session completion must be rejected");

    // Correct session completion arrives
    processor.event_bus().publish(Event::TrackFinished {
        session_id: sid_active.clone(),
        track_id: track_id.clone(),
        seconds_listened: 200.0,
        completed: true,
    }).unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "Valid session completion must be recorded");

    // 2. Recent history is scoped by user_id
    let hs = processor.history_service();
    let user_history = hs.get_recent_history(10, user_id).await.unwrap();
    assert_eq!(user_history.len(), 1);
    assert_eq!(user_history[0].track_id, track_id);

    let other_history = hs.get_recent_history(10, "other_user").await.unwrap();
    assert_eq!(other_history.len(), 0, "Recent history must be scoped to requested user");

    // 3. Multi-device track statistics SUM verification
    let dev_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM track_device_statistics WHERE user_id = ? AND track_id = ?"
    )
    .bind(user_id)
    .bind(&track_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(dev_count >= 1, "track_device_statistics must receive playback entries");
}
