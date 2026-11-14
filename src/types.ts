export interface OnlinePlaybackSource { provider: "jiosaavn"; track_id: string; }

// --- Domain Models ---

export interface Track {
  /** Placeholder shown while the backend's real track details load. */
  pending?: boolean;
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
  /** Playback source that owns the queue: "library", "flow", "archive" (Downloads list), ... */
  source?: string;
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
  status: "QUEUED" | "DOWNLOADING" | "COMPLETED" | "FAILED" | "CANCELLED" | "MISSING";
  /** Whether the saved file is still on disk (from GetDownloads). */
  file_present?: boolean;
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
  /** Library already holds this recording locally. */
  local_copy?: boolean;
  recommendation_reason: string;
  in_wishlist: boolean;
}

export interface RecommendationSetup { user_id: string; status: "pending" | "completed" | "skipped"; artists: string[]; genres: string[]; revision: number; should_prompt: boolean; }

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
    enable_lastfm_similarity?: boolean;
  };
  downloads: {
    download_dir?: string;
    /** Kaze's own download folder (canonical path), e.g. ~/Music/Kaze. */
    resolved_dir?: string;
    auto_import: boolean;
    max_concurrent_downloads: number;
  };
  close_button_behavior?: "minimize_to_tray" | "quit_app";
}

// --- Commands & Queries (the IPC contract lives in ipcTypes.ts) ---
export type { Command, Query, JioSaavnImportedTrack, JioSaavnPlaylistImport } from "./ipcTypes";

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

export interface RankedGenreItem {
  genre_id: string;
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
  top_genres?: RankedGenreItem[];
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
  session_style?: string | null;
  session_adjustment?: number | null;
  similar_artist?: string | null;
  context_bucket?: string | null;
  rec_slot?: "explore" | "familiar" | null;
  explore_bias?: number | null;
  selected_at?: number | null;
  from_existing_queue: boolean;
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
  first_track_id?: string;
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

export type EntityFeedbackKind = "artist" | "genre";
export type EntityFeedbackLevel = "block" | "less";
export interface EntityFeedbackRow {
  kind: EntityFeedbackKind;
  key: string;
  display: string;
  level: EntityFeedbackLevel;
  created_at: number;
}
