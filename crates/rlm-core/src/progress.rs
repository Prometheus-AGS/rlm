//! Progress tracking for recursive calls in RLM execution.
//!
//! This module provides comprehensive progress tracking for recursive LLM calls,
//! enabling real-time monitoring of complex recursive processing operations.
//! It integrates with the streaming SSE system to provide live progress updates.

use crate::{RlmEvent, RlmEventData, CallStatus, RlmResult, RlmError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

/// Progress tracking information for a single recursive call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallProgress {
    /// Unique identifier for this call.
    pub call_id: String,
    /// Parent call ID (None for root call).
    pub parent_id: Option<String>,
    /// Current recursion depth.
    pub depth: u32,
    /// Current status of the call.
    pub status: CallStatus,
    /// When this call was initiated.
    pub started_at: SystemTime,
    /// When this call completed (if completed).
    pub completed_at: Option<SystemTime>,
    /// Number of tokens in the prompt.
    pub prompt_tokens: u32,
    /// Number of tokens generated so far.
    pub completion_tokens: u32,
    /// Progress percentage (0-100).
    pub progress_percentage: f32,
    /// Human-readable status message.
    pub status_message: String,
    /// Any error message if call failed.
    pub error_message: Option<String>,
    /// Child call IDs spawned by this call.
    pub child_calls: Vec<String>,
}

impl CallProgress {
    /// Create a new call progress tracker.
    pub fn new(call_id: String, parent_id: Option<String>, depth: u32) -> Self {
        Self {
            call_id,
            parent_id,
            depth,
            status: CallStatus::Pending,
            started_at: SystemTime::now(),
            completed_at: None,
            prompt_tokens: 0,
            completion_tokens: 0,
            progress_percentage: 0.0,
            status_message: "Initializing recursive call".to_string(),
            error_message: None,
            child_calls: Vec::new(),
        }
    }

    /// Update the progress of this call.
    pub fn update_progress(&mut self, percentage: f32, message: String) {
        self.progress_percentage = percentage.clamp(0.0, 100.0);
        self.status_message = message;

        // Update status based on progress
        if self.status == CallStatus::Pending && percentage > 0.0 {
            self.status = CallStatus::InProgress;
        }
    }

    /// Mark this call as completed successfully.
    pub fn complete(&mut self, completion_tokens: u32) {
        self.status = CallStatus::Completed;
        self.completion_tokens = completion_tokens;
        self.progress_percentage = 100.0;
        self.status_message = "Call completed successfully".to_string();
        self.completed_at = Some(SystemTime::now());
    }

    /// Mark this call as failed with an error.
    pub fn fail(&mut self, error: String) {
        self.status = CallStatus::Failed;
        self.progress_percentage = 0.0;
        self.status_message = "Call failed".to_string();
        self.error_message = Some(error);
        self.completed_at = Some(SystemTime::now());
    }

    /// Add a child call to this call's tracking.
    pub fn add_child_call(&mut self, child_id: String) {
        if !self.child_calls.contains(&child_id) {
            self.child_calls.push(child_id);
        }
    }

    /// Calculate the duration of this call.
    pub fn duration(&self) -> Duration {
        let end_time = self.completed_at.unwrap_or_else(SystemTime::now);
        end_time.duration_since(self.started_at).unwrap_or_default()
    }

    /// Check if this call is terminal (completed or failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, CallStatus::Completed | CallStatus::Failed)
    }
}

/// Overall progress information for an entire RLM execution session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProgress {
    /// Session identifier.
    pub session_id: String,
    /// Root call ID for this session.
    pub root_call_id: String,
    /// When this session started.
    pub started_at: SystemTime,
    /// Total number of recursive calls in this session.
    pub total_calls: u32,
    /// Number of completed calls.
    pub completed_calls: u32,
    /// Number of failed calls.
    pub failed_calls: u32,
    /// Number of running calls.
    pub running_calls: u32,
    /// Overall session progress (0-100).
    pub overall_progress: f32,
    /// Maximum recursion depth reached.
    pub max_depth_reached: u32,
    /// Total tokens consumed across all calls.
    pub total_tokens_consumed: u32,
    /// Estimated time remaining (if calculable).
    pub estimated_time_remaining: Option<Duration>,
}

impl SessionProgress {
    /// Create a new session progress tracker.
    pub fn new(session_id: String, root_call_id: String) -> Self {
        Self {
            session_id,
            root_call_id,
            started_at: SystemTime::now(),
            total_calls: 1, // Start with the root call
            completed_calls: 0,
            failed_calls: 0,
            running_calls: 0,
            overall_progress: 0.0,
            max_depth_reached: 0,
            total_tokens_consumed: 0,
            estimated_time_remaining: None,
        }
    }

