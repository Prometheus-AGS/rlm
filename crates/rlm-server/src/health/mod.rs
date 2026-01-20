//! Health checking and monitoring system.
//!
//! This module provides comprehensive health checking capabilities for
//! the RLM server, including backend connectivity testing, system
//! resource monitoring, and structured health reporting.

pub mod backend_check;

pub use backend_check::{
    BackendHealth, BackendHealthChecker, HealthCheckConfig,
    HealthCheckConfigBuilder, HealthStatus, SystemHealth,
    SystemResourceInfo, MemoryInfo,
};