//! Tracing subscriber infrastructure for structured logging.

use anyhow::Result;
use tracing::Level;
use tracing_subscriber::{
    fmt::{format::FmtSpan, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Initialize tracing subscriber based on configuration.
pub fn init_tracing(level: &str, format: &str) -> Result<()> {
    let log_level = parse_log_level(level)?;

    // Create base filter
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| {
            EnvFilter::new(format!("{}={}", env!("CARGO_CRATE_NAME"), level))
        });

    // Create the subscriber based on format preference
    match format.to_lowercase().as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_target(true)
                        .with_current_span(false)
                        .with_span_list(true)
                        .with_timer(UtcTime::rfc_3339())
                        .with_filter(env_filter),
                )
                .init();
        }
        "pretty" | _ => {
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .pretty()
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_thread_names(true)
                        .with_span_events(FmtSpan::CLOSE)
                        .with_timer(UtcTime::rfc_3339())
                        .with_filter(env_filter),
                )
                .init();
        }
    }

    tracing::info!(
        level = %log_level,
        format = format,
        "Tracing initialized"
    );

    Ok(())
}

/// Parse log level string into tracing Level.
fn parse_log_level(level: &str) -> Result<Level> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(Level::TRACE),
        "debug" => Ok(Level::DEBUG),
        "info" => Ok(Level::INFO),
        "warn" | "warning" => Ok(Level::WARN),
        "error" => Ok(Level::ERROR),
        _ => anyhow::bail!("Invalid log level: {}. Valid levels: trace, debug, info, warn, error", level),
    }
}

/// Create a structured log context for RLM operations.
pub fn create_request_span(request_id: &str, operation: &str) -> tracing::Span {
    tracing::info_span!(
        "rlm_request",
        request_id = request_id,
        operation = operation,
        tokens.input = tracing::field::Empty,
        tokens.output = tracing::field::Empty,
        tokens.recursive = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
        success = tracing::field::Empty,
    )
}

/// Add token usage information to the current span.
pub fn record_token_usage(input: u32, output: u32, recursive: u32) {
    let span = tracing::Span::current();
    span.record("tokens.input", input);
    span.record("tokens.output", output);
    span.record("tokens.recursive", recursive);
}

/// Record operation completion in the current span.
pub fn record_completion(duration_ms: u64, success: bool) {
    let span = tracing::Span::current();
    span.record("duration_ms", duration_ms);
    span.record("success", success);

    if success {
        tracing::info!("Operation completed successfully");
    } else {
        tracing::error!("Operation failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("trace").unwrap(), Level::TRACE);
        assert_eq!(parse_log_level("DEBUG").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("Info").unwrap(), Level::INFO);
        assert_eq!(parse_log_level("WARN").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("error").unwrap(), Level::ERROR);

        assert!(parse_log_level("invalid").is_err());
    }
}