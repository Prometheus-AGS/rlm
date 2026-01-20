//! RLM Session management for tracking request processing state.

use crate::{RlmError, RlmResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Session status during RLM processing lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session created, awaiting context analysis.
    Created,
    /// Context has been analyzed and loaded into REPL.
    ContextLoaded,
    /// Currently processing recursive calls.
    Processing,
    /// Aggregating results from recursive calls.
    Aggregating,
    /// Session completed successfully.
    Completed,
    /// Session failed with error.
    Failed,
    /// Session suspended due to resource limits.
    Suspended,
    /// Session expired due to timeout.
    Expired,
}

/// Core session entity for tracking RLM request processing state.
///
/// This entity represents the processing session for a single RLM request,
/// managing the lifecycle from initial request through recursive decomposition
/// to final response aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmSession {
    /// Unique session identifier.
    pub session_id: String,

    /// Original request ID that created this session.
    pub request_id: String,

    /// Current session status.
    pub status: SessionStatus,

    /// Timestamp when session was created.
    #[serde(with = "crate::types::systemtime_serde")]
    pub created_at: SystemTime,

    /// Timestamp of last activity (updated on each operation).
    #[serde(with = "crate::types::systemtime_serde")]
    pub last_activity: SystemTime,

    /// Optional completion timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<SystemTime>,

    /// Total processing duration when completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<Duration>,

    /// Context size in bytes (for tracking memory usage).
    pub context_size_bytes: usize,

    /// Context size in tokens (estimated).
    pub context_size_tokens: u32,

    /// Maximum recursion depth allowed for this session.
    pub max_recursion_depth: u32,

    /// Current recursion depth (tracked during processing).
    pub current_recursion_depth: u32,

    /// Total number of recursive calls made.
    pub total_recursive_calls: u32,

    /// Total tokens consumed by all recursive calls.
    pub total_tokens_used: u32,

    /// Session metadata and configuration.
    pub metadata: HashMap<String, String>,

    /// Optional error information if session failed.
    pub error: Option<String>,

    /// REPL session ID (None until REPL is initialized).
    pub repl_session_id: Option<String>,
}

impl RlmSession {
    /// Create a new RLM session for the given request.
    pub fn new(request_id: String, context_size_bytes: usize, context_size_tokens: u32) -> Self {
        let now = SystemTime::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            request_id,
            status: SessionStatus::Created,
            created_at: now,
            last_activity: now,
            completed_at: None,
            total_duration: None,
            context_size_bytes,
            context_size_tokens,
            max_recursion_depth: 10, // Default from data model
            current_recursion_depth: 0,
            total_recursive_calls: 0,
            total_tokens_used: 0,
            metadata: HashMap::new(),
            error: None,
            repl_session_id: None,
        }
    }

    /// Update session status and activity timestamp.
    pub fn update_status(&mut self, status: SessionStatus) {
        // Set completion timestamp when session is finished
        if matches!(&status, SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Expired) {
            self.last_activity = SystemTime::now();
            self.completed_at = Some(self.last_activity);
            if let Ok(duration) = self.last_activity.duration_since(self.created_at) {
                self.total_duration = Some(duration);
            }
        } else {
            self.last_activity = SystemTime::now();
        }

        self.status = status;
    }

    /// Record a recursive call completion.
    pub fn record_recursive_call(&mut self, depth: u32, tokens_used: u32) {
        self.total_recursive_calls += 1;
        self.total_tokens_used += tokens_used;
        self.current_recursion_depth = depth.max(self.current_recursion_depth);
        self.last_activity = SystemTime::now();
    }

    /// Check if session has exceeded maximum recursion depth.
    pub fn is_max_depth_exceeded(&self, depth: u32) -> bool {
        depth > self.max_recursion_depth
    }

    /// Check if session has timed out based on configuration.
    pub fn is_expired(&self, timeout: Duration) -> bool {
        if let Ok(elapsed) = self.last_activity.elapsed() {
            elapsed > timeout
        } else {
            false
        }
    }

    /// Mark session as failed with error message.
    pub fn mark_failed(&mut self, error: String) {
        self.error = Some(error);
        self.update_status(SessionStatus::Failed);
    }

    /// Mark session as completed successfully.
    pub fn mark_completed(&mut self) {
        self.update_status(SessionStatus::Completed);
    }

    /// Set the REPL session ID when REPL is initialized.
    pub fn set_repl_session(&mut self, repl_session_id: String) {
        self.repl_session_id = Some(repl_session_id);
        self.update_status(SessionStatus::ContextLoaded);
    }

    /// Add metadata key-value pair to session.
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value by key.
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Check if session is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Expired
        )
    }

    /// Check if session is currently active and processing.
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Created | SessionStatus::ContextLoaded | SessionStatus::Processing | SessionStatus::Aggregating
        )
    }

    /// Validate session state transitions.
    pub fn can_transition_to(&self, new_status: &SessionStatus) -> RlmResult<()> {
        match (&self.status, new_status) {
            // Valid forward transitions
            (SessionStatus::Created, SessionStatus::ContextLoaded) => Ok(()),
            (SessionStatus::ContextLoaded, SessionStatus::Processing) => Ok(()),
            (SessionStatus::Processing, SessionStatus::Aggregating) => Ok(()),
            (SessionStatus::Aggregating, SessionStatus::Completed) => Ok(()),

            // Failure transitions from any active state
            (status, SessionStatus::Failed) if !matches!(status, SessionStatus::Completed) => Ok(()),

            // Expiration from any active state
            (status, SessionStatus::Expired) if !matches!(status, SessionStatus::Completed | SessionStatus::Failed) => Ok(()),

            // Suspension from processing states
            (SessionStatus::Processing, SessionStatus::Suspended) => Ok(()),
            (SessionStatus::Suspended, SessionStatus::Processing) => Ok(()), // Resume

            // Invalid transitions
            _ => Err(RlmError::Other(format!(
                "Invalid session state transition from {:?} to {:?}",
                self.status, new_status
            ))),
        }
    }
}

