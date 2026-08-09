# Development

Developer notes for the `google-reviews` Neleto plugin. For the user-facing
description, see [readme.md](readme.md).

> Status: **P1 + P2** of the [plugin roadmap](../plugin-cli/docs/plugin-roadmap.md).
> P1: Places fetch + cache + component + settings UI. P2: dashboard cards +
> AggregateRating JSON-LD + hide/moderate. **P3 (not started):** Google Business
> Profile OAuth for the full review set + owner replies.

## How it works

The plugin has no CMS dependency — it stores everything in its own database
(`DATABASE_URL`) and talks to Google. It calls the **Places API (New)** Place
Details endpoint:

```
GET https://places.googleapis.com/v1/places/{PLACE_ID}
  X-Goog-Api-Key: <key>
  X-Goog-FieldMask: id,displayName,rating,userRatingCount,googleMapsUri,reviews
```

The API returns the aggregate `rating` + `userRatingCount` and **at most 5
reviews**, and its terms forbid long-term caching — so a refresh *replaces* the
cached review set (`google.rs`): reviews are upserted by their Places resource
name and any that Google no longer returns are deleted. The `hidden` moderation
flag is **preserved across refreshes** (it's excluded from the upsert's update
set), so hiding a review sticks.

The API key lives in `review_settings`, never in the CMS.

## Freshness (no scheduler)

There's no cron in the plugin runtime, so freshness comes from three places:

1. **Background ticker** — a `tokio::interval` (30 min) in `main` calls
   `refresh_stale`, which refreshes only places past their TTL (`ttl_minutes`,
   default 720). First tick fires on boot.
2. **On render** — the `google_reviews` / `reviews_aggregate_rating` helpers
   spawn a background refresh for a place whose cache is stale (never blocking
   the page). A Place ID used in a template but not yet registered is
   **auto-added**, so `{{ google_reviews("PID", 5) }}` works with no admin step.
3. **Manual** — the admin "Refresh" buttons (`/api/reviews/refresh`), and adding
   a place fetches it synchronously so the admin sees data immediately.

TTL is clamped to ≥ 5 min to respect Google's refresh-rate expectations and
control quota/cost.

## Rendering, attribution & XSS

`render.rs` builds the block and the JSON-LD. Everything Google-supplied (author
names, review text, author/photo URLs) is escaped (`escape_html` /
`escape_multiline`) — the XSS boundary is this module. Each block links back to
the place on Google, as the terms require. The AggregateRating JSON-LD escapes
`<` → `<` so it can't break out of its `<script>` tag, and rounds the rating
in f64 so it serializes exactly (an f32 `4.6` widens to `4.599…` otherwise).

## Manifest hooks

| Hook | Route | Purpose |
|---|---|---|
| `helpers` | `/helper/google_reviews` | `{{ google_reviews(place_id, count) }}` — the block |
| `helpers` | `/helper/aggregate_rating` | `{{ reviews_aggregate_rating(place_id) }}` — JSON-LD |
| `components` | `components/google-reviews` | Editor block; a thin wrapper over the helper |
| `dashboard_cards` | `/dashboard/rating`, `/dashboard/latest` | Average rating + latest reviews |
| `api` | `/api/places` | List / add (fetches) / delete places |
| `api` | `/api/reviews`, `/api/reviews/moderate`, `/api/reviews/refresh` | Moderation list + hide + refresh |
| `api` | `/api/settings`, `/api/stats` | Settings + admin counts |
| `ui` | `/ui` | Admin panel |

## Running locally

```sh
cp .env.example .env      # DATABASE_URL at local Postgres (CMS_DATABASE_URL unused)
cargo run
```

Set a real Places API key + Place ID in the admin to see live data. `cargo test`
covers the pure rendering + JSON-LD logic. The Linux release binary is built with
`cross` (see the justfile), then `plugin-cli package`.

## Layout

```
src/
  main.rs        axum app + routing + background refresh ticker
  lib.rs         AppState, DB + template setup
  lang.rs        de/en strings for the visitor UI
  model.rs       Place, Review, ReviewSettings, Stats
  google.rs      Places API (New) client + refresh orchestration
  database.rs    places/reviews CRUD, refresh upserts, stats, settings
  render.rs      reviews block + stars + attribution + AggregateRating JSON-LD (+ tests)
  api/
    public.rs    google_reviews + aggregate_rating helpers (+ on-render refresh)
    admin.rs     places / reviews / refresh / moderate / stats
    settings.rs  get / update settings
    dashboard.rs rating + latest cards
templates/       minijinja dashboard-card fragments
components/google-reviews/  editor block (calls the helper)
ui/dist/         static admin panel
migrations/      plugin schema (place, review, review_settings)
```

## Roadmap (next — P3)

- **Google Business Profile API (OAuth)** — the owner-authenticated API returns
  the *full* review history (not just 5) and lets you post **replies**. This is a
  sizeable, separate effort (OAuth flow + token storage) and is the main reason
  P3 is deferred.
- **Rating trend** — store a small history of `(rating, total, fetched_at)` per
  place to chart movement over time on the dashboard.
- **Per-place min-rating / language** overrides (currently global settings).
- **Legacy Places API fallback** if a project can't enable the New API.
```
