//! Workflow cache

use std::collections::HashMap;
use std::sync::RwLock;

use crate::models::workflow::Workflow;

/// Workflow cache entry
#[derive(Debug, Clone)]
pub struct WorkflowCacheEntry {
    pub workflow: Workflow,
    pub digest: String,
    pub cached_at: u64,
}

/// In-memory workflow cache
pub struct WorkflowCache {
    entries: RwLock<HashMap<String, WorkflowCacheEntry>>,
    capacity: u64,
}

impl WorkflowCache {
    /// Create a new workflow cache
    pub fn new(capacity: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            capacity,
        }
    }

    /// Get a workflow from cache
    pub fn get(&self, workflow_id: &str) -> Option<WorkflowCacheEntry> {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.get(workflow_id).cloned()
    }

    /// Get a workflow by digest
    pub fn get_by_digest(&self, digest: &str) -> Option<WorkflowCacheEntry> {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.values().find(|e| e.digest == digest).cloned()
    }

    /// Insert a workflow into cache
    pub fn insert(&self, workflow: Workflow, digest: String) {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());

        // Evict oldest if at capacity
        if entries.len() as u64 >= self.capacity {
            if let Some(oldest_id) = entries
                .iter()
                .min_by_key(|(_, e)| e.cached_at)
                .map(|(id, _)| id.clone())
            {
                entries.remove(&oldest_id);
            }
        }

        let entry = WorkflowCacheEntry {
            workflow: workflow.clone(),
            digest,
            cached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        entries.insert(workflow.id.clone(), entry);
    }

    /// Remove a workflow from cache
    pub fn remove(&self, workflow_id: &str) -> Option<WorkflowCacheEntry> {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        entries.remove(workflow_id)
    }

    /// Clear the cache
    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        entries.clear();
    }

    /// Get all cached workflow IDs
    pub fn keys(&self) -> Vec<String> {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.keys().cloned().collect()
    }

    /// Get all cached digests
    pub fn digests(&self) -> Vec<(String, String)> {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries
            .iter()
            .map(|(id, e)| (id.clone(), e.digest.clone()))
            .collect()
    }

    /// Get cache size
    pub fn len(&self) -> usize {
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
        entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
