//! Statistics records returned to the processor and the UI.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RankedTrackItem {
    pub track_id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub play_count: i64,
    pub total_seconds: f64,
    pub completion_count: i64,
    pub skip_count: i64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RankedArtistItem {
    pub artist_id: String,
    pub name: String,
    pub play_count: i64,
    pub total_seconds: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RankedAlbumItem {
    pub album_id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub play_count: i64,
    pub total_seconds: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RankedGenreItem {
    pub genre_id: String,
    pub name: String,
    pub play_count: i64,
    pub total_seconds: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TopListeningDay {
    pub day_date: String,
    pub day_name: String,
    pub total_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyListeningPoint {
    pub day: u32,
    pub date: String,
    pub total_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsOverview {
    pub app_start_date: i64,
    pub user_joined_date: i64,
    pub current_year: i32,
    pub selected_year: i32,
    pub available_years: Vec<i32>,
    pub selected_month: u32,
    pub available_months: Vec<u32>,
    pub daily_seconds: f64,
    pub weekly_seconds: f64,
    pub monthly_seconds: f64,
    pub total_year_seconds: f64,
    #[serde(default)]
    pub lifetime_seconds: f64,
    #[serde(default)]
    pub history_started_at: Option<i64>,
    pub monthly_graph: Vec<DailyListeningPoint>,
    pub top_songs: Vec<RankedTrackItem>,
    pub top_artists: Vec<RankedArtistItem>,
    #[serde(default)]
    pub top_genres: Vec<RankedGenreItem>,
    pub top_days: Vec<TopListeningDay>,
}

/// One playback's contribution to a user's daily totals.
#[derive(Debug, Clone, Copy)]
pub struct DailyPlayback {
    pub seconds: f64,
    /// Counted as a play (listened long enough or completed).
    pub is_meaningful: bool,
    pub completed: bool,
    pub skipped: bool,
}
