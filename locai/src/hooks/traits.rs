//! Traits for memory operation hooks.
//!
//! This module provides trait definitions for implementing hooks that respond to
//! memory lifecycle events. Hooks allow you to run custom logic when memories are
//! created, accessed, updated, or deleted.
//!
//! # Examples
//!
//! ```no_run
//! use async_trait::async_trait;
//! use locai::hooks::{MemoryHook, HookResult};
//! use locai::models::Memory;
//!
//! #[derive(Debug)]
//! struct LoggingHook;
//!
//! #[async_trait]
//! impl MemoryHook for LoggingHook {
//!     async fn on_memory_created(&self, memory: &Memory) -> HookResult {
//!         println!("Memory created: {}", memory.id);
//!         HookResult::Continue
//!     }
//! }
//! ```

use crate::models::Memory;
use async_trait::async_trait;

/// Result type for hook execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookResult {
    /// Continue with the memory operation
    Continue,
    /// Veto the operation (only valid for `before_memory_deleted`)
    /// Contains a reason for the veto
    Veto(String),
}

impl Default for HookResult {
    fn default() -> Self {
        HookResult::Continue
    }
}

/// Trait for memory operation hooks
///
/// Implement this trait to respond to memory lifecycle events. Each method has a default
/// implementation that does nothing and returns `HookResult::Continue`.
///
/// # Hook Execution
///
/// - Hooks are executed in priority order (higher priority first)
/// - Multiple hooks can be registered for the same event
/// - Hook failures are logged but don't fail the memory operation (unless they veto)
/// - Each hook has a configurable timeout (default: 5000ms)
#[async_trait]
pub trait MemoryHook: Send + Sync + std::fmt::Debug {
    /// Called after a memory is successfully created
    ///
    /// This hook is always called asynchronously and non-blocking. The memory
    /// has already been persisted when this hook is called.
    ///
    /// # Arguments
    /// * `memory` - The newly created memory
    ///
    /// # Returns
    /// `HookResult::Continue` on success, or `HookResult::Veto` to reject (though
    /// rejection doesn't prevent creation since the memory is already persisted)
    async fn on_memory_created(&self, _memory: &Memory) -> HookResult {
        HookResult::Continue
    }

    /// Called after a memory is successfully accessed (read)
    ///
    /// This hook fires when `get_memory()` is called. Depending on configuration,
    /// the `access_count` and `last_accessed` fields will be updated before this
    /// hook is called.
    ///
    /// # Arguments
    /// * `memory` - The accessed memory
    ///
    /// # Returns
    /// `HookResult::Continue` to proceed
    async fn on_memory_accessed(&self, _memory: &Memory) -> HookResult {
        HookResult::Continue
    }

    /// Called after a memory is successfully updated
    ///
    /// This hook fires when `update_memory()` is called.
    ///
    /// # Arguments
    /// * `old` - The memory before the update
    /// * `new` - The memory after the update
    ///
    /// # Returns
    /// `HookResult::Continue` to proceed
    async fn on_memory_updated(&self, old: &Memory, new: &Memory) -> HookResult {
        let _ = (old, new); // Silence unused variable warnings in default implementation
        HookResult::Continue
    }

    /// Called before a memory is deleted (can veto)
    ///
    /// This hook is called *before* deletion, giving hooks the ability to prevent
    /// deletion by returning `HookResult::Veto`.
    ///
    /// # Arguments
    /// * `memory` - The memory about to be deleted
    ///
    /// # Returns
    /// `HookResult::Continue` to allow deletion, or `HookResult::Veto(reason)` to prevent it
    async fn before_memory_deleted(&self, memory: &Memory) -> HookResult {
        let _ = memory; // Silence unused variable warnings in default implementation
        HookResult::Continue
    }

    /// Get the priority of this hook (higher = runs first)
    ///
    /// Hooks with higher priority values execute before hooks with lower priority values.
    /// Default priority is 0.
    ///
    /// # Returns
    /// Priority value (i32, no bounds). Higher values indicate higher priority.
    fn priority(&self) -> i32 {
        0
    }

    /// Get the timeout in milliseconds for this hook
    ///
    /// If a hook takes longer than this duration, it will be cancelled and the
    /// operation will continue (the hook failure is logged but doesn't fail the operation).
    /// Default timeout is 5000ms (5 seconds).
    ///
    /// # Returns
    /// Timeout duration in milliseconds
    fn timeout_ms(&self) -> u64 {
        5000
    }

    /// Get a descriptive name for this hook (optional, for logging)
    ///
    /// Default implementation returns "anonymous_hook".
    ///
    /// # Returns
    /// A string identifying this hook for debugging purposes
    fn name(&self) -> &str {
        "anonymous_hook"
    }
}
