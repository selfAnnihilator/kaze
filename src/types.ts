export interface OnlinePlaybackSource { provider: "jiosaavn"; track_id: string; }

// --- Domain Models ---

export interface Track {
  playback_source?: OnlinePlaybackSource;
  id: string;
  file_path: string;
  title: string;
  artist_name?: string;
  album_title?: string;
  genre_name?: string;
  track_number?: number;
  year?: number;
  duration_secs: number;
  bitrate?: number;
  sample_rate?: number;
  format: string;
  has_cover_art: number;
  cover_art_url?: string;
  preview_url?: string;
  musicbrainz_track_id?: string;
  spotify_id?: string;
  manual_like?: number; // 1 = liked, -1 = disliked, 0 = neutral
}

export interface Artist {
  id: string;
  name: string;
  bio?: string;
  image_url?: string;
  track_count?: number;
}

export interface Album {
  id: string;
  title: string;
  artist_id?: string;
  artist_name?: string;
  year?: number;
  track_count?: number;
  cover_art_path?: string;
  first_track_id?: string;
}

export interface Playlist {
  id: string;
  name: string;
  description?: string;
  is_smart_mix: number;
  mix_type?: string;
  generation_reason?: string;
  track_count?: number;
  cover_art_url?: string;
  first_track_id?: string;
}

export interface PlaybackState {
  current_track_id?: string | null;
  current_queue_index?: number | null;
  current_track?: Track;
  is_playing: boolean;
  is_paused?: boolean;
  position_secs: number;
  duration_secs: number;
  volume: number;
  is_muted: boolean;
  repeat_mode: "off" | "one" | "all";
  is_shuffled: boolean;
  queue_track_ids?: string[];
  can_seek?: boolean;
  is_buffering?: boolean;
}

export interface WishlistItem {
  id: string;
  title: string;
  artist: string;
  album?: string;
  external_track_id?: string;
  status: "WANT" | "IGNORE" | "ALREADY_OWN" | "DOWNLOADED";
  notes?: string;
  created_at: number;
  updated_at: number;
}

export interface DownloadTask {
  source_track_id?: string;
  track_ids?: string[];
  id: string;
  provider: string;
  provider_task_id?: string;
  title: string;
  artist: string;
  album?: string;
  filename: string;
  destination_path?: string;
  file_size?: number;
  bytes_downloaded: number;
  status: "QUEUED" | "DOWNLOADING" | "COMPLETED" | "FAILED" | "CANCELLED";
  error_message?: string;
  wishlist_id?: string;
  created_at: number;
  completed_at?: number;
}

export interface DownloadSearchResult {
  id: string;
  provider: string;
  username: string;
  filename: string;
  file_size: number;
  bitrate?: number;
  sample_rate?: number;
  format: string;
  slots_free: boolean;
  speed_bps: number;
}

export interface DiscoveryRecommendation {
  playback_source?: OnlinePlaybackSource;
  external_track_id: string;
  provider: string;
  provider_id: string;
  title: string;
  artist: string;
  album?: string;
  duration_secs?: number;
  cover_art_url?: string;
  preview_url?: string;
  genre?: string;
  match_status: "EXACT_MATCH" | "LIKELY_MATCH" | "POSSIBLE_MATCH" | "NOT_FOUND";
  matched_local_track_id?: string;
  recommendation_reason: string;
  in_wishlist: boolean;
}

export interface OnlinePlayingTrack {
  id: string;
  title: string;
  artist: string;
  album?: string;
  cover_art_url?: string;
  preview_url?: string;
  duration: number;
  currentTime: number;
  isPlaying: boolean;
  isLoading?: boolean;
}

export interface TasteProfile {
  top_artists: Array<{ id_or_name: string; display_name: string; affinity: number;
  }>;
  top_genres: Array<{ id_or_name: string; display_name: string; affinity: number;
  }>;
  top_eras: Array<{ id_or_name: string; display_name: string; affinity: number;
  }>;
}

export interface OnboardingStatus {
  completed: boolean;
  default_music_dir: string;
  configured_folders: Array<{ id: string; path: string; track_count: number }>;
}

export interface AppSettings {
  interface_style?: "spatial" | "brutalist";
  database_path: string;
  cache_dir: string;
  audio: {
    default_volume: number;
    output_device?: string;
  };
  history: {
    min_meaningful_seconds: number;
    min_meaningful_percentage: number;
  };
  metadata: {
    provider_priority: string[];
    enable_jiosaavn?: boolean;
    auto_fill_metadata?: boolean;
  };
  downloads: {
    download_dir?: string;
    auto_import: boolean;
    max_concurrent_downloads: number;
  };
  close_button_behavior?: "minimize_to_tray" | "quit_app";
}