    /// Update session statistics based on call progress.
    pub fn update_from_calls(&mut self, calls: &HashMap<String, CallProgress>) {
        self.total_calls = calls.len() as u32;
        self.completed_calls = calls.values().filter(|c| c.status == CallStatus::Completed).count() as u32;
        self.failed_calls = calls.values().filter(|c| c.status == CallStatus::Failed).count() as u32;
        self.running_calls = calls.values().filter(|c| c.status == CallStatus::InProgress).count() as u32;

        // Calculate overall progress
        if self.total_calls > 0 {
            self.overall_progress = (self.completed_calls as f32 / self.total_calls as f32) * 100.0;
        }

        // Find maximum depth
        self.max_depth_reached = calls.values().map(|c| c.depth).max().unwrap_or(0);

        // Sum total tokens
        self.total_tokens_consumed = calls.values()
            .map(|c| c.prompt_tokens + c.completion_tokens)
            .sum();

        // Estimate remaining time based on average completion time
        self.estimated_time_remaining = self.calculate_estimated_time_remaining(calls);
    }

    /// Calculate estimated time remaining based on current progress.
    fn calculate_estimated_time_remaining(&self, calls: &HashMap<String, CallProgress>) -> Option<Duration> {
        if self.completed_calls == 0 || self.running_calls == 0 {
            return None;
        }

        // Calculate average time per completed call
        let total_completed_duration: Duration = calls.values()
            .filter(|c| c.status == CallStatus::Completed)
            .map(|c| c.duration())
            .sum();

        if total_completed_duration.is_zero() {
            return None;
        }

        let avg_duration = total_completed_duration / self.completed_calls;
        let estimated_remaining = avg_duration * self.running_calls;

        Some(estimated_remaining)
    }

    /// Check if the session is complete.
    pub fn is_complete(&self) -> bool {
        self.total_calls > 0 && (self.completed_calls + self.failed_calls) == self.total_calls
    }
}

/// Progress tracker for managing recursive call progress across an entire RLM session.
#[derive(Debug)]
pub struct ProgressTracker {
    /// All call progress information indexed by call ID.
    calls: Arc<RwLock<HashMap<String, CallProgress>>>,
    /// Session-level progress information.
    session: Arc<RwLock<SessionProgress>>,
    /// Start time for performance tracking.
    start_time: Instant,
}

impl ProgressTracker {
    /// Create a new progress tracker for a session.
    pub fn new(session_id: String) -> Self {
        let root_call_id = Uuid::new_v4().to_string();
        let session = SessionProgress::new(session_id, root_call_id.clone());

        Self {
            calls: Arc::new(RwLock::new(HashMap::new())),
            session: Arc::new(RwLock::new(session)),
            start_time: Instant::now(),
        }
    }

    /// Start tracking a new recursive call.
    #[instrument(skip(self))]
    pub async fn start_call(
        &self,
        call_id: String,
        parent_id: Option<String>,
        depth: u32,
        prompt_tokens: u32,
    ) -> RlmResult<()> {
        let mut call_progress = CallProgress::new(call_id.clone(), parent_id.clone(), depth);
        call_progress.prompt_tokens = prompt_tokens;
        call_progress.status = CallStatus::InProgress;
        call_progress.status_message = "Starting LLM call".to_string();

        {
            let mut calls = self.calls.write().await;
            calls.insert(call_id.clone(), call_progress);

            // Add this call as a child to its parent
            if let Some(parent_id) = parent_id {
                if let Some(parent) = calls.get_mut(&parent_id) {
                    parent.add_child_call(call_id.clone());
                }
            }
        }

        // Update session progress
        {
            let calls = self.calls.read().await;
            let mut session = self.session.write().await;
            session.update_from_calls(&calls);
        }

        debug!("Started tracking call {} at depth {}", call_id, depth);
        Ok(())
    }

    /// Update progress for a specific call.
    #[instrument(skip(self))]
    pub async fn update_call_progress(
        &self,
        call_id: &str,
        percentage: f32,
        message: String,
    ) -> RlmResult<()> {
        {
            let mut calls = self.calls.write().await;
            if let Some(call) = calls.get_mut(call_id) {
                call.update_progress(percentage, message);
            } else {
                warn!("Attempted to update progress for unknown call: {}", call_id);
                return Err(RlmError::Other(format!("Unknown call ID: {}", call_id)));
            }
        }

        // Update session progress
        {
            let calls = self.calls.read().await;
            let mut session = self.session.write().await;
            session.update_from_calls(&calls);
        }

        debug!("Updated progress for call {}: {}%", call_id, percentage);
        Ok(())
    }

