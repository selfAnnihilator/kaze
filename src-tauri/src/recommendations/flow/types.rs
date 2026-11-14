//! Flow and Home data shared with the frontend.
use super::*;

/// Recorded evidence only; presentation belongs to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowExplanation {
    pub primary: String,
    pub is_liked: bool,
    pub is_unplayed: bool,
    pub play_count: Option<i64>,
    pub days_since_played: Option<i64>,
    pub artist_affinity: Option<f64>,
    pub artist_name: Option<String>,
    pub familiarity_ratio: Option<f64>,
    pub selected_at: Option<i64>,
    pub from_existing_queue: bool,
    /// Style this session's listening pushed the pick toward (genre or language).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_style: Option<String>,
    /// Bounded in-session score shift applied to this pick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_adjustment: Option<f64>,
    /// Seed artist whose fans also like this pick's artist ("Fans of X also like this").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub similar_artist: Option<String>,
    /// Day part whose listening this pick fits ("Fits your evenings").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_bucket: Option<String>,
    /// "explore" or "familiar": which Flow slot this pick filled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rec_slot: Option<String>,
    /// Learned exploration nudge (−0.12..0.12) when it shaped this pick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explore_bias: Option<f64>,
}

/// Minimum similarity for an online candidate to pass the pre-filter on similarity alone.
pub(super) const SIMILAR_CANDIDATE_MIN: f64 = 0.2;
/// Weight of the "fans also like" boost; applied only to artists the user barely knows.
pub(super) const SIMILAR_WEIGHT: f64 = 0.12;
/// Score cost of a full tempo/energy/brightness jump between neighbours.
pub const TRANSITION_WEIGHT: f64 = 0.10;

/// A track selected by the Flow engine with deterministic scoring and explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTrackItem {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: f64,
    pub cover_art_url: Option<String>,
    pub score: f64,
    pub reason: String,
    pub reason_category: String,
    pub explanation: FlowExplanation,
}

/// Lightweight persistent state for Flow session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSessionState {
    #[serde(default)]
    pub generation: Option<u64>,
    pub is_active: bool,
    pub familiarity_ratio: f64, // 0.0 = pure rediscover, 1.0 = pure familiar
    pub updated_at: i64,
    pub last_track_id: Option<String>,
    pub last_track_title: Option<String>,
    pub last_track_artist: Option<String>,
    pub flow_label: String,
}

/// A track in the Recent Rotation section.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecentTrackItem {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: f64,
    pub cover_art_url: Option<String>,
    pub started_at: i64,
}

/// A track in the Rediscover section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RediscoverTrackItem {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: f64,
    pub cover_art_url: Option<String>,
    pub last_played_at: Option<i64>,
    pub play_count: i64,
    pub is_liked: bool,
    pub reason: String,
    pub explanation: FlowExplanation,
}

/// Spotlight moment from the user's library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryMoment {
    pub album_id: String,
    pub first_track_id: Option<String>,
    pub album_title: String,
    pub artist_name: String,
    pub cover_art_path: Option<String>,
    pub total_tracks: i64,
    pub release_year: Option<i64>,
    pub subtitle: String,
}

/// Listening summary for the last 7 days.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YourWeekStats {
    pub total_seconds: f64,
    pub track_count: u64,
    pub artist_count: u64,
    pub formatted_time: String,
}

/// Composite payload for Kaze Home screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeOverview {
    pub flow_session: Option<FlowSessionState>,
    pub recent_rotation: Vec<RecentTrackItem>,
    pub rediscover: Vec<RediscoverTrackItem>,
    pub for_you: Vec<RecommendedTrackDetail>,
    pub library_moment: Option<LibraryMoment>,
    pub your_week: Option<YourWeekStats>,
}
