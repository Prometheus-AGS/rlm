//! SSE connection management and cleanup.
//!
//! This module provides comprehensive connection management for Server-Sent Events,
//! including connection pooling, timeout handling, client disconnect detection,
//! and resource cleanup to ensure robust streaming functionality.

use anyhow::Result;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{mpsc, RwLock},
    task::JoinHandle,
    time::{sleep, timeout},
};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

/// Connection information for an active SSE stream.
#[derive(Debug)]
pub struct ConnectionInfo {
    /// Unique connection identifier.
    pub connection_id: String,
    /// Associated request ID.
    pub request_id: String,
    /// Connection establishment time.
    pub connected_at: SystemTime,
    /// Last activity timestamp.
    pub last_activity: SystemTime,
    /// Client IP address if available.
    pub client_ip: Option<String>,
    /// User agent if available.
    pub user_agent: Option<String>,
    /// Total events sent on this connection.
    pub events_sent: AtomicU64,
    /// Total bytes sent on this connection.
    pub bytes_sent: AtomicU64,
}

impl ConnectionInfo {
    /// Create a new connection info.
    pub fn new(request_id: String) -> Self {
        let now = SystemTime::now();
        Self {
            connection_id: Uuid::new_v4().to_string(),
            request_id,
            connected_at: now,
            last_activity: now,
            client_ip: None,
            user_agent: None,
            events_sent: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
        }
    }

    /// Update last activity timestamp.
    pub fn update_activity(&mut self) {
        self.last_activity = SystemTime::now();
    }

    /// Record an event being sent.
    pub fn record_event(&self, bytes: u64) {
        self.events_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Check if the connection is stale (no activity for given duration).
    pub fn is_stale(&self, stale_threshold: Duration) -> bool {
        if let Ok(elapsed) = self.last_activity.elapsed() {
            elapsed > stale_threshold
        } else {
            true // If we can't determine elapsed time, consider it stale
        }
    }

    /// Get connection age.
    pub fn age(&self) -> Duration {
        self.connected_at.elapsed().unwrap_or_default()
    }
}

/// Handle for managing an active SSE connection.
#[derive(Debug)]
pub struct ConnectionHandle {
    /// Connection information.
    pub info: Arc<RwLock<ConnectionInfo>>,
    /// Sender for sending close signals.
    close_sender: Option<mpsc::UnboundedSender<()>>,
    /// Join handle for the connection monitor task.
    monitor_task: Option<JoinHandle<()>>,
}

impl ConnectionHandle {
    /// Create a new connection handle.
    fn new(info: ConnectionInfo, close_sender: mpsc::UnboundedSender<()>) -> Self {
        Self {
            info: Arc::new(RwLock::new(info)),
            close_sender: Some(close_sender),
            monitor_task: None,
        }
    }

    /// Update connection activity.
    pub async fn update_activity(&self) {
        let mut info = self.info.write().await;
        info.update_activity();
    }

    /// Record an event being sent on this connection.
    pub async fn record_event(&self, bytes: u64) {
        let info = self.info.read().await;
        info.record_event(bytes);
    }

    /// Close the connection gracefully.
    pub async fn close(&mut self) {
        debug!("Closing SSE connection handle");

        // Send close signal if available
        if let Some(sender) = self.close_sender.take() {
            let _ = sender.send(());
        }

        // Cancel monitor task
        if let Some(task) = self.monitor_task.take() {
            task.abort();
        }
    }

    /// Get connection statistics.
    pub async fn get_stats(&self) -> Option<(u64, u64, Duration)> {
        let info = self.info.read().await;
        Some((
            info.events_sent.load(Ordering::Relaxed),
            info.bytes_sent.load(Ordering::Relaxed),
            info.age(),
        ))
    }
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        // Ensure cleanup happens even if close() wasn't called
        if let Some(sender) = self.close_sender.take() {
            let _ = sender.send(());
        }
        if let Some(task) = self.monitor_task.take() {
            task.abort();
        }
    }
}

