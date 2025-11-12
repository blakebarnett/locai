//! Lifecycle tracking and batched update management.
//!
//! This module provides infrastructure for tracking and batching memory lifecycle updates.
//! Instead of updating every memory access immediately, updates can be aggregated and
//! flushed in batches to reduce database load.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents a pending lifecycle update for a single memory.
#[derive(Debug, Clone)]
pub struct LifecycleUpdate {
    /// The memory ID to update
    pub memory_id: String,

    /// New access count (additive)
    pub access_count_delta: u32,

    /// Most recent access time
    pub last_accessed: DateTime<Utc>,

    /// When this update was first queued
    pub queued_at: DateTime<Utc>,
}

impl LifecycleUpdate {
    /// Create a new lifecycle update
    pub fn new(memory_id: String) -> Self {
        let now = Utc::now();
        Self {
            memory_id,
            access_count_delta: 1,
            last_accessed: now,
            queued_at: now,
        }
    }

    /// Merge another update into this one (combines deltas)
    pub fn merge(&mut self, other: LifecycleUpdate) {
        self.access_count_delta = self
            .access_count_delta
            .saturating_add(other.access_count_delta);
        self.last_accessed = other.last_accessed; // Use the most recent timestamp
    }
}

/// A queue for batching lifecycle updates.
///
/// This queue aggregates multiple lifecycle updates and flushes them in batches,
/// reducing database write load. Updates for the same memory are automatically merged.
#[derive(Debug, Clone)]
pub struct LifecycleUpdateQueue {
    /// Pending updates, keyed by memory ID
    pending_updates: Arc<Mutex<HashMap<String, LifecycleUpdate>>>,

    /// Maximum size of a single batch before forcing a flush
    max_batch_size: usize,
}

impl LifecycleUpdateQueue {
    /// Create a new lifecycle update queue
    pub fn new(max_batch_size: usize) -> Self {
        Self {
            pending_updates: Arc::new(Mutex::new(HashMap::new())),
            max_batch_size,
        }
    }

    /// Queue an update, merging with existing updates for the same memory
    pub async fn queue_update(&self, update: LifecycleUpdate) -> Result<(), String> {
        let mut updates = self.pending_updates.lock().await;

        // Check if we're about to exceed max batch size
        if updates.len() >= self.max_batch_size {
            return Err(format!(
                "Lifecycle update queue full: {} updates pending",
                updates.len()
            ));
        }

        // Merge with existing update or insert new one
        if let Some(existing) = updates.get_mut(&update.memory_id) {
            existing.merge(update);
        } else {
            updates.insert(update.memory_id.clone(), update);
        }

        Ok(())
    }

    /// Get the current pending updates without removing them
    pub async fn peek(&self) -> Vec<LifecycleUpdate> {
        let updates = self.pending_updates.lock().await;
        updates.values().cloned().collect()
    }

    /// Get and clear all pending updates (for flushing)
    pub async fn drain(&self) -> Vec<LifecycleUpdate> {
        let mut updates = self.pending_updates.lock().await;
        updates.drain().map(|(_, v)| v).collect()
    }

    /// Get the number of pending updates
    pub async fn len(&self) -> usize {
        let updates = self.pending_updates.lock().await;
        updates.len()
    }

    /// Check if the queue is empty
    pub async fn is_empty(&self) -> bool {
        let updates = self.pending_updates.lock().await;
        updates.is_empty()
    }

    /// Check if the queue should be flushed based on size
    pub async fn should_flush_by_size(&self) -> bool {
        let updates = self.pending_updates.lock().await;
        updates.len() > self.max_batch_size / 2 // Flush at 50% capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lifecycle_update_new() {
        let update = LifecycleUpdate::new("memory_123".to_string());
        assert_eq!(update.memory_id, "memory_123");
        assert_eq!(update.access_count_delta, 1);
    }

    #[tokio::test]
    async fn test_lifecycle_update_merge() {
        let mut update1 = LifecycleUpdate::new("memory_123".to_string());
        let update2 = LifecycleUpdate::new("memory_123".to_string());

        update1.merge(update2);
        assert_eq!(update1.access_count_delta, 2);
    }

    #[tokio::test]
    async fn test_queue_new() {
        let queue = LifecycleUpdateQueue::new(100);
        assert!(queue.is_empty().await);
        assert_eq!(queue.len().await, 0);
    }

    #[tokio::test]
    async fn test_queue_add_update() {
        let queue = LifecycleUpdateQueue::new(100);
        let update = LifecycleUpdate::new("memory_123".to_string());

        queue.queue_update(update).await.unwrap();
        assert_eq!(queue.len().await, 1);
    }

    #[tokio::test]
    async fn test_queue_merge_updates() {
        let queue = LifecycleUpdateQueue::new(100);
        let update1 = LifecycleUpdate::new("memory_123".to_string());
        let update2 = LifecycleUpdate::new("memory_123".to_string());

        queue.queue_update(update1).await.unwrap();
        queue.queue_update(update2).await.unwrap();

        // Should have merged into a single entry
        assert_eq!(queue.len().await, 1);

        let updates = queue.peek().await;
        assert_eq!(updates[0].access_count_delta, 2);
    }

    #[tokio::test]
    async fn test_queue_drain() {
        let queue = LifecycleUpdateQueue::new(100);
        let update = LifecycleUpdate::new("memory_123".to_string());

        queue.queue_update(update).await.unwrap();
        assert_eq!(queue.len().await, 1);

        let drained = queue.drain().await;
        assert_eq!(drained.len(), 1);
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_queue_overflow() {
        let queue = LifecycleUpdateQueue::new(2);

        queue
            .queue_update(LifecycleUpdate::new("memory_1".to_string()))
            .await
            .unwrap();
        queue
            .queue_update(LifecycleUpdate::new("memory_2".to_string()))
            .await
            .unwrap();

        // Next update should fail
        let result = queue
            .queue_update(LifecycleUpdate::new("memory_3".to_string()))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_should_flush_by_size() {
        let queue = LifecycleUpdateQueue::new(100);

        // Add 50 updates (50% of capacity)
        for i in 0..50 {
            queue
                .queue_update(LifecycleUpdate::new(format!("memory_{}", i)))
                .await
                .unwrap();
        }

        // Should not flush at exactly 50%
        assert!(!queue.should_flush_by_size().await);

        // Add one more to exceed 50% threshold
        queue
            .queue_update(LifecycleUpdate::new("memory_50".to_string()))
            .await
            .unwrap();

        assert!(queue.should_flush_by_size().await);
    }
}
