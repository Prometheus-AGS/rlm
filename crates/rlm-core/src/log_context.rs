//! Structured logging context for RLM execution chains.
//!
//! This module provides a comprehensive logging context system that maintains
//! structured information throughout recursive call chains, enabling detailed
//! observability and debugging capabilities.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

/// Structured logging context for RLM execution.
#[derive(Debug, Clone)]
pub struct LogContext {
    /// Context storage.
    storage: Arc<RwLock<LogContextStorage>>,
}

/// Internal storage for log context data.
#[derive(Debug, Default)]
struct LogContextStorage {
    /// Active execution contexts by session ID.
    sessions: HashMap<String, SessionContext>,
    /// Active recursive call contexts.
    recursive_calls: HashMap<String, RecursiveCallContext>,
    /// Global context data.
    global_context: HashMap<String, String>,
}

/// Context data for an RLM execution session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Unique session identifier.
    pub session_id: String,
    /// Original request ID for correlation.
    pub request_id: String,
    /// Session start time.
    pub started_at: SystemTime,
    /// User query being processed.
    pub query: String,
    /// Context size in characters.
    pub context_size: usize,
    /// Maximum iterations allowed.
    pub max_iterations: u32,
    /// Maximum recursion depth allowed.
    pub max_recursion_depth: u32,
    /// Current iteration number.
    pub current_iteration: u32,
    /// Current recursion depth.
    pub current_depth: u32,
    /// Session status.
    pub status: SessionStatus,
    /// Custom metadata.
    pub metadata: HashMap<String, String>,
    /// Parent session ID (for recursive calls).
    pub parent_session: Option<String>,
    /// Child session IDs.
    pub child_sessions: Vec<String>,
}

/// Context data for a recursive call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveCallContext {
    /// Unique call identifier.
    pub call_id: String,
    /// Parent session ID.
    pub session_id: String,
    /// Parent call ID (for nested recursion).
    pub parent_call_id: Option<String>,
    /// Call start time.
    pub started_at: SystemTime,
    /// Recursion depth level.
    pub depth: u32,
    /// Call query/prompt.
    pub query: String,
    /// Call status.
    pub status: CallStatus,
    /// Input token count.
    pub input_tokens: u32,
    /// Output token count.
    pub output_tokens: u32,
    /// Call duration (if completed).
    pub duration: Option<Duration>,
    /// Error message (if failed).
    pub error_message: Option<String>,
    /// Custom call metadata.
    pub metadata: HashMap<String, String>,
    /// Child call IDs.
    pub child_calls: Vec<String>,
}

/// Status of an execution session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is starting up.
    Initializing,
    /// Session is actively processing.
    Running,
    /// Session is waiting for recursive calls.
    WaitingForRecursion,
    /// Session completed successfully.
    Completed,
    /// Session failed with error.
    Failed,
    /// Session was cancelled.
    Cancelled,
}

/// Status of a recursive call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallStatus {
    /// Call is pending execution.
    Pending,
    /// Call is in progress.
    InProgress,
    /// Call completed successfully.
    Completed,
    /// Call failed with error.
    Failed,
    /// Call was cancelled.
    Cancelled,
}

/// Log context data for structured logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSpan {
    /// Session context.
    pub session: Option<SessionContext>,
    /// Current recursive call context.
    pub current_call: Option<RecursiveCallContext>,
    /// Call chain (from root to current).
    pub call_chain: Vec<String>,
    /// Trace ID for correlation.
    pub trace_id: String,
    /// Span ID for this specific operation.
    pub span_id: String,
    /// Parent span ID.
    pub parent_span_id: Option<String>,
}

