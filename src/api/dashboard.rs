use axum::{extract::State, response::Html};
use minijinja::context;
use serde::Serialize;

use crate::{AppState, database, model::Review, utils::AppResult};

fn stars(rating: i32) -> String {
    let n = rating.clamp(0, 5) as usize;
    format!("{}{}", "★".repeat(n), "☆".repeat(5 - n))
}

#[derive(Serialize)]
struct CardRow {
    author: String,
    stars: String,
    snippet: String,
    time: String,
}

fn card_rows(reviews: Vec<Review>) -> Vec<CardRow> {
    reviews
        .into_iter()
        .map(|r| CardRow {
            author: r.author,
            stars: stars(r.rating),
            snippet: truncate(r.text.as_deref().unwrap_or(""), 80),
            time: r.relative_time.unwrap_or_default(),
        })
        .collect()
}

fn truncate(s: &str, max: usize) -> String {
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max {
        return collapsed;
    }
    format!("{}…", collapsed.chars().take(max).collect::<String>().trim_end())
}

/// GET /dashboard/rating — average rating + total across all places.
pub async fn dashboard_rating(State(state): State<AppState>) -> AppResult<Html<String>> {
    let s = database::stats(&state.db).await?;
    let avg = s.avg_rating.map(|v| format!("{v:.1}")).unwrap_or_else(|| "–".to_string());
    let star_str = s.avg_rating.map(|v| stars(v.round() as i32)).unwrap_or_default();
    let tmpl = state.env.get_template("dashboard_rating.html")?;
    Ok(Html(tmpl.render(context! {
        avg => avg,
        stars => star_str,
        total => s.total_ratings,
        places => s.places,
    })?))
}

/// GET /dashboard/latest — most recently fetched reviews.
pub async fn dashboard_latest(State(state): State<AppState>) -> AppResult<Html<String>> {
    let s = database::stats(&state.db).await?;
    let tmpl = state.env.get_template("dashboard_latest.html")?;
    Ok(Html(tmpl.render(context! {
        reviews => card_rows(s.latest),
    })?))
}
