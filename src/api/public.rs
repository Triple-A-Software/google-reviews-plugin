//! Visitor-facing inline helpers:
//! - `{{ google_reviews(place_id, count) }}` renders the reviews block.
//! - `{{ reviews_aggregate_rating(place_id) }}` emits AggregateRating JSON-LD.

use std::collections::HashMap;

use axum::{Json, extract::State};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::{AppState, database, google, lang::Lang, model::{Place, ReviewSettings}, render};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServicePluginApiResponse<T> {
    #[allow(dead_code)]
    Error(String),
    Data(T),
}

type HelperResp = Json<ServicePluginApiResponse<String>>;

fn data(html: String) -> HelperResp {
    Json(ServicePluginApiResponse::Data(html))
}

#[derive(Deserialize)]
pub struct PageRenderInput {
    pub language: Option<String>,
}

#[derive(Deserialize)]
pub struct HelperBody {
    pub json_args: Vec<Value>,
    pub page: PageRenderInput,
    #[allow(dead_code)]
    pub query: HashMap<String, String>,
    #[allow(dead_code)]
    pub params: HashMap<String, String>,
    #[allow(dead_code)]
    pub route: String,
    #[allow(dead_code)]
    pub interactive: bool,
}

fn is_stale(place: Option<&Place>, ttl_minutes: i32) -> bool {
    match place {
        None => true,
        Some(p) => match p.fetched_at {
            None => true,
            Some(t) => Utc::now() - t > Duration::minutes(ttl_minutes.max(1) as i64),
        },
    }
}

/// When a rendered place's cache is stale (or absent), kick off a background
/// refresh — never blocking the page. A place referenced in a template but not
/// yet registered is auto-added, so `{{ google_reviews("PID", 5) }}` just works.
async fn ensure_fresh(state: &AppState, place_id: &str, place: Option<&Place>, settings: &ReviewSettings) {
    if !is_stale(place, settings.ttl_minutes) {
        return;
    }
    let Some(key) = settings.api_key.clone().filter(|k| !k.is_empty()) else {
        return;
    };
    if place.is_none() {
        let _ = database::add_place(&state.db, place_id, None).await;
    }
    let db = state.db.clone();
    let pid = place_id.to_string();
    tokio::spawn(async move {
        if let Err(e) = google::refresh_place(&db, &key, &pid).await {
            tracing::warn!("google-reviews: on-render refresh {pid} failed: {e}");
        }
    });
}

fn arg_place_id(body: &HelperBody) -> Option<&str> {
    body.json_args
        .first()
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// `google_reviews(place_id, count)` — the reviews block.
pub async fn google_reviews(State(state): State<AppState>, Json(body): Json<HelperBody>) -> HelperResp {
    let Some(place_id) = arg_place_id(&body) else {
        return data(String::new());
    };
    let settings = database::get_settings(&state.db).await.unwrap_or_default();
    let count = body
        .json_args
        .get(1)
        .and_then(Value::as_i64)
        .unwrap_or(settings.default_count as i64)
        .clamp(1, 5);

    let place = database::get_place(&state.db, place_id).await.ok().flatten();
    ensure_fresh(&state, place_id, place.as_ref(), &settings).await;

    let reviews = database::public_reviews(&state.db, place_id, settings.min_rating, count)
        .await
        .unwrap_or_default();
    let lang = Lang::from_code(body.page.language.as_deref());
    data(render::reviews_block(place.as_ref(), &reviews, lang))
}

/// `reviews_aggregate_rating(place_id)` — AggregateRating JSON-LD for rich results.
pub async fn aggregate_rating(State(state): State<AppState>, Json(body): Json<HelperBody>) -> HelperResp {
    let Some(place_id) = arg_place_id(&body) else {
        return data(String::new());
    };
    let settings = database::get_settings(&state.db).await.unwrap_or_default();
    let place = database::get_place(&state.db, place_id).await.ok().flatten();
    ensure_fresh(&state, place_id, place.as_ref(), &settings).await;
    match place {
        Some(p) => data(render::aggregate_rating_jsonld(&p)),
        None => data(String::new()),
    }
}
