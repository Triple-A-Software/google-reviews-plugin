use axum::{Json, extract::State};

use crate::{AppState, database, model::ReviewSettings, utils::AppResult};

/// GET /api/settings
pub async fn route_get_settings(State(state): State<AppState>) -> AppResult<Json<ReviewSettings>> {
    Ok(Json(database::get_settings(&state.db).await?))
}

/// PUT /api/settings
pub async fn route_update_settings(
    State(state): State<AppState>,
    Json(body): Json<ReviewSettings>,
) -> AppResult<Json<ReviewSettings>> {
    let clean = ReviewSettings {
        api_key: body.api_key.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        // Places API returns at most 5 reviews.
        default_count: body.default_count.clamp(1, 5),
        min_rating: body.min_rating.clamp(1, 5),
        // Don't let the TTL drop below 5 minutes (quota / TOS on refresh rate).
        ttl_minutes: body.ttl_minutes.max(5),
    };
    database::update_settings(&state.db, &clean).await?;
    Ok(Json(database::get_settings(&state.db).await?))
}
