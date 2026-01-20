//! Backend health checking system.
//!
//! This module implements comprehensive health checking for all configured
//! backend providers. It integrates with the backend factory to test
//! connectivity, measure response times, and provide structured health
//! status information for monitoring and diagnostics.

use rlm_core::{
    ports::LlmProvider,
    RlmError, RlmResult,
};
use crate::backend_factory::BackendFactory;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::timeout;
use tracing::{debug, instrument, warn};

/// Overall health status enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Some issues detected but service partially functional
    Degraded,
    /// Critical issues, service may be unavailable
    Unhealthy,
}

/// Individual backend health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendHealth {
    /// Backend health status
    pub status: HealthStatus,
    /// Response time in milliseconds for health check
    pub response_time_ms: Option<u64>,
    /// Last successful check timestamp
    pub last_check: Option<String>,
    /// Error message if unhealthy
    pub error: Option<String>,
    /// Provider-specific metadata
    pub metadata: HashMap<String, String>,
}

/// System-wide health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall system status
    pub status: HealthStatus,
    /// ISO 8601 timestamp of this health check
    pub timestamp: String,
    /// Server version information
    pub version: String,
    /// Individual backend health status
    pub backends: HashMap<String, BackendHealth>,
    /// System resource information
    pub system: SystemResourceInfo,
}

/// System resource monitoring information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceInfo {
    /// Memory usage information
    pub memory: MemoryInfo,
    /// Active session count
    pub active_sessions: u64,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
}

/// Memory usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total memory usage in bytes
    pub used_bytes: u64,
    /// Available memory in bytes
    pub available_bytes: Option<u64>,
    /// Memory usage percentage (0-100)
    pub usage_percent: Option<f64>,
}

/// Configuration for health check behavior.
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Timeout for individual backend health checks
    pub check_timeout: Duration,
    /// Maximum number of concurrent health checks
    pub max_concurrent_checks: usize,
    /// Interval between health check caching
    pub cache_duration: Duration,
    /// Whether to include detailed error information
    pub include_error_details: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_timeout: Duration::from_secs(10),
            max_concurrent_checks: 5,
            cache_duration: Duration::from_secs(30),
            include_error_details: true,
        }
    }
}

/// Backend health checker service.
#[derive(Debug)]
pub struct BackendHealthChecker {
    config: HealthCheckConfig,
    server_start_time: Instant,
    version: String,
}

impl BackendHealthChecker {
    /// Create a new backend health checker.
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            server_start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Create a health checker with default configuration.
    pub fn default() -> Self {
        Self::new(HealthCheckConfig::default())
    }

    /// Perform comprehensive health check of all backends.
    ///
    /// This method checks the health of all configured backends and returns
    /// a structured health report suitable for the /health API endpoint.
    #[instrument(skip(self, providers))]
    pub async fn check_system_health(
        &self,
        providers: &HashMap<String, Arc<dyn LlmProvider + Send + Sync>>,
    ) -> RlmResult<SystemHealth> {
        debug!("Starting system health check for {} backends", providers.len());

        let start_time = Instant::now();

        // Perform backend health checks
        let backend_results = self.check_all_backends(providers).await;

        // Calculate overall system status
        let overall_status = self.calculate_overall_status(&backend_results);

        // Generate timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RlmError::Other(format!("Failed to get timestamp: {}", e)))?
            .as_secs();

        let timestamp_iso = format!("{}Z",
            chrono::DateTime::from_timestamp(timestamp as i64, 0)
                .ok_or_else(|| RlmError::Other("Invalid timestamp".to_string()))?
                .format("%Y-%m-%dT%H:%M:%S")
        );

        // Collect system resource information
        let system_info = self.collect_system_info();

        let health = SystemHealth {
            status: overall_status,
            timestamp: timestamp_iso,
            version: self.version.clone(),
            backends: backend_results,
            system: system_info,
        };

        let check_duration = start_time.elapsed();
        debug!(
            "System health check completed in {}ms, status: {:?}",
            check_duration.as_millis(),
            health.status
        );

        Ok(health)
    }

