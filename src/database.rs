use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::{
    model::{Place, Review, ReviewSettings, Stats},
    utils::AppResult,
};

// ---------------------------------------------------------------------------
// Places
// ---------------------------------------------------------------------------

/// Add (or relabel) a place the site should show reviews for.
pub async fn add_place(db: &PgPool, place_id: &str, label: Option<&str>) -> AppResult<()> {
    sqlx::query(
        r#"insert into place (place_id, label) values ($1, $2)
           on conflict (place_id) do update set label = coalesce(excluded.label, place.label)"#,
    )
    .bind(place_id)
    .bind(label)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete_place(db: &PgPool, place_id: &str) -> AppResult<()> {
    sqlx::query(r#"delete from place where place_id = $1"#)
        .bind(place_id)
        .execute(db)
        .await?;
    Ok(())
}

/// Update a place's cached aggregate figures after a Google fetch. Keeps an
/// admin-set label; otherwise adopts Google's display name.
pub async fn upsert_place_meta(
    db: &PgPool,
    place_id: &str,
    rating: Option<f64>,
    total: Option<i64>,
    maps_uri: Option<&str>,
    google_name: Option<&str>,
) -> AppResult<()> {
    sqlx::query(
        r#"insert into place (place_id, rating, total, maps_uri, label, fetched_at)
           values ($1, $2::real, $3::int, $4, $5, now())
           on conflict (place_id) do update set
               rating = excluded.rating,
               total = excluded.total,
               maps_uri = excluded.maps_uri,
               label = coalesce(place.label, excluded.label),
               fetched_at = now()"#,
    )
    .bind(place_id)
    .bind(rating)
    .bind(total)
    .bind(maps_uri)
    .bind(google_name)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_place(db: &PgPool, place_id: &str) -> AppResult<Option<Place>> {
    Ok(sqlx::query_as(
        r#"select place_id, label, rating, total, maps_uri, fetched_at, added_at
           from place where place_id = $1"#,
    )
    .bind(place_id)
    .fetch_optional(db)
    .await?)
}

pub async fn list_places(db: &PgPool) -> AppResult<Vec<Place>> {
    Ok(sqlx::query_as(
        r#"select place_id, label, rating, total, maps_uri, fetched_at, added_at
           from place order by added_at asc"#,
    )
    .fetch_all(db)
    .await?)
}

pub async fn list_place_ids(db: &PgPool) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(r#"select place_id from place"#).fetch_all(db).await?)
}

/// Place ids whose cache is stale (older than `ttl_minutes`) or never fetched.
pub async fn list_stale_place_ids(db: &PgPool, ttl_minutes: i32) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        r#"select place_id from place
           where fetched_at is null or fetched_at < now() - make_interval(mins => $1)"#,
    )
    .bind(ttl_minutes)
    .fetch_all(db)
    .await?)
}

// ---------------------------------------------------------------------------
// Reviews
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub async fn upsert_review(
    db: &PgPool,
    place_id: &str,
    google_id: &str,
    author: &str,
    author_url: Option<&str>,
    photo_url: Option<&str>,
    rating: i32,
    text: Option<&str>,
    lang: Option<&str>,
    published_at: Option<DateTime<Utc>>,
    relative_time: Option<&str>,
) -> AppResult<()> {
    // Note: `hidden` is intentionally not in the update set — moderation
    // survives refreshes.
    sqlx::query(
        r#"insert into review
               (place_id, google_id, author, author_url, photo_url, rating, text, lang,
                published_at, relative_time, fetched_at)
           values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
           on conflict (place_id, google_id) do update set
               author = excluded.author,
               author_url = excluded.author_url,
               photo_url = excluded.photo_url,
               rating = excluded.rating,
               text = excluded.text,
               lang = excluded.lang,
               published_at = excluded.published_at,
               relative_time = excluded.relative_time,
               fetched_at = now()"#,
    )
    .bind(place_id)
    .bind(google_id)
    .bind(author)
    .bind(author_url)
    .bind(photo_url)
    .bind(rating)
    .bind(text)
    .bind(lang)
    .bind(published_at)
    .bind(relative_time)
    .execute(db)
    .await?;
    Ok(())
}

