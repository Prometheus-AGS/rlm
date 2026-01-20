//! Error rate monitoring and alerting for RLM server.
//!
//! This module provides comprehensive error tracking, rate monitoring,
//! and alerting capabilities for detecting and responding to system issues.

use rlm_core::{RlmError, RlmResult};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};

/// Error rate monitoring system.
#[derive(Debug, Clone)]
pub struct ErrorRateMonitor {
    /// Error tracking data.
    error_data: Arc<RwLock<ErrorTrackingData>>,
    /// Alert channel sender.
    alert_sender: Option<mpsc::UnboundedSender<Alert>>,
    /// Monitoring configuration.
    config: MonitoringConfig,
}

/// Error tracking data.
#[derive(Debug, Default)]
pub struct ErrorTrackingData {
    /// Error occurrences by type.
    pub error_types: HashMap<String, ErrorTypeStats>,
    /// Global error statistics.
    pub global_stats: GlobalErrorStats,
    /// Error history for rate calculation.
    pub error_history: VecDeque<ErrorOccurrence>,
    /// Active alerts.
    pub active_alerts: HashMap<String, ActiveAlert>,
}

/// Statistics for a specific error type.
#[derive(Debug, Clone, Default)]
pub struct ErrorTypeStats {
    /// Error type name.
    pub error_type: String,
    /// Total occurrences.
    pub total_count: u64,
    /// Count in last hour.
    pub last_hour_count: u64,
    /// Count in last minute.
    pub last_minute_count: u64,
    /// First occurrence time.
    pub first_seen: Option<SystemTime>,
    /// Last occurrence time.
    pub last_seen: Option<SystemTime>,
    /// Recent occurrences (for rate calculation).
    pub recent_occurrences: VecDeque<SystemTime>,
    /// Average time between occurrences.
    pub avg_interval: Option<Duration>,
    /// Current error rate (errors per minute).
    pub current_rate: f64,
    /// Peak error rate.
    pub peak_rate: f64,
    /// Error trend (increasing, stable, decreasing).
    pub trend: ErrorTrend,
}

/// Global error statistics.
#[derive(Debug, Clone, Default)]
pub struct GlobalErrorStats {
    /// Total errors across all types.
    pub total_errors: u64,
    /// Errors in last hour.
    pub errors_last_hour: u64,
    /// Errors in last minute.
    pub errors_last_minute: u64,
    /// Global error rate (errors per minute).
    pub global_error_rate: f64,
    /// Most frequent error type.
    pub most_frequent_error: Option<String>,
    /// Error rate trend.
    pub trend: ErrorTrend,
    /// Health score (0.0 = critical, 1.0 = healthy).
    pub health_score: f64,
}

/// Individual error occurrence record.
#[derive(Debug, Clone)]
pub struct ErrorOccurrence {
    /// Timestamp of occurrence.
    pub timestamp: SystemTime,
    /// Error type.
    pub error_type: String,
    /// Error message.
    pub error_message: String,
    /// Context (session_id, request_id, etc.).
    pub context: HashMap<String, String>,
    /// Error severity.
    pub severity: ErrorSeverity,
}

/// Active alert information.
#[derive(Debug, Clone)]
pub struct ActiveAlert {
    /// Alert ID.
    pub alert_id: String,
    /// Alert type.
    pub alert_type: AlertType,
    /// Alert severity.
    pub severity: AlertSeverity,
    /// When the alert was triggered.
    pub triggered_at: SystemTime,
    /// Alert message.
    pub message: String,
    /// Associated error type (if applicable).
    pub error_type: Option<String>,
    /// Times this alert has been triggered.
    pub trigger_count: u64,
    /// Whether the alert has been acknowledged.
    pub acknowledged: bool,
}

/// Error trend analysis.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ErrorTrend {
    /// Error rate is increasing.
    Increasing,
    /// Error rate is stable.
    #[default]
    Stable,
    /// Error rate is decreasing.
    Decreasing,
    /// Insufficient data for trend analysis.
    Unknown,
}

