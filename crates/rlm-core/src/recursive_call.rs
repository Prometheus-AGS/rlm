//! Recursive call tree management for tracking hierarchical LLM invocations.

use crate::{CallStatus, RlmError, RlmResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Individual recursive call node in the call tree.
///
/// Represents a single LLM invocation within the recursive decomposition process,
/// tracking its relationship to parent/child calls and execution metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursiveCall {
    /// Unique identifier for this call.
    pub call_id: String,

    /// Optional parent call ID (None for root call).
    pub parent_id: Option<String>,

    /// Session ID this call belongs to.
    pub session_id: String,

    /// Recursion depth (0 for initial request).
    pub depth: u32,

    /// Prompt sent to the LLM for this call.
    pub prompt: String,

    /// Response received from the LLM (empty until completed).
    pub response: String,

    /// Token count for this specific call.
    pub tokens_used: u32,

    /// Processing duration for this call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Call initiation timestamp.
    #[serde(with = "crate::types::systemtime_serde")]
    pub created_at: SystemTime,

    /// Call completion timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<SystemTime>,

    /// Current call status.
    pub status: CallStatus,

    /// Optional error message if call failed.
    pub error: Option<String>,

    /// Model used for this call (may differ from session default).
    pub model: String,

    /// Temperature used for this call.
    pub temperature: f32,

    /// Maximum tokens for this call response.
    pub max_tokens: Option<u32>,

    /// Additional metadata for this call.
    pub metadata: HashMap<String, String>,
}

impl RecursiveCall {
    /// Create a new recursive call.
    pub fn new(
        session_id: String,
        parent_id: Option<String>,
        depth: u32,
        prompt: String,
        model: String,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Self {
        Self {
            call_id: Uuid::new_v4().to_string(),
            parent_id,
            session_id,
            depth,
            prompt,
            response: String::new(),
            tokens_used: 0,
            duration_ms: None,
            created_at: SystemTime::now(),
            completed_at: None,
            status: CallStatus::Pending,
            error: None,
            model,
            temperature,
            max_tokens,
            metadata: HashMap::new(),
        }
    }

    /// Complete the call with response and metrics.
    pub fn complete(&mut self, response: String, tokens_used: u32) {
        self.response = response;
        self.tokens_used = tokens_used;
        self.completed_at = Some(SystemTime::now());
        self.status = CallStatus::Completed;

        if let Ok(duration) = self.completed_at.unwrap().duration_since(self.created_at) {
            self.duration_ms = Some(duration.as_millis() as u64);
        }
    }

    /// Mark the call as failed with error message.
    pub fn mark_failed(&mut self, error: String) {
        self.error = Some(error);
        self.status = CallStatus::Failed;
        self.completed_at = Some(SystemTime::now());

        if let Ok(duration) = self.completed_at.unwrap().duration_since(self.created_at) {
            self.duration_ms = Some(duration.as_millis() as u64);
        }
    }

    /// Update call status.
    pub fn update_status(&mut self, status: CallStatus) {
        self.status = status;
    }

    /// Add metadata key-value pair.
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Check if call is terminal (completed or failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, CallStatus::Completed | CallStatus::Failed)
    }

    /// Get processing duration if call is complete.
    pub fn get_duration(&self) -> Option<Duration> {
        self.duration_ms.map(Duration::from_millis)
    }
}

/// Manages the tree structure of recursive calls for a session.
///
/// Provides tree operations like adding children, finding nodes,
/// traversal, and cycle detection for safe recursive decomposition.
#[derive(Debug, Default)]
pub struct RecursiveCallTree {
    /// All calls in the tree by call ID.
    calls: HashMap<String, RecursiveCall>,

    /// Root call ID (first call in the session).
    root_id: Option<String>,

    /// Maximum allowed tree depth.
    max_depth: u32,

    /// Maximum children per node to prevent fan-out explosion.
    max_children_per_node: u32,
}

impl RecursiveCallTree {
    /// Create a new call tree with safety limits.
    pub fn new(max_depth: u32, max_children_per_node: u32) -> Self {
        Self {
            calls: HashMap::new(),
            root_id: None,
            max_depth,
            max_children_per_node,
        }
    }

