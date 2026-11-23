/** The IPC contract: every Command and Query the frontend can send (re-exported by types.ts). */
import type { EntityFeedbackKind } from "./types";

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
  /** Library already holds this recording locally. */
  local_copy?: boolean;
}

export interface JioSaavnPlaylistImport {
  playlist_id: string;
  title: string;
  cover_url?: string;
  total_tracks: number;
  matched_tracks: number;
  missing_tracks: number;
  tracks: JioSaavnImportedTrack[];
  /** "spotify" imports are metadata-only and matched onto JioSaavn. */
  source?: "jiosaavn" | "spotify";
  /** Source songs with no confident JioSaavn match. */
  unmatched?: { title: string; artist: string }[];
  /** Playable songs in the source playlist (only the first 50 are checked). */
  source_total?: number;
}

export type Command =
  | { command: "DownloadTrack"; payload: { track_id: string } }
  | { command: "RemoveDownload"; payload: { track_id: string; source_id?: string } }
  | { command: "FillLibraryGaps"; payload: { retry: boolean } }
  | { command: "RemoveProviderEnrichment" }
  | { command: "SetMetadataPreference"; payload: { key: string; enabled: boolean } }
  | { command: "PlayTrack"; payload: { track_id: string; source?: string } }
  | { command: "SetArchiveShuffle"; payload: { enabled: boolean } }
  | { command: "PlayTrackList"; payload: { track_ids: string[] | null; start_track_id?: string | null; shuffle: boolean; scope?: "local" | "downloads" } }
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
  | { command: "EnqueueTrack"; payload: { track_id: string; play_next: boolean; /** The user picked it: plays before the app's queue. */ manual?: boolean };
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
        /** The user picked it: plays before the app's queue. */
        manual?: boolean;
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
  | { command: "SaveRecommendationSetup"; payload: { expected_user_id?: string; artists: string[]; genres: string[]; skipped: boolean } }
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
  | { command: "BlockArtist"; payload: { track_id: string } }
  | { command: "LessLikeThis"; payload: { track_id: string } }
  | { command: "ClearEntityFeedback"; payload: { kind: EntityFeedbackKind; key: string } }
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
  | { command: "UploadAvatar"; payload?: { file_path?: string; image_base64?: string } }
  | { command: "RemoveAvatar" }
  | { command: "SyncCloudData" }
  | { command: "SetCloudServerUrl"; payload: { url: string } }
  | { command: "TogglePlayPause" }
  | { command: "SetCloseButtonBehavior"; payload: { behavior: "minimize_to_tray" | "quit_app" };
    }
  | {
      command: "SetInterfaceStyle";
      payload: { style: "spatial" | "brutalist" };
    };

export type Query =
  | { query: "GetRecommendationSetup" }
  | { query: "GetRecommendationFeedback" }
  | { query: "GetEntityFeedback" }
  | { query: "RefreshCharts" }
  | { query: "GetOnboardingStatus" }
  | { query: "GetPlaybackState" }
  | { query: "GetTracks"; payload: { offset: number; limit: number; sort_by?: string; ascending: boolean;
      };
    }
  | { query: "GetArtists"; payload: { offset: number; limit: number } }
  | { query: "GetLibraryTracksPage" | "GetLibraryAlbumsPage"; payload: { offset: number; limit: number; scope?: "local" | "downloads" } }
  | { query: "GetAlbums"; payload: { offset: number; limit: number } }
  | { query: "GetAlbumDetails"; payload: { album_id: string } }
  | { query: "GetPlaylists" }
  | { query: "GetPlaylistTracks"; payload: { playlist_id: string } }
  | { query: "GetSmartMixes" }
  | { query: "RefreshSmartMixes" }
  | { query: "GetTasteProfile" }
  | { query: "GetLocalRecommendations"; payload: { limit: number } }
  | { query: "GetDiscoveryRecommendations"; payload: { limit: number; force_refresh?: boolean };
    }
  | { query: "GetCachedWorldTrending"; payload: { limit: number } }
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
