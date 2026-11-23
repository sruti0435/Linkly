mod api;
mod auth;
mod config;
mod error;
mod models;
mod state;

use axum::{Router, extract::DefaultBodyLimit, http::Method, routing::get};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = config::Config::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let state = state::AppState::new(pool, config.clone())?;
    let cors = CorsLayer::new()
        .allow_origin(config.frontend_origin.parse::<axum::http::HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(api::router(state))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let address = SocketAddr::new(config.api_host.parse()?, config.api_port);
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "RustURL API listening");
    axum::serve(listener, app).await?;
    Ok(())
}