    /// Add a new call to the tree.
    pub fn add_call(&mut self, call: RecursiveCall) -> RlmResult<String> {
        let call_id = call.call_id.clone();

        // Validate depth limit
        if call.depth > self.max_depth {
            return Err(RlmError::MaxRecursion(self.max_depth));
        }

        // Validate parent exists if specified
        if let Some(parent_id) = &call.parent_id {
            if !self.calls.contains_key(parent_id) {
                return Err(RlmError::Other(format!("Parent call not found: {}", parent_id)));
            }

            // Check children limit for parent
            let child_count = self.get_children(parent_id).len();
            if child_count >= self.max_children_per_node as usize {
                return Err(RlmError::Other(format!(
                    "Maximum children limit ({}) exceeded for parent: {}",
                    self.max_children_per_node, parent_id
                )));
            }
        }

        // Detect cycles by checking if any ancestor has the same prompt
        if let Err(cycle_error) = self.detect_cycle(&call) {
            return Err(cycle_error);
        }

        // Set as root if first call
        if self.root_id.is_none() {
            self.root_id = Some(call_id.clone());
        }

        self.calls.insert(call_id.clone(), call);
        Ok(call_id)
    }

    /// Get a call by ID.
    pub fn get_call(&self, call_id: &str) -> Option<&RecursiveCall> {
        self.calls.get(call_id)
    }

    /// Get a mutable call by ID.
    pub fn get_call_mut(&mut self, call_id: &str) -> Option<&mut RecursiveCall> {
        self.calls.get_mut(call_id)
    }

    /// Get all child calls for a given parent.
    pub fn get_children(&self, parent_id: &str) -> Vec<&RecursiveCall> {
        self.calls
            .values()
            .filter(|call| call.parent_id.as_deref() == Some(parent_id))
            .collect()
    }

    /// Get the root call of the tree.
    pub fn get_root(&self) -> Option<&RecursiveCall> {
        self.root_id.as_ref().and_then(|id| self.calls.get(id))
    }

    /// Get all calls at a specific depth level.
    pub fn get_calls_at_depth(&self, depth: u32) -> Vec<&RecursiveCall> {
        self.calls
            .values()
            .filter(|call| call.depth == depth)
            .collect()
    }

    /// Get total number of calls in the tree.
    pub fn call_count(&self) -> usize {
        self.calls.len()
    }

    /// Get maximum depth reached in the tree.
    pub fn max_depth_reached(&self) -> u32 {
        self.calls.values().map(|call| call.depth).max().unwrap_or(0)
    }

    /// Get total tokens used across all calls.
    pub fn total_tokens_used(&self) -> u32 {
        self.calls.values().map(|call| call.tokens_used).sum()
    }

    /// Get calls by status.
    pub fn get_calls_by_status(&self, status: CallStatus) -> Vec<&RecursiveCall> {
        self.calls
            .values()
            .filter(|call| call.status == status)
            .collect()
    }

    /// Traverse the tree depth-first and apply a function to each node.
    pub fn depth_first_traversal<F>(&self, mut visitor: F)
    where
        F: FnMut(&RecursiveCall),
    {
        if let Some(root) = self.get_root() {
            self.dfs_visit(root, &mut visitor);
        }
    }

    /// Traverse the tree breadth-first and apply a function to each node.
    pub fn breadth_first_traversal<F>(&self, mut visitor: F)
    where
        F: FnMut(&RecursiveCall),
    {
        if let Some(root) = self.get_root() {
            let mut queue = vec![root];

            while let Some(call) = queue.pop() {
                visitor(call);

                let mut children = self.get_children(&call.call_id);
                children.sort_by_key(|c| c.created_at); // Process children in creation order
                queue.extend(children.into_iter().rev()); // Reverse for queue order
            }
        }
    }

    /// Check if all calls in the tree are terminal.
    pub fn is_complete(&self) -> bool {
        !self.calls.is_empty() && self.calls.values().all(|call| call.is_terminal())
    }

    /// Get tree statistics for monitoring.
    pub fn get_statistics(&self) -> TreeStatistics {
        let total_calls = self.call_count();
        let completed_calls = self.get_calls_by_status(CallStatus::Completed).len();
        let failed_calls = self.get_calls_by_status(CallStatus::Failed).len();
        let pending_calls = self.get_calls_by_status(CallStatus::Pending).len();
        let in_progress_calls = self.get_calls_by_status(CallStatus::InProgress).len();

        let total_tokens = self.total_tokens_used();
        let max_depth = self.max_depth_reached();

        let avg_tokens_per_call = if total_calls > 0 {
            total_tokens as f64 / total_calls as f64
        } else {
            0.0
        };

        TreeStatistics {
            total_calls,
            completed_calls,
            failed_calls,
            pending_calls,
            in_progress_calls,
            total_tokens,
            avg_tokens_per_call,
            max_depth,
        }
    }