impl LogContext {
    /// Create a new log context.
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(LogContextStorage::default())),
        }
    }

    /// Start a new execution session.
    #[instrument(skip(self, query, metadata))]
    pub async fn start_session(
        &self,
        session_id: String,
        request_id: String,
        query: String,
        context_size: usize,
        max_iterations: u32,
        max_recursion_depth: u32,
        metadata: HashMap<String, String>,
        parent_session: Option<String>,
    ) -> Result<SessionContext, String> {
        let mut storage = self.storage.write().await;

        let session_context = SessionContext {
            session_id: session_id.clone(),
            request_id,
            started_at: SystemTime::now(),
            query,
            context_size,
            max_iterations,
            max_recursion_depth,
            current_iteration: 0,
            current_depth: 0,
            status: SessionStatus::Initializing,
            metadata,
            parent_session: parent_session.clone(),
            child_sessions: Vec::new(),
        };

        // Add to parent's child list if applicable.
        if let Some(parent_id) = &parent_session {
            if let Some(parent_context) = storage.sessions.get_mut(parent_id) {
                parent_context.child_sessions.push(session_id.clone());
            }
        }

        storage.sessions.insert(session_id.clone(), session_context.clone());

        info!(
            session_id = %session_id,
            request_id = %session_context.request_id,
            context_size = context_size,
            max_iterations = max_iterations,
            max_recursion_depth = max_recursion_depth,
            parent_session = ?parent_session,
            "Started RLM execution session"
        );

        Ok(session_context)
    }

    /// Update session status and iteration.
    #[instrument(skip(self))]
    pub async fn update_session(
        &self,
        session_id: &str,
        status: SessionStatus,
        current_iteration: Option<u32>,
        current_depth: Option<u32>,
    ) -> Result<(), String> {
        let mut storage = self.storage.write().await;

        let session = storage
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        session.status = status.clone();
        if let Some(iteration) = current_iteration {
            session.current_iteration = iteration;
        }
        if let Some(depth) = current_depth {
            session.current_depth = depth;
        }

        debug!(
            session_id = %session_id,
            status = ?status,
            iteration = session.current_iteration,
            depth = session.current_depth,
            "Updated session context"
        );

        Ok(())
    }

    /// Start a recursive call.
    #[instrument(skip(self, query, metadata))]
    pub async fn start_recursive_call(
        &self,
        call_id: String,
        session_id: String,
        parent_call_id: Option<String>,
        depth: u32,
        query: String,
        metadata: HashMap<String, String>,
    ) -> Result<RecursiveCallContext, String> {
        let mut storage = self.storage.write().await;

        let call_context = RecursiveCallContext {
            call_id: call_id.clone(),
            session_id: session_id.clone(),
            parent_call_id: parent_call_id.clone(),
            started_at: SystemTime::now(),
            depth,
            query,
            status: CallStatus::Pending,
            input_tokens: 0,
            output_tokens: 0,
            duration: None,
            error_message: None,
            metadata,
            child_calls: Vec::new(),
        };

        // Add to parent call's child list if applicable.
        if let Some(parent_id) = &parent_call_id {
            if let Some(parent_call) = storage.recursive_calls.get_mut(parent_id) {
                parent_call.child_calls.push(call_id.clone());
            }
        }

        storage.recursive_calls.insert(call_id.clone(), call_context.clone());

        info!(
            call_id = %call_id,
            session_id = %session_id,
            parent_call_id = ?parent_call_id,
            depth = depth,
            query = %call_context.query,
            "Started recursive call"
        );

        Ok(call_context)
    }

    /// Update recursive call status and token usage.
    #[instrument(skip(self))]
    pub async fn update_recursive_call(
        &self,
        call_id: &str,
        status: CallStatus,
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
        error_message: Option<String>,
    ) -> Result<(), String> {
        let mut storage = self.storage.write().await;

        let call = storage
            .recursive_calls
            .get_mut(call_id)
            .ok_or_else(|| format!("Recursive call not found: {}", call_id))?;

        let started_at = call.started_at;
        call.status = status.clone();

        if let Some(tokens) = input_tokens {
            call.input_tokens = tokens;
        }
        if let Some(tokens) = output_tokens {
            call.output_tokens = tokens;
        }
        if let Some(error) = error_message {
            call.error_message = Some(error);
        }

        // Calculate duration if call is completed/failed.
        if matches!(status, CallStatus::Completed | CallStatus::Failed | CallStatus::Cancelled) {
            call.duration = SystemTime::now().duration_since(started_at).ok();
        }

        debug!(
            call_id = %call_id,
            status = ?status,
            input_tokens = call.input_tokens,
            output_tokens = call.output_tokens,
            duration_ms = ?call.duration.map(|d| d.as_millis()),
            "Updated recursive call context"
        );

        Ok(())
    }

    /// Get current context span for structured logging.
    #[instrument(skip(self))]
    pub async fn get_context_span(
        &self,
        session_id: &str,
        call_id: Option<&str>,
    ) -> Option<ContextSpan> {
        let storage = self.storage.read().await;

        let session = storage.sessions.get(session_id)?.clone();
        let current_call = if let Some(id) = call_id {
            storage.recursive_calls.get(id).cloned()
        } else {
            None
        };

        // Build call chain from root to current.
        let call_chain = self.build_call_chain(&storage, call_id).await;

        // Generate trace and span IDs.
        let trace_id = format!("trace-{}", session_id);
        let span_id = if let Some(id) = call_id {
            format!("span-{}", id)
        } else {
            format!("span-{}", session_id)
        };

        let parent_span_id = if let Some(call) = &current_call {
            call.parent_call_id
                .as_ref()
                .map(|id| format!("span-{}", id))
        } else {
            None
        };

        Some(ContextSpan {
            session: Some(session),
            current_call,
            call_chain,
            trace_id,
            span_id,
            parent_span_id,
        })
    }

    /// Build call chain from root to current call.
    async fn build_call_chain(
        &self,
        storage: &LogContextStorage,
        call_id: Option<&str>,
    ) -> Vec<String> {
        let mut chain = Vec::new();

        if let Some(mut current_id) = call_id {
            while let Some(call) = storage.recursive_calls.get(current_id) {
                chain.push(current_id.to_string());

                if let Some(parent_id) = &call.parent_call_id {
                    current_id = parent_id;
                } else {
                    break;
                }
            }
        }

        // Reverse to get root-to-current order.
        chain.reverse();
        chain
    }

    /// Get session statistics.
    #[instrument(skip(self))]
    pub async fn get_session_stats(&self, session_id: &str) -> Option<SessionStats> {
        let storage = self.storage.read().await;

        let session = storage.sessions.get(session_id)?;
        let recursive_calls: Vec<_> = storage
            .recursive_calls
            .values()
            .filter(|call| call.session_id == session_id)
            .cloned()
            .collect();

        let total_input_tokens: u32 = recursive_calls.iter().map(|c| c.input_tokens).sum();
        let total_output_tokens: u32 = recursive_calls.iter().map(|c| c.output_tokens).sum();
        let completed_calls = recursive_calls.iter().filter(|c| c.status == CallStatus::Completed).count();
        let failed_calls = recursive_calls.iter().filter(|c| c.status == CallStatus::Failed).count();

        let total_duration = if matches!(session.status, SessionStatus::Completed | SessionStatus::Failed) {
            SystemTime::now().duration_since(session.started_at).ok()
        } else {
            None
        };

        Some(SessionStats {
            session_id: session_id.to_string(),
            total_recursive_calls: recursive_calls.len(),
            completed_calls,
            failed_calls,
            total_input_tokens,
            total_output_tokens,
            current_iteration: session.current_iteration,
            max_depth_reached: recursive_calls.iter().map(|c| c.depth).max().unwrap_or(0),
            total_duration,
        })
    }

    /// Clean up completed sessions and calls.
    #[instrument(skip(self))]
    pub async fn cleanup_old_contexts(&self, retention_duration: Duration) -> Result<usize, String> {
        let mut storage = self.storage.write().await;
        let cutoff_time = SystemTime::now()
            .checked_sub(retention_duration)
            .ok_or_else(|| "Retention duration too large".to_string())?;

        let mut removed_count = 0;

        // Remove old completed/failed sessions.
        let sessions_to_remove: Vec<String> = storage
            .sessions
            .iter()
            .filter(|(_, session)| {
                matches!(session.status, SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Cancelled)
                    && session.started_at < cutoff_time
            })
            .map(|(id, _)| id.clone())
            .collect();

        for session_id in &sessions_to_remove {
            storage.sessions.remove(session_id);
            removed_count += 1;
        }

        // Remove recursive calls for removed sessions.
        let calls_to_remove: Vec<String> = storage
            .recursive_calls
            .iter()
            .filter(|(_, call)| sessions_to_remove.contains(&call.session_id))
            .map(|(id, _)| id.clone())
            .collect();

        for call_id in calls_to_remove {
            storage.recursive_calls.remove(&call_id);
            removed_count += 1;
        }

        if removed_count > 0 {
            info!(
                removed_contexts = removed_count,
                retention_hours = retention_duration.as_secs() / 3600,
                "Cleaned up old log contexts"
            );
        }

        Ok(removed_count)
    }

    /// Set global context value.
    pub async fn set_global_context(&self, key: String, value: String) {
        let mut storage = self.storage.write().await;
        storage.global_context.insert(key, value);
    }

    /// Get global context value.
    pub async fn get_global_context(&self, key: &str) -> Option<String> {
        let storage = self.storage.read().await;
        storage.global_context.get(key).cloned()
    }
}