    /// Check health of all backends concurrently.
    async fn check_all_backends(
        &self,
        providers: &HashMap<String, Arc<dyn LlmProvider + Send + Sync>>,
    ) -> HashMap<String, BackendHealth> {
        let mut results = HashMap::new();

        // Use semaphore to limit concurrent checks
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_checks));
        let mut tasks = Vec::new();

        for (name, provider) in providers {
            let name = name.clone();
            let provider = provider.clone();
            let semaphore = semaphore.clone();
            let timeout_duration = self.config.check_timeout;
            let include_errors = self.config.include_error_details;

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("Semaphore closed");
                let result = Self::check_single_backend(&name, &provider, timeout_duration, include_errors).await;
                (name, result)
            });

            tasks.push(task);
        }

        // Collect results
        for task in tasks {
            if let Ok((name, health)) = task.await {
                results.insert(name, health);
            }
        }

        results
    }

    /// Check health of a single backend provider.
    async fn check_single_backend(
        name: &str,
        provider: &Arc<dyn LlmProvider + Send + Sync>,
        timeout_duration: Duration,
        include_error_details: bool,
    ) -> BackendHealth {
        let start_time = Instant::now();

        debug!("Checking health of backend: {}", name);

        // Perform health check with timeout
        let health_result = timeout(timeout_duration, provider.health_check()).await;

        let response_time_ms = start_time.elapsed().as_millis() as u64;

        match health_result {
            Ok(Ok(is_healthy)) => {
                let status = if is_healthy {
                    HealthStatus::Healthy
                } else {
                    HealthStatus::Degraded
                };

                debug!("Backend {} health check: {:?} ({}ms)", name, status, response_time_ms);

                BackendHealth {
                    status,
                    response_time_ms: Some(response_time_ms),
                    last_check: Some(Self::current_timestamp()),
                    error: None,
                    metadata: HashMap::new(),
                }
            }
            Ok(Err(e)) => {
                warn!("Backend {} health check failed: {}", name, e);

                BackendHealth {
                    status: HealthStatus::Unhealthy,
                    response_time_ms: Some(response_time_ms),
                    last_check: Some(Self::current_timestamp()),
                    error: if include_error_details {
                        Some(e.to_string())
                    } else {
                        Some("Health check failed".to_string())
                    },
                    metadata: HashMap::new(),
                }
            }
            Err(_) => {
                warn!("Backend {} health check timed out after {}ms", name, timeout_duration.as_millis());

                BackendHealth {
                    status: HealthStatus::Unhealthy,
                    response_time_ms: Some(timeout_duration.as_millis() as u64),
                    last_check: Some(Self::current_timestamp()),
                    error: if include_error_details {
                        Some(format!("Health check timed out after {}ms", timeout_duration.as_millis()))
                    } else {
                        Some("Health check timeout".to_string())
                    },
                    metadata: HashMap::new(),
                }
            }
        }
    }

    /// Calculate overall system status based on backend health.
    fn calculate_overall_status(&self, backends: &HashMap<String, BackendHealth>) -> HealthStatus {
        if backends.is_empty() {
            return HealthStatus::Unhealthy;
        }

        let healthy_count = backends.values()
            .filter(|health| health.status == HealthStatus::Healthy)
            .count();

        let degraded_count = backends.values()
            .filter(|health| health.status == HealthStatus::Degraded)
            .count();

        let total_backends = backends.len();

        // System status logic:
        // - Healthy: All backends healthy
        // - Degraded: Some backends healthy, some degraded/unhealthy
        // - Unhealthy: No backends healthy
        if healthy_count == total_backends {
            HealthStatus::Healthy
        } else if healthy_count > 0 || degraded_count > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        }
    }

    /// Collect system resource information.
    fn collect_system_info(&self) -> SystemResourceInfo {
        SystemResourceInfo {
            memory: self.collect_memory_info(),
            active_sessions: 0, // TODO: Implement session tracking
            uptime_seconds: self.server_start_time.elapsed().as_secs(),
        }
    }

    /// Collect memory usage information.
    fn collect_memory_info(&self) -> MemoryInfo {
        // Basic memory information - could be enhanced with system-specific details
        MemoryInfo {
            used_bytes: 0, // TODO: Implement actual memory tracking
            available_bytes: None,
            usage_percent: None,
        }
    }

    /// Get current timestamp in ISO 8601 format.
    fn current_timestamp() -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        chrono::DateTime::from_timestamp(now as i64, 0)
            .map(|dt| format!("{}Z", dt.format("%Y-%m-%dT%H:%M:%S")))
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
    }

    /// Test connectivity to all providers using the backend factory.
    ///
    /// This is a convenience method that integrates with BackendFactory
    /// to perform health checks on all configured providers.
    #[instrument(skip(providers))]
    pub async fn test_providers_connectivity(
        providers: &HashMap<String, Arc<dyn LlmProvider + Send + Sync>>,
    ) -> HashMap<String, bool> {
        BackendFactory::health_check_providers(providers).await
    }
}

