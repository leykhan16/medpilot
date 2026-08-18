mod auth;
mod config;
mod error;
mod extractors;
mod models;
mod routes;
mod services;
mod state;

use axum::{
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
use config::Config;
use serde_json::json;
use services::ai::{mock::MockAnalyzer, AiAnalyzer};
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "medpilot=debug,tower_http=debug".into()),
        )
        .init();

    let config = Config::from_env()?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("migrations applied");

    let ai: Arc<dyn AiAnalyzer> = Arc::new(MockAnalyzer::new());

    let state = AppState { db, config: config.clone(), ai };

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/login", post(routes::auth::login))
        .route("/me", get(routes::auth::me))
        .route("/cases", post(routes::cases::create_case))
        .route("/cases", get(routes::cases::list_cases))
        .route("/cases/:id", get(routes::cases::get_case))
        .route("/cases/:id/messages", post(routes::cases::send_message))
        .route("/cases/:id/analyze", post(routes::cases::analyze_case))
        .route("/cases/:id/book", post(routes::appointments::book_appointment))
        .route("/appointments", get(routes::appointments::list_appointments))
        .route("/alerts", get(routes::appointments::list_alerts))
        .route("/alerts/:id/acknowledge", post(routes::appointments::acknowledge_alert))
        .route("/dashboard/summary", get(routes::dashboard::summary))
        .layer(CorsLayer::new().allow_origin(Any).allow_headers(Any).allow_methods(Any))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
