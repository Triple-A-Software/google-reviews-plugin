//! Google Places API (New) client + refresh orchestration.
//!
//! We call Place Details (`GET https://places.googleapis.com/v1/places/{id}`)
//! with a field mask and cache the result. The API returns at most 5 reviews and
//! its terms forbid long-term caching, so a refresh replaces the cached set
//! while preserving each review's `hidden` flag (matched by its resource name).
//! There's no scheduler in the plugin runtime, so freshness comes from three
//! places: a background `tokio::interval` (see `main`), a TTL check when a page
//! renders, and the admin "Refresh" button.

use std::time::Duration;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{database, utils::AppResult};

const FIELD_MASK: &str = "id,displayName,rating,userRatingCount,googleMapsUri,reviews";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaceDetails {
    #[serde(default)]
    rating: Option<f64>,
    #[serde(default)]
    user_rating_count: Option<i64>,
    #[serde(default)]
    google_maps_uri: Option<String>,
    #[serde(default)]
    display_name: Option<LocalizedText>,
    #[serde(default)]
    reviews: Vec<GReview>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LocalizedText {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    language_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GReview {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    rating: Option<i64>,
    #[serde(default)]
    text: Option<LocalizedText>,
    #[serde(default)]
    relative_publish_time_description: Option<String>,
    #[serde(default)]
    publish_time: Option<String>,
    #[serde(default)]
    author_attribution: Option<AuthorAttribution>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthorAttribution {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    uri: Option<String>,
    #[serde(default)]
    photo_uri: Option<String>,
}

async fn fetch_place_details(api_key: &str, place_id: &str) -> AppResult<PlaceDetails> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let res = client
        .get(format!("https://places.googleapis.com/v1/places/{place_id}"))
        .header("X-Goog-Api-Key", api_key)
        .header("X-Goog-FieldMask", FIELD_MASK)
        .send()
        .await?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(anyhow!("Places API {status} for {place_id}: {body}").into());
    }
    Ok(res.json::<PlaceDetails>().await?)
}

/// A stable id for a review, used to dedup across refreshes and preserve its
/// `hidden` flag. Prefers the Places resource name; falls back to a composite.
fn review_key(r: &GReview) -> String {
    if let Some(name) = &r.name
        && !name.is_empty()
    {
        return name.clone();
    }
    let author = r
        .author_attribution
        .as_ref()
        .and_then(|a| a.display_name.as_deref())
        .unwrap_or("anon");
    let when = r.publish_time.as_deref().unwrap_or_default();
    format!("{author}|{when}")
}

fn parse_time(s: Option<&str>) -> Option<DateTime<Utc>> {
    s.and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
}

/// Fetch a place from Google and replace its cached reviews (preserving hidden
/// state), updating the place's aggregate rating.
pub async fn refresh_place(db: &PgPool, api_key: &str, place_id: &str) -> AppResult<()> {
    let details = fetch_place_details(api_key, place_id).await?;

    database::upsert_place_meta(
        db,
        place_id,
        details.rating,
        details.user_rating_count,
        details.google_maps_uri.as_deref(),
        details.display_name.as_ref().and_then(|d| d.text.as_deref()),
    )
    .await?;

    let mut keep: Vec<String> = Vec::with_capacity(details.reviews.len());
    for r in &details.reviews {
        let key = review_key(r);
        keep.push(key.clone());
        let attr = r.author_attribution.as_ref();
        database::upsert_review(
            db,
            place_id,
            &key,
            attr.and_then(|a| a.display_name.as_deref()).unwrap_or("Google-Nutzer"),
            attr.and_then(|a| a.uri.as_deref()),
            attr.and_then(|a| a.photo_uri.as_deref()),
            r.rating.unwrap_or(0) as i32,
            r.text.as_ref().and_then(|t| t.text.as_deref()),
            r.text.as_ref().and_then(|t| t.language_code.as_deref()),
            parse_time(r.publish_time.as_deref()),
            r.relative_publish_time_description.as_deref(),
        )
        .await?;
    }
    database::delete_stale_reviews(db, place_id, &keep).await?;
    Ok(())
}

/// Refresh every place. Returns how many refreshed successfully. Per-place errors
/// are logged and skipped so one bad place never fails the whole run.
pub async fn refresh_all(db: &PgPool) -> AppResult<usize> {
    let Some(key) = api_key(db).await? else {
        return Ok(0);
    };
    let mut done = 0;
    for place_id in database::list_place_ids(db).await? {
        match refresh_place(db, &key, &place_id).await {
            Ok(()) => done += 1,
            Err(e) => tracing::warn!("google-reviews: refresh {place_id} failed: {e}"),
        }
    }
    Ok(done)
}

/// Refresh only places whose cache is older than the configured TTL (or never
/// fetched). Used by the background interval and the render-time check.
pub async fn refresh_stale(db: &PgPool) -> AppResult<usize> {
    let settings = database::get_settings(db).await?;
    let Some(key) = settings.api_key.filter(|k| !k.is_empty()) else {
        return Ok(0);
    };
    let mut done = 0;
    for place_id in database::list_stale_place_ids(db, settings.ttl_minutes).await? {
        match refresh_place(db, &key, &place_id).await {
            Ok(()) => done += 1,
            Err(e) => tracing::warn!("google-reviews: refresh {place_id} failed: {e}"),
        }
    }
    Ok(done)
}

async fn api_key(db: &PgPool) -> AppResult<Option<String>> {
    Ok(database::get_settings(db).await?.api_key.filter(|k| !k.is_empty()))
}
