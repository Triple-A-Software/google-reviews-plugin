use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Router,
    routing::{get, post},
};
use google_reviews::{
    AppState,
    api::{admin, dashboard, public, settings},
    create_db, create_env, google,
};
use tokio::net::TcpListener;
use tower_http::{
    normalize_path::NormalizePathLayer,
    services::ServeDir,
    trace::{self, TraceLayer},
};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).compact().init();

    let db = create_db().await;
    let state = AppState {
        db,
        env: Arc::new(create_env()),
    };

    // No scheduler exists in the plugin runtime, so keep caches fresh with a
    // background ticker that refreshes only places past their TTL. The first
    // tick fires immediately (refresh on boot).
    {
        let db = state.db.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(30 * 60));
            loop {
                ticker.tick().await;
                if let Err(e) = google::refresh_stale(&db).await {
                    tracing::warn!("google-reviews: scheduled refresh failed: {e}");
                }
            }
        });
    }

    let router = Router::new()
        // Admin UI (static assets)
        .nest_service("/ui", ServeDir::new("ui/dist"))
        // Admin API
        .route(
            "/api/places",
            get(admin::list_places).post(admin::add_place).delete(admin::delete_place),
        )
        .route("/api/reviews", get(admin::list_reviews))
        .route("/api/reviews/moderate", post(admin::moderate))
        .route("/api/reviews/refresh", post(admin::refresh))
        .route("/api/stats", get(admin::stats))
        .route(
            "/api/settings",
            get(settings::route_get_settings).put(settings::route_update_settings),
        )
        // Dashboard cards (server-rendered HTML)
        .route("/dashboard/rating", get(dashboard::dashboard_rating))
        .route("/dashboard/latest", get(dashboard::dashboard_latest))
        // Inline helpers
        .route("/helper/google_reviews", post(public::google_reviews))
        .route("/helper/aggregate_rating", post(public::aggregate_rating))
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind port");
    println!("google-reviews listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
