//! Custom filtering for the logging system.
//!
//! This module provides advanced log filtering capabilities to control
//! which log events are recorded based on various criteria.

use std::marker::PhantomData;
use tracing::Subscriber;
use tracing_subscriber::Layer;

/// Filter that enables sampling of high-volume logs.
pub struct SamplingFilter<S> {
    // Sampling rate (1 in N)
    rate: u32,
    // Counter for tracking events
    counter: std::sync::atomic::AtomicU32,
    // Phantom data for the subscriber type
    _subscriber: PhantomData<S>,
}

impl<S> SamplingFilter<S> {
    /// Create a new sampling filter with the given rate.
    ///
    /// # Arguments
    ///
    /// * `rate` - Sample 1 in every `rate` events (e.g., rate=100 means log 1% of events)
    #[allow(dead_code)]
    pub fn new(rate: u32) -> Self {
        SamplingFilter {
            rate,
            counter: std::sync::atomic::AtomicU32::new(0),
            _subscriber: PhantomData,
        }
    }
}

impl<S> Layer<S> for SamplingFilter<S>
where
    S: Subscriber,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        // Always enable error and warn levels
        if metadata.level() <= &tracing::Level::WARN {
            return true;
        }

        // Sample other events based on rate
        let counter = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        counter % self.rate == 0
    }
}

#[cfg(feature = "dynamic-logging")]
/// Filter that dynamically adjusts log levels based on modules/targets.
pub struct DynamicTargetFilter {
    filters: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, tracing::Level>>>,
    default_level: tracing::Level,
}

#[cfg(feature = "dynamic-logging")]
impl DynamicTargetFilter {
    /// Create a new dynamic target filter with the given default level.
    #[allow(dead_code)]
    pub fn new(default_level: tracing::Level) -> Self {
        DynamicTargetFilter {
            filters: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            default_level,
        }
    }

    /// Set the log level for a specific target.
    #[allow(dead_code)]
    pub fn set_target_level(&self, target: &str, level: tracing::Level) {
        if let Ok(mut filters) = self.filters.write() {
            filters.insert(target.to_string(), level);
        }
    }

    /// Remove a target-specific filter.
    #[allow(dead_code)]
    pub fn clear_target_level(&self, target: &str) {
        if let Ok(mut filters) = self.filters.write() {
            filters.remove(target);
        }
    }

    /// Set the default log level.
    #[allow(dead_code)]
    pub fn set_default_level(&mut self, level: tracing::Level) {
        self.default_level = level;
    }
}

#[cfg(feature = "dynamic-logging")]
impl<S> Layer<S> for DynamicTargetFilter
where
    S: Subscriber,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        // Get the log level for this target, or use default
        let target = metadata.target();
        let level = if let Ok(filters) = self.filters.read() {
            *filters.get(target).unwrap_or(&self.default_level)
        } else {
            self.default_level
        };

        // Check if this event's level meets the threshold
        metadata.level() <= &level
    }
}
