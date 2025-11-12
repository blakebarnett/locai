//! Memory operation hooks system
//!
//! This module provides a flexible hook/callback system for responding to memory lifecycle events.
//! Hooks allow applications to:
//! - React to memory creation, access, updates, and deletion
//! - Implement custom logic (e.g., entity promotion, consolidation, notifications)
//! - Veto deletion operations
//! - Track metrics and analytics
//!
//! # Architecture
//!
//! - `traits.rs`: Core `MemoryHook` trait and `HookResult` types
//! - `registry.rs`: `HookRegistry` for managing hook registration and execution
//! - `webhook.rs`: Webhook-based hook implementation for remote integrations
//!
//! # Examples
//!
//! See the examples directory for complete working examples of custom hooks.

pub mod registry;
pub mod traits;
pub mod webhook;

pub use registry::HookRegistry;
pub use traits::{HookResult, MemoryHook};
pub use webhook::Webhook;