    /// Mark a call as completed.
    #[instrument(skip(self))]
    pub async fn complete_call(
        &self,
        call_id: &str,
        completion_tokens: u32,
    ) -> RlmResult<()> {
        {
            let mut calls = self.calls.write().await;
            if let Some(call) = calls.get_mut(call_id) {
                call.complete(completion_tokens);
            } else {
                warn!("Attempted to complete unknown call: {}", call_id);
                return Err(RlmError::Other(format!("Unknown call ID: {}", call_id)));
            }
        }

        // Update session progress
        {
            let calls = self.calls.read().await;
            let mut session = self.session.write().await;
            session.update_from_calls(&calls);
        }

        debug!("Completed call {}", call_id);
        Ok(())
    }

    /// Mark a call as failed.
    #[instrument(skip(self))]
    pub async fn fail_call(&self, call_id: &str, error: String) -> RlmResult<()> {
        {
            let mut calls = self.calls.write().await;
            if let Some(call) = calls.get_mut(call_id) {
                call.fail(error);
            } else {
                warn!("Attempted to fail unknown call: {}", call_id);
                return Err(RlmError::Other(format!("Unknown call ID: {}", call_id)));
            }
        }

        // Update session progress
        {
            let calls = self.calls.read().await;
            let mut session = self.session.write().await;
            session.update_from_calls(&calls);
        }

        debug!("Failed call {}", call_id);
        Ok(())
    }

    /// Get progress for a specific call.
    pub async fn get_call_progress(&self, call_id: &str) -> Option<CallProgress> {
        let calls = self.calls.read().await;
        calls.get(call_id).cloned()
    }

    /// Get all call progress information.
    pub async fn get_all_calls(&self) -> HashMap<String, CallProgress> {
        let calls = self.calls.read().await;
        calls.clone()
    }

    /// Get session-level progress.
    pub async fn get_session_progress(&self) -> SessionProgress {
        let session = self.session.read().await;
        session.clone()
    }

    /// Generate a progress event for streaming.
    pub async fn create_progress_event(&self) -> RlmResult<RlmEvent> {
        let session = self.get_session_progress().await;
        let calls = self.get_all_calls().await;

        // Find the most recently updated call for detailed progress
        let most_recent_call = calls.values()
            .filter(|c| c.status == CallStatus::InProgress)
            .max_by_key(|c| c.started_at);

        let _status_message = if let Some(call) = most_recent_call {
            format!(
                "Processing depth {}: {} ({}%)",
                call.depth, call.status_message, call.progress_percentage as u32
            )
        } else if session.is_complete() {
            "All recursive calls completed".to_string()
        } else {
            format!(
                "Progress: {}/{} calls completed ({}%)",
                session.completed_calls, session.total_calls, session.overall_progress as u32
            )
        };

        Ok(RlmEvent {
            event_id: Uuid::new_v4().to_string(),
            request_id: session.session_id.clone(),
            data: RlmEventData::RecursiveCall {
                call_id: session.root_call_id.clone(),
                depth: session.max_depth_reached,
                status: if session.is_complete() {
                    CallStatus::Completed
                } else {
                    CallStatus::InProgress
                },
                prompt_tokens: session.total_tokens_consumed / 2, // Rough estimate
                completion_tokens: session.total_tokens_consumed / 2,
            },
            timestamp: SystemTime::now(),
        })
    }

    /// Get overall progress percentage (0-100).
    pub async fn get_overall_progress(&self) -> f32 {
        let session = self.session.read().await;
        session.overall_progress
    }

    /// Check if all calls are complete.
    pub async fn is_complete(&self) -> bool {
        let session = self.session.read().await;
        session.is_complete()
    }

    /// Get the total elapsed time for the session.
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get statistics for all calls in the session.
    pub async fn get_statistics(&self) -> ProgressStatistics {
        let calls = self.calls.read().await;
        let session = self.session.read().await;

        let total_duration = calls.values()
            .filter_map(|c| c.completed_at)
            .map(|_| self.elapsed_time())
            .fold(Duration::ZERO, |acc, d| acc + d);

        ProgressStatistics {
            total_calls: session.total_calls,
            completed_calls: session.completed_calls,
            failed_calls: session.failed_calls,
            running_calls: session.running_calls,
            max_depth: session.max_depth_reached,
            total_tokens: session.total_tokens_consumed,
            total_duration,
            average_call_duration: if session.completed_calls > 0 {
                Some(total_duration / session.completed_calls)
            } else {
                None
            },
            estimated_completion: session.estimated_time_remaining,
        }
    }
}