// --- Commands & Queries ---

export interface JioSaavnImportedTrack {
  track_id: string;
  album?: string;
  cover_art_url?: string;
  title: string;
  artist: string;
  duration_secs?: number;
  in_library: boolean;
  match_status: string;
  matched_local_track_id?: string;
}

export interface JioSaavnPlaylistImport {
  playlist_id: string;
  title: string;
  cover_url?: string;
  total_tracks: number;
  matched_tracks: number;
  missing_tracks: number;
  tracks: JioSaavnImportedTrack[];
}

export type Command =
  | { command: "DownloadTrack"; payload: { track_id: string } }
  | { command: "FillLibraryGaps"; payload: { retry: boolean } }
  | { command: "RemoveProviderEnrichment" }
  | { command: "SetMetadataPreference"; payload: { key: string; enabled: boolean } }
  | { command: "PlayTrack"; payload: { track_id: string; source?: string } }
  | { command: "PlayQueueIndex"; payload: { index: number } }
  | { command: "Pause" }
  | { command: "Resume" }
  | { command: "Stop" }
  | { command: "NextTrack" }
  | { command: "PreviousTrack" }
  | { command: "Seek"; payload: { position_secs: number } }
  | { command: "SetVolume"; payload: { volume: number } }
  | { command: "ToggleMute" }
  | { command: "SetRepeatMode"; payload: { mode: "off" | "one" | "all" } }
  | { command: "SetShuffle"; payload: { enabled: boolean } }
  | { command: "EnqueueTrack"; payload: { track_id: string; play_next: boolean };
    }
  | {
      command: "PlayOnlineTrack";
      payload: {
        track_id: string;
        title: string;
        artist: string;
        album?: string;
        duration_secs?: number;
        cover_art_url?: string;
        preview_url?: string;
        source?: string;
      };
    }
  | {
      command: "EnqueueOnlineTrack";
      payload: {
        track_id: string;
        title: string;
        artist: string;
        album?: string;
        duration_secs?: number;
        cover_art_url?: string;
        preview_url?: string;
        play_next: boolean;
      };
    }
  | { command: "DequeueTrack"; payload: { track_id: string } }
  | { command: "ClearQueue" }
  | { command: "ClearRemoteAudioCache" }
  | { command: "StartFlow"; payload?: { familiarity_ratio?: number } }
  | { command: "ContinueFlow" }
  | { command: "StopFlow" }
  | { command: "SetFlowSteering"; payload: { familiarity_ratio: number } }
  | { command: "CompleteOnboarding"; payload: { music_folders: string[]; start_scan: boolean };
    }
  | { command: "ResetOnboarding" }
  | { command: "AddLibraryFolder"; payload: { path: string } }
  | { command: "RemoveLibraryFolder"; payload: { folder_id: string } }
  | { command: "ScanLibrary"; payload: { folder_id?: string; incremental: boolean };
    }
  | { command: "GenerateSmartMix"; payload: { mix_type: string } }
  | { command: "CreatePlaylist"; payload: { name: string; description?: string };
    }
  | { command: "DeletePlaylist"; payload: { playlist_id: string } }
  | { command: "RenamePlaylist"; payload: { playlist_id: string; name: string };
    }
  | { command: "EnsureLikedSongsPlaylist" }
  | {
      command: "AddTrackToPlaylist";
      payload: {
        playlist_id: string;
        track_id: string;
        title?: string;
        artist?: string;
        album?: string;
        duration_secs?: number;
        cover_art_url?: string;
        preview_url?: string;
      };
    }
  | { command: "RemoveTrackFromPlaylist"; payload: { playlist_id: string; track_id: string };
    }
  | { command: "LikeTrack"; payload: { track_id: string } }
  | { command: "DislikeTrack"; payload: { track_id: string } }
  | { command: "RemoveTrackFeedback"; payload: { track_id: string } }
  | { command: "AddToWishlist"; payload: { title: string; artist: string; album?: string; external_id?: string;
      };
    }
  | { command: "AddMissingToWishlist"; payload: { tracks: any[] } }
  | { command: "UpdateWishlistStatus"; payload: { wishlist_id: string; status: "want" | "ignore" | "already_own" | "downloaded";
      };
    }
  | { command: "SearchDownloads"; payload: { artist: string; title: string; album?: string };
    }
  | { command: "StartDownload"; payload: { search_result_id: string; wishlist_id?: string };
    }
  | { command: "CancelDownload"; payload: { task_id: string } }
  | { command: "SignUp"; payload: { username: string; password: string } }
  | { command: "Login"; payload: { username: string; password: string } }
  | { command: "Logout" }
  | { command: "LogoutAll" }
  | { command: "RevokeSession"; payload: { session_id: string } }
  | { command: "UploadAvatar"; payload?: { file_path?: string } }
  | { command: "RemoveAvatar" }
  | { command: "SyncCloudData" }
  | { command: "SetCloudServerUrl"; payload: { url: string } }
  | { command: "TogglePlayPause" }
  | { command: "SetCloseButtonBehavior"; payload: { behavior: "minimize_to_tray" | "quit_app" };
    }
  | {
      command: "SetInterfaceStyle";
      payload: { style: "spatial" | "brutalist" };
    }
  | {
      command: "RecordPlaybackSession";
      payload: {
        track_id: string;
        title: string;
        artist?: string;
        album?: string;
        duration_secs: number;
        seconds_listened: number;
        completed: boolean;
        skipped: boolean;
        source: string;
      };
    };

