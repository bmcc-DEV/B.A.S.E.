//! base-api — canonical v1: identify · prove · usage (lab · saas_production=false)

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use base_api::{routes, AppState, BillingState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("base_api=info".parse()?))
        .init();

    let api_key =
        std::env::var("BASE_API_KEY").unwrap_or_else(|_| "sk-base-dev-local".into());
    let credits: u64 = std::env::var("BASE_API_CREDITS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    let bind = std::env::var("BASE_API_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());

    let state = Arc::new(AppState {
        billing: BillingState::new_dev(api_key.clone(), credits),
    });

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/v1/health", get(routes::health))
        .route("/v1/openapi.yaml", get(routes::openapi_yaml))
        .route("/openapi.yaml", get(routes::openapi_yaml))
        .route("/v1/prices", get(routes::prices))
        .route("/v1/usage", get(routes::usage))
        .route("/v1/identify", post(routes::identify))
        .route("/v1/prove", post(routes::prove))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = bind.parse()?;
    tracing::info!(
        "base-api v1 on http://{addr}  key={api_key} credits={credits} saas_production=false"
    );
    tracing::info!("canonical: POST /v1/identify · POST /v1/prove · GET /v1/usage");
    tracing::info!("spec: GET /v1/openapi.yaml");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
