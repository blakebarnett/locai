//! Hook registry for managing memory operation hooks
//!
//! This module provides the `HookRegistry` for registering and executing hooks
//! in response to memory lifecycle events. The registry handles:
//! - Hook registration and unregistration
//! - Priority-based hook ordering
//! - Timeout enforcement for individual hooks
//! - Safe failure handling (failed hooks don't stop operations)

use crate::models::Memory;
use super::traits::{MemoryHook, HookResult};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Entry in the hook registry
#[derive(Debug)]
struct HookEntry {
    /// The hook instance
    hook: Arc<dyn MemoryHook>,
    /// Priority for execution order
    priority: i32,
}

impl HookEntry {
    /// Create a new hook entry
    fn new(hook: Arc<dyn MemoryHook>, priority: i32) -> Self {
        Self { hook, priority }
    }
}

/// Registry for managing memory operation hooks
///
/// The registry maintains a list of hooks and executes them in response to
/// memory lifecycle events. Hooks are executed in priority order (higher priority first).
///
/// # Thread Safety
///
/// This registry is thread-safe and can be safely shared across async tasks.
/// All operations use async-safe locks.
///
/// # Error Handling
///
/// - Hook failures are logged but don't fail memory operations
/// - Only `before_memory_deleted` hooks can veto (prevent) operations
/// - Hooks that timeout are logged but don't fail operations
#[derive(Debug, Clone)]
pub struct HookRegistry {
    /// Vector of registered hooks, kept sorted by priority
    hooks: Arc<RwLock<Vec<HookEntry>>>,
}