/// Session execution statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// Session ID.
    pub session_id: String,
    /// Total number of recursive calls made.
    pub total_recursive_calls: usize,
    /// Number of completed calls.
    pub completed_calls: usize,
    /// Number of failed calls.
    pub failed_calls: usize,
    /// Total input tokens across all calls.
    pub total_input_tokens: u32,
    /// Total output tokens across all calls.
    pub total_output_tokens: u32,
    /// Current iteration number.
    pub current_iteration: u32,
    /// Maximum recursion depth reached.
    pub max_depth_reached: u32,
    /// Total session duration (if completed).
    pub total_duration: Option<Duration>,
}

impl Default for LogContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience macro for structured logging with RLM context.
#[macro_export]
macro_rules! log_with_context {
    ($level:ident, $span:expr, $($args:tt)*) => {
        if let Some(span) = $span {
            tracing::$level!(
                session_id = %span.session.as_ref().map(|s| s.session_id.as_str()).unwrap_or("unknown"),
                request_id = %span.session.as_ref().map(|s| s.request_id.as_str()).unwrap_or("unknown"),
                trace_id = %span.trace_id,
                span_id = %span.span_id,
                parent_span_id = ?span.parent_span_id,
                call_id = ?span.current_call.as_ref().map(|c| c.call_id.as_str()),
                depth = ?span.current_call.as_ref().map(|c| c.depth),
                iteration = ?span.session.as_ref().map(|s| s.current_iteration),
                call_chain = ?span.call_chain,
                $($args)*
            );
        } else {
            tracing::$level!($($args)*);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let context = LogContext::new();
        let session_id = "test-session".to_string();
        let request_id = "test-request".to_string();

        // Start session.
        let session = context
            .start_session(
                session_id.clone(),
                request_id.clone(),
                "test query".to_string(),
                1000,
                10,
                2,
                HashMap::new(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(session.session_id, session_id);
        assert_eq!(session.status, SessionStatus::Initializing);

        // Update session.
        context
            .update_session(&session_id, SessionStatus::Running, Some(1), Some(0))
            .await
            .unwrap();

        // Get context span.
        let span = context.get_context_span(&session_id, None).await.unwrap();
        assert_eq!(span.session.unwrap().status, SessionStatus::Running);
    }

    #[tokio::test]
    async fn test_recursive_call_chain() {
        let context = LogContext::new();
        let session_id = "test-session".to_string();

        // Start session.
        context
            .start_session(
                session_id.clone(),
                "test-request".to_string(),
                "test query".to_string(),
                1000,
                10,
                2,
                HashMap::new(),
                None,
            )
            .await
            .unwrap();

        // Start first recursive call.
        let call1_id = "call-1".to_string();
        context
            .start_recursive_call(
                call1_id.clone(),
                session_id.clone(),
                None,
                1,
                "recursive query 1".to_string(),
                HashMap::new(),
            )
            .await
            .unwrap();

        // Start nested recursive call.
        let call2_id = "call-2".to_string();
        context
            .start_recursive_call(
                call2_id.clone(),
                session_id.clone(),
                Some(call1_id.clone()),
                2,
                "recursive query 2".to_string(),
                HashMap::new(),
            )
            .await
            .unwrap();

        // Get context span for nested call.
        let span = context
            .get_context_span(&session_id, Some(&call2_id))
            .await
            .unwrap();

        assert_eq!(span.call_chain, vec![call1_id, call2_id]);
        assert_eq!(span.current_call.unwrap().depth, 2);
    }

    #[tokio::test]
    async fn test_session_stats() {
        let context = LogContext::new();
        let session_id = "test-session".to_string();

        // Start session.
        context
            .start_session(
                session_id.clone(),
                "test-request".to_string(),
                "test query".to_string(),
                1000,
                10,
                2,
                HashMap::new(),
                None,
            )
            .await
            .unwrap();

        // Start and complete recursive calls.
        let call_id = "call-1".to_string();
        context
            .start_recursive_call(
                call_id.clone(),
                session_id.clone(),
                None,
                1,
                "recursive query".to_string(),
                HashMap::new(),
            )
            .await
            .unwrap();

        context
            .update_recursive_call(&call_id, CallStatus::Completed, Some(100), Some(50), None)
            .await
            .unwrap();

        // Get session statistics.
        let stats = context.get_session_stats(&session_id).await.unwrap();
        assert_eq!(stats.total_recursive_calls, 1);
        assert_eq!(stats.completed_calls, 1);
        assert_eq!(stats.total_input_tokens, 100);
        assert_eq!(stats.total_output_tokens, 50);
    }
}