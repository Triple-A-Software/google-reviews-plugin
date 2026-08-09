use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A Google place (business location) with its cached aggregate figures.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Place {
    pub place_id: String,
    pub label: Option<String>,
    pub rating: Option<f32>,
    pub total: Option<i32>,
    pub maps_uri: Option<String>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub added_at: DateTime<Utc>,
}

/// A cached review. Public rendering reads the same rows (filtered to
/// `hidden = false`); the admin sees hidden ones too.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Review {
    pub id: i64,
    pub place_id: String,
    pub author: String,
    pub author_url: Option<String>,
    pub photo_url: Option<String>,
    pub rating: i32,
    pub text: Option<String>,
    pub lang: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub relative_time: Option<String>,
    pub hidden: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaceAdd {
    pub place_id: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModerationRequest {
    pub id: i64,
    pub hidden: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RefreshRequest {
    /// Refresh a single place; `None` refreshes all.
    #[serde(default)]
    pub place_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Stats {
    pub places: i64,
    pub reviews: i64,
    pub hidden: i64,
    pub avg_rating: Option<f64>,
    pub total_ratings: i64,
    pub latest: Vec<Review>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReviewSettings {
    pub api_key: Option<String>,
    pub default_count: i32,
    pub min_rating: i32,
    pub ttl_minutes: i32,
}

impl Default for ReviewSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            default_count: 5,
            min_rating: 1,
            ttl_minutes: 720,
        }
    }
}