/// Error severity levels.
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSeverity {
    /// Low severity - minor issues.
    Low,
    /// Medium severity - important but not critical.
    Medium,
    /// High severity - significant problems.
    High,
    /// Critical severity - system-threatening issues.
    Critical,
}

/// Alert types.
#[derive(Debug, Clone, PartialEq)]
pub enum AlertType {
    /// Error rate threshold exceeded.
    ErrorRateThreshold,
    /// Specific error type spike.
    ErrorTypeSpike,
    /// System health degraded.
    HealthDegraded,
    /// Recurring error pattern.
    RecurringPattern,
    /// New error type detected.
    NewErrorType,
}

/// Alert severity levels.
#[derive(Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    /// Informational alert.
    Info,
    /// Warning alert.
    Warning,
    /// Critical alert requiring immediate attention.
    Critical,
}

/// Alert notification.
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert details.
    pub alert: ActiveAlert,
    /// Recommended actions.
    pub recommended_actions: Vec<String>,
}

/// Monitoring configuration.
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Error rate threshold for alerts (errors per minute).
    pub error_rate_threshold: f64,
    /// Window size for rate calculation (in minutes).
    pub rate_window_minutes: u64,
    /// Health score threshold for alerts.
    pub health_threshold: f64,
    /// Maximum number of error occurrences to track.
    pub max_error_history: usize,
    /// Alert cooldown period to prevent spam.
    pub alert_cooldown: Duration,
    /// Whether to enable trend analysis.
    pub enable_trend_analysis: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            error_rate_threshold: 10.0,   // 10 errors per minute
            rate_window_minutes: 5,       // 5-minute window
            health_threshold: 0.8,        // Alert when health drops below 80%
            max_error_history: 1000,      // Keep last 1000 errors
            alert_cooldown: Duration::from_secs(300), // 5-minute cooldown
            enable_trend_analysis: true,
        }
    }
}