export type Query =
  | { query: "GetRecommendationFeedback" }
  | { query: "RefreshCharts" }
  | { query: "GetOnboardingStatus" }
  | { query: "GetPlaybackState" }
  | { query: "GetTracks"; payload: { offset: number; limit: number; sort_by?: string; ascending: boolean;
      };
    }
  | { query: "GetArtists"; payload: { offset: number; limit: number } }
  | { query: "GetAlbums"; payload: { offset: number; limit: number } }
  | { query: "GetPlaylists" }
  | { query: "GetPlaylistTracks"; payload: { playlist_id: string } }
  | { query: "GetSmartMixes" }
  | { query: "RefreshSmartMixes" }
  | { query: "GetTasteProfile" }
  | { query: "GetLocalRecommendations"; payload: { limit: number } }
  | { query: "GetDiscoveryRecommendations"; payload: { limit: number; force_refresh?: boolean };
    }
  | { query: "GetWorldTrending"; payload: { limit: number; force_refresh?: boolean };
    }
  | { query: "GetChartSongs"; payload: { chart_id: string; limit: number } }
  | { query: "GetWishlist" }
  | { query: "GetDownloads"; payload: { status_filter?: string; limit: number };
    }
  | { query: "SearchLibrary"; payload: { query_text: string; limit: number } }
  | { query: "GetSettings" }
  | { query: "GetCharts" }
  | { query: "ImportJioSaavnPlaylist"; payload: { url_or_id: string } }
  | { query: "SearchOnlineMusic"; payload: { query: string; limit?: number; provider?: "jiosaavn" } }
  | { query: "GetTrackCoverArt"; payload: { track_id: string } }
  | { query: "GetTrackPlaylistMemberships" }
  | { query: "GetTrackLyrics"; payload: { track_id?: string; artist: string; title: string; duration_secs?: number;
      };
    }
  | { query: "GetStatsOverview"; payload?: { year?: number; month?: number } }
  | { query: "GetCurrentUser" }
  | { query: "GetProfile" }
  | { query: "GetAvatar"; payload?: { user_id?: string } }
  | { query: "GetCloudSyncStatus" }
  | { query: "GetSessionState" }
  | { query: "ListSessions" }
  | { query: "GetRemoteAudioCacheStats" }
  | { query: "GetHomeOverview" }
  | { query: "GetFlowQueue"; payload?: { familiarity_ratio?: number; limit?: number; exclude_track_ids?: string[];
      };
    }
  | { query: "GetFlowTrackReason"; payload: { track_id: string } }
  | { query: "GetFlowSession" };

export interface RemoteAudioCacheStats {
  total_size_bytes: number;
  file_count: number;
  max_size_bytes: number;
  partial_file_count: number;
}

export type SessionExpiredReason = "idle_timeout" | "absolute_timeout" | "revoked" | "other";

export type AuthSessionState =
  | { type: "SignedOut" }
  | { type: "Authenticating" }
  | { type: "OnlineAuthenticated"; user: { id: string; username: string; created_at: number }; session_id: string;
    }
  | { type: "OfflineAuthenticated"; user: { id: string; username: string; created_at: number }; session_id: string;
    }
  | { type: "SessionExpired"; reason: SessionExpiredReason }
  | { type: "CloudUnavailable" }
  | { type: "SyncPaused" };

export interface SessionInfo {
  id: string;
  device_id: string;
  device_name: string;
  client_version?: string | null;
  created_at?: number | null;
  last_used_at?: number | null;
  idle_expires_at: number;
  absolute_expires_at: number;
  is_current: boolean;
}

