use super::*;
use crate::config::AppConfig;
use crate::database::create_in_memory_pool;
use crate::playback::backend::MockAudioBackend;
use std::time::Duration;

async fn fixture() -> (Arc<CoreProcessor>, MprisPlayer, MockAudioBackend) {
    let pool = create_in_memory_pool().await.unwrap();
    for id in ["one", "two"] {
        sqlx::query("INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title, duration_secs, format, created_at, updated_at) VALUES (?, ?, 1, 1, ?, ?, 180.0, 'wav', 1, 1)")
            .bind(id).bind(format!("/test/{id}.wav")).bind(id).bind(id)
            .execute(&pool).await.unwrap();
    }
    let backend = MockAudioBackend::new();
    let processor = Arc::new(CoreProcessor::new_with_backend(
        pool,
        AppConfig::default_with_dirs(),
        Box::new(backend.clone()),
    ));
    let player = MprisPlayer::new(
        processor.playback_service(),
        Arc::new(RwLock::new(None)),
        Arc::new(RwLock::new(build_mpris_metadata(None))),
    );
    (processor, player, backend)
}

#[tokio::test]
async fn mpris_metadata_and_capabilities_follow_real_selection() {
    let (processor, player, _) = fixture().await;
    player.refresh_metadata(&processor).await;
    assert_eq!(player.playback_status().await, "Stopped");
    assert!(!player.can_play().await);
    assert!(!player.can_pause().await);
    assert!(!player.can_seek().await);
    assert!(!player.can_go_next().await);
    assert!(!player.metadata().await.contains_key("xesam:title"));
    // Merely having a populated library must not select an arbitrary song.
    player.play().await.unwrap();
    assert_eq!(player.playback_status().await, "Stopped");
    player.pause().await.unwrap();
    assert_eq!(player.playback_status().await, "Stopped");

    processor
        .playback_service()
        .play_track("one", None)
        .await
        .unwrap();
    player.refresh_metadata(&processor).await;
    let metadata = player.metadata().await;
    assert_eq!(player.playback_status().await, "Playing");
    assert!(player.can_play().await && player.can_pause().await && player.can_seek().await);
    assert!(player.can_go_next().await); // backend can refill the queue
    player.pause().await.unwrap();
    assert_eq!(player.playback_status().await, "Paused");
    assert!(player.can_play().await && player.can_pause().await && player.can_seek().await);
    assert_eq!(metadata, player.metadata().await);
    player.play().await.unwrap();
    assert_eq!(player.playback_status().await, "Playing");
    player.stop().await.unwrap();
    player.refresh_metadata(&processor).await;
    assert_eq!(player.playback_status().await, "Stopped");
    assert!(player.can_play().await);
    assert!(!player.can_pause().await && !player.can_seek().await);
    assert_eq!(metadata, player.metadata().await);
    processor.playback_service().clear_queue().await;
    player.refresh_metadata(&processor).await;
    assert!(!player.can_play().await);
    assert!(!player.metadata().await.contains_key("xesam:title"));
    player.play_pause().await.unwrap();
    assert_eq!(player.playback_status().await, "Stopped");
}

#[tokio::test]
async fn mpris_seeking_and_navigation_use_backend_identity() {
    let (processor, player, _) = fixture().await;
    let ps = processor.playback_service();
    ps.play_track("one", None).await.unwrap();
    ps.enqueue_track("two", false).await.unwrap();
    player.refresh_metadata(&processor).await;
    player.seek(10_000_000).await.unwrap();
    assert_eq!(player.position().await, 10_000_000);
    player
        .set_position(
            ObjectPath::try_from(track_id_to_object_path("two")).unwrap(),
            20_000_000,
        )
        .await
        .unwrap();
    assert_eq!(player.position().await, 10_000_000);
    player
        .set_position(
            ObjectPath::try_from(track_id_to_object_path("one")).unwrap(),
            20_000_000,
        )
        .await
        .unwrap();
    assert_eq!(player.position().await, 20_000_000);
    player
        .set_position(
            ObjectPath::try_from(track_id_to_object_path("one")).unwrap(),
            -1,
        )
        .await
        .unwrap();
    assert_eq!(player.position().await, 20_000_000);
    let mut events = processor.event_bus().subscribe();
    player.next().await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if matches!(events.recv().await.unwrap(), Event::PlaybackStarted { track_id, .. } if track_id == "two") { break; }
        }
    }).await.unwrap();
    // Metadata may still be catching up; old TrackId must never seek the new track.
    player
        .set_position(
            ObjectPath::try_from(track_id_to_object_path("one")).unwrap(),
            30_000_000,
        )
        .await
        .unwrap();
    assert_eq!(player.position().await, 0);
    player.previous().await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if matches!(events.recv().await.unwrap(), Event::PlaybackStarted { track_id, .. } if track_id == "one") { break; }
        }
    }).await.unwrap();
    player.stop().await.unwrap();
    player.seek(1_000_000).await.unwrap();
    assert_eq!(player.position().await, 0);
}