impl HookRegistry {
    /// Create a new empty hook registry
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new hook
    ///
    /// The hook will be added to the registry and will fire on subsequent
    /// memory operations. If multiple hooks are registered, they will execute
    /// in priority order (highest priority first).
    ///
    /// # Arguments
    /// * `hook` - The hook to register
    pub async fn register(&self, hook: Arc<dyn MemoryHook>) {
        let priority = hook.priority();
        let name = hook.name().to_string();
        let mut hooks = self.hooks.write().await;
        
        hooks.push(HookEntry::new(hook, priority));
        
        // Keep hooks sorted by priority (highest first)
        hooks.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.hook.name().cmp(b.hook.name()))
        });
        
        debug!("Hook registered: {} (priority: {})", name, priority);
    }

    /// Execute the `on_memory_created` hook for all registered hooks
    ///
    /// Hooks are executed in priority order. If a hook fails or times out,
    /// it is logged but does not affect other hooks or the memory operation.
    ///
    /// # Arguments
    /// * `memory` - The newly created memory
    ///
    /// # Returns
    /// An error only if critical infrastructure fails; individual hook
    /// failures don't cause this to return an error
    pub async fn execute_on_created(&self, memory: &Memory) -> Result<(), String> {
        let hooks = self.hooks.read().await;
        
        for entry in hooks.iter() {
            let hook = entry.hook.clone();
            let timeout_ms = hook.timeout_ms();
            let name = hook.name();
            
            let future = async {
                hook.on_memory_created(memory).await
            };
            
            match tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                future
            ).await {
                Ok(HookResult::Continue) => {
                    debug!("Hook '{}' completed successfully", name);
                }
                Ok(HookResult::Veto(reason)) => {
                    // For "after" events, veto doesn't prevent the operation
                    debug!("Hook '{}' returned veto (ignored): {}", name, reason);
                }
                Err(_) => {
                    warn!(
                        "Hook '{}' timed out after {}ms",
                        name, timeout_ms
                    );
                }
            }
        }
        
        Ok(())
    }

    /// Execute the `on_memory_accessed` hook for all registered hooks
    ///
    /// # Arguments
    /// * `memory` - The accessed memory
    pub async fn execute_on_accessed(&self, memory: &Memory) -> Result<(), String> {
        let hooks = self.hooks.read().await;
        
        for entry in hooks.iter() {
            let hook = entry.hook.clone();
            let timeout_ms = hook.timeout_ms();
            let name = hook.name();
            
            let future = async {
                hook.on_memory_accessed(memory).await
            };
            
            match tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                future
            ).await {
                Ok(HookResult::Continue) => {
                    debug!("Hook '{}' completed successfully", name);
                }
                Ok(HookResult::Veto(reason)) => {
                    // For "after" events, veto doesn't prevent the operation
                    debug!("Hook '{}' returned veto (ignored): {}", name, reason);
                }
                Err(_) => {
                    warn!(
                        "Hook '{}' timed out after {}ms",
                        name, timeout_ms
                    );
                }
            }
        }
        
        Ok(())
    }

    /// Execute the `on_memory_updated` hook for all registered hooks
    ///
    /// # Arguments
    /// * `old` - The memory before the update
    /// * `new` - The memory after the update
    pub async fn execute_on_updated(&self, old: &Memory, new: &Memory) -> Result<(), String> {
        let hooks = self.hooks.read().await;
        
        for entry in hooks.iter() {
            let hook = entry.hook.clone();
            let timeout_ms = hook.timeout_ms();
            let name = hook.name();
            
            let future = async {
                hook.on_memory_updated(old, new).await
            };
            
            match tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                future
            ).await {
                Ok(result) => {
                    if let HookResult::Continue = result {
                        debug!("Hook '{}' completed successfully", name);
                    } else {
                        // Non-critical veto on update (doesn't prevent update)
                        debug!("Hook '{}' returned non-Continue result", name);
                    }
                }
                Err(_) => {
                    warn!(
                        "Hook '{}' timed out after {}ms",
                        name, timeout_ms
                    );
                }
            }
        }
        
        Ok(())
    }

    /// Execute the `before_memory_deleted` hook for all registered hooks
    ///
    /// This hook can veto deletion. If any hook returns `HookResult::Veto`,
    /// the deletion will be prevented.
    ///
    /// # Arguments
    /// * `memory` - The memory about to be deleted
    ///
    /// # Returns
    /// `Ok(true)` if deletion should proceed, `Ok(false)` if deletion is vetoed
    pub async fn execute_before_deleted(&self, memory: &Memory) -> Result<bool, String> {
        let hooks = self.hooks.read().await;
        
        for entry in hooks.iter() {
            let hook = entry.hook.clone();
            let timeout_ms = hook.timeout_ms();
            let name = hook.name();
            
            let future = async {
                hook.before_memory_deleted(memory).await
            };
            
            match tokio::time::timeout(
                std::time::Duration::from_millis(timeout_ms),
                future
            ).await {
                Ok(HookResult::Continue) => {
                    debug!("Hook '{}' allowed deletion", name);
                }
                Ok(HookResult::Veto(reason)) => {
                    warn!("Hook '{}' vetoed deletion: {}", name, reason);
                    return Ok(false);
                }
                Err(_) => {
                    warn!(
                        "Hook '{}' timed out after {}ms during deletion check",
                        name, timeout_ms
                    );
                    // Don't fail on timeout - allow deletion to proceed
                    debug!("Proceeding with deletion after hook timeout");
                }
            }
        }
        
        Ok(true)
    }

    /// Get the number of registered hooks
    pub async fn hook_count(&self) -> usize {
        self.hooks.read().await.len()
    }

    /// Clear all registered hooks
    pub async fn clear(&self) {
        self.hooks.write().await.clear();
        debug!("All hooks cleared from registry");
    }

    /// Get a list of hook names and priorities (for debugging)
    pub async fn list_hooks(&self) -> Vec<(String, i32)> {
        self.hooks
            .read()
            .await
            .iter()
            .map(|entry| (entry.hook.name().to_string(), entry.priority))
            .collect()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering::SeqCst};

    #[derive(Debug)]
    struct TestHook {
        call_count: Arc<AtomicU32>,
        priority: i32,
    }

    #[async_trait]
    impl MemoryHook for TestHook {
        async fn on_memory_created(&self, _memory: &Memory) -> HookResult {
            self.call_count.fetch_add(1, SeqCst);
            HookResult::Continue
        }

        fn priority(&self) -> i32 {
            self.priority
        }

        fn name(&self) -> &str {
            "test_hook"
        }
    }

    #[tokio::test]
    async fn test_hook_registration() {
        let registry = HookRegistry::new();
        assert_eq!(registry.hook_count().await, 0);

        let hook = Arc::new(TestHook {
            call_count: Arc::new(AtomicU32::new(0)),
            priority: 0,
        });

        registry.register(hook).await;
        assert_eq!(registry.hook_count().await, 1);
    }

    #[tokio::test]
    async fn test_hook_priority_ordering() {
        let registry = HookRegistry::new();

        let hook1 = Arc::new(TestHook {
            call_count: Arc::new(AtomicU32::new(0)),
            priority: 5,
        });

        let hook2 = Arc::new(TestHook {
            call_count: Arc::new(AtomicU32::new(0)),
            priority: 10,
        });

        registry.register(hook1).await;
        registry.register(hook2).await;

        let hooks = registry.list_hooks().await;
        assert_eq!(hooks[0].1, 10); // Highest priority first
        assert_eq!(hooks[1].1, 5);
    }

    #[tokio::test]
    async fn test_hook_execution() {
        let registry = HookRegistry::new();
        let call_count = Arc::new(AtomicU32::new(0));

        let hook = Arc::new(TestHook {
            call_count: call_count.clone(),
            priority: 0,
        });

        registry.register(hook).await;

        let memory = Memory::new(
            "test_id".to_string(),
            "test content".to_string(),
            crate::models::MemoryType::Fact,
        );

        registry.execute_on_created(&memory).await.ok();
        assert_eq!(call_count.load(SeqCst), 1);
    }

    #[tokio::test]
    async fn test_clear_hooks() {
        let registry = HookRegistry::new();

        let hook = Arc::new(TestHook {
            call_count: Arc::new(AtomicU32::new(0)),
            priority: 0,
        });

        registry.register(hook).await;
        assert_eq!(registry.hook_count().await, 1);

        registry.clear().await;
        assert_eq!(registry.hook_count().await, 0);
    }
}