/// Configuration for the SSE connection manager.
#[derive(Debug, Clone)]
pub struct ConnectionManagerConfig {
    /// Maximum number of concurrent connections.
    pub max_connections: usize,
    /// Timeout for individual connections.
    pub connection_timeout: Duration,
    /// Stale connection threshold.
    pub stale_threshold: Duration,
    /// Cleanup interval.
    pub cleanup_interval: Duration,
    /// Keep-alive interval.
    pub keep_alive_interval: Duration,
    /// Enable connection monitoring.
    pub enable_monitoring: bool,
}

impl Default for ConnectionManagerConfig {
    fn default() -> Self {
        Self {
            max_connections: 1000,
            connection_timeout: Duration::from_secs(300), // 5 minutes
            stale_threshold: Duration::from_secs(60),     // 1 minute
            cleanup_interval: Duration::from_secs(30),    // 30 seconds
            keep_alive_interval: Duration::from_secs(30), // 30 seconds
            enable_monitoring: true,
        }
    }
}

/// Manager for SSE connections with cleanup and monitoring capabilities.
#[derive(Debug)]
pub struct SseConnectionManager {
    /// Manager configuration.
    config: ConnectionManagerConfig,
    /// Active connections.
    connections: Arc<RwLock<HashMap<String, Arc<ConnectionHandle>>>>,
    /// Cleanup task handle.
    cleanup_task: Option<JoinHandle<()>>,
    /// Shutdown signal sender.
    shutdown_sender: Option<mpsc::UnboundedSender<()>>,
}

impl SseConnectionManager {
    /// Create a new SSE connection manager.
    pub fn new(config: ConnectionManagerConfig) -> Self {
        let connections = Arc::new(RwLock::new(HashMap::new()));
        let (shutdown_sender, shutdown_receiver) = mpsc::unbounded_channel();

        let mut manager = Self {
            config,
            connections,
            cleanup_task: None,
            shutdown_sender: Some(shutdown_sender),
        };

        // Start cleanup task if monitoring is enabled
        if manager.config.enable_monitoring {
            manager.start_cleanup_task(shutdown_receiver);
        }

        manager
    }

