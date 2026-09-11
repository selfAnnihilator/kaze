// --- Domain Models ---

export interface Track {
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
}

export interface Playlist {
  id: string;
  name: string;
  description?: string;
  is_smart_mix: number;
  mix_type?: string;
  generation_reason?: string;
  track_count?: number;
}

export interface PlaybackState {
  current_track?: Track;
  is_playing: boolean;
  position_secs: number;
  duration_secs: number;
  volume: number;
  is_muted: boolean;
  repeat_mode: "off" | "one" | "all";
  is_shuffled: boolean;
  queue_track_ids?: string[];
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
  external_track_id: string;
  provider: string;
  provider_id: string;
  title: string;
  artist: string;
  album?: string;
  duration_secs?: number;
  cover_art_url?: string;
  match_status: "EXACT_MATCH" | "LIKELY_MATCH" | "POSSIBLE_MATCH" | "NOT_FOUND";
  matched_local_track_id?: string;
  recommendation_reason: string;
  in_wishlist: boolean;
}

export interface TasteProfile {
  top_artists: Array<{ id_or_name: string; display_name: string; affinity: number }>;
  top_genres: Array<{ id_or_name: string; display_name: string; affinity: number }>;
  top_eras: Array<{ id_or_name: string; display_name: string; affinity: number }>;
}

export interface OnboardingStatus {
  completed: boolean;
  default_music_dir: string;
  configured_folders: Array<{ id: string; path: string; track_count: number }>;
}

export interface AppSettings {
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
    enable_musicbrainz: boolean;
    enable_spotify: boolean;
    spotify_client_id?: string;
    spotify_client_secret?: string;
  };
  downloads: {
    download_dir?: string;
    slskd_host: string;
    slskd_port: number;
    slskd_api_key?: string;
    auto_import: boolean;
    max_concurrent_downloads: number;
  };
}

// --- Commands & Queries ---

export interface SpotifyImportedTrack {
  spotify_id: string;
  title: string;
  artist: string;
  duration_secs?: number;
  in_library: boolean;
  match_status: string;
  matched_local_track_id?: string;
}

export interface SpotifyPlaylistImport {
  playlist_id: string;
  title: string;
  cover_url?: string;
  total_tracks: number;
  matched_tracks: number;
  missing_tracks: number;
  tracks: SpotifyImportedTrack[];
}

export type Command =
  | { command: "PlayTrack"; payload: { track_id: string; source?: string } }
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
  | { command: "EnqueueTrack"; payload: { track_id: string; play_next: boolean } }
  | { command: "DequeueTrack"; payload: { track_id: string } }
  | { command: "ClearQueue" }
  | { command: "CompleteOnboarding"; payload: { music_folders: string[]; start_scan: boolean } }
  | { command: "ResetOnboarding" }
  | { command: "AddLibraryFolder"; payload: { path: string } }
  | { command: "RemoveLibraryFolder"; payload: { folder_id: string } }
  | { command: "ScanLibrary"; payload: { folder_id?: string; incremental: boolean } }
  | { command: "GenerateSmartMix"; payload: { mix_type: string } }
  | { command: "CreatePlaylist"; payload: { name: string; description?: string } }
  | { command: "DeletePlaylist"; payload: { playlist_id: string } }
  | { command: "AddTrackToPlaylist"; payload: { playlist_id: string; track_id: string } }
  | { command: "LikeTrack"; payload: { track_id: string } }
  | { command: "DislikeTrack"; payload: { track_id: string } }
  | { command: "RemoveTrackFeedback"; payload: { track_id: string } }
  | { command: "AddToWishlist"; payload: { title: string; artist: string; album?: string; external_id?: string } }
  | { command: "AddMissingToWishlist"; payload: { tracks: any[] } }
  | { command: "UpdateWishlistStatus"; payload: { wishlist_id: string; status: "want" | "ignore" | "already_own" | "downloaded" } }
  | { command: "SearchSoulseek"; payload: { artist: string; title: string; album?: string } }
  | { command: "LaunchSoulseek"; payload: { search_query?: string } }
  | { command: "ImportSoulseekDownloads" }
  | { command: "StartDownload"; payload: { search_result_id: string; wishlist_id?: string } }
  | { command: "CancelDownload"; payload: { task_id: string } };

export type Query =
  | { query: "GetOnboardingStatus" }
  | { query: "GetPlaybackState" }
  | { query: "GetTracks"; payload: { offset: number; limit: number; sort_by?: string; ascending: boolean } }
  | { query: "GetArtists"; payload: { offset: number; limit: number } }
  | { query: "GetAlbums"; payload: { offset: number; limit: number } }
  | { query: "GetPlaylists" }
  | { query: "GetPlaylistTracks"; payload: { playlist_id: string } }
  | { query: "GetSmartMixes" }
  | { query: "GetTasteProfile" }
  | { query: "GetLocalRecommendations"; payload: { limit: number } }
  | { query: "GetDiscoveryRecommendations"; payload: { limit: number } }
  | { query: "GetWishlist" }
  | { query: "GetDownloads"; payload: { status_filter?: string; limit: number } }
  | { query: "SearchLibrary"; payload: { query_text: string; limit: number } }
  | { query: "GetSettings" }
  | { query: "ImportSpotifyPlaylist"; payload: { url_or_id: string } };

export interface QueryResponse {
  type: string;
  data?: any;
  completed?: boolean;
  default_music_dir?: string;
  configured_folders?: any[];
}
