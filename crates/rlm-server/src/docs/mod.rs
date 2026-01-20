//! API Documentation Module
//!
//! This module provides comprehensive API documentation for the RLM OpenAI-compatible server.
//! It includes OpenAPI specifications, usage examples, and interactive documentation generation.

pub mod openapi;
pub mod examples;
pub mod schemas;

use axum::{response::Html, http::StatusCode};

/// Generate HTML documentation page
pub async fn serve_docs() -> Result<Html<String>, StatusCode> {
    let html_content = include_str!("templates/api_docs.html");
    Ok(Html(html_content.to_string()))
}

/// Health check endpoint for documentation service
pub async fn docs_health() -> Result<&'static str, StatusCode> {
    Ok("API Documentation service is healthy")
}