use std::{env, sync::Arc};

use minijinja::{Environment, path_loader};
use sqlx::PgPool;

pub mod api;
pub mod database;
pub mod google;
pub mod lang;
pub mod model;
pub mod render;
pub mod utils;

/// Shared, cheaply-cloneable application state handed to every axum handler.
#[derive(Clone)]
pub struct AppState {
    /// The plugin's own database (places, reviews, settings).
    pub db: PgPool,
    /// minijinja environment loading dashboard-card templates from `./templates`.
    pub env: Arc<Environment<'static>>,
}

/// Connect to the plugin's own database and run migrations.
pub async fn create_db() -> PgPool {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPool::connect(&db_url)
        .await
        .expect("failed to connect to plugin database");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("failed to run migrations");
    db
}

/// Build the template environment used for server-rendered dashboard cards.
pub fn create_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.set_loader(path_loader("./templates"));
    env
}