    /// Detect potential cycles by checking prompt similarity in ancestor chain.
    fn detect_cycle(&self, new_call: &RecursiveCall) -> RlmResult<()> {
        if let Some(parent_id) = &new_call.parent_id {
            let mut current_id = Some(parent_id.as_str());

            while let Some(id) = current_id {
                if let Some(ancestor) = self.calls.get(id) {
                    // Simple cycle detection: same prompt indicates potential cycle
                    if ancestor.prompt == new_call.prompt {
                        return Err(RlmError::Other(format!(
                            "Cycle detected: call with identical prompt found in ancestor chain. Call: {}, Ancestor: {}",
                            new_call.call_id, ancestor.call_id
                        )));
                    }
                    current_id = ancestor.parent_id.as_deref();
                } else {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Depth-first traversal helper.
    fn dfs_visit<F>(&self, call: &RecursiveCall, visitor: &mut F)
    where
        F: FnMut(&RecursiveCall),
    {
        visitor(call);

        let mut children = self.get_children(&call.call_id);
        children.sort_by_key(|c| c.created_at); // Process children in creation order

        for child in children {
            self.dfs_visit(child, visitor);
        }
    }
}

/// Statistics about a recursive call tree.
#[derive(Debug, Clone)]
pub struct TreeStatistics {
    /// Total number of calls in the tree.
    pub total_calls: usize,
    /// Number of completed calls.
    pub completed_calls: usize,
    /// Number of failed calls.
    pub failed_calls: usize,
    /// Number of pending calls.
    pub pending_calls: usize,
    /// Number of calls in progress.
    pub in_progress_calls: usize,
    /// Total tokens used across all calls.
    pub total_tokens: u32,
    /// Average tokens per call.
    pub avg_tokens_per_call: f64,
    /// Maximum recursion depth reached.
    pub max_depth: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_call(session_id: &str, parent_id: Option<String>, depth: u32, prompt: &str) -> RecursiveCall {
        RecursiveCall::new(
            session_id.to_string(),
            parent_id,
            depth,
            prompt.to_string(),
            "gpt-4".to_string(),
            0.7,
            Some(1000),
        )
    }

    #[test]
    fn test_recursive_call_creation() {
        let call = create_test_call("session-1", None, 0, "What is the capital of France?");

        assert_eq!(call.session_id, "session-1");
        assert_eq!(call.parent_id, None);
        assert_eq!(call.depth, 0);
        assert_eq!(call.prompt, "What is the capital of France?");
        assert_eq!(call.status, CallStatus::Pending);
        assert!(!call.is_terminal());
    }

    #[test]
    fn test_call_completion() {
        let mut call = create_test_call("session-1", None, 0, "Test prompt");

        call.complete("Paris is the capital of France.".to_string(), 50);

        assert_eq!(call.response, "Paris is the capital of France.");
        assert_eq!(call.tokens_used, 50);
        assert_eq!(call.status, CallStatus::Completed);
        assert!(call.is_terminal());
        assert!(call.completed_at.is_some());
        assert!(call.duration_ms.is_some());
    }

    #[test]
    fn test_call_failure() {
        let mut call = create_test_call("session-1", None, 0, "Test prompt");

        call.mark_failed("API timeout".to_string());

        assert_eq!(call.error, Some("API timeout".to_string()));
        assert_eq!(call.status, CallStatus::Failed);
        assert!(call.is_terminal());
        assert!(call.completed_at.is_some());
    }

    #[test]
    fn test_call_tree_creation() {
        let tree = RecursiveCallTree::new(10, 5);

        assert_eq!(tree.call_count(), 0);
        assert!(tree.get_root().is_none());
        assert_eq!(tree.max_depth_reached(), 0);
    }

    #[test]
    fn test_add_root_call() {
        let mut tree = RecursiveCallTree::new(10, 5);
        let call = create_test_call("session-1", None, 0, "Root prompt");

        let call_id = tree.add_call(call).unwrap();

        assert_eq!(tree.call_count(), 1);
        assert!(tree.get_root().is_some());
        assert_eq!(tree.get_root().unwrap().call_id, call_id);
    }

    #[test]
    fn test_add_child_call() {
        let mut tree = RecursiveCallTree::new(10, 5);

        let root_call = create_test_call("session-1", None, 0, "Root prompt");
        let root_id = tree.add_call(root_call).unwrap();

        let child_call = create_test_call("session-1", Some(root_id.clone()), 1, "Child prompt");
        let child_id = tree.add_call(child_call).unwrap();

        assert_eq!(tree.call_count(), 2);

        let children = tree.get_children(&root_id);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].call_id, child_id);
    }

    #[test]
    fn test_depth_limit() {
        let mut tree = RecursiveCallTree::new(2, 5); // Max depth 2

        let root_call = create_test_call("session-1", None, 0, "Root");
        let root_id = tree.add_call(root_call).unwrap();

        let child1 = create_test_call("session-1", Some(root_id.clone()), 1, "Child 1");
        let child1_id = tree.add_call(child1).unwrap();

        let child2 = create_test_call("session-1", Some(child1_id), 2, "Child 2");
        let child2_id = tree.add_call(child2).unwrap();

        // This should fail due to depth limit
        let child3 = create_test_call("session-1", Some(child2_id), 3, "Child 3");
        assert!(tree.add_call(child3).is_err());
    }

    #[test]
    fn test_children_limit() {
        let mut tree = RecursiveCallTree::new(10, 2); // Max 2 children per node

        let root_call = create_test_call("session-1", None, 0, "Root");
        let root_id = tree.add_call(root_call).unwrap();

        // Add 2 children (should succeed)
        let child1 = create_test_call("session-1", Some(root_id.clone()), 1, "Child 1");
        tree.add_call(child1).unwrap();

        let child2 = create_test_call("session-1", Some(root_id.clone()), 1, "Child 2");
        tree.add_call(child2).unwrap();

        // Third child should fail
        let child3 = create_test_call("session-1", Some(root_id), 1, "Child 3");
        assert!(tree.add_call(child3).is_err());
    }

    #[test]
    fn test_cycle_detection() {
        let mut tree = RecursiveCallTree::new(10, 5);

        let root_call = create_test_call("session-1", None, 0, "Analyze document");
        let root_id = tree.add_call(root_call).unwrap();

        let child_call = create_test_call("session-1", Some(root_id.clone()), 1, "Summarize section");
        let child_id = tree.add_call(child_call).unwrap();

        // Try to add a grandchild with same prompt as root (cycle)
        let grandchild = create_test_call("session-1", Some(child_id), 2, "Analyze document");
        assert!(tree.add_call(grandchild).is_err());
    }

    #[test]
    fn test_tree_traversal() {
        let mut tree = RecursiveCallTree::new(10, 5);

        let root_call = create_test_call("session-1", None, 0, "Root");
        let root_id = tree.add_call(root_call).unwrap();

        let child1 = create_test_call("session-1", Some(root_id.clone()), 1, "Child 1");
        tree.add_call(child1).unwrap();

        let child2 = create_test_call("session-1", Some(root_id), 1, "Child 2");
        tree.add_call(child2).unwrap();

        // Test depth-first traversal
        let mut dfs_calls = Vec::new();
        tree.depth_first_traversal(|call| {
            dfs_calls.push(call.prompt.clone());
        });

        assert_eq!(dfs_calls.len(), 3);
        assert_eq!(dfs_calls[0], "Root");

        // Test breadth-first traversal
        let mut bfs_calls = Vec::new();
        tree.breadth_first_traversal(|call| {
            bfs_calls.push(call.prompt.clone());
        });

        assert_eq!(bfs_calls.len(), 3);
        assert_eq!(bfs_calls[0], "Root");
    }

    #[test]
    fn test_tree_statistics() {
        let mut tree = RecursiveCallTree::new(10, 5);

        let mut root_call = create_test_call("session-1", None, 0, "Root");
        root_call.complete("Root response".to_string(), 100);
        tree.add_call(root_call).unwrap();

        let mut child_call = create_test_call("session-1", Some(tree.get_root().unwrap().call_id.clone()), 1, "Child");
        child_call.mark_failed("Error occurred".to_string());
        tree.add_call(child_call).unwrap();

        let stats = tree.get_statistics();

        assert_eq!(stats.total_calls, 2);
        assert_eq!(stats.completed_calls, 1);
        assert_eq!(stats.failed_calls, 1);
        assert_eq!(stats.pending_calls, 0);
        assert_eq!(stats.total_tokens, 100);
        assert_eq!(stats.max_depth, 1);
    }
}