/// Statistical information about progress tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressStatistics {
    /// Total number of calls.
    pub total_calls: u32,
    /// Number of completed calls.
    pub completed_calls: u32,
    /// Number of failed calls.
    pub failed_calls: u32,
    /// Number of currently running calls.
    pub running_calls: u32,
    /// Maximum recursion depth reached.
    pub max_depth: u32,
    /// Total tokens consumed.
    pub total_tokens: u32,
    /// Total duration of all completed calls.
    pub total_duration: Duration,
    /// Average duration per completed call.
    pub average_call_duration: Option<Duration>,
    /// Estimated time to completion.
    pub estimated_completion: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_call_progress_creation() {
        let call_id = "test-call-1".to_string();
        let progress = CallProgress::new(call_id.clone(), None, 0);

        assert_eq!(progress.call_id, call_id);
        assert_eq!(progress.depth, 0);
        assert_eq!(progress.status, CallStatus::Pending);
        assert_eq!(progress.progress_percentage, 0.0);
        assert_eq!(progress.prompt_tokens, 0);
        assert_eq!(progress.completion_tokens, 0);
    }

    #[tokio::test]
    async fn test_progress_updates() {
        let mut progress = CallProgress::new("test-call".to_string(), None, 1);

        progress.update_progress(25.0, "Processing chunk 1 of 4".to_string());
        assert_eq!(progress.progress_percentage, 25.0);
        assert_eq!(progress.status, CallStatus::InProgress);

        progress.complete(150);
        assert_eq!(progress.status, CallStatus::Completed);
        assert_eq!(progress.progress_percentage, 100.0);
        assert_eq!(progress.completion_tokens, 150);
        assert!(progress.is_terminal());
    }

    #[tokio::test]
    async fn test_progress_tracker() {
        let session_id = "test-session".to_string();
        let tracker = ProgressTracker::new(session_id.clone());

        // Start a call
        let call_id = "call-1".to_string();
        tracker.start_call(call_id.clone(), None, 0, 100).await.unwrap();

        // Update progress
        tracker.update_call_progress(&call_id, 50.0, "Halfway done".to_string()).await.unwrap();

        // Complete the call
        tracker.complete_call(&call_id, 75).await.unwrap();

        // Check final state
        let progress = tracker.get_call_progress(&call_id).await.unwrap();
        assert_eq!(progress.status, CallStatus::Completed);
        assert_eq!(progress.completion_tokens, 75);

        let session = tracker.get_session_progress().await;
        assert!(session.is_complete());
        assert_eq!(session.completed_calls, 1);
    }

    #[tokio::test]
    async fn test_hierarchical_calls() {
        let tracker = ProgressTracker::new("test-session".to_string());

        // Start parent call
        let parent_id = "parent-call".to_string();
        tracker.start_call(parent_id.clone(), None, 0, 100).await.unwrap();

        // Start child call
        let child_id = "child-call".to_string();
        tracker.start_call(child_id.clone(), Some(parent_id.clone()), 1, 50).await.unwrap();

        // Verify parent-child relationship
        let parent_progress = tracker.get_call_progress(&parent_id).await.unwrap();
        assert!(parent_progress.child_calls.contains(&child_id));

        let child_progress = tracker.get_call_progress(&child_id).await.unwrap();
        assert_eq!(child_progress.parent_id, Some(parent_id));
        assert_eq!(child_progress.depth, 1);
    }

    #[tokio::test]
    async fn test_session_statistics() {
        let tracker = ProgressTracker::new("test-session".to_string());

        // Create multiple calls with different outcomes
        tracker.start_call("call-1".to_string(), None, 0, 100).await.unwrap();
        tracker.start_call("call-2".to_string(), None, 0, 150).await.unwrap();
        tracker.start_call("call-3".to_string(), None, 0, 75).await.unwrap();

        // Complete first call
        tracker.complete_call("call-1", 50).await.unwrap();

        // Fail second call
        tracker.fail_call("call-2", "Test error".to_string()).await.unwrap();

        // Keep third call running
        tracker.update_call_progress("call-3", 30.0, "In progress".to_string()).await.unwrap();

        let stats = tracker.get_statistics().await;
        assert_eq!(stats.total_calls, 3);
        assert_eq!(stats.completed_calls, 1);
        assert_eq!(stats.failed_calls, 1);
        assert_eq!(stats.running_calls, 1);

        let session = tracker.get_session_progress().await;
        assert!(!session.is_complete());
    }
}