//! Webhook implementation for remote memory event notifications
//!
//! This module provides the `Webhook` implementation that sends HTTP requests
//! to remote endpoints when memory lifecycle events occur. It supports:
//! - POST and PUT methods
//! - Custom headers
//! - Exponential backoff retry logic
//! - Configurable timeouts
//! - Graceful error handling

use super::traits::{HookResult, MemoryHook};
use crate::models::Memory;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Retry policy for webhook requests
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff duration in milliseconds
    pub initial_backoff_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f32,
    /// Maximum backoff duration in milliseconds
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 10000,
        }
    }
}

impl RetryPolicy {
    /// Calculate backoff duration for a given attempt number
    fn backoff_duration(&self, attempt: u32) -> Duration {
        let backoff_ms =
            (self.initial_backoff_ms as f32 * self.backoff_multiplier.powi(attempt as i32)) as u64;
        let backoff_ms = backoff_ms.min(self.max_backoff_ms);
        Duration::from_millis(backoff_ms)
    }
}

/// Webhook for sending memory events to remote endpoints
///
/// This hook sends HTTP POST or PUT requests to a configured URL when memory
/// lifecycle events occur. It supports retry with exponential backoff and
/// custom headers.
///
/// # Example
///
/// ```no_run
/// use locai::hooks::Webhook;
///
/// let hook = Webhook::new("http://app-server:8000/api/memory-events".to_string())
///     .with_method("POST".to_string())
///     .with_header("Authorization".to_string(), "Bearer token123".to_string());
/// ```
#[derive(Debug, Clone)]
pub struct Webhook {
    /// The URL to POST/PUT events to
    pub url: String,
    /// HTTP method (POST or PUT)
    pub method: String,
    /// Custom headers to include in requests
    pub headers: HashMap<String, String>,
    /// Request timeout duration
    pub timeout: Duration,
    /// Retry policy for failed requests
    pub retry_policy: RetryPolicy,
}