#[tokio::test]
#[ignore = "requires a private dbus-run-session; run scripts/test-mpris-session.sh"]
async fn mpris_private_bus_lifecycle_and_signals() {
    use zbus::export::futures_core::Stream;
    assert_eq!(
        std::env::var("KAZE_PRIVATE_MPRIS_TEST").as_deref(),
        Ok("1"),
        "use scripts/test-mpris-session.sh to isolate this test from desktop players"
    );
    let (processor, _, backend) = fixture().await;
    let service = start_mpris_service(processor.clone(), None).await.unwrap();
    let conn = zbus::Connection::session().await.unwrap();
    let proxy = zbus::proxy::Builder::<zbus::Proxy<'_>>::new(&conn)
        .destination("org.mpris.MediaPlayer2.kaze")
        .unwrap()
        .path("/org/mpris/MediaPlayer2")
        .unwrap()
        .interface("org.mpris.MediaPlayer2.Player")
        .unwrap()
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
        .unwrap();
    let root = zbus::Proxy::new(
        &conn,
        "org.mpris.MediaPlayer2.kaze",
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2",
    )
    .await
    .unwrap();
    assert_eq!(
        root.get_property::<String>("Identity").await.unwrap(),
        "Kaze"
    );
    assert_eq!(
        root.get_property::<String>("DesktopEntry").await.unwrap(),
        "kaze"
    );
    assert!(proxy.get_property::<bool>("CanControl").await.unwrap());
    assert_eq!(
        proxy
            .get_property::<String>("PlaybackStatus")
            .await
            .unwrap(),
        "Stopped"
    );
    assert!(start_mpris_service(processor.clone(), None).await.is_err());
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")
        .unwrap()
        .member("PropertiesChanged")
        .unwrap()
        .path("/org/mpris/MediaPlayer2")
        .unwrap()
        .build();
    let mut signals = zbus::MessageStream::for_match_rule(rule, &conn, Some(128))
        .await
        .unwrap();
    processor
        .playback_service()
        .play_track("one", None)
        .await
        .unwrap();
    let signal = tokio::time::timeout(
        Duration::from_secs(2),
        std::future::poll_fn(|cx| std::pin::Pin::new(&mut signals).poll_next(cx)),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    let (_, changed, _): (String, HashMap<String, OwnedValue>, Vec<String>) =
        signal.body().deserialize().unwrap();
    assert!(!changed.contains_key("Position"));
    proxy.call::<_, _, ()>("Pause", &()).await.unwrap();
    assert_eq!(
        proxy
            .get_property::<String>("PlaybackStatus")
            .await
            .unwrap(),
        "Paused"
    );
    assert!(proxy.get_property::<bool>("CanPause").await.unwrap());
    processor
        .playback_service()
        .set_repeat_mode(crate::core::command::RepeatMode::One)
        .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let message =
                std::future::poll_fn(|cx| std::pin::Pin::new(&mut signals).poll_next(cx))
                    .await
                    .unwrap()
                    .unwrap();
            let (_, changed, _): (String, HashMap<String, OwnedValue>, Vec<String>) =
                message.body().deserialize().unwrap();
            if changed
                .get("LoopStatus")
                .and_then(|v| <&str>::try_from(v).ok())
                == Some("Track")
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    proxy.call::<_, _, ()>("Play", &()).await.unwrap();
    assert_eq!(
        proxy
            .get_property::<String>("PlaybackStatus")
            .await
            .unwrap(),
        "Playing"
    );
    backend.advance_position(Duration::from_secs(4));
    assert_eq!(
        proxy.get_property::<i64>("Position").await.unwrap(),
        4_000_000
    );
    let mut seeks = proxy.receive_signal("Seeked").await.unwrap();
    proxy
        .call::<_, _, ()>("Seek", &(1_000_000_i64,))
        .await
        .unwrap();
    let seeked = tokio::time::timeout(
        Duration::from_secs(2),
        std::future::poll_fn(|cx| std::pin::Pin::new(&mut seeks).poll_next(cx)),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(seeked.body().deserialize::<i64>().unwrap(), 5_000_000);
    // Let all real transitions drain, then prove ticks do not generate activity.
    tokio::time::sleep(Duration::from_millis(100)).await;
    while tokio::time::timeout(
        Duration::from_millis(30),
        std::future::poll_fn(|cx| std::pin::Pin::new(&mut signals).poll_next(cx)),
    )
    .await
    .is_ok()
    {}
    for _ in 0..20 {
        processor
            .event_bus()
            .publish(Event::PlaybackPositionChanged {
                session_id: "test".into(),
                position_secs: 4.0,
                duration_secs: 180.0,
            })
            .unwrap();
    }
    assert!(tokio::time::timeout(
        Duration::from_millis(350),
        std::future::poll_fn(|cx| std::pin::Pin::new(&mut signals).poll_next(cx))
    )
    .await
    .is_err());
    proxy.call::<_, _, ()>("Stop", &()).await.unwrap();
    assert_eq!(
        proxy
            .get_property::<String>("PlaybackStatus")
            .await
            .unwrap(),
        "Stopped"
    );
    assert!(!proxy.get_property::<bool>("CanSeek").await.unwrap());
    tokio::join!(service.shutdown(), service.shutdown());
    service.shutdown().await;
    let bus = zbus::fdo::DBusProxy::new(&conn).await.unwrap();
    assert!(!bus
        .name_has_owner("org.mpris.MediaPlayer2.kaze".try_into().unwrap())
        .await
        .unwrap());
    let relaunched = start_mpris_service(processor.clone(), None).await.unwrap();
    drop(relaunched);
    tokio::time::timeout(Duration::from_secs(2), async {
        while bus
            .name_has_owner("org.mpris.MediaPlayer2.kaze".try_into().unwrap())
            .await
            .unwrap()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    // Race registration with a real playback transition; final metadata catches up.
    let ps = processor.playback_service();
    let (service, played) = tokio::join!(
        start_mpris_service(processor.clone(), None),
        ps.play_track("two", None)
    );
    played.unwrap();
    let service = service.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let metadata = proxy
                .get_property::<HashMap<String, OwnedValue>>("Metadata")
                .await
                .unwrap();
            if metadata
                .get("xesam:title")
                .and_then(|v| <&str>::try_from(v).ok())
                == Some("two")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    service.shutdown().await;
}
