//! Messaging system integration
//! 
//! This module handles integration with the messaging system,
//! including message storage as memories and live query subscriptions.

use crate::storage::filters::MemoryFilter;
use crate::storage::traits::GraphStore;
use crate::models::{Memory, MemoryType};
use crate::{LocaiError, Result};
use std::sync::Arc;
use async_stream;

/// Messaging system integration
#[derive(Debug)]
pub struct MessagingIntegration {
    storage: Arc<dyn GraphStore>,
}

impl MessagingIntegration {
    /// Create a new messaging integration handler
    pub fn new(storage: Arc<dyn GraphStore>) -> Self {
        Self { storage }
    }

    /// Subscribe to memory changes with live queries (for messaging system)
    /// 
    /// # Arguments
    /// * `filter` - Filter for which memories to monitor
    /// 
    /// # Returns
    /// Stream of memory change events
    pub async fn subscribe_to_memory_changes(
        &self,
        filter: MemoryFilter,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<Memory>> + Send>>> {
        use crate::storage::shared_storage::live_query::DbEvent;
        
        // Get the live query receiver from storage
        if self.storage.supports_live_queries() {
            match self.storage.setup_live_queries().await {
                Ok(Some(receiver_any)) => {
                    if let Ok(mut receiver) = receiver_any.downcast::<tokio::sync::broadcast::Receiver<DbEvent>>() {
                        let filter_clone = filter.clone();
                        
                        let stream = async_stream::stream! {
                            while let Ok(event) = receiver.recv().await {
                                // Only process memory table events
                                if event.table == "memory" && (event.action == "CREATE" || event.action == "UPDATE") {
                                    if let Some(memory) = convert_db_event_to_memory(&event) {
                                        // Apply filtering
                                        if matches_memory_filter_detailed(&memory, &filter_clone) {
                                            yield Ok(memory);
                                        }
                                    }
                                }
                            }
                        };
                        
                        return Ok(Box::pin(stream));
                    }
                }
                Ok(None) => {
                    tracing::debug!("Live queries not available for this storage configuration");
                }
                Err(e) => {
                    tracing::error!("Failed to setup live queries: {}", e);
                    return Err(LocaiError::Storage(format!("Live query setup failed: {}", e)));
                }
            }
        }
        
        // Fallback to empty stream if live queries aren't available
        tracing::debug!("Live queries not supported, returning empty memory stream");
        use futures::stream;
        let empty_stream = stream::empty::<Result<Memory>>();
        Ok(Box::pin(empty_stream))
    }
    
    /// Store a message as a memory record (specialized method for messaging)
    /// 
    /// # Arguments
    /// * `message` - The message to store
    /// * `memory_operations` - Reference to memory operations for storing
    /// 
    /// # Returns
    /// Memory ID of the stored message
    pub async fn store_message(
        &self, 
        message: &crate::messaging::types::Message,
        memory_operations: &crate::memory::operations::MemoryOperations,
    ) -> Result<String> {
        let content = serde_json::to_string(message)
            .map_err(|e| LocaiError::Storage(format!("Failed to serialize message: {}", e)))?;
        
        let topic_base = crate::messaging::filters::extract_topic_base(&message.topic);
        let tags = self.extract_message_tags(message);
        let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
        let properties = self.extract_message_properties(message);
        
        memory_operations.store_memory(
            crate::models::MemoryBuilder::new_with_content(content)
                .memory_type(MemoryType::Custom(format!("msg:{}", topic_base)))
                .source(&message.sender)
                .tags(tag_refs)
                .properties(properties)
                .build()
        ).await
    }
    
    /// Get message history (specialized method for messaging)
    /// 
    /// # Arguments
    /// * `filter` - Message filter criteria
    /// * `limit` - Maximum number of messages to return
    /// 
    /// # Returns
    /// List of messages matching the criteria
    pub async fn get_message_history(
        &self,
        filter: &crate::messaging::types::MessageFilter,
        limit: Option<usize>,
    ) -> Result<Vec<crate::messaging::types::Message>> {
        let memory_filter = crate::messaging::filters::convert_message_filter_to_memory_filter(filter)?;
        let memories = self.storage.list_memories(Some(memory_filter), limit, None).await
            .map_err(|e| LocaiError::Storage(format!("Failed to get message history: {}", e)))?;
        
        let mut messages = Vec::new();
        for memory in memories {
            match serde_json::from_str::<crate::messaging::types::Message>(&memory.content) {
                Ok(message) => {
                    if filter.include_expired || !message.is_expired() {
                        messages.push(message);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize message from memory {}: {}", memory.id, e);
                }
            }
        }
        
        // Sort by timestamp (most recent first)
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(messages)
    }
    
    /// Extract tags from a message for memory storage
    fn extract_message_tags(&self, message: &crate::messaging::types::Message) -> Vec<String> {
        let mut tags = vec![
            "message".to_string(),
            format!("sender:{}", message.sender),
            format!("topic:{}", crate::messaging::filters::extract_topic_base(&message.topic)),
        ];
        
        // Add custom tags from the message
        tags.extend(message.tags.clone());
        
        // Add recipient tags
        for recipient in &message.recipients {
            tags.push(format!("recipient:{}", recipient));
        }
        
        tags
    }
    
    /// Extract properties from a message for memory storage
    fn extract_message_properties(&self, message: &crate::messaging::types::Message) -> std::collections::HashMap<&str, serde_json::Value> {
        let mut properties = std::collections::HashMap::new();
        properties.insert("message_id", serde_json::Value::String(message.id.as_str().to_string()));
        properties.insert("topic", serde_json::Value::String(message.topic.clone()));
        properties.insert("namespace", serde_json::Value::String(self.extract_namespace_from_topic(&message.topic)));
        properties.insert("sender", serde_json::Value::String(message.sender.clone()));
        properties.insert("recipients", serde_json::to_value(&message.recipients).unwrap_or_default());
        properties.insert("timestamp", serde_json::to_value(&message.timestamp).unwrap_or_default());
        if let Some(expires_at) = &message.expires_at {
            properties.insert("expires_at", serde_json::to_value(expires_at).unwrap_or_default());
        }
        properties.insert("headers", serde_json::to_value(&message.headers).unwrap_or_default());
        properties.insert("tags", serde_json::to_value(&message.tags).unwrap_or_default());
        properties
    }
    
    /// Calculate message importance based on content and metadata
    #[allow(dead_code)]
    fn calculate_message_importance(&self, message: &crate::messaging::types::Message) -> f64 {
        // If importance is explicitly set, use it
        if let Some(importance) = message.importance {
            return importance;
        }
        
        // Otherwise, calculate based on various factors
        let mut score: f64 = 0.5; // Base importance
        
        // Boost importance for specific recipients (not broadcast)
        if !message.recipients.is_empty() {
            score += 0.1;
        }
        
        // Boost importance for urgent headers
        if let Some(priority) = message.get_header("priority") {
            match priority.as_str() {
                "urgent" | "high" => score += 0.3,
                "normal" => score += 0.1,
                _ => {}
            }
        }
        
        // Boost importance for certain tags
        if message.has_tag("important") || message.has_tag("urgent") {
            score += 0.2;
        }
        
        // Boost importance for expiring messages
        if message.expires_at.is_some() {
            score += 0.1;
        }
        
        // Cap at 1.0
        score.min(1.0)
    }
    
    /// Extract namespace from a topic
    fn extract_namespace_from_topic(&self, topic: &str) -> String {
        if let Some(dot_pos) = topic.find('.') {
            topic[..dot_pos].to_string()
        } else {
            topic.to_string()
        }
    }

    /// Get access to the underlying storage service
    pub fn storage(&self) -> &Arc<dyn GraphStore> {
        &self.storage
    }
}

/// Convert a database event to a Memory object
fn convert_db_event_to_memory(event: &crate::storage::shared_storage::live_query::DbEvent) -> Option<Memory> {
    use serde_json::Value;
    
    // Extract memory data from the event result
    let result = &event.result;
    
    // Extract basic fields
    let id = result.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;
    
    let content = result.get("content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;
    
    // Parse timestamps
    let created_at = result.get("created_at")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);
    
    // Extract metadata fields
    let metadata = result.get("metadata").cloned().unwrap_or(Value::Null);
    
    // Parse memory type from metadata
    let memory_type = metadata.get("memory_type")
        .and_then(|v| match v {
            Value::String(s) => Some(parse_memory_type(s)),
            _ => None
        })
        .unwrap_or(MemoryType::Episodic);
    
    // Parse other metadata fields
    let last_accessed = metadata.get("last_accessed")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    
    let access_count = metadata.get("access_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    
    let priority = metadata.get("priority")
        .and_then(|v| match v {
            Value::String(s) => Some(parse_memory_priority(s)),
            _ => None
        })
        .unwrap_or(crate::models::MemoryPriority::Normal);
    
    let tags = metadata.get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    
    let source = metadata.get("source")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    let expires_at = metadata.get("expires_at")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    
    let properties = metadata.get("properties")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    
    let related_memories = metadata.get("related_memories")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    
    // Extract embedding if present
    let embedding = result.get("embedding")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect());
    
    Some(Memory {
        id,
        content,
        memory_type,
        last_accessed,
        access_count,
        priority,
        tags,
        source,
        expires_at,
        properties,
        related_memories,
        embedding,
        created_at,
    })
}

/// Parse memory type string into MemoryType enum
fn parse_memory_type(type_str: &str) -> MemoryType {
    match type_str {
        "conversation" => MemoryType::Conversation,
        "fact" => MemoryType::Fact,
        "procedural" => MemoryType::Procedural,
        "episodic" => MemoryType::Episodic,
        "identity" => MemoryType::Identity,
        "world" => MemoryType::World,
        "action" => MemoryType::Action,
        "event" => MemoryType::Event,
        s if s.starts_with("custom:") => MemoryType::Custom(s[7..].to_string()),
        s => MemoryType::Custom(s.to_string()),
    }
}

/// Parse memory priority string into MemoryPriority enum
fn parse_memory_priority(priority_str: &str) -> crate::models::MemoryPriority {
    match priority_str {
        "low" => crate::models::MemoryPriority::Low,
        "normal" => crate::models::MemoryPriority::Normal,
        "high" => crate::models::MemoryPriority::High,
        "critical" => crate::models::MemoryPriority::Critical,
        _ => crate::models::MemoryPriority::Normal,
    }
}

/// Check if a memory matches the filter criteria
fn matches_memory_filter_detailed(
    memory: &Memory,
    filter: &MemoryFilter,
) -> bool {
    // Check memory type
    if let Some(filter_type) = &filter.memory_type {
        let memory_type_str = memory.memory_type.to_string();
        if filter_type != &memory_type_str {
            return false;
        }
    }
    
    // Check content
    if let Some(content_query) = &filter.content {
        let content_lower = memory.content.to_lowercase();
        let query_lower = content_query.to_lowercase();
        if !content_lower.contains(&query_lower) {
            return false;
        }
    }
    
    // Check source
    if let Some(filter_source) = &filter.source {
        if &memory.source != filter_source {
            return false;
        }
    }
    
    // Check tags
    if let Some(filter_tags) = &filter.tags {
        if !filter_tags.iter().all(|tag| memory.tags.contains(tag)) {
            return false;
        }
    }
    
    // Check time range
    if let Some(created_after) = &filter.created_after {
        if memory.created_at < *created_after {
            return false;
        }
    }
    
    if let Some(created_before) = &filter.created_before {
        if memory.created_at > *created_before {
            return false;
        }
    }
    
    true
} 