    /// Create a manager with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ConnectionManagerConfig::default())
    }

    /// Register a new SSE connection.
    #[instrument(skip(self))]
    pub async fn register_connection(&self, request_id: String) -> Result<Arc<ConnectionHandle>> {
        // Check connection limits
        {
            let connections = self.connections.read().await;
            if connections.len() >= self.config.max_connections {
                warn!(
                    "Connection limit reached ({}/{}), rejecting new connection",
                    connections.len(),
                    self.config.max_connections
                );
                return Err(anyhow::anyhow!("Connection limit reached"));
            }
        }

        // Create connection info
        let info = ConnectionInfo::new(request_id.clone());
        let connection_id = info.connection_id.clone();

        debug!("Registering new SSE connection: {}", connection_id);

        // Create close channel for this connection
        let (close_sender, mut close_receiver) = mpsc::unbounded_channel();

        // Create connection handle
        let handle = Arc::new(ConnectionHandle::new(info, close_sender));

        // Start connection monitor if enabled
        let _monitor_task = if self.config.enable_monitoring {
            let weak_handle = Arc::downgrade(&handle);
            let connection_timeout = self.config.connection_timeout;
            let manager_connections = Arc::downgrade(&self.connections);
            let conn_id = connection_id.clone();

            Some(tokio::spawn(async move {
                tokio::select! {
                    _ = close_receiver.recv() => {
                        debug!("Connection {} received close signal", conn_id);
                    }
                    _ = sleep(connection_timeout) => {
                        warn!("Connection {} timed out after {:?}", conn_id, connection_timeout);

                        // Remove from manager if still exists
                        if let Some(manager_connections) = manager_connections.upgrade() {
                            let mut connections = manager_connections.write().await;
                            connections.remove(&conn_id);
                        }
                    }
                }

                // Cleanup when monitor task finishes
                if let Some(_handle) = weak_handle.upgrade() {
                    debug!("Cleaning up connection {}", conn_id);
                }
            }))
        } else {
            None
        };

        // Store the monitor task in the handle (requires unsafe due to self-reference)
        // For now, we'll skip this to avoid complexity, but in a production implementation
        // we might want to store the task handle separately

        // Register the connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), handle.clone());
        }

        info!(
            "Successfully registered SSE connection: {} for request: {}",
            connection_id, request_id
        );

        Ok(handle)
    }

    /// Unregister a connection.
    #[instrument(skip(self))]
    pub async fn unregister_connection(&self, connection_id: &str) -> bool {
        debug!("Unregistering SSE connection: {}", connection_id);

        let mut connections = self.connections.write().await;
        if let Some(handle) = connections.remove(connection_id) {
            debug!("Successfully unregistered connection: {}", connection_id);
            drop(handle); // This will trigger cleanup in the Drop implementation
            true
        } else {
            warn!("Attempted to unregister unknown connection: {}", connection_id);
            false
        }
    }

    /// Get connection statistics.
    pub async fn get_connection_stats(&self) -> HashMap<String, (u64, u64, Duration)> {
        let connections = self.connections.read().await;
        let mut stats = HashMap::new();

        for (conn_id, handle) in connections.iter() {
            if let Some(stat) = handle.get_stats().await {
                stats.insert(conn_id.clone(), stat);
            }
        }

        stats
    }

    /// Get the number of active connections.
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Get list of connection IDs.
    pub async fn list_connections(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }

    /// Force cleanup of stale connections.
    #[instrument(skip(self))]
    pub async fn cleanup_stale_connections(&self) -> usize {
        let mut connections = self.connections.write().await;
        let mut stale_connections = Vec::new();

        // Identify stale connections
        for (conn_id, handle) in connections.iter() {
            let info = handle.info.read().await;
            if info.is_stale(self.config.stale_threshold) {
                stale_connections.push(conn_id.clone());
            }
        }

        // Remove stale connections
        for conn_id in &stale_connections {
            if let Some(handle) = connections.remove(conn_id) {
                debug!("Cleaned up stale connection: {}", conn_id);
                drop(handle);
            }
        }

        let cleaned_count = stale_connections.len();
        if cleaned_count > 0 {
            info!("Cleaned up {} stale SSE connections", cleaned_count);
        }

        cleaned_count
    }

    /// Start the background cleanup task.
    fn start_cleanup_task(&mut self, mut shutdown_receiver: mpsc::UnboundedReceiver<()>) {
        let connections = Arc::downgrade(&self.connections);
        let cleanup_interval = self.config.cleanup_interval;
        let stale_threshold = self.config.stale_threshold;

        self.cleanup_task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Perform cleanup
                        if let Some(connections_arc) = connections.upgrade() {
                            let mut connections = connections_arc.write().await;
                            let mut stale_connections = Vec::new();

                            // Identify stale connections
                            for (conn_id, handle) in connections.iter() {
                                let info = handle.info.read().await;
                                if info.is_stale(stale_threshold) {
                                    stale_connections.push(conn_id.clone());
                                }
                            }

                            // Remove stale connections
                            for conn_id in &stale_connections {
                                if let Some(handle) = connections.remove(conn_id) {
                                    debug!("Cleaned up stale connection: {}", conn_id);
                                    drop(handle);
                                }
                            }

                            if !stale_connections.is_empty() {
                                debug!("Periodic cleanup removed {} stale connections", stale_connections.len());
                            }
                        } else {
                            debug!("Connection manager has been dropped, stopping cleanup task");
                            break;
                        }
                    }
                    _ = shutdown_receiver.recv() => {
                        debug!("Cleanup task received shutdown signal");
                        break;
                    }
                }
            }

            debug!("SSE connection manager cleanup task finished");
        }));
    }

    /// Gracefully shutdown the connection manager.
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down SSE connection manager");

        // Send shutdown signal
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(());
        }

        // Wait for cleanup task to finish
        if let Some(task) = self.cleanup_task.take() {
            let _ = timeout(Duration::from_secs(5), task).await;
        }

        // Close all active connections
        {
            let mut connections = self.connections.write().await;
            for (conn_id, handle) in connections.drain() {
                debug!("Closing connection during shutdown: {}", conn_id);
                drop(handle);
            }
        }

        info!("SSE connection manager shutdown complete");
        Ok(())
    }
}

