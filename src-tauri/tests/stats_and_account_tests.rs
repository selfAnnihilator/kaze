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
    let processor = CoreProcessor::new_with_backend(pool, config, backend);

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
    let processor = CoreProcessor::new_with_backend(pool, config, backend);

    // Record some playback sessions
    let record_cmd1 = Command::RecordPlaybackSession {
        track_id: "track_1".to_string(),
        title: "All of the Lights".to_string(),
        artist: Some("Kanye West".to_string()),
        album: Some("My Beautiful Dark Twisted Fantasy".to_string()),
        duration_secs: 300.0,
        seconds_listened: 240.0,
        completed: true,
        skipped: false,
        source: "discover".to_string(),
    };
    processor.dispatch_command(record_cmd1).await.expect("record 1 ok");

    let record_cmd2 = Command::RecordPlaybackSession {
        track_id: "track_2".to_string(),
        title: "Heartless".to_string(),
        artist: Some("Kanye West".to_string()),
        album: Some("808s & Heartbreak".to_string()),
        duration_secs: 210.0,
        seconds_listened: 180.0,
        completed: true,
        skipped: false,
        source: "discover".to_string(),
    };
    processor.dispatch_command(record_cmd2).await.expect("record 2 ok");

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
            assert_eq!(pls.len(), 1);
            assert_eq!(pls[0]["name"], "Preserved Favorites");
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


