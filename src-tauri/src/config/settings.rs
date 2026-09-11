use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database_path: PathBuf,
    pub cache_dir: PathBuf,
    pub audio: AudioConfig,
    pub history: HistoryConfig,
    pub ranking: RankingWeightsConfig,
    pub recommendation: RecommendationWeightsConfig,
    pub metadata: MetadataConfig,
    pub downloads: DownloadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub default_volume: f32,
    pub output_device: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            default_volume: 0.8,
            output_device: None,
        }
    }
}

/// Parameters defining what counts as a meaningful play.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub min_meaningful_seconds: f64,
    pub min_meaningful_percentage: f64,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            min_meaningful_seconds: 30.0,
            min_meaningful_percentage: 0.50,
        }
    }
}

/// Configurable weights for multi-factor ranking calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingWeightsConfig {
    pub play_count_weight: f64,
    pub listening_duration_weight: f64,
    pub completion_weight: f64,
    pub recency_weight: f64,
    pub user_preference_weight: f64,
    pub skip_penalty: f64,
}

impl Default for RankingWeightsConfig {
    fn default() -> Self {
        Self {
            play_count_weight: 1.0,
            listening_duration_weight: 0.8,
            completion_weight: 1.2,
            recency_weight: 1.5,
            user_preference_weight: 2.0,
            skip_penalty: 1.0,
        }
    }
}

/// Weights contributing to recommendation scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationWeightsConfig {
    pub artist_affinity_weight: f64,
    pub genre_affinity_weight: f64,
    pub track_similarity_weight: f64,
    pub recency_factor_weight: f64,
    pub exploration_weight: f64,
    pub repetition_penalty: f64,
}

impl Default for RecommendationWeightsConfig {
    fn default() -> Self {
        Self {
            artist_affinity_weight: 0.35,
            genre_affinity_weight: 0.25,
            track_similarity_weight: 0.20,
            recency_factor_weight: 0.15,
            exploration_weight: 0.10,
            repetition_penalty: 0.30,
        }
    }
}

/// Metadata provider priority and toggle configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    pub provider_priority: Vec<String>,
    pub enable_musicbrainz: bool,
    pub enable_spotify: bool,
    pub spotify_client_id: Option<String>,
    pub spotify_client_secret: Option<String>,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            provider_priority: vec![
                "embedded".to_string(),
                "musicbrainz".to_string(),
                "spotify".to_string(),
            ],
            enable_musicbrainz: true,
            enable_spotify: false,
            spotify_client_id: None,
            spotify_client_secret: None,
        }
    }
}

/// Soulseek and download provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadConfig {
    pub download_dir: Option<PathBuf>,
    pub slskd_host: String,
    pub slskd_port: u16,
    pub slskd_api_key: Option<String>,
    pub auto_import: bool,
    pub max_concurrent_downloads: usize,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            download_dir: None,
            slskd_host: "localhost".to_string(),
            slskd_port: 5030,
            slskd_api_key: None,
            auto_import: true,
            max_concurrent_downloads: 2,
        }
    }
}

impl AppConfig {
    /// Constructs default configuration based on standard desktop directories.
    pub fn default_with_dirs() -> Self {
        let (db_path, cache_dir) = if let Some(proj_dirs) =
            ProjectDirs::from("org", "intelligentmusicplayer", "music-player")
        {
            let data_dir = proj_dirs.data_dir().to_path_buf();
            let cache = proj_dirs.cache_dir().to_path_buf();
            (data_dir.join("music_player.db"), cache)
        } else {
            (
                PathBuf::from("music_player.db"),
                PathBuf::from(".cache"),
            )
        };

        Self {
            database_path: db_path,
            cache_dir,
            audio: AudioConfig::default(),
            history: HistoryConfig::default(),
            ranking: RankingWeightsConfig::default(),
            recommendation: RecommendationWeightsConfig::default(),
            metadata: MetadataConfig::default(),
            downloads: DownloadConfig::default(),
        }
    }
}
