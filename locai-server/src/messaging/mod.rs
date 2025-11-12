//! Messaging server implementation for locai-server

pub mod handlers;
pub mod server;

pub use handlers::handle_messaging_websocket;
pub use server::MessagingServer;

use crate::error::ServerError;
use locai::messaging::types::{Message, MessageFilter, MessageId};
use locai::models::{Memory, MemoryPriority, MemoryType};
use locai::storage::filters::MemoryFilter;
use std::collections::HashMap;
use std::sync::Arc;

/// Result type for messaging operations
pub type Result<T> = std::result::Result<T, ServerError>;

/// Messaging storage abstraction using shared storage
#[derive(Debug)]
pub struct MessagingStorage {
    /// Shared storage instance from MemoryManager
    shared_storage: Arc<dyn locai::storage::traits::GraphStore>,
}

impl MessagingStorage {
    /// Create a messaging storage instance using shared storage
    ///
    /// This avoids creating separate database instances and eliminates lock conflicts
    pub fn new_shared(shared_storage: Arc<dyn locai::storage::traits::GraphStore>) -> Self {
        Self { shared_storage }
    }

    /// Store a message
    pub async fn store_message(&self, message: &Message) -> Result<()> {
        // Convert Message to Memory for storage
        let memory = Memory::new(
            message.id.as_str().to_string(),
            message.content.to_string(),
            MemoryType::Event,
        );

        let mut memory = memory;
        memory.source = message.sender.clone();
        memory.last_accessed = Some(message.timestamp);
        memory.expires_at = message.expires_at;
        memory.priority = MemoryPriority::Normal;
        memory.set_property("topic", serde_json::Value::String(message.topic.clone()));
        memory.set_property(
            "recipients",
            serde_json::to_value(&message.recipients).unwrap_or_default(),
        );
        memory.set_property(
            "headers",
            serde_json::to_value(&message.headers).unwrap_or_default(),
        );
        memory.set_property(
            "tags",
            serde_json::to_value(&message.tags).unwrap_or_default(),
        );
        memory.set_property(
            "message_type",
            serde_json::Value::String("messaging".to_string()),
        );
        if let Some(expires_at) = message.expires_at {
            memory.set_property(
                "expires_at",
                serde_json::to_value(expires_at).unwrap_or_default(),
            );
        }
        if let Some(importance) = message.importance
            && let Some(number) = serde_json::Number::from_f64(importance)
        {
            memory.set_property("importance", serde_json::Value::Number(number));
        }

        self.shared_storage
            .create_memory(memory)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to store message: {}", e)))?;

        Ok(())
    }

    /// Get message history with filtering
    pub async fn get_message_history(
        &self,
        filter: Option<MessageFilter>,
        limit: Option<usize>,
    ) -> Result<Vec<Message>> {
        // Convert MessageFilter to MemoryFilter
        let memory_filter = convert_message_filter_to_memory_filter(filter)?;

        let memories = self
            .shared_storage
            .list_memories(Some(memory_filter), limit, None)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to get message history: {}", e)))?;

        // Convert memories back to messages
        let messages = memories
            .into_iter()
            .filter_map(|memory| convert_memory_to_message(memory).ok())
            .collect();

        Ok(messages)
    }
}

/// Convert MessageFilter to Locai MemoryFilter
fn convert_message_filter_to_memory_filter(filter: Option<MessageFilter>) -> Result<MemoryFilter> {
    let mut memory_filter = MemoryFilter::default();

    if let Some(f) = filter {
        // Add message type filter
        let mut properties = HashMap::new();
        properties.insert(
            "message_type".to_string(),
            serde_json::Value::String("messaging".to_string()),
        );

        // Convert topic patterns to content filters
        if let Some(patterns) = f.topic_patterns {
            for pattern in patterns {
                properties.insert("topic".to_string(), serde_json::Value::String(pattern));
            }
        }

        memory_filter.properties = Some(properties);

        // Convert senders to source filter
        if let Some(senders) = f.senders {
            // Use first sender for single source filter
            if let Some(sender) = senders.first() {
                memory_filter.source = Some(sender.clone());
            }
        }

        // Convert time range
        if let Some((start, end)) = f.time_range {
            memory_filter.created_after = Some(start);
            memory_filter.created_before = Some(end);
        }
    }

    Ok(memory_filter)
}

/// Convert Locai Memory to Message
fn convert_memory_to_message(memory: Memory) -> Result<Message> {
    let properties = &memory.properties;

    let topic = properties
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let recipients = properties
        .get("recipients")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let headers = properties
        .get("headers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let tags = properties
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let expires_at = properties
        .get("expires_at")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let importance = properties.get("importance").and_then(|v| v.as_f64());

    let content: serde_json::Value = serde_json::from_str(&memory.content)
        .unwrap_or_else(|_| serde_json::Value::String(memory.content.clone()));

    Ok(Message {
        id: MessageId::from_string(memory.id),
        topic,
        sender: memory.source,
        recipients,
        content,
        headers,
        timestamp: memory.last_accessed.unwrap_or(memory.created_at),
        expires_at,
        importance,
        tags,
    })
}