impl Drop for SseConnectionManager {
    fn drop(&mut self) {
        // Send shutdown signal if not already sent
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.send(());
        }

        // Cancel cleanup task
        if let Some(task) = self.cleanup_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_connection_registration() {
        let manager = SseConnectionManager::with_defaults();

        let handle = manager
            .register_connection("test-request".to_string())
            .await
            .expect("Should register connection");

        assert_eq!(manager.connection_count().await, 1);

        let connection_id = {
            let info = handle.info.read().await;
            info.connection_id.clone()
        };

        let removed = manager.unregister_connection(&connection_id).await;
        assert!(removed);
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_connection_limits() {
        let config = ConnectionManagerConfig {
            max_connections: 2,
            ..Default::default()
        };
        let manager = SseConnectionManager::new(config);

        // Register maximum connections
        let _handle1 = manager
            .register_connection("request-1".to_string())
            .await
            .expect("Should register first connection");

        let _handle2 = manager
            .register_connection("request-2".to_string())
            .await
            .expect("Should register second connection");

        // Third connection should fail
        let result = manager.register_connection("request-3".to_string()).await;
        assert!(result.is_err());

        assert_eq!(manager.connection_count().await, 2);
    }

    #[tokio::test]
    async fn test_connection_activity_tracking() {
        let manager = SseConnectionManager::with_defaults();

        let handle = manager
            .register_connection("test-request".to_string())
            .await
            .expect("Should register connection");

        // Record some activity
        handle.record_event(100).await;
        handle.record_event(200).await;

        let stats = handle.get_stats().await;
        assert!(stats.is_some());

        let (events, bytes, _age) = stats.unwrap();
        assert_eq!(events, 2);
        assert_eq!(bytes, 300);
    }

    #[tokio::test]
    async fn test_stale_connection_detection() {
        let mut info = ConnectionInfo::new("test".to_string());

        // Connection should not be stale immediately
        assert!(!info.is_stale(Duration::from_secs(60)));

        // Simulate old last_activity
        info.last_activity = SystemTime::now() - Duration::from_secs(120);

        // Should be stale now
        assert!(info.is_stale(Duration::from_secs(60)));
    }

    #[tokio::test]
    async fn test_manager_cleanup() {
        let config = ConnectionManagerConfig {
            cleanup_interval: Duration::from_millis(100),
            stale_threshold: Duration::from_millis(50),
            enable_monitoring: false, // Disable automatic monitoring for this test
            ..Default::default()
        };

        let manager = SseConnectionManager::new(config);

        let _handle = manager
            .register_connection("test-request".to_string())
            .await
            .expect("Should register connection");

        // Wait for connection to become stale
        sleep(Duration::from_millis(60)).await;

        // Manually trigger cleanup
        let cleaned = manager.cleanup_stale_connections().await;
        assert_eq!(cleaned, 1);
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_connection_handle_drop() {
        let manager = SseConnectionManager::with_defaults();

        let handle = manager
            .register_connection("test-request".to_string())
            .await
            .expect("Should register connection");

        let connection_id = {
            let info = handle.info.read().await;
            info.connection_id.clone()
        };

        // Drop the handle
        drop(handle);

        // Connection should still be in manager until explicitly unregistered
        assert_eq!(manager.connection_count().await, 1);

        // Unregister explicitly
        let removed = manager.unregister_connection(&connection_id).await;
        assert!(removed);
        assert_eq!(manager.connection_count().await, 0);
    }
}