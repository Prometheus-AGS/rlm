//! HTTP server implementation.

use crate::config::ServerConfig;
use anyhow::Result;
use axum::{
    routing::get,
    Router,
};
use std::future::Future;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{info, Level};

pub mod routes;
pub mod middleware;
pub mod streaming;
pub mod connection_manager;
pub mod metrics;

/// Main RLM server.
#[derive(Debug)]
pub struct RlmServer {
    /// Server configuration.
    config: ServerConfig,
    /// HTTP router.
    router: Router,
}

impl RlmServer {
    /// Create a new RLM server.
    pub async fn new(config: ServerConfig) -> Result<Self> {
        let router = create_router(&config).await?;

        Ok(Self { config, router })
    }

    /// Serve with graceful shutdown.
    pub async fn serve_with_shutdown<F>(
        self,
        shutdown_signal: F,
    ) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("RLM server listening on {}", addr);

        axum::serve(listener, self.router)
            .with_graceful_shutdown(shutdown_signal)
            .await?;

        Ok(())
    }

    /// Get server configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

async fn create_router(_config: &ServerConfig) -> Result<Router> {
    // Create app state with OpenAI provider
    let app_state = routes::AppState::new()?;

    let router = Router::new()
        // Comprehensive system health check endpoint
        .merge(routes::create_health_router())
        // Metrics endpoint (placeholder)
        .route("/metrics", get(metrics))
        // v1 API routes - integrated with OpenAI provider and app state
        .nest("/v1", routes::create_v1_router())
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .on_request(DefaultOnRequest::new().level(Level::INFO))
                        .on_response(DefaultOnResponse::new().level(Level::INFO)),
                )
                .layer(CorsLayer::permissive()),
        );

    Ok(router)
}

// Removed basic health check - now using comprehensive system health check
// from routes::system_health_check() at /health endpoint

// Metrics handler now implemented in metrics.rs module
use crate::server::metrics::metrics_handler as metrics;