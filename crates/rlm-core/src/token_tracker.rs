//! Token usage tracking across recursive RLM calls.
//!
//! This module provides comprehensive token usage tracking that follows
//! token consumption through recursive calls, enabling cost analysis
//! and optimization insights for RLM processing.

use crate::{
    error::{RlmError, RlmResult},
    types::{RlmEvent, RlmEventData, TokenUsage},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tracing::{debug, instrument, warn};

/// Token usage tracker for RLM operations.
#[derive(Debug, Clone)]
pub struct TokenTracker {
    /// Per-session token tracking data.
    sessions: Arc<RwLock<HashMap<String, SessionTokenUsage>>>,
    /// Global aggregated token statistics.
    global_stats: Arc<RwLock<GlobalTokenStats>>,
}

/// Token usage data for a specific session.
#[derive(Debug, Clone)]
pub struct SessionTokenUsage {
    /// Session identifier.
    pub session_id: String,
    /// Session start time.
    pub started_at: SystemTime,
    /// Total tokens used in this session.
    pub total_tokens: u32,
    /// Prompt tokens used in this session.
    pub prompt_tokens: u32,
    /// Completion tokens generated in this session.
    pub completion_tokens: u32,
    /// Per-recursive-call token breakdown.
    pub recursive_call_tokens: HashMap<String, RecursiveCallTokens>,
    /// Token usage timeline for analysis.
    pub token_timeline: Vec<TokenUsageSnapshot>,
    /// Estimated cost in USD.
    pub estimated_cost_usd: f64,
    /// Current token rate (tokens per minute).
    pub current_rate_tpm: f64,
}

/// Token usage for a specific recursive call.
#[derive(Debug, Clone)]
pub struct RecursiveCallTokens {
    /// Unique call identifier.
    pub call_id: String,
    /// Recursion depth level.
    pub depth: u32,
    /// Parent call ID (if any).
    pub parent_call_id: Option<String>,
    /// Tokens used in this call.
    pub call_tokens: TokenUsage,
    /// Child call token usage.
    pub child_calls: Vec<RecursiveCallTokens>,
    /// Call start time.
    pub started_at: SystemTime,
    /// Call completion time.
    pub completed_at: Option<SystemTime>,
    /// Call status.
    pub status: CallTokenStatus,
}

/// Token usage snapshot at a point in time.
#[derive(Debug, Clone)]
pub struct TokenUsageSnapshot {
    /// Snapshot timestamp.
    pub timestamp: SystemTime,
    /// Cumulative tokens at this point.
    pub cumulative_tokens: u32,
    /// Tokens used since last snapshot.
    pub delta_tokens: u32,
    /// Current recursion depth.
    pub current_depth: u32,
    /// Active recursive calls at this point.
    pub active_calls: u32,
}

/// Status of a token-tracked recursive call.
#[derive(Debug, Clone, PartialEq)]
pub enum CallTokenStatus {
    /// Call is in progress.
    InProgress,
    /// Call completed successfully.
    Completed,
    /// Call failed.
    Failed,
    /// Call timed out.
    TimedOut,
}

/// Global token usage statistics.
#[derive(Debug, Default, Clone)]
pub struct GlobalTokenStats {
    /// Total tokens processed across all sessions.
    pub total_tokens_processed: u64,
    /// Total estimated cost across all sessions.
    pub total_estimated_cost_usd: f64,
    /// Average tokens per session.
    pub avg_tokens_per_session: f64,
    /// Average tokens per recursive call.
    pub avg_tokens_per_recursive_call: f64,
    /// Peak token usage rate (tokens per minute).
    pub peak_token_rate_tpm: f64,
    /// Token usage distribution by depth.
    pub depth_distribution: HashMap<u32, u64>,
    /// Session count.
    pub session_count: u64,
    /// Recursive call count.
    pub recursive_call_count: u64,
}

/// Token pricing configuration for cost estimation.
#[derive(Debug, Clone)]
pub struct TokenPricing {
    /// Cost per prompt token in USD.
    pub prompt_token_cost: f64,
    /// Cost per completion token in USD.
    pub completion_token_cost: f64,
    /// Provider name (for reference).
    pub provider: String,
    /// Model name.
    pub model: String,
}

impl Default for TokenPricing {
    fn default() -> Self {
        Self {
            // Default to GPT-4 pricing as of 2024
            prompt_token_cost: 0.00003,     // $0.03 per 1K tokens
            completion_token_cost: 0.00006, // $0.06 per 1K tokens
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
        }
    }
}

impl TokenTracker {
    /// Create a new token tracker.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            global_stats: Arc::new(RwLock::new(GlobalTokenStats::default())),
        }
    }

    /// Start tracking tokens for a new session.
    #[instrument(skip(self), fields(session_id = %session_id))]
    pub fn start_session(&self, session_id: &str) -> RlmResult<()> {
        let mut sessions = self.sessions.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

        let session_usage = SessionTokenUsage {
            session_id: session_id.to_string(),
            started_at: SystemTime::now(),
            total_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            recursive_call_tokens: HashMap::new(),
            token_timeline: Vec::new(),
            estimated_cost_usd: 0.0,
            current_rate_tpm: 0.0,
        };

        sessions.insert(session_id.to_string(), session_usage);

        // Update global stats
        let mut global_stats = self.global_stats.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire global stats lock: {}", e)))?;
        global_stats.session_count += 1;

        debug!(session_id = %session_id, "Started token tracking for session");
        Ok(())
    }

    /// Record token usage for a recursive call.
    #[instrument(skip(self), fields(session_id = %session_id, call_id = %call_id))]
    pub fn record_recursive_call(
        &self,
        session_id: &str,
        call_id: &str,
        depth: u32,
        parent_call_id: Option<&str>,
        token_usage: &TokenUsage,
    ) -> RlmResult<()> {
        let mut sessions = self.sessions.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

        let session = sessions.get_mut(session_id)
            .ok_or_else(|| RlmError::Other(format!("Session not found: {}", session_id)))?;

        // Create recursive call token record
        let recursive_call_tokens = RecursiveCallTokens {
            call_id: call_id.to_string(),
            depth,
            parent_call_id: parent_call_id.map(|s| s.to_string()),
            call_tokens: token_usage.clone(),
            child_calls: Vec::new(),
            started_at: SystemTime::now(),
            completed_at: None,
            status: CallTokenStatus::InProgress,
        };

        // Add to session tracking
        session.recursive_call_tokens.insert(call_id.to_string(), recursive_call_tokens);

        // Update session totals
        session.total_tokens += token_usage.total_tokens;
        session.prompt_tokens += token_usage.prompt_tokens;
        session.completion_tokens += token_usage.completion_tokens;

        // Add timeline snapshot
        let snapshot = TokenUsageSnapshot {
            timestamp: SystemTime::now(),
            cumulative_tokens: session.total_tokens,
            delta_tokens: token_usage.total_tokens,
            current_depth: depth,
            active_calls: session.recursive_call_tokens.len() as u32,
        };
        session.token_timeline.push(snapshot);

        // Update token rate
        self.update_token_rate(session_id)?;

        // Update global statistics
        self.update_global_stats(token_usage, depth)?;

        debug!(
            session_id = %session_id,
            call_id = %call_id,
            depth = depth,
            tokens = token_usage.total_tokens,
            "Recorded recursive call token usage"
        );

        Ok(())
    }

    /// Mark a recursive call as completed.
    #[instrument(skip(self), fields(session_id = %session_id, call_id = %call_id))]
    pub fn complete_recursive_call(
        &self,
        session_id: &str,
        call_id: &str,
        success: bool,
    ) -> RlmResult<()> {
        let mut sessions = self.sessions.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

        let session = sessions.get_mut(session_id)
            .ok_or_else(|| RlmError::Other(format!("Session not found: {}", session_id)))?;

        if let Some(call_tokens) = session.recursive_call_tokens.get_mut(call_id) {
            call_tokens.completed_at = Some(SystemTime::now());
            call_tokens.status = if success {
                CallTokenStatus::Completed
            } else {
                CallTokenStatus::Failed
            };

            debug!(
                session_id = %session_id,
                call_id = %call_id,
                success = success,
                "Completed recursive call token tracking"
            );
        }

        Ok(())
    }

    /// Calculate estimated cost for a session.
    #[instrument(skip(self, pricing), fields(session_id = %session_id))]
    pub fn calculate_session_cost(
        &self,
        session_id: &str,
        pricing: &TokenPricing,
    ) -> RlmResult<f64> {
        let sessions = self.sessions.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

        let session = sessions.get(session_id)
            .ok_or_else(|| RlmError::Other(format!("Session not found: {}", session_id)))?;

        let prompt_cost = session.prompt_tokens as f64 * pricing.prompt_token_cost;
        let completion_cost = session.completion_tokens as f64 * pricing.completion_token_cost;
        let total_cost = prompt_cost + completion_cost;

        debug!(
            session_id = %session_id,
            prompt_tokens = session.prompt_tokens,
            completion_tokens = session.completion_tokens,
            total_cost = total_cost,
            "Calculated session cost"
        );

        Ok(total_cost)
    }

    /// Get token usage for a specific session.
    pub fn get_session_usage(&self, session_id: &str) -> RlmResult<Option<SessionTokenUsage>> {
        let sessions = self.sessions.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

        Ok(sessions.get(session_id).cloned())
    }

    /// Get global token statistics.
    pub fn get_global_stats(&self) -> RlmResult<GlobalTokenStats> {
        let global_stats = self.global_stats.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire global stats lock: {}", e)))?;

        Ok(global_stats.clone())
    }

    /// Get recursive call hierarchy for a session.
    pub fn get_call_hierarchy(&self, session_id: &str) -> RlmResult<Vec<RecursiveCallTokens>> {
        let sessions = self.sessions.read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

        let session = sessions.get(session_id)
            .ok_or_else(|| RlmError::Other(format!("Session not found: {}", session_id)))?;

        // Build hierarchical structure
        let mut hierarchy = Vec::new();
        let calls: HashMap<String, RecursiveCallTokens> = session.recursive_call_tokens.clone();

        // Find root calls (those without parents)
        for (_call_id, call_tokens) in calls.iter() {
            if call_tokens.parent_call_id.is_none() {
                let mut root_call = call_tokens.clone();
                self.build_call_tree(&mut root_call, &calls);
                hierarchy.push(root_call);
            }
        }

        Ok(hierarchy)
    }

    /// Process RLM event for token tracking.
    #[instrument(skip(self, event))]
    pub fn process_event(&self, event: &RlmEvent, session_id: &str) -> RlmResult<()> {
        match &event.data {
            RlmEventData::RecursiveCall {
                call_id,
                depth,
                prompt_tokens,
                completion_tokens,
                ..
            } => {
                let token_usage = TokenUsage {
                    prompt_tokens: *prompt_tokens,
                    completion_tokens: *completion_tokens,
                    total_tokens: prompt_tokens + completion_tokens,
                    recursive_calls: 1,
                };

                self.record_recursive_call(
                    session_id,
                    call_id,
                    *depth,
                    None, // Parent relationship would need additional context
                    &token_usage,
                )?;
            }
            RlmEventData::Done { total_tokens, .. } => {
                // Update session totals on completion
                let mut sessions = self.sessions.write()
                    .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

                if let Some(session) = sessions.get_mut(session_id) {
                    session.total_tokens = *total_tokens;

                    // Update estimated cost with default pricing
                    let pricing = TokenPricing::default();
                    let cost = self.calculate_session_cost(session_id, &pricing)?;
                    session.estimated_cost_usd = cost;
                }
            }
            _ => {
                // Other events don't directly affect token tracking
            }
        }

        Ok(())
    }

    /// Clean up old session data.
    #[instrument(skip(self))]
    pub fn cleanup_old_sessions(&self, max_age: Duration) -> RlmResult<u64> {
        let mut sessions = self.sessions.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

        let now = SystemTime::now();
        let mut removed_count = 0;

        sessions.retain(|_session_id, session| {
            match now.duration_since(session.started_at) {
                Ok(age) if age > max_age => {
                    removed_count += 1;
                    false
                }
                _ => true,
            }
        });

        if removed_count > 0 {
            debug!(removed_sessions = removed_count, "Cleaned up old token tracking sessions");
        }

        Ok(removed_count)
    }

    /// Update token usage rate for a session.
    fn update_token_rate(&self, session_id: &str) -> RlmResult<()> {
        let mut sessions = self.sessions.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire sessions lock: {}", e)))?;

        let session = sessions.get_mut(session_id)
            .ok_or_else(|| RlmError::Other(format!("Session not found: {}", session_id)))?;

        // Calculate tokens per minute
        if let Ok(duration) = SystemTime::now().duration_since(session.started_at) {
            let minutes = duration.as_secs_f64() / 60.0;
            if minutes > 0.0 {
                session.current_rate_tpm = session.total_tokens as f64 / minutes;
            }
        }

        Ok(())
    }

    /// Update global token statistics.
    fn update_global_stats(&self, token_usage: &TokenUsage, depth: u32) -> RlmResult<()> {
        let mut global_stats = self.global_stats.write()
            .map_err(|e| RlmError::Other(format!("Failed to acquire global stats lock: {}", e)))?;

        global_stats.total_tokens_processed += token_usage.total_tokens as u64;
        global_stats.recursive_call_count += 1;

        // Update depth distribution
        *global_stats.depth_distribution.entry(depth).or_insert(0) += 1;

        // Recalculate averages
        if global_stats.session_count > 0 {
            global_stats.avg_tokens_per_session =
                global_stats.total_tokens_processed as f64 / global_stats.session_count as f64;
        }
        if global_stats.recursive_call_count > 0 {
            global_stats.avg_tokens_per_recursive_call =
                global_stats.total_tokens_processed as f64 / global_stats.recursive_call_count as f64;
        }

        Ok(())
    }

    /// Build recursive call tree structure.
    fn build_call_tree(
        &self,
        parent: &mut RecursiveCallTokens,
        all_calls: &HashMap<String, RecursiveCallTokens>,
    ) {
        for (_call_id, call_tokens) in all_calls.iter() {
            if let Some(ref parent_id) = call_tokens.parent_call_id {
                if parent_id == &parent.call_id {
                    let mut child = call_tokens.clone();
                    self.build_call_tree(&mut child, all_calls);
                    parent.child_calls.push(child);
                }
            }
        }
    }
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SessionTokenUsage {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            started_at: SystemTime::now(),
            total_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            recursive_call_tokens: HashMap::new(),
            token_timeline: Vec::new(),
            estimated_cost_usd: 0.0,
            current_rate_tpm: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_tracker_creation() {
        let tracker = TokenTracker::new();
        let stats = tracker.get_global_stats().expect("Failed to get global stats");

        assert_eq!(stats.total_tokens_processed, 0);
        assert_eq!(stats.session_count, 0);
    }

    #[test]
    fn test_session_tracking() {
        let tracker = TokenTracker::new();

        tracker.start_session("test_session").expect("Failed to start session");

        let usage = tracker.get_session_usage("test_session")
            .expect("Failed to get session usage")
            .expect("Session not found");

        assert_eq!(usage.session_id, "test_session");
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_recursive_call_tracking() {
        let tracker = TokenTracker::new();
        tracker.start_session("test_session").expect("Failed to start session");

        let token_usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            recursive_calls: 1,
        };

        tracker.record_recursive_call(
            "test_session",
            "call_1",
            1,
            None,
            &token_usage,
        ).expect("Failed to record recursive call");

        let usage = tracker.get_session_usage("test_session")
            .expect("Failed to get session usage")
            .expect("Session not found");

        assert_eq!(usage.total_tokens, 150);
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.recursive_call_tokens.len(), 1);
    }

    #[test]
    fn test_cost_calculation() {
        let tracker = TokenTracker::new();
        tracker.start_session("test_session").expect("Failed to start session");

        let token_usage = TokenUsage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
            recursive_calls: 1,
        };

        tracker.record_recursive_call(
            "test_session",
            "call_1",
            1,
            None,
            &token_usage,
        ).expect("Failed to record recursive call");

        let pricing = TokenPricing::default();
        let cost = tracker.calculate_session_cost("test_session", &pricing)
            .expect("Failed to calculate cost");

        // With default pricing: 1000 * 0.00003 + 500 * 0.00006 = 0.03 + 0.03 = 0.06
        assert!((cost - 0.06).abs() < 0.001);
    }

    #[test]
    fn test_call_completion() {
        let tracker = TokenTracker::new();
        tracker.start_session("test_session").expect("Failed to start session");

        let token_usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            recursive_calls: 1,
        };

        tracker.record_recursive_call(
            "test_session",
            "call_1",
            1,
            None,
            &token_usage,
        ).expect("Failed to record recursive call");

        tracker.complete_recursive_call("test_session", "call_1", true)
            .expect("Failed to complete call");

        let usage = tracker.get_session_usage("test_session")
            .expect("Failed to get session usage")
            .expect("Session not found");

        let call_tokens = usage.recursive_call_tokens.get("call_1").unwrap();
        assert_eq!(call_tokens.status, CallTokenStatus::Completed);
        assert!(call_tokens.completed_at.is_some());
    }

    #[test]
    fn test_global_stats_update() {
        let tracker = TokenTracker::new();
        tracker.start_session("test_session").expect("Failed to start session");

        let token_usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
            recursive_calls: 1,
        };

        tracker.record_recursive_call(
            "test_session",
            "call_1",
            2,
            None,
            &token_usage,
        ).expect("Failed to record recursive call");

        let stats = tracker.get_global_stats().expect("Failed to get global stats");

        assert_eq!(stats.total_tokens_processed, 150);
        assert_eq!(stats.session_count, 1);
        assert_eq!(stats.recursive_call_count, 1);
        assert_eq!(stats.depth_distribution.get(&2), Some(&1));
        assert!(stats.avg_tokens_per_session > 0.0);
    }
}