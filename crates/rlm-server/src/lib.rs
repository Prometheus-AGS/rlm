//! RLM Server library
//!
//! This crate provides an HTTP server that exposes RLM functionality
//! through an OpenAI-compatible REST API.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod adapters;
pub mod backend_factory;
pub mod config;
pub mod health;
pub mod logging;
pub mod monitoring;
pub mod performance_config;
pub mod server;

pub use adapters::{AzureOpenAiProvider, OpenAiConfig, OpenAiConfigBuilder, OpenAiProvider};
pub use backend_factory::{BackendFactory, BackendFactoryBuilder};
pub use config::{ServerConfig, ConfigSourceInfo, EnvVarInfo, DefaultFileInfo};
pub use health::{BackendHealth, BackendHealthChecker, HealthCheckConfig, HealthStatus, SystemHealth};
pub use monitoring::{Alert, AlertSeverity, AlertType, ErrorRateMonitor, ErrorTrend, MonitoringConfig};
pub use performance_config::{ServerPerformanceConfig, ServerPerformanceBuilder, PerformanceProfiles, PerformanceUtils};
pub use server::RlmServer;

/// Create a test app for integration testing.
#[cfg(test)]
pub async fn create_test_app() -> axum::Router {
    use server::routes::AppState;

    let app_state = AppState::new().expect("Failed to create test app state");

    axum::Router::new()
        .nest("/v1", server::routes::create_v1_router())
        .with_state(app_state)
}
