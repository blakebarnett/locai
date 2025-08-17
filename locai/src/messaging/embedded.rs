//! Embedded messaging implementation using SurrealDB live queries

use crate::core::MemoryManager;
use crate::messaging::filters::convert_message_filter_to_memory_filter;
use crate::messaging::stream::MessageStream;
use crate::messaging::types::{Message, MessageFilter, MessageId};
use crate::models::MemoryType;
use crate::storage::shared_storage::live_query::DbEvent;
use crate::{LocaiError, Result};
use async_stream;
use futures::{Stream, StreamExt};
use serde_json::json;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

/// Embedded messaging system that uses SurrealDB live queries for real-time messaging
pub struct EmbeddedMessaging {
    memory_manager: Arc<MemoryManager>,
    event_receiver: Option<broadcast::Receiver<DbEvent>>,
}

impl Clone for EmbeddedMessaging {
    fn clone(&self) -> Self {
        Self {
            memory_manager: self.memory_manager.clone(),
            event_receiver: self.event_receiver.as_ref().map(|r| r.resubscribe()),
        }
    }
}

impl EmbeddedMessaging {
    /// Create a new embedded messaging instance
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            memory_manager,
            event_receiver: None,
        }
    }

    /// Initialize the messaging system with live query support
    pub async fn initialize(&mut self) -> Result<()> {
        debug!("Initializing embedded messaging system");

        // Get the live query receiver from the memory manager's storage
        #[cfg(any(feature = "surrealdb-embedded", feature = "surrealdb-remote"))]
        {
            let storage = self.memory_manager.storage();
            if storage.supports_live_queries() {
                match storage.setup_live_queries().await {
                    Ok(Some(receiver_any)) => {
                        if let Ok(receiver) =
                            receiver_any.downcast::<broadcast::Receiver<DbEvent>>()
                        {
                            self.event_receiver = Some(*receiver);
                            debug!("Live query receiver initialized for messaging");
                        } else {
                            warn!("Failed to downcast live query receiver");
                        }
                    }
                    Ok(None) => {
                        debug!("Live queries not available for this storage configuration");
                    }
                    Err(e) => {
                        error!("Failed to setup live queries: {}", e);
                        return Err(LocaiError::Storage(format!(
                            "Live query setup failed: {}",
                            e
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Send a message through the embedded messaging system
    pub async fn send_message(
        &self,
        namespace: &str,
        app_id: &str,
        topic: &str,
        content: serde_json::Value,
    ) -> Result<MessageId> {
        let message = Message::new(
            format!("{}.{}", namespace, topic),
            app_id.to_string(),
            content,
        );

        self.send_complete_message(message).await
    }

    /// Send a complete message with all options
    pub async fn send_complete_message(&self, message: Message) -> Result<MessageId> {
        send_complete_message(&self.memory_manager, message).await
    }

    /// Subscribe to messages with filtering and real-time updates
    pub async fn subscribe_filtered(&self, filter: MessageFilter) -> Result<MessageStream> {
        subscribe_filtered(&self.memory_manager, filter).await
    }

    /// Get message history with optional filtering
    pub async fn get_message_history(
        &self,
        filter: Option<MessageFilter>,
        limit: Option<usize>,
    ) -> Result<Vec<Message>> {
        get_message_history(&self.memory_manager, filter, limit).await
    }

    /// Create a message stream from live query events
    pub async fn create_live_message_stream(&self, filter: MessageFilter) -> Result<MessageStream> {
        if let Some(mut receiver) = self.event_receiver.as_ref().map(|r| r.resubscribe()) {
            let memory_filter = convert_message_filter_to_memory_filter(&filter)?;
            let filter_clone = filter.clone();

            let stream = async_stream::stream! {
                while let Ok(event) = receiver.recv().await {
                    // Only process memory table events
                    if event.table == "memory" {
                        if let Some(message) = convert_db_event_to_message(&event).await {
                            // Apply memory-level filtering first
                            if matches_memory_filter(&message, &memory_filter) {
                                // Apply message-level filtering
                                if crate::messaging::stream::utils::matches_filter(&message, &filter_clone) {
                                    yield Ok(message);
                                }
                            }
                        }
                    }
                }
            };

            Ok(Box::pin(stream))
        } else {
            // Fallback to empty stream if live queries aren't available
            debug!("Live queries not available, returning empty message stream");
            use futures::stream;
            let empty_stream = stream::empty::<Result<Message>>();
            Ok(Box::pin(empty_stream))
        }
    }
}

/// Send a message through the embedded messaging system
///
/// # Arguments
/// * `memory_manager` - Shared MemoryManager instance
/// * `namespace` - Namespace for the message
/// * `app_id` - Application ID of the sender
/// * `topic` - Topic to send message to
/// * `content` - Message content
///
/// # Returns
/// Message ID of the sent message
pub async fn send_message(
    memory_manager: &Arc<MemoryManager>,
    namespace: &str,
    app_id: &str,
    topic: &str,
    content: serde_json::Value,
) -> Result<MessageId> {
    let message = Message::new(
        format!("{}.{}", namespace, topic),
        app_id.to_string(),
        content,
    );

    send_complete_message(memory_manager, message).await
}

/// Send a complete message with all options
///
/// # Arguments
/// * `memory_manager` - Shared MemoryManager instance
/// * `message` - Complete message to send
///
/// # Returns
/// Message ID of the sent message
pub async fn send_complete_message(
    memory_manager: &Arc<MemoryManager>,
    message: Message,
) -> Result<MessageId> {
    debug!("Sending message to topic: {}", message.topic);

    // Extract topic base for memory type
    let topic_base = extract_topic_base(&message.topic);
    debug!(
        "Storing message for topic: {} (base: {})",
        message.topic, topic_base
    );

    // Store message as a memory record - this triggers live query notifications
    let memory_id = memory_manager
        .add_memory_with_options(
            &serde_json::to_string(&message)
                .map_err(|e| LocaiError::Storage(format!("Failed to serialize message: {}", e)))?,
            |builder| {
                let topic_base = extract_topic_base(&message.topic);
                let tags = build_message_tags(&message);
                let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
                let properties = build_message_properties(&message);

                builder
                    .memory_type(MemoryType::Custom(format!("msg:{}", topic_base)))
                    .source(&message.sender)
                    .tags(tag_refs)
                    .properties(properties)
            },
        )
        .await?;

    // Create process entity relationship if it doesn't exist
    let process_entity_id = format!("process:{}", message.sender);
    if memory_manager
        .get_entity(&process_entity_id)
        .await?
        .is_none()
    {
        let process_entity = crate::storage::models::Entity {
            id: process_entity_id.clone(),
            entity_type: "process".to_string(),
            properties: json!({
                "app_id": message.sender,
                "namespace": extract_namespace_from_topic(&message.topic),
                "created_at": message.timestamp,
            }),
            created_at: message.timestamp,
            updated_at: message.timestamp,
        };

        if let Err(e) = memory_manager.create_entity(process_entity).await {
            warn!("Failed to create process entity: {}", e);
        }
    }

    // Create relationship between process and message
    let message_entity_id = format!("message:{}", message.id.as_str());
    if let Err(e) = memory_manager
        .create_relationship(&process_entity_id, &message_entity_id, "sent_message")
        .await
    {
        warn!("Failed to create process-message relationship: {}", e);
    }

    debug!("Message sent with memory ID: {}", memory_id);
    Ok(message.id)
}

/// Subscribe to messages with filtering
///
/// # Arguments
/// * `memory_manager` - Shared MemoryManager instance
/// * `filter` - Message filter
///
/// # Returns
/// Stream of matching messages
pub async fn subscribe_filtered(
    memory_manager: &Arc<MemoryManager>,
    filter: MessageFilter,
) -> Result<MessageStream> {
    debug!("Creating filtered message subscription");

    // Convert message filter to memory filter
    let memory_filter = convert_message_filter_to_memory_filter(&filter)
        .map_err(|e| LocaiError::Storage(format!("Failed to convert message filter: {}", e)))?;

    // Subscribe to memory changes using live queries
    let stream = create_message_stream_from_memory_manager(memory_manager, memory_filter).await?;

    // Apply additional filtering that couldn't be done at the database level
    let filtered_stream = Box::pin(stream.filter_map(move |result| {
        let filter = filter.clone();
        async move {
            match result {
                Ok(message) => {
                    if crate::messaging::stream::utils::matches_filter(&message, &filter) {
                        Some(Ok(message))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            }
        }
    }));

    Ok(filtered_stream)
}

/// Get message history with optional filtering
///
/// # Arguments
/// * `memory_manager` - Shared MemoryManager instance
/// * `filter` - Optional message filter
/// * `limit` - Maximum number of messages to return
///
/// # Returns
/// List of messages matching the criteria
pub async fn get_message_history(
    memory_manager: &Arc<MemoryManager>,
    filter: Option<MessageFilter>,
    limit: Option<usize>,
) -> Result<Vec<Message>> {
    debug!("Retrieving message history");

    let memory_filter = if let Some(ref filter) = filter {
        convert_message_filter_to_memory_filter(filter)?
    } else {
        // Default filter for message memories
        crate::storage::filters::MemoryFilter::default()
    };

    let memories = memory_manager
        .filter_memories(memory_filter, None, None, limit)
        .await?;

    let mut messages = Vec::new();
    for memory in memories {
        // Only process memories that are message type
        let memory_type_str = memory.memory_type.to_string();
        if memory_type_str.starts_with("msg:") {
            match serde_json::from_str::<Message>(&memory.content) {
                Ok(message) => {
                    // Additional filtering for expired messages if needed
                    if filter.as_ref().is_none_or(|f| f.include_expired) || !message.is_expired() {
                        messages.push(message);
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to deserialize message from memory {}: {}",
                        memory.id, e
                    );
                }
            }
        }
    }

    // Sort by timestamp (most recent first)
    messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(messages)
}

/// Create a message stream from memory manager changes using live queries
async fn create_message_stream_from_memory_manager(
    memory_manager: &Arc<MemoryManager>,
    memory_filter: crate::storage::filters::MemoryFilter,
) -> Result<Pin<Box<dyn Stream<Item = Result<Message>> + Send>>> {
    debug!("Creating message stream from memory manager live queries");

    // Use the memory manager's live query subscription
    let memory_stream = memory_manager
        .subscribe_to_memory_changes(memory_filter)
        .await?;

    // Convert memory events to message events
    let message_stream = memory_stream.filter_map(|memory_result| async move {
        match memory_result {
            Ok(memory) => {
                // Only process memories that are message type
                let memory_type_str = memory.memory_type.to_string();
                if memory_type_str.starts_with("msg:") {
                    match serde_json::from_str::<Message>(&memory.content) {
                        Ok(message) => Some(Ok(message)),
                        Err(e) => {
                            warn!("Failed to deserialize message from memory: {}", e);
                            Some(Err(LocaiError::Storage(format!(
                                "Message deserialization failed: {}",
                                e
                            ))))
                        }
                    }
                } else {
                    None
                }
            }
            Err(e) => Some(Err(e)),
        }
    });

    Ok(Box::pin(message_stream))
}

/// Convert a database event to a message (if it's a message-related event)
async fn convert_db_event_to_message(event: &DbEvent) -> Option<Message> {
    // Only process CREATE and UPDATE events for memory table
    if event.table != "memory" || (event.action != "CREATE" && event.action != "UPDATE") {
        return None;
    }

    // Extract the memory content from the event result
    if let Some(content_value) = event.result.get("content") {
        if let Some(content_str) = content_value.as_str() {
            match serde_json::from_str::<Message>(content_str) {
                Ok(message) => Some(message),
                Err(e) => {
                    debug!("Failed to parse message from database event: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Check if a message matches memory-level filters
fn matches_memory_filter(
    message: &Message,
    filter: &crate::storage::filters::MemoryFilter,
) -> bool {
    // Check memory type filter
    if let Some(memory_type) = &filter.memory_type {
        let topic_base = extract_topic_base(&message.topic);
        let expected_type = format!("msg:{}", topic_base);
        if memory_type != &expected_type {
            return false;
        }
    }

    // Check source filter (maps to sender)
    if let Some(source) = &filter.source {
        if source != &message.sender {
            return false;
        }
    }

    // Check time range
    if let Some(created_after) = &filter.created_after {
        if message.timestamp < *created_after {
            return false;
        }
    }

    if let Some(created_before) = &filter.created_before {
        if message.timestamp > *created_before {
            return false;
        }
    }

    // Check content filter
    if let Some(content_query) = &filter.content {
        let content_str = message.content.to_string().to_lowercase();
        let query_lower = content_query.to_lowercase();
        if !content_str.contains(&query_lower) {
            return false;
        }
    }

    // Check tags - need to check against the tags that would be stored in memory
    if let Some(filter_tags) = &filter.tags {
        let memory_tags = build_message_tags(message);
        if !filter_tags.iter().all(|tag| memory_tags.contains(tag)) {
            return false;
        }
    }

    true
}

/// Build tags for a message when storing as memory
fn build_message_tags(message: &Message) -> Vec<String> {
    let mut tags = vec![
        "message".to_string(),
        format!("sender:{}", message.sender),
        format!("topic:{}", extract_topic_base(&message.topic)),
    ];

    // Add custom tags from the message
    tags.extend(message.tags.clone());

    // Add recipient tags
    for recipient in &message.recipients {
        tags.push(format!("recipient:{}", recipient));
    }

    tags
}

/// Build properties for a message when storing as memory
fn build_message_properties(
    message: &Message,
) -> std::collections::HashMap<&str, serde_json::Value> {
    let mut properties = std::collections::HashMap::new();
    properties.insert(
        "message_id",
        serde_json::Value::String(message.id.as_str().to_string()),
    );
    properties.insert("topic", serde_json::Value::String(message.topic.clone()));
    properties.insert(
        "namespace",
        serde_json::Value::String(extract_namespace_from_topic(&message.topic)),
    );
    properties.insert("sender", serde_json::Value::String(message.sender.clone()));
    properties.insert(
        "recipients",
        serde_json::to_value(&message.recipients).unwrap_or_default(),
    );
    properties.insert(
        "timestamp",
        serde_json::to_value(message.timestamp).unwrap_or_default(),
    );
    if let Some(expires_at) = &message.expires_at {
        properties.insert(
            "expires_at",
            serde_json::to_value(expires_at).unwrap_or_default(),
        );
    }
    properties.insert(
        "headers",
        serde_json::to_value(&message.headers).unwrap_or_default(),
    );
    properties.insert(
        "tags",
        serde_json::to_value(&message.tags).unwrap_or_default(),
    );
    properties
}

/// Extract topic base from a full topic path
fn extract_topic_base(full_topic: &str) -> String {
    crate::messaging::filters::extract_topic_base(full_topic)
}

/// Extract namespace from a topic
fn extract_namespace_from_topic(topic: &str) -> String {
    if let Some(dot_pos) = topic.find('.') {
        topic[..dot_pos].to_string()
    } else {
        topic.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_message_tags() {
        let message = Message::new(
            "app:sender.character.action".to_string(),
            "test_sender".to_string(),
            json!({}),
        )
        .add_tag("important")
        .add_recipient("recipient1");

        let tags = build_message_tags(&message);

        assert!(tags.contains(&"message".to_string()));
        assert!(tags.contains(&"sender:test_sender".to_string()));
        assert!(tags.contains(&"topic:character.action".to_string()));
        assert!(tags.contains(&"important".to_string()));
        assert!(tags.contains(&"recipient:recipient1".to_string()));
    }

    #[test]
    fn test_build_message_properties() {
        let message = Message::new("test.topic".to_string(), "sender".to_string(), json!({}))
            .add_header("priority", "high");

        let properties = build_message_properties(&message);

        assert_eq!(properties["topic"], "test.topic");
        assert_eq!(properties["sender"], "sender");
        assert_eq!(properties["namespace"], "test");
        assert!(!properties["headers"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_extract_topic_base() {
        assert_eq!(
            extract_topic_base("app:sender.character.action"),
            "character.action"
        );
        assert_eq!(extract_topic_base("character.action"), "character.action");
    }

    #[test]
    fn test_extract_namespace_from_topic() {
        assert_eq!(
            extract_namespace_from_topic("app:sender.character.action"),
            "app:sender"
        );
        assert_eq!(extract_namespace_from_topic("test.topic"), "test");
        assert_eq!(extract_namespace_from_topic("simple"), "simple");
    }

    #[test]
    fn test_matches_memory_filter() {
        let message = Message::new(
            "test.topic".to_string(),
            "sender1".to_string(),
            json!({"content": "test"}),
        )
        .add_tag("important");

        let mut filter = crate::storage::filters::MemoryFilter::default();
        filter.memory_type = Some("msg:test.topic".to_string());
        filter.source = Some("sender1".to_string());
        filter.tags = Some(vec!["important".to_string()]);

        assert!(matches_memory_filter(&message, &filter));

        // Test mismatch
        filter.source = Some("other_sender".to_string());
        assert!(!matches_memory_filter(&message, &filter));
    }
}
