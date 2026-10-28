/** Browser-only backend fixture. Production never imports this module. */
export function installDualInterfaceFixture() {
  const covers = [
    "migration",
    "modal-soul",
    "con-todo",
    "in-rainbows",
    "moon-safari",
    "mezzanine",
    "dummy",
    "currents",
    "bloom",
    "jubilee",
  ];
  const names = [
    "Kerala",
    "Feather",
    "Friday Morning",
    "Weird Fishes",
    "All I Need",
    "Teardrop",
    "Glory Box",
    "Let It Happen",
    "Myth",
    "Paprika",
  ];
  const artists = [
    "Bonobo",
    "Nujabes",
    "Khruangbin",
    "Radiohead",
    "Air",
    "Massive Attack",
    "Portishead",
    "Tame Impala",
    "Beach House",
    "Japanese Breakfast",
  ];
  const tracks = Array.from({ length: 1036 }, (_, i) => ({
    id: `t${i}`,
    title: i < 10 ? names[i] : `Library track ${i}`,
    artist_name: artists[i % 10],
    album_title: covers[i % 10],
    duration_secs: 240,
    file_path: `/fixture/${i}.flac`,
    format: "flac",
    has_cover_art: 1,
    cover_art_url: `/design-lab/artwork/${covers[i % 10]}.jpg`,
    manual_like: i === 1 ? 1 : 0,
  }));
  const recs = tracks
    .slice(10, 45)
    .map((t, i) => ({
      external_track_id: `online:${i}`,
      provider: "fixture",
      provider_id: String(i),
      title: names[i % 10],
      artist: t.artist_name,
      album: t.album_title,
      duration_secs: 240,
      cover_art_url: t.cover_art_url,
      recommendation_reason: "Based on your favorite artists",
      match_status: "NOT_FOUND",
      in_wishlist: false,
    }));
  const playlists = [
    { id: "liked", name: "Liked Songs", is_smart_mix: 0, track_count: 1 },
    {
      id: "p1",
      name: "After hours",
      description: "A late-night selection",
      is_smart_mix: 0,
      track_count: 1036,
      cover_art_url: tracks[0].cover_art_url,
    },
    {
      id: "mix1",
      name: "Rediscovery Mix",
      description: "Your library, revisited",
      is_smart_mix: 1,
      track_count: 15,
      cover_art_url: tracks[3].cover_art_url,
    },
  ];
  const settings = {
    interface_style:
      sessionStorage.getItem("test.interface-style") || "spatial",
    close_button_behavior: "minimize_to_tray",
    database_path: "/fixture/app.db",
    cache_dir: "/fixture/cache",
    audio: { default_volume: 0.8 },
    history: { min_meaningful_seconds: 30, min_meaningful_percentage: 0.5 },
    metadata: {
      provider_priority: [],
      enable_musicbrainz: true,
      enable_spotify: false,
    },
    downloads: {
      slskd_host: "localhost",
      slskd_port: 5030,
      auto_import: true,
      max_concurrent_downloads: 3,
    },
  };
  const playback = {
    current_track: tracks[0],
    current_track_id: "t0",
    current_queue_index: 0,
    queue_track_ids: ["t1", "t2", "t3"],
    is_playing: true,
    is_paused: false,
    position_secs: 47,
    duration_secs: 240,
    volume: 0.8,
    is_muted: false,
    repeat_mode: "off",
    is_shuffled: false,
    can_seek: true,
    is_buffering: false,
  };
  const listeners = new Set(),
    commands = [],
    queries = [];
  let flow = {
    id: "fixture-flow",
    is_active: true,
    last_track_id: "t0",
    last_track_title: "Kerala",
    last_track_artist: "Bonobo",
    familiarity_ratio: 0.5,
    flow_label: "Your music, in motion",
  };
  const evidence = {
    primary: "rediscovery",
    days_since_played: 42,
    play_count: 2,
    familiarity_ratio: 0.5,
    selected_at: 1700000000,
  };
  const downloads = [
    {
      id: "d1",
      filename: "Feather.flac",
      title: "Feather",
      artist: "Nujabes",
      status: "DOWNLOADING",
      provider: "ytdlp",
      bytes_downloaded: 1048576,
      file_size: 10485760,
      destination_path: "/fixture/Feather.flac",
      created_at: 1,
    },
  ];
  const notifications = [
    {
      id: "n1",
      type: "success",
      title: "Library ready",
      message: "Your music is ready to play.",
      is_read: false,
      created_at: new Date().toISOString(),
    },
  ];
  const emit = (event) => {
    for (const fn of listeners) fn(event);
  };
  const queueEvent = () =>
    emit({
      event: "QueueUpdated",
      payload: {
        current_index: playback.current_queue_index,
        items: [
          playback.current_track,
          ...playback.queue_track_ids.map(
            (id) =>
              tracks.find((t) => t.id === id) || {
                id,
                title: id,
                artist_name: "Online artist",
              },
          ),
        ].map((t, i) => ({
          queue_id: `q${i}`,
          track_id: t.id,
          title: t.title,
          artist: t.artist_name,
          duration_secs: 240,
        })),
        queue_track_ids: playback.queue_track_ids,
      },
    });
  const f = (window.__kazeFixture = {
    tracks,
    recs,
    playlists,
    settings,
    playback,
    commands,
    queries,
    downloads,
    notifications,
    emit,
    listeners,
    failStyle: false,
    failSearch: false,
    empty: false,
    get flow() {
      return flow;
    },
    async query(q) {
      queries.push(q);
      const p = q.payload || {};
      switch (q.query) {
        case "GetSettings":
          return { type: "Settings", data: { ...settings } };
        case "GetOnboardingStatus":
          return {
            type: "OnboardingStatus",
            completed: true,
            default_music_dir: "/fixture",
            configured_folders: [
              { id: "f1", path: "/fixture", track_count: tracks.length },
            ],
          };
        case "GetPlaybackState":
          return { type: "PlaybackState", data: { ...playback } };
        case "GetTracks":
          return { type: "Tracks", data: f.empty ? [] : tracks };
        case "SearchLibrary":
          return {
            type: "Tracks",
            data: tracks.filter((t) =>
              `${t.title} ${t.artist_name}`
                .toLowerCase()
                .includes((p.query_text || "").toLowerCase()),
            ),
          };
        case "GetTrackById":
          return {
            type: "Track",
            data: tracks.find((t) => t.id === p.track_id),
          };
        case "GetAlbums":
          return {
            type: "Albums",
            data: f.empty
              ? []
              : Array.from({ length: 130 }, (_, i) => ({
                  id: `a${i}`,
                  title: covers[i % 10],
                  artist_name: artists[i % 10],
                  first_track_id: `t${i % 10}`,
                  track_count: 10,
                  cover_art_path: tracks[i % 10].cover_art_url,
                })),
          };
        case "GetAlbumTracks":
          return { type: "Tracks", data: f.empty ? [] : tracks };
        case "GetPlaylistTracks":
          return { type: "Tracks", data: f.empty ? [] : p.playlist_id === "liked" ? tracks.filter(track => track.manual_like === 1) : tracks };
        case "GetPlaylists":
          return { type: "Playlists", data: playlists };
        case "GetTrackPlaylistMemberships":
          return {
            type: "TrackPlaylistMemberships",
            data: { t0: ["p1"], t1: ["liked"] },
          };
        case "GetTrackCoverArt":
          return {
            type: "CoverArt",
            data:
              tracks.find((t) => t.id === p.track_id)?.cover_art_url ||
              tracks[0].cover_art_url,
          };
        case "GetWishlist":
          return {
            type: "Wishlist",
            data: [
              {
                id: "w1",
                title: "Comfortably Numb",
                artist: "Pink Floyd",
                album: "The Wall",
                status: "WANT",
                created_at: 1,
                updated_at: 1,
              },
            ],
          };
        case "GetDownloads":
          return { type: "Downloads", data: downloads };
        case "GetDiscoveryRecommendations":
          return {
            type: "DiscoveryRecommendations",
            data: f.empty ? [] : recs,
          };
        case "GetCharts":
          return { type: "Charts", data: [{ id: "1134548194", title: "India Superhits Top 50" }, { id: "110858205", title: "English chart" }] };
        case "GetWorldTrending":
          return { type: "WorldTrending", data: f.empty ? [] : recs };
        case "SearchOnlineMusic":
          if (f.failSearch) throw Error("Fixture search failure");
          return { type: "SearchResults", data: recs };
        case "GetHomeOverview":
          return {
            type: "HomeOverview",
            data: {
              flow_session: flow,
              recent_rotation: f.empty
                ? []
                : tracks
                    .slice(0, 6)
                    .map((t) => ({
                      track_id: t.id,
                      title: t.title,
                      artist: t.artist_name,
                      cover_art_url: t.cover_art_url,
                    })),
              rediscover: f.empty
                ? []
                : tracks
                    .slice(6, 9)
                    .map((t) => ({
                      track_id: t.id,
                      title: t.title,
                      artist: t.artist_name,
                      cover_art_url: t.cover_art_url,
                      explanation: evidence,
                    })),
              for_you: f.empty
                ? []
                : tracks
                    .slice(10, 15)
                    .map((t) => ({
                      ...t,
                      track_id: t.id,
                      explanation: evidence,
                    })),
              library_moment: {
                album_id: "a0",
                album_title: "Migration",
                artist_name: "Bonobo",
                total_tracks: 10,
              },
              your_week: {
                formatted_time: "4h 32m",
                track_count: 47,
                artist_count: 18,
              },
            },
          };
        case "GetFlowSession":
          return { type: "FlowSession", data: flow };
        case "GetFlowTrackReason":
          return { type: "FlowTrackReason", data: evidence };
        case "GetCurrentUser":
          return {
            type: "CurrentUser",
            data: {
              id: "u1",
              username: "listener",
              display_name: "Music Listener",
              created_at: 1700000000,
            },
          };
        case "GetStatsOverview":
          return {
            type: "StatsOverview",
            data: {
              app_start_date: 1700000000,
              user_joined_date: 1700000000,
              current_year: 2026,
              selected_year: 2026,
              available_years: [2026],
              selected_month: 10,
              available_months: [10],
              daily_seconds: 1200,
              weekly_seconds: 16320,
              monthly_seconds: 40000,
              total_year_seconds: 50000,
              lifetime_seconds: 60000,
              monthly_graph: [],
              top_songs: [],
              top_artists: [],
              top_days: [],
            },
          };
        case "GetCloudSyncStatus":
          return {
            type: "CloudSyncStatus",
            data: {
              is_configured: false,
              has_session: false,
              worker_url: "https://example.com",
            },
          };
        case "GetNotifications":
          return { type: "Notifications", data: notifications };
        case "GetRemoteAudioCacheStats":
          return {
            type: "RemoteAudioCacheStats",
            data: { total_size_bytes: 0, file_count: 0 },
          };
        case "InspectSpotifyPlaylist":
          return {
            type: "SpotifyPlaylistImport",
            data: {
              playlist_id: "sp1",
              title: "Imported records",
              total_tracks: 1,
              matched_tracks: 1,
              missing_tracks: 0,
              tracks: [
                {
                  spotify_id: "s1",
                  title: "Kerala",
                  artist: "Bonobo",
                  in_library: true,
                  matched_local_track_id: "t0",
                  match_status: "EXACT_MATCH",
                },
              ],
            },
          };
        default:
          return { type: "Empty", data: null };
      }
    },
    async command(c) {
      commands.push(c);
      const p = c.payload || {};
      switch (c.command) {
        case "SearchDownloads":
          return {status:"DownloadSearchResults",data:Array.from({length:120},(_,index)=>({id:`jiosaavn:fixture_${index}`,provider:"jiosaavn",username:`Fixture artist ${index}`,filename:`${p.artist} - ${p.title} ${index}.flac`,file_size:10485760,bitrate:1000,format:"flac",slots_free:true,speed_bps:1048576}))};

        case "SetInterfaceStyle":
          if (f.failStyle) throw Error("Fixture storage unavailable");
          settings.interface_style = p.style;
          sessionStorage.setItem("test.interface-style", p.style);
          break;
        case "Pause":
          playback.is_playing = false;
          playback.is_paused = true;
          emit({
            event: "PlaybackPaused",
            payload: { position_secs: playback.position_secs },
          });
          break;
        case "Resume":
        case "ContinueFlow":
        case "TogglePlayPause":
          playback.is_playing =
            ["Resume", "ContinueFlow"].includes(c.command) ? true : !playback.is_playing;
          playback.is_paused = !playback.is_playing;
          emit({
            event: playback.is_playing ? "PlaybackResumed" : "PlaybackPaused",
            payload: { position_secs: playback.position_secs },
          });
          break;
        case "PlayTrack":
        case "PlayOnlineTrack": {
          const t = tracks.find((t) => t.id === p.track_id) || {
            ...tracks[0],
            id: p.track_id,
            title: p.title,
            artist_name: p.artist,
            format: "online",
            file_path: "online://fixture",
          };
          Object.assign(playback, {
            current_track: t,
            current_track_id: t.id,
            is_playing: true,
            position_secs: 0,
          });
          emit({
            event: "PlaybackStarted",
            payload: {
              track_id: t.id,
              title: t.title,
              artist: t.artist_name,
              duration_secs: 240,
              current_track: t,
              source: "library",
            },
          });
          break;
        }
        case "NextTrack": {
          const id = playback.queue_track_ids.shift();
          if (id)
            await f.command({
              command: "PlayTrack",
              payload: { track_id: id },
            });
          queueEvent();
          break;
        }
        case "PreviousTrack":
          await f.command({
            command: "PlayTrack",
            payload: { track_id: "t0" },
          });
          break;
        case "Seek":
        case "SeekTo":
          playback.position_secs = p.position_secs ?? p.seconds ?? 0;
          emit({
            event: "PlaybackSeeked",
            payload: { position_secs: playback.position_secs },
          });
          break;
        case "SetVolume":
          playback.volume = p.volume;
          emit({ event: "VolumeChanged", payload: { volume: p.volume } });
          break;
        case "ToggleMute":
          playback.is_muted = !playback.is_muted;
          emit({
            event: "VolumeChanged",
            payload: { is_muted: playback.is_muted },
          });
          break;
        case "SetShuffle":
          playback.is_shuffled = p.enabled;
          break;
        case "SetRepeatMode":
          playback.repeat_mode = p.mode;
          break;
        case "EnqueueTrack":
        case "EnqueueOnlineTrack":
          playback.queue_track_ids.push(p.track_id);
          queueEvent();
          break;
        case "DequeueTrack":
          playback.queue_track_ids = playback.queue_track_ids.filter(
            (id) => id !== p.track_id,
          );
          queueEvent();
          break;
        case "ClearQueue":
          playback.queue_track_ids = [];
          queueEvent();
          break;
        case "StartFlow":
          playback.is_playing = true;
          playback.is_paused = false;
          emit({ event: "PlaybackResumed", payload: { position_secs: playback.position_secs } });
          flow = {
            ...flow,
            is_active: true,
            last_track_id: playback.current_track.id,
            familiarity_ratio: p.familiarity_ratio,
          };
          queueEvent();
          break;
        case "StopFlow":
          flow = { ...flow, is_active: false };
          break;
        case "SetFlowSteering":
          flow = { ...flow, familiarity_ratio: p.familiarity_ratio };
          break;
        case "CancelDownload":
          downloads[0].status = "CANCELLED";
          emit({ event: "DownloadsUpdated", payload: {} });
          break;
      }
      return { status: "Ok" };
    },
  });
}