/// Session manager for tracking multiple active RLM sessions.
#[derive(Debug, Default)]
pub struct SessionManager {
    /// Active sessions by session ID.
    sessions: HashMap<String, RlmSession>,

    /// Session timeout duration.
    session_timeout: Duration,
}

impl SessionManager {
    /// Create a new session manager with specified timeout.
    pub fn new(session_timeout: Duration) -> Self {
        Self {
            sessions: HashMap::new(),
            session_timeout,
        }
    }

    /// Create and register a new session.
    pub fn create_session(&mut self, request_id: String, context_size_bytes: usize, context_size_tokens: u32) -> String {
        let session = RlmSession::new(request_id, context_size_bytes, context_size_tokens);
        let session_id = session.session_id.clone();
        self.sessions.insert(session_id.clone(), session);
        session_id
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<&RlmSession> {
        self.sessions.get(session_id)
    }

    /// Get a mutable session by ID.
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut RlmSession> {
        self.sessions.get_mut(session_id)
    }

    /// Update session status with validation.
    pub fn update_session_status(&mut self, session_id: &str, status: SessionStatus) -> RlmResult<()> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| RlmError::Other(format!("Session not found: {}", session_id)))?;

        session.can_transition_to(&status)?;
        session.update_status(status);
        Ok(())
    }

    /// Remove completed or expired sessions.
    pub fn cleanup_sessions(&mut self) -> usize {
        let initial_count = self.sessions.len();

        // Remove expired sessions
        let _expired_ids: Vec<String> = self.sessions
            .iter_mut()
            .filter_map(|(id, session)| {
                if session.is_expired(self.session_timeout) {
                    session.update_status(SessionStatus::Expired);
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        // Remove terminal sessions
        self.sessions.retain(|_, session| !session.is_terminal());

        initial_count - self.sessions.len()
    }

    /// Get count of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get sessions by status.
    pub fn sessions_by_status(&self, status: SessionStatus) -> Vec<&RlmSession> {
        self.sessions
            .values()
            .filter(|session| session.status == status)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_session_creation() {
        let session = RlmSession::new("req-123".to_string(), 1000, 250);

        assert_eq!(session.request_id, "req-123");
        assert_eq!(session.status, SessionStatus::Created);
        assert_eq!(session.context_size_bytes, 1000);
        assert_eq!(session.context_size_tokens, 250);
        assert_eq!(session.max_recursion_depth, 10);
        assert!(!session.is_terminal());
        assert!(session.is_active());
    }

    #[test]
    fn test_session_status_transitions() {
        let mut session = RlmSession::new("req-123".to_string(), 1000, 250);

        // Valid transitions
        assert!(session.can_transition_to(&SessionStatus::ContextLoaded).is_ok());
        session.update_status(SessionStatus::ContextLoaded);

        assert!(session.can_transition_to(&SessionStatus::Processing).is_ok());
        session.update_status(SessionStatus::Processing);

        assert!(session.can_transition_to(&SessionStatus::Aggregating).is_ok());
        session.update_status(SessionStatus::Aggregating);

        assert!(session.can_transition_to(&SessionStatus::Completed).is_ok());
        session.update_status(SessionStatus::Completed);

        assert!(session.is_terminal());
        assert!(!session.is_active());
        assert!(session.completed_at.is_some());
    }

    #[test]
    fn test_invalid_status_transitions() {
        let session = RlmSession::new("req-123".to_string(), 1000, 250);

        // Cannot go directly from Created to Processing
        assert!(session.can_transition_to(&SessionStatus::Processing).is_err());

        // Cannot transition from terminal state
        let mut completed_session = RlmSession::new("req-456".to_string(), 1000, 250);
        completed_session.update_status(SessionStatus::Completed);
        assert!(completed_session.can_transition_to(&SessionStatus::Processing).is_err());
    }

    #[test]
    fn test_recursive_call_tracking() {
        let mut session = RlmSession::new("req-123".to_string(), 1000, 250);

        session.record_recursive_call(1, 100);
        assert_eq!(session.total_recursive_calls, 1);
        assert_eq!(session.total_tokens_used, 100);
        assert_eq!(session.current_recursion_depth, 1);

        session.record_recursive_call(2, 150);
        assert_eq!(session.total_recursive_calls, 2);
        assert_eq!(session.total_tokens_used, 250);
        assert_eq!(session.current_recursion_depth, 2);

        // Depth tracking should use maximum depth seen
        session.record_recursive_call(1, 50);
        assert_eq!(session.current_recursion_depth, 2);
    }

    #[test]
    fn test_max_depth_check() {
        let session = RlmSession::new("req-123".to_string(), 1000, 250);

        assert!(!session.is_max_depth_exceeded(5));
        assert!(!session.is_max_depth_exceeded(10));
        assert!(session.is_max_depth_exceeded(11));
    }

    #[test]
    fn test_session_expiration() {
        let timeout = Duration::from_millis(100);
        let session = RlmSession::new("req-123".to_string(), 1000, 250);

        // Should not be expired immediately
        assert!(!session.is_expired(timeout));

        // Wait for timeout
        thread::sleep(Duration::from_millis(150));
        assert!(session.is_expired(timeout));
    }

    #[test]
    fn test_session_metadata() {
        let mut session = RlmSession::new("req-123".to_string(), 1000, 250);

        session.add_metadata("model".to_string(), "gpt-4".to_string());
        session.add_metadata("temperature".to_string(), "0.7".to_string());

        assert_eq!(session.get_metadata("model"), Some(&"gpt-4".to_string()));
        assert_eq!(session.get_metadata("temperature"), Some(&"0.7".to_string()));
        assert_eq!(session.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_session_manager() {
        let timeout = Duration::from_secs(30);
        let mut manager = SessionManager::new(timeout);

        let session_id = manager.create_session("req-123".to_string(), 1000, 250);
        assert_eq!(manager.active_session_count(), 1);

        // Update session status
        assert!(manager.update_session_status(&session_id, SessionStatus::ContextLoaded).is_ok());

        let session = manager.get_session(&session_id).unwrap();
        assert_eq!(session.status, SessionStatus::ContextLoaded);

        // Complete session
        assert!(manager.update_session_status(&session_id, SessionStatus::Processing).is_ok());
        assert!(manager.update_session_status(&session_id, SessionStatus::Aggregating).is_ok());
        assert!(manager.update_session_status(&session_id, SessionStatus::Completed).is_ok());

        // Cleanup should remove completed sessions
        let removed = manager.cleanup_sessions();
        assert_eq!(removed, 1);
        assert_eq!(manager.active_session_count(), 0);
    }

    #[test]
    fn test_session_manager_invalid_transitions() {
        let timeout = Duration::from_secs(30);
        let mut manager = SessionManager::new(timeout);

        let session_id = manager.create_session("req-123".to_string(), 1000, 250);

        // Try invalid transition
        assert!(manager.update_session_status(&session_id, SessionStatus::Processing).is_err());
    }
}