/// Drop cached reviews for a place that Google no longer returns.
pub async fn delete_stale_reviews(db: &PgPool, place_id: &str, keep: &[String]) -> AppResult<()> {
    sqlx::query(r#"delete from review where place_id = $1 and not (google_id = any($2))"#)
        .bind(place_id)
        .bind(keep)
        .execute(db)
        .await?;
    Ok(())
}

const REVIEW_COLS: &str =
    "id, place_id, author, author_url, photo_url, rating, text, lang, published_at, relative_time, hidden";

/// Visible reviews for a place, newest first — what the public block renders.
pub async fn public_reviews(
    db: &PgPool,
    place_id: &str,
    min_rating: i32,
    count: i64,
) -> AppResult<Vec<Review>> {
    Ok(sqlx::query_as(&format!(
        r#"select {REVIEW_COLS} from review
           where place_id = $1 and hidden = false and rating >= $2
           order by published_at desc nulls last, id desc
           limit $3"#,
    ))
    .bind(place_id)
    .bind(min_rating)
    .bind(count)
    .fetch_all(db)
    .await?)
}

/// All reviews for the moderation UI (optionally a single place).
pub async fn list_reviews_admin(db: &PgPool, place_id: Option<&str>) -> AppResult<Vec<Review>> {
    Ok(sqlx::query_as(&format!(
        r#"select {REVIEW_COLS} from review
           where $1::text is null or place_id = $1
           order by hidden asc, published_at desc nulls last, id desc
           limit 500"#,
    ))
    .bind(place_id)
    .fetch_all(db)
    .await?)
}

pub async fn moderate_review(db: &PgPool, id: i64, hidden: bool) -> AppResult<()> {
    sqlx::query(r#"update review set hidden = $2 where id = $1"#)
        .bind(id)
        .bind(hidden)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn stats(db: &PgPool) -> AppResult<Stats> {
    let scalar = |sql: &'static str| async move {
        sqlx::query_scalar::<_, i64>(sql).fetch_one(db).await
    };
    let latest: Vec<Review> = sqlx::query_as(&format!(
        r#"select {REVIEW_COLS} from review order by published_at desc nulls last, id desc limit 8"#,
    ))
    .fetch_all(db)
    .await?;
    let avg_rating: Option<f64> = sqlx::query_scalar(
        r#"select case when sum(total) > 0
                       then sum(rating::float8 * total) / sum(total) else null end
           from place where rating is not null and total is not null"#,
    )
    .fetch_one(db)
    .await?;
    Ok(Stats {
        places: scalar("select count(*) from place").await?,
        reviews: scalar("select count(*) from review").await?,
        hidden: scalar("select count(*) from review where hidden").await?,
        total_ratings: scalar("select coalesce(sum(total), 0)::int8 from place").await?,
        avg_rating,
        latest,
    })
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

pub async fn get_settings(db: &PgPool) -> AppResult<ReviewSettings> {
    Ok(sqlx::query_as(
        r#"insert into review_settings (id) values ('settings')
           on conflict (id) do update set id = 'settings'
           returning api_key, default_count, min_rating, ttl_minutes"#,
    )
    .fetch_one(db)
    .await?)
}

pub async fn update_settings(db: &PgPool, s: &ReviewSettings) -> AppResult<()> {
    sqlx::query(
        r#"insert into review_settings (id, api_key, default_count, min_rating, ttl_minutes)
           values ('settings', $1, $2, $3, $4)
           on conflict (id) do update set
               api_key = excluded.api_key,
               default_count = excluded.default_count,
               min_rating = excluded.min_rating,
               ttl_minutes = excluded.ttl_minutes"#,
    )
    .bind(s.api_key.as_deref())
    .bind(s.default_count)
    .bind(s.min_rating)
    .bind(s.ttl_minutes)
    .execute(db)
    .await?;
    Ok(())
}