/// Builder for health check configuration.
#[derive(Debug)]
pub struct HealthCheckConfigBuilder {
    config: HealthCheckConfig,
}

impl HealthCheckConfigBuilder {
    /// Create a new health check configuration builder.
    pub fn new() -> Self {
        Self {
            config: HealthCheckConfig::default(),
        }
    }

    /// Set the timeout for individual backend health checks.
    pub fn check_timeout(mut self, timeout: Duration) -> Self {
        self.config.check_timeout = timeout;
        self
    }

    /// Set the maximum number of concurrent health checks.
    pub fn max_concurrent_checks(mut self, max: usize) -> Self {
        self.config.max_concurrent_checks = max;
        self
    }

    /// Set the cache duration for health check results.
    pub fn cache_duration(mut self, duration: Duration) -> Self {
        self.config.cache_duration = duration;
        self
    }

    /// Set whether to include detailed error information.
    pub fn include_error_details(mut self, include: bool) -> Self {
        self.config.include_error_details = include;
        self
    }

    /// Build the health check configuration.
    pub fn build(self) -> HealthCheckConfig {
        self.config
    }
}

impl Default for HealthCheckConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_health_status_serialization() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::Healthy).unwrap(),
            "\"healthy\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Degraded).unwrap(),
            "\"degraded\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Unhealthy).unwrap(),
            "\"unhealthy\""
        );
    }

    #[test]
    fn test_backend_health_serialization() {
        let health = BackendHealth {
            status: HealthStatus::Healthy,
            response_time_ms: Some(150),
            last_check: Some("2024-01-19T12:00:00Z".to_string()),
            error: None,
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("\"healthy\""));
        assert!(json.contains("150"));
    }

    #[test]
    fn test_health_check_config_builder() {
        let config = HealthCheckConfigBuilder::new()
            .check_timeout(Duration::from_secs(5))
            .max_concurrent_checks(3)
            .include_error_details(false)
            .build();

        assert_eq!(config.check_timeout, Duration::from_secs(5));
        assert_eq!(config.max_concurrent_checks, 3);
        assert_eq!(config.include_error_details, false);
    }

    #[test]
    fn test_overall_status_calculation() {
        let checker = BackendHealthChecker::default();

        // Empty backends should be unhealthy
        let empty_backends = HashMap::new();
        assert_eq!(
            checker.calculate_overall_status(&empty_backends),
            HealthStatus::Unhealthy
        );

        // All healthy backends
        let mut healthy_backends = HashMap::new();
        healthy_backends.insert("backend1".to_string(), BackendHealth {
            status: HealthStatus::Healthy,
            response_time_ms: Some(100),
            last_check: Some("2024-01-19T12:00:00Z".to_string()),
            error: None,
            metadata: HashMap::new(),
        });
        healthy_backends.insert("backend2".to_string(), BackendHealth {
            status: HealthStatus::Healthy,
            response_time_ms: Some(150),
            last_check: Some("2024-01-19T12:00:00Z".to_string()),
            error: None,
            metadata: HashMap::new(),
        });
        assert_eq!(
            checker.calculate_overall_status(&healthy_backends),
            HealthStatus::Healthy
        );

        // Mixed health status should be degraded
        let mut mixed_backends = healthy_backends.clone();
        mixed_backends.insert("backend3".to_string(), BackendHealth {
            status: HealthStatus::Unhealthy,
            response_time_ms: Some(5000),
            last_check: Some("2024-01-19T12:00:00Z".to_string()),
            error: Some("Connection failed".to_string()),
            metadata: HashMap::new(),
        });
        assert_eq!(
            checker.calculate_overall_status(&mixed_backends),
            HealthStatus::Degraded
        );
    }

    #[test]
    fn test_current_timestamp_format() {
        let timestamp = BackendHealthChecker::current_timestamp();
        assert!(timestamp.ends_with('Z'));
        assert!(timestamp.contains('T'));
        // Should be in ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(timestamp.len(), 20);
    }
}