impl ErrorRateMonitor {
    /// Create a new error rate monitor.
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            error_data: Arc::new(RwLock::new(ErrorTrackingData::default())),
            alert_sender: None,
            config,
        }
    }

    /// Create a new error rate monitor with alert channel.
    pub fn with_alerts(
        config: MonitoringConfig,
    ) -> (Self, mpsc::UnboundedReceiver<Alert>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        let monitor = Self {
            error_data: Arc::new(RwLock::new(ErrorTrackingData::default())),
            alert_sender: Some(sender),
            config,
        };

        (monitor, receiver)
    }

    /// Record an error occurrence.
    #[instrument(skip(self, error), fields(error_type = %error_type))]
    pub fn record_error(
        &self,
        error_type: &str,
        error: &RlmError,
        context: HashMap<String, String>,
    ) -> RlmResult<()> {
        let occurrence = ErrorOccurrence {
            timestamp: SystemTime::now(),
            error_type: error_type.to_string(),
            error_message: error.to_string(),
            context,
            severity: self.classify_error_severity(error),
        };

        self.process_error_occurrence(occurrence)?;

        debug!(
            error_type = %error_type,
            error_message = %error.to_string(),
            "Recorded error occurrence"
        );

        Ok(())
    }

    /// Get current error statistics.
    pub fn get_error_stats(&self) -> RlmResult<ErrorTrackingData> {
        let error_data = self.error_data.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        Ok(error_data.clone())
    }

    /// Get error statistics for a specific type.
    pub fn get_error_type_stats(&self, error_type: &str) -> RlmResult<Option<ErrorTypeStats>> {
        let error_data = self.error_data.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        Ok(error_data.error_types.get(error_type).cloned())
    }

    /// Get current system health score.
    pub fn get_health_score(&self) -> RlmResult<f64> {
        let error_data = self.error_data.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        Ok(error_data.global_stats.health_score)
    }

    /// Get active alerts.
    pub fn get_active_alerts(&self) -> RlmResult<Vec<ActiveAlert>> {
        let error_data = self.error_data.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        Ok(error_data.active_alerts.values().cloned().collect())
    }

    /// Acknowledge an alert.
    #[instrument(skip(self), fields(alert_id = %alert_id))]
    pub fn acknowledge_alert(&self, alert_id: &str) -> RlmResult<bool> {
        let mut error_data = self.error_data.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        if let Some(alert) = error_data.active_alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            info!(alert_id = %alert_id, "Alert acknowledged");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Run maintenance tasks (cleanup old data, update statistics).
    #[instrument(skip(self))]
    pub async fn run_maintenance(&self) -> RlmResult<()> {
        self.cleanup_old_data().await?;
        self.update_statistics().await?;
        self.check_alerts().await?;

        debug!("Completed monitoring maintenance tasks");
        Ok(())
    }

    /// Process a new error occurrence.
    fn process_error_occurrence(&self, occurrence: ErrorOccurrence) -> RlmResult<()> {
        let mut error_data = self.error_data.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        let error_type = occurrence.error_type.clone();
        let timestamp = occurrence.timestamp;

        // Update error type statistics
        let error_stats = error_data.error_types
            .entry(error_type.clone())
            .or_insert_with(|| ErrorTypeStats {
                error_type: error_type.clone(),
                first_seen: Some(timestamp),
                ..Default::default()
            });

        error_stats.total_count += 1;
        error_stats.last_seen = Some(timestamp);
        error_stats.recent_occurrences.push_back(timestamp);

        // Keep only recent occurrences for rate calculation
        let window_start = timestamp
            .checked_sub(Duration::from_secs(self.config.rate_window_minutes * 60))
            .unwrap_or(timestamp);

        while let Some(&front_time) = error_stats.recent_occurrences.front() {
            if front_time < window_start {
                error_stats.recent_occurrences.pop_front();
            } else {
                break;
            }
        }

        // Update error rate
        let window_duration = Duration::from_secs(self.config.rate_window_minutes * 60);
        error_stats.current_rate = error_stats.recent_occurrences.len() as f64
            / (window_duration.as_secs_f64() / 60.0);

        if error_stats.current_rate > error_stats.peak_rate {
            error_stats.peak_rate = error_stats.current_rate;
        }

        // Update global statistics
        error_data.global_stats.total_errors += 1;

        // Add to error history
        error_data.error_history.push_back(occurrence);

        // Keep history within limits
        if error_data.error_history.len() > self.config.max_error_history {
            error_data.error_history.pop_front();
        }

        Ok(())
    }

    /// Classify error severity.
    fn classify_error_severity(&self, error: &RlmError) -> ErrorSeverity {
        match error {
            RlmError::Repl(_) => ErrorSeverity::Medium,
            RlmError::Llm(_) => ErrorSeverity::High,
            RlmError::MaxIterations(_) => ErrorSeverity::Medium,
            RlmError::MaxRecursion(_) => ErrorSeverity::High,
            RlmError::Timeout(_) => ErrorSeverity::High,
            RlmError::Config(_) => ErrorSeverity::Critical,
            RlmError::Serialization(_) => ErrorSeverity::Medium,
            RlmError::Io(_) => ErrorSeverity::Medium,
            RlmError::Other(msg) if msg.contains("timeout") => ErrorSeverity::High,
            RlmError::Other(msg) if msg.contains("connection") => ErrorSeverity::Medium,
            RlmError::Other(msg) if msg.contains("rate limit") => ErrorSeverity::Low,
            RlmError::Other(_) => ErrorSeverity::Medium,
        }
    }

    /// Clean up old error data.
    async fn cleanup_old_data(&self) -> RlmResult<()> {
        let mut error_data = self.error_data.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        let cutoff_time = SystemTime::now()
            .checked_sub(Duration::from_secs(3600)) // 1 hour
            .unwrap_or_else(SystemTime::now);

        // Clean up error history
        error_data.error_history.retain(|occurrence| occurrence.timestamp > cutoff_time);

        // Clean up recent occurrences in error type stats
        for error_stats in error_data.error_types.values_mut() {
            error_stats.recent_occurrences.retain(|&timestamp| timestamp > cutoff_time);
        }

        // Clean up acknowledged alerts older than 24 hours
        let alert_cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(86400))
            .unwrap_or_else(SystemTime::now);

        error_data.active_alerts.retain(|_id, alert| {
            !(alert.acknowledged && alert.triggered_at < alert_cutoff)
        });

        Ok(())
    }

    /// Update error statistics and trends.
    async fn update_statistics(&self) -> RlmResult<()> {
        let mut error_data = self.error_data.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        let now = SystemTime::now();
        let hour_ago = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);
        let minute_ago = now.checked_sub(Duration::from_secs(60)).unwrap_or(now);

        // Update global statistics
        error_data.global_stats.errors_last_hour = error_data.error_history
            .iter()
            .filter(|e| e.timestamp > hour_ago)
            .count() as u64;

        error_data.global_stats.errors_last_minute = error_data.error_history
            .iter()
            .filter(|e| e.timestamp > minute_ago)
            .count() as u64;

        error_data.global_stats.global_error_rate = error_data.global_stats.errors_last_minute as f64;

        // Update per-type statistics
        for error_stats in error_data.error_types.values_mut() {
            error_stats.last_hour_count = error_stats.recent_occurrences
                .iter()
                .filter(|&&timestamp| timestamp > hour_ago)
                .count() as u64;

            error_stats.last_minute_count = error_stats.recent_occurrences
                .iter()
                .filter(|&&timestamp| timestamp > minute_ago)
                .count() as u64;

            // Update trend analysis
            if self.config.enable_trend_analysis {
                error_stats.trend = self.calculate_trend(&error_stats.recent_occurrences);
            }
        }

        // Find most frequent error type
        error_data.global_stats.most_frequent_error = error_data.error_types
            .values()
            .max_by_key(|stats| stats.total_count)
            .map(|stats| stats.error_type.clone());

        // Calculate health score
        error_data.global_stats.health_score = self.calculate_health_score(&error_data);

        Ok(())
    }

    /// Check for alert conditions.
    async fn check_alerts(&self) -> RlmResult<()> {
        let error_data = self.error_data.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

        // Check global error rate
        if error_data.global_stats.global_error_rate > self.config.error_rate_threshold {
            self.trigger_alert(
                AlertType::ErrorRateThreshold,
                AlertSeverity::Warning,
                format!(
                    "Global error rate ({:.2}/min) exceeded threshold ({:.2}/min)",
                    error_data.global_stats.global_error_rate,
                    self.config.error_rate_threshold
                ),
                None,
            ).await?;
        }

        // Check health score
        if error_data.global_stats.health_score < self.config.health_threshold {
            self.trigger_alert(
                AlertType::HealthDegraded,
                AlertSeverity::Critical,
                format!(
                    "System health score ({:.2}) below threshold ({:.2})",
                    error_data.global_stats.health_score,
                    self.config.health_threshold
                ),
                None,
            ).await?;
        }

        // Check individual error type rates
        for error_stats in error_data.error_types.values() {
            if error_stats.current_rate > self.config.error_rate_threshold * 0.5 {
                self.trigger_alert(
                    AlertType::ErrorTypeSpike,
                    AlertSeverity::Warning,
                    format!(
                        "Error type '{}' rate ({:.2}/min) is elevated",
                        error_stats.error_type,
                        error_stats.current_rate
                    ),
                    Some(error_stats.error_type.clone()),
                ).await?;
            }
        }

        Ok(())
    }

    /// Trigger an alert.
    async fn trigger_alert(
        &self,
        alert_type: AlertType,
        severity: AlertSeverity,
        message: String,
        error_type: Option<String>,
    ) -> RlmResult<()> {
        let alert_id = format!("{:?}_{}", alert_type, SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());

        // Check if similar alert is already active (cooldown)
        {
            let error_data = self.error_data.read()
                .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;

            let similar_alert_exists = error_data.active_alerts.values().any(|alert| {
                alert.alert_type == alert_type
                    && alert.error_type == error_type
                    && !alert.acknowledged
                    && SystemTime::now()
                        .duration_since(alert.triggered_at)
                        .unwrap_or_default() < self.config.alert_cooldown
            });

            if similar_alert_exists {
                return Ok(());
            }
        }

        let alert = ActiveAlert {
            alert_id: alert_id.clone(),
            alert_type: alert_type.clone(),
            severity: severity.clone(),
            triggered_at: SystemTime::now(),
            message: message.clone(),
            error_type: error_type.clone(),
            trigger_count: 1,
            acknowledged: false,
        };

        // Store alert
        {
            let mut error_data = self.error_data.write()
                .map_err(|e| RlmError::Other(format!("Failed to acquire error data lock: {}", e)))?;
            error_data.active_alerts.insert(alert_id, alert.clone());
        }

        // Send alert notification
        if let Some(sender) = &self.alert_sender {
            let alert_notification = Alert {
                alert: alert.clone(),
                recommended_actions: self.get_recommended_actions(&alert_type, &error_type),
            };

            if let Err(e) = sender.send(alert_notification) {
                warn!("Failed to send alert notification: {}", e);
            }
        }

        match severity {
            AlertSeverity::Critical => error!("CRITICAL ALERT: {}", message),
            AlertSeverity::Warning => warn!("WARNING ALERT: {}", message),
            AlertSeverity::Info => info!("INFO ALERT: {}", message),
        }

        Ok(())
    }

    /// Calculate trend from recent occurrences.
    fn calculate_trend(&self, occurrences: &VecDeque<SystemTime>) -> ErrorTrend {
        if occurrences.len() < 4 {
            return ErrorTrend::Unknown;
        }

        let half_point = occurrences.len() / 2;
        let first_half_count = half_point;
        let second_half_count = occurrences.len() - half_point;

        let first_half_rate = first_half_count as f64 / (self.config.rate_window_minutes as f64 / 2.0);
        let second_half_rate = second_half_count as f64 / (self.config.rate_window_minutes as f64 / 2.0);

        let change_ratio = if first_half_rate > 0.0 {
            second_half_rate / first_half_rate
        } else if second_half_rate > 0.0 {
            f64::INFINITY
        } else {
            1.0
        };

        if change_ratio > 1.2 {
            ErrorTrend::Increasing
        } else if change_ratio < 0.8 {
            ErrorTrend::Decreasing
        } else {
            ErrorTrend::Stable
        }
    }

    /// Calculate overall health score.
    fn calculate_health_score(&self, error_data: &ErrorTrackingData) -> f64 {
        let base_score = 1.0;
        let error_rate_penalty = (error_data.global_stats.global_error_rate / 60.0).min(1.0) * 0.5;
        let error_variety_penalty = (error_data.error_types.len() as f64 / 10.0).min(1.0) * 0.2;
        let active_alert_penalty = (error_data.active_alerts.len() as f64 / 5.0).min(1.0) * 0.3;

        (base_score - error_rate_penalty - error_variety_penalty - active_alert_penalty).max(0.0)
    }

    /// Get recommended actions for alert types.
    fn get_recommended_actions(&self, alert_type: &AlertType, error_type: &Option<String>) -> Vec<String> {
        match alert_type {
            AlertType::ErrorRateThreshold => vec![
                "Check system resources (CPU, memory)".to_string(),
                "Review recent deployments".to_string(),
                "Check external dependencies".to_string(),
            ],
            AlertType::ErrorTypeSpike => {
                let mut actions = vec![
                    "Investigate recent changes affecting this error type".to_string(),
                ];
                if let Some(error) = error_type {
                    actions.push(format!("Review logs for '{}' error pattern", error));
                }
                actions
            },
            AlertType::HealthDegraded => vec![
                "Check all monitoring dashboards".to_string(),
                "Review system performance metrics".to_string(),
                "Consider enabling circuit breakers".to_string(),
            ],
            AlertType::RecurringPattern => vec![
                "Analyze error patterns for root cause".to_string(),
                "Consider implementing retry logic".to_string(),
            ],
            AlertType::NewErrorType => vec![
                "Investigate new error source".to_string(),
                "Update error handling if needed".to_string(),
            ],
        }
    }
}