export interface CloudSyncStatus {
  connected: boolean;
  worker_url: string;
  user_id?: string | null;
  username?: string | null;
  session_id?: string | null;
  device_name?: string | null;
  idle_expires_at?: number | null;
  absolute_expires_at?: number | null;
  last_synced_at?: number | null;
  session_state?: AuthSessionState | null;
}

export interface RankedTrackItem {
  track_id: string;
  title: string;
  artist_name?: string;
  album_title?: string;
  play_count: number;
  total_seconds: number;
  completion_count: number;
  skip_count: number;
  score: number;
}

export interface RankedArtistItem {
  artist_id: string;
  name: string;
  play_count: number;
  total_seconds: number;
  score: number;
}

export interface TopListeningDay {
  day_date: string;
  day_name: string;
  total_seconds: number;
}

export interface DailyListeningPoint {
  day: number;
  date: string;
  total_seconds: number;
}

export interface StatsOverview {
  app_start_date: number;
  user_joined_date: number;
  current_year: number;
  selected_year: number;
  available_years: number[];
  selected_month: number;
  available_months: number[];
  daily_seconds: number;
  weekly_seconds: number;
  monthly_seconds: number;
  total_year_seconds: number;
  lifetime_seconds: number;
  history_started_at?: number | null;
  monthly_graph: DailyListeningPoint[];
  top_songs: RankedTrackItem[];
  top_artists: RankedArtistItem[];
  top_days: TopListeningDay[];
}

export interface UserProfile {
  id: string;
  username: string;
  created_at: number;
  display_name?: string;
  avatar_key?: string;
  avatar_public_id?: string;
  avatar_url?: string;
  avatar_version?: number;
  avatar_updated_at?: number;
  avatar_data_url?: string;
}

export interface LyricLine {
  timeSecs: number;
  text: string;
}

export interface TrackLyricsData {
  trackName?: string;
  artistName?: string;
  plainLyrics?: string;
  syncedLyrics?: string;
  instrumental?: boolean;
}

export interface QueryResponse {
  type: string;
  data?: any;
  completed?: boolean;
  default_music_dir?: string;
  configured_folders?: any[];
}

export interface AppNotification {
  id: string;
  type: "success" | "error" | "info" | "warning";
  title: string;
  message: string;
  timestamp: number;
  read: boolean;
}

export interface FlowExplanation {
  primary: string;
  is_liked: boolean;
  is_unplayed: boolean;
  play_count?: number | null;
  days_since_played?: number | null;
  artist_affinity?: number | null;
  artist_name?: string | null;
  familiarity_ratio?: number | null;
  selected_at?: number | null;
  from_existing_queue: boolean;
}

export interface FlowTrackItem {
  track_id: string;
  title: string;
  artist: string;
  album?: string;
  duration_secs: number;
  cover_art_url?: string;
  score: number;
  reason: string;
  reason_category: string;
  explanation?: FlowExplanation;
}

export interface FlowSessionState {
  generation?: number | null;
  is_active: boolean;
  familiarity_ratio: number;
  updated_at: number;
  last_track_id?: string;
  last_track_title?: string;
  last_track_artist?: string;
  flow_label: string;
}

export interface RecentTrackItem {
  track_id: string;
  title: string;
  artist: string;
  album?: string;
  duration_secs: number;
  cover_art_url?: string;
  started_at: number;
}

export interface RediscoverTrackItem {
  track_id: string;
  title: string;
  artist: string;
  album?: string;
  duration_secs: number;
  cover_art_url?: string;
  last_played_at?: number;
  play_count: number;
  is_liked: boolean;
  reason: string;
  explanation?: FlowExplanation | null;
}

export interface LibraryMoment {
  album_id: string;
  album_title: string;
  artist_name: string;
  cover_art_path?: string;
  total_tracks: number;
  release_year?: number;
  subtitle: string;
}

export interface YourWeekStats {
  total_seconds: number;
  track_count: number;
  artist_count: number;
  formatted_time: string;
}

export interface RecommendedTrackDetail {
  track_id: string;
  title: string;
  artist_name?: string;
  album_title?: string;
  genre_name?: string;
  duration_secs: number;
  score: number;
  reasons: Array<{ factor: string; description: string; weight: number }>;
  is_discovery: boolean;
  explanation?: FlowExplanation | null;
}

export interface HomeOverview {
  flow_session?: FlowSessionState | null;
  recent_rotation: RecentTrackItem[];
  rediscover: RediscoverTrackItem[];
  for_you: RecommendedTrackDetail[];
  library_moment?: LibraryMoment | null;
  your_week?: YourWeekStats | null;
}