impl Webhook {
    /// Create a new webhook hook with the given URL
    ///
    /// # Arguments
    /// * `url` - The endpoint URL to send webhooks to
    pub fn new(url: String) -> Self {
        Self {
            url,
            method: "POST".to_string(),
            headers: HashMap::new(),
            timeout: Duration::from_secs(10),
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Set the HTTP method (POST or PUT)
    pub fn with_method(mut self, method: String) -> Self {
        self.method = method;
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Send a webhook request with retry logic
    async fn send_with_retry(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let mut last_error: Option<String> = None;

        // Attempt the request with retries
        for attempt in 0..=self.retry_policy.max_retries {
            match self.send_request(&client, event_type, &payload).await {
                Ok(_) => {
                    debug!(
                        "Webhook request succeeded (event: {}, url: {}, attempts: {})",
                        event_type,
                        self.url,
                        attempt + 1
                    );
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e.clone());

                    if attempt < self.retry_policy.max_retries {
                        let backoff = self.retry_policy.backoff_duration(attempt);
                        warn!(
                            "Webhook request failed (event: {}, attempt: {}/{}), retrying in {:?}: {}",
                            event_type,
                            attempt + 1,
                            self.retry_policy.max_retries + 1,
                            backoff,
                            e
                        );
                        tokio::time::sleep(backoff).await;
                    } else {
                        warn!(
                            "Webhook request failed after {} attempts (event: {}, url: {}): {}",
                            self.retry_policy.max_retries + 1,
                            event_type,
                            self.url,
                            e
                        );
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Unknown error".to_string()))
    }

    /// Send a single webhook request
    async fn send_request(
        &self,
        client: &reqwest::Client,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        let request_builder = match self.method.to_uppercase().as_str() {
            "PUT" => client.put(&self.url),
            _ => client.post(&self.url),
        };

        let mut request_builder = request_builder.json(payload);

        // Add custom headers
        for (key, value) in &self.headers {
            request_builder = request_builder.header(key, value);
        }

        // Add standard headers
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .header("X-Webhook-Event", event_type)
            .header("User-Agent", "Locai-Webhook/0.1.0");

        let response = request_builder
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "HTTP error: {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            ))
        }
    }
}

#[async_trait]
impl MemoryHook for Webhook {
    async fn on_memory_created(&self, memory: &Memory) -> HookResult {
        // Serialize memory to JSON
        let memory_json = serde_json::to_value(memory)
            .unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize memory"}));

        let payload = serde_json::json!({
            "event": "memory.created",
            "timestamp": Utc::now().to_rfc3339(),
            "data": memory_json,
        });

        match self.send_with_retry("memory.created", payload).await {
            Ok(_) => HookResult::Continue,
            Err(e) => {
                error!("Webhook hook failed for on_memory_created: {}", e);
                HookResult::Continue // Don't fail the operation
            }
        }
    }

    async fn on_memory_accessed(&self, memory: &Memory) -> HookResult {
        // Serialize memory to JSON
        let memory_json = serde_json::to_value(memory)
            .unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize memory"}));

        let payload = serde_json::json!({
            "event": "memory.accessed",
            "timestamp": Utc::now().to_rfc3339(),
            "data": memory_json,
        });

        match self.send_with_retry("memory.accessed", payload).await {
            Ok(_) => HookResult::Continue,
            Err(e) => {
                debug!("Webhook hook failed for on_memory_accessed: {}", e);
                HookResult::Continue // Don't fail the operation
            }
        }
    }

    async fn on_memory_updated(&self, _old: &Memory, new: &Memory) -> HookResult {
        // Serialize new memory to JSON (spec says to send the updated memory)
        let memory_json = serde_json::to_value(new)
            .unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize memory"}));

        let payload = serde_json::json!({
            "event": "memory.updated",
            "timestamp": Utc::now().to_rfc3339(),
            "data": memory_json,
        });

        match self.send_with_retry("memory.updated", payload).await {
            Ok(_) => HookResult::Continue,
            Err(e) => {
                error!("Webhook hook failed for on_memory_updated: {}", e);
                HookResult::Continue // Don't fail the operation
            }
        }
    }

    async fn before_memory_deleted(&self, memory: &Memory) -> HookResult {
        // Serialize memory to JSON
        let memory_json = serde_json::to_value(memory)
            .unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize memory"}));

        let payload = serde_json::json!({
            "event": "memory.deleted",
            "timestamp": Utc::now().to_rfc3339(),
            "data": memory_json,
        });

        match self.send_with_retry("memory.deleted", payload).await {
            Ok(_) => HookResult::Continue,
            Err(e) => {
                error!("Webhook hook failed for before_memory_deleted: {}", e);
                HookResult::Continue // Don't prevent deletion on webhook failure
            }
        }
    }

    fn timeout_ms(&self) -> u64 {
        self.timeout.as_millis() as u64
    }

    fn name(&self) -> &str {
        "webhook"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_creation() {
        let hook = Webhook::new("http://example.com/webhook".to_string());
        assert_eq!(hook.url, "http://example.com/webhook");
        assert_eq!(hook.method, "POST");
        assert_eq!(hook.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_webhook_with_method() {
        let hook =
            Webhook::new("http://example.com/webhook".to_string()).with_method("PUT".to_string());
        assert_eq!(hook.method, "PUT");
    }

    #[test]
    fn test_webhook_with_headers() {
        let hook = Webhook::new("http://example.com/webhook".to_string())
            .with_header("Authorization".to_string(), "Bearer token".to_string())
            .with_header("X-Custom".to_string(), "value".to_string());

        assert_eq!(
            hook.headers.get("Authorization"),
            Some(&"Bearer token".to_string())
        );
        assert_eq!(hook.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_webhook_with_timeout() {
        let timeout = Duration::from_secs(30);
        let hook = Webhook::new("http://example.com/webhook".to_string()).with_timeout(timeout);
        assert_eq!(hook.timeout, timeout);
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_backoff_ms, 100);
        assert_eq!(policy.backoff_multiplier, 2.0);
        assert_eq!(policy.max_backoff_ms, 10000);
    }

    #[test]
    fn test_retry_policy_backoff_calculation() {
        let policy = RetryPolicy::default();

        let backoff_0 = policy.backoff_duration(0);
        assert_eq!(backoff_0.as_millis(), 100);

        let backoff_1 = policy.backoff_duration(1);
        assert_eq!(backoff_1.as_millis(), 200);

        let backoff_2 = policy.backoff_duration(2);
        assert_eq!(backoff_2.as_millis(), 400);

        let backoff_3 = policy.backoff_duration(3);
        assert_eq!(backoff_3.as_millis(), 800);
    }

    #[test]
    fn test_retry_policy_backoff_capped() {
        let policy = RetryPolicy::default();

        // When backoff would exceed max, cap it
        let backoff_10 = policy.backoff_duration(10);
        assert!(backoff_10.as_millis() <= policy.max_backoff_ms as u128);
        assert_eq!(backoff_10.as_millis(), 10000);
    }

    #[test]
    fn test_webhook_hook_with_retry_policy() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_backoff_ms: 50,
            backoff_multiplier: 1.5,
            max_backoff_ms: 5000,
        };

        let hook = Webhook::new("http://example.com/webhook".to_string())
            .with_retry_policy(policy.clone());

        assert_eq!(hook.retry_policy.max_retries, 5);
        assert_eq!(hook.retry_policy.initial_backoff_ms, 50);
    }

    #[test]
    fn test_webhook_timeout_ms() {
        let hook = Webhook::new("http://example.com/webhook".to_string())
            .with_timeout(Duration::from_secs(15));

        assert_eq!(hook.timeout_ms(), 15000);
    }

    #[test]
    fn test_webhook_name() {
        let hook = Webhook::new("http://example.com/webhook".to_string());
        assert_eq!(hook.name(), "webhook");
    }
}