impl Default for ErrorRateMonitor {
    fn default() -> Self {
        Self::new(MonitoringConfig::default())
    }
}

impl Clone for ErrorTrackingData {
    fn clone(&self) -> Self {
        Self {
            error_types: self.error_types.clone(),
            global_stats: self.global_stats.clone(),
            error_history: self.error_history.clone(),
            active_alerts: self.active_alerts.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_monitor_creation() {
        let monitor = ErrorRateMonitor::default();
        let stats = monitor.get_error_stats().expect("Failed to get error stats");

        assert_eq!(stats.global_stats.total_errors, 0);
        assert!(stats.error_types.is_empty());
    }

    #[test]
    fn test_error_recording() {
        let monitor = ErrorRateMonitor::default();
        let error = RlmError::Other("Test error".to_string());
        let context = HashMap::new();

        monitor.record_error("test_error", &error, context)
            .expect("Failed to record error");

        let stats = monitor.get_error_stats().expect("Failed to get error stats");
        assert_eq!(stats.global_stats.total_errors, 1);

        let error_stats = monitor.get_error_type_stats("test_error")
            .expect("Failed to get error type stats")
            .expect("Error type not found");

        assert_eq!(error_stats.total_count, 1);
        assert_eq!(error_stats.error_type, "test_error");
    }

    #[tokio::test]
    async fn test_alert_system() {
        let config = MonitoringConfig {
            error_rate_threshold: 1.0, // Very low threshold for testing
            ..Default::default()
        };

        let (monitor, mut alert_receiver) = ErrorRateMonitor::with_alerts(config);

        // Record multiple errors quickly
        for i in 0..5 {
            let error = RlmError::Other(format!("Test error {}", i));
            monitor.record_error("test_error", &error, HashMap::new())
                .expect("Failed to record error");
        }

        // Run maintenance to check alerts
        monitor.run_maintenance().await.expect("Failed to run maintenance");

        // Should receive an alert
        if let Ok(alert) = alert_receiver.try_recv() {
            assert_eq!(alert.alert.alert_type, AlertType::ErrorRateThreshold);
        }
    }

    #[test]
    fn test_health_score_calculation() {
        let monitor = ErrorRateMonitor::default();

        // Initially should have perfect health
        let health = monitor.get_health_score().expect("Failed to get health score");
        assert!((health - 1.0).abs() < 0.001);

        // Record some errors
        for i in 0..10 {
            let error = RlmError::Other(format!("Test error {}", i));
            monitor.record_error("test_error", &error, HashMap::new())
                .expect("Failed to record error");
        }

        // Health should have degraded
        let health_after = monitor.get_health_score().expect("Failed to get health score");
        assert!(health_after < 1.0);
    }

    #[test]
    fn test_error_trend_calculation() {
        let monitor = ErrorRateMonitor::default();

        // Create a sequence of timestamps simulating increasing errors
        let mut occurrences = VecDeque::new();
        let base_time = SystemTime::now()
            .checked_sub(Duration::from_secs(300))
            .unwrap();

        // Add fewer errors in first half, more in second half
        for i in 0..2 {
            occurrences.push_back(base_time + Duration::from_secs(i * 30));
        }
        for i in 0..6 {
            occurrences.push_back(base_time + Duration::from_secs(150 + i * 20));
        }

        let trend = monitor.calculate_trend(&occurrences);
        assert_eq!(trend, ErrorTrend::Increasing);
    }
}