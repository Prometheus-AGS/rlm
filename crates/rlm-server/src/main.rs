//! RLM OpenAI-Compatible Server
//!
//! A production-ready HTTP server that provides OpenAI-compatible chat completions API
//! with RLM (Recursive Language Model) capabilities for handling arbitrarily long contexts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

use anyhow::Result;
use clap::Parser;
use rlm_server::{config::ServerConfig, server::RlmServer};
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "rlm-server")]
#[command(about = "RLM OpenAI-Compatible Server")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Server host
    #[arg(long, env = "RLM_SERVER_HOST")]
    host: Option<String>,

    /// Server port
    #[arg(short, long, env = "RLM_SERVER_PORT")]
    port: Option<u16>,

    /// Log level
    #[arg(long, env = "RLM_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Log format (json or pretty)
    #[arg(long, env = "RLM_LOG_FORMAT", default_value = "pretty")]
    log_format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(&cli.log_level, &cli.log_format)?;

    info!(
        "Starting RLM Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    let config = ServerConfig::load(cli.config.as_deref())?;

    // Override with CLI arguments
    let config = override_config_with_cli(config, &cli);

    // Validate configuration
    config.validate()?;

    info!("Configuration loaded successfully");
    info!("Server will bind to {}:{}", config.server.host, config.server.port);

    // Create and start server
    let server = RlmServer::new(config).await?;

    // Handle graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        warn!("Shutdown signal received, stopping server...");
    };

    // Start server with graceful shutdown
    if let Err(e) = server.serve_with_shutdown(shutdown_signal).await {
        tracing::error!("Server error: {}", e);
        return Err(e);
    }

    info!("Server stopped gracefully");
    Ok(())
}

fn init_logging(level: &str, format: &str) -> Result<()> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .expect("Invalid log level");

    match format {
        "json" => {
            tracing_subscriber::registry()
                .with(fmt::layer().json())
                .with(env_filter)
                .init();
        }
        "pretty" | _ => {
            tracing_subscriber::registry()
                .with(fmt::layer().pretty())
                .with(env_filter)
                .init();
        }
    }

    Ok(())
}

fn override_config_with_cli(mut config: ServerConfig, cli: &Cli) -> ServerConfig {
    if let Some(host) = &cli.host {
        config.server.host = host.clone();
    }
    if let Some(port) = cli.port {
        config.server.port = port;
    }
    config
}