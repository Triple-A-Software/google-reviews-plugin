use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState, database, google,
    model::{ModerationRequest, Place, PlaceAdd, RefreshRequest, Review, Stats},
    utils::{AppError, AppResult},
};

// ---------------------------------------------------------------------------
// Places
// ---------------------------------------------------------------------------

/// GET /api/places
pub async fn list_places(State(state): State<AppState>) -> AppResult<Json<Vec<Place>>> {
    Ok(Json(database::list_places(&state.db).await?))
}

/// POST /api/places — add a place and fetch it immediately so the admin sees
/// data right away. Returns the place plus any fetch error (so a bad key / id is
/// visible without the row silently staying empty).
pub async fn add_place(
    State(state): State<AppState>,
    Json(body): Json<PlaceAdd>,
) -> AppResult<Json<Value>> {
    let place_id = body.place_id.trim();
    if place_id.is_empty() {
        return Err(AppError::BadRequest("place_id is required".into()));
    }
    database::add_place(&state.db, place_id, body.label.as_deref()).await?;

    let mut fetch_error: Option<String> = None;
    if let Some(key) = database::get_settings(&state.db).await?.api_key.filter(|k| !k.is_empty()) {
        if let Err(e) = google::refresh_place(&state.db, &key, place_id).await {
            fetch_error = Some(e.to_string());
        }
    } else {
        fetch_error = Some("No API key configured — set it in settings, then refresh.".into());
    }

    let place = database::get_place(&state.db, place_id).await?;
    Ok(Json(json!({ "place": place, "error": fetch_error })))
}

#[derive(Deserialize)]
pub struct DeletePlace {
    pub place_id: String,
}

/// DELETE /api/places
pub async fn delete_place(
    State(state): State<AppState>,
    Json(body): Json<DeletePlace>,
) -> AppResult<Json<Value>> {
    database::delete_place(&state.db, body.place_id.trim()).await?;
    Ok(Json(json!({ "success": true })))
}

// ---------------------------------------------------------------------------
// Reviews
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ReviewQuery {
    pub place_id: Option<String>,
}

/// GET /api/reviews?place_id=…
pub async fn list_reviews(
    State(state): State<AppState>,
    Query(q): Query<ReviewQuery>,
) -> AppResult<Json<Vec<Review>>> {
    Ok(Json(database::list_reviews_admin(&state.db, q.place_id.as_deref()).await?))
}

/// POST /api/reviews/moderate — {id, hidden}
pub async fn moderate(
    State(state): State<AppState>,
    Json(req): Json<ModerationRequest>,
) -> AppResult<Json<Value>> {
    database::moderate_review(&state.db, req.id, req.hidden).await?;
    Ok(Json(json!({ "success": true })))
}

/// POST /api/reviews/refresh — {place_id?} (a single place, or all)
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<Value>> {
    match req.place_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(place_id) => {
            let key = database::get_settings(&state.db)
                .await?
                .api_key
                .filter(|k| !k.is_empty())
                .ok_or_else(|| AppError::BadRequest("No API key configured.".into()))?;
            google::refresh_place(&state.db, &key, place_id).await?;
            Ok(Json(json!({ "refreshed": 1 })))
        }
        None => {
            let n = google::refresh_all(&state.db).await?;
            Ok(Json(json!({ "refreshed": n })))
        }
    }
}

/// GET /api/stats
pub async fn stats(State(state): State<AppState>) -> AppResult<Json<Stats>> {
    Ok(Json(database::stats(&state.db).await?))
}
