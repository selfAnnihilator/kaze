use serde::{Deserialize, Serialize};

/// Time windows for aggregation and rankings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeWindow {
    Today,
    Last7Days,
    Last30Days,
    Last6Months,
    LastYear,
    AllTime,
}

/// Entity types for ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankingEntity {
    Tracks,
    Artists,
    Albums,
    Genres,
}

/// Queries represent read-only requests for state or data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "query", content = "payload")]
pub enum Query {
    GetOnboardingStatus,
    GetLibraryOverview,
    GetTracks {
        offset: u32,
        limit: u32,
        sort_by: Option<String>,
        ascending: bool,
    },
    GetTrackById {
        track_id: String,
    },
    GetArtists {
        offset: u32,
        limit: u32,
    },
    GetArtistDetails {
        artist_id: String,
    },
    GetAlbums {
        offset: u32,
        limit: u32,
    },
    GetAlbumDetails {
        album_id: String,
    },
    GetPlaylists,
    GetPlaylistTracks {
        playlist_id: String,
    },
    SearchLibrary {
        query_text: String,
        limit: u32,
    },
    GetTopRankings {
        window: TimeWindow,
        entity: RankingEntity,
        limit: u32,
    },
    GetTasteProfile,
    GetSmartMixes,
    GetLocalRecommendations {
        limit: u32,
    },
    GetDiscoveryRecommendations {
        limit: u32,
        #[serde(default)]
        force_refresh: Option<bool>,
    },
    GetWishlist,
    GetDownloads {
        status_filter: Option<String>,
        limit: u32,
    },
    GetSettings,
    GetPlaybackState,
    ImportSpotifyPlaylist {
        url_or_id: String,
    },
    ResolveFullTrackAudio {
        artist: String,
        title: String,
    },
    SearchOnlineMusic {
        query: String,
        #[serde(default)]
        limit: Option<u32>,
    },
}

/// Query response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum QueryResponse {
    OnboardingStatus {
        completed: bool,
        default_music_dir: String,
        configured_folders: Vec<serde_json::Value>,
    },
    Tracks(Vec<serde_json::Value>),
    Track(Option<serde_json::Value>),
    Artists(Vec<serde_json::Value>),
    Albums(Vec<serde_json::Value>),
    Playlists(Vec<serde_json::Value>),
    PlaylistTracks(Vec<serde_json::Value>),
    Rankings(Vec<serde_json::Value>),
    TasteProfile(serde_json::Value),
    SmartMixes(Vec<serde_json::Value>),
    Recommendations(Vec<serde_json::Value>),
    SearchResults(Vec<serde_json::Value>),
    PlaybackState(serde_json::Value),
    Settings(serde_json::Value),
    Wishlist(Vec<serde_json::Value>),
    DiscoveryRecommendations(Vec<serde_json::Value>),
    Downloads(Vec<serde_json::Value>),
    SpotifyPlaylistImport(serde_json::Value),
    FullTrackAudio {
        stream_url: String,
        duration_secs: f64,
    },
    Empty,
}
