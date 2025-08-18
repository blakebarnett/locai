//! Memory utility functions
//!
//! This module contains utility functions for memory management,
//! including parsing, filtering, and conversion utilities.

use crate::models::{Memory, MemoryPriority, MemoryType};
use crate::storage::filters::MemoryFilter;

/// Parse memory type string into MemoryType enum
pub fn parse_memory_type(type_str: &str) -> MemoryType {
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
pub fn parse_memory_priority(priority_str: &str) -> MemoryPriority {
    match priority_str {
        "low" => MemoryPriority::Low,
        "normal" => MemoryPriority::Normal,
        "high" => MemoryPriority::High,
        "critical" => MemoryPriority::Critical,
        _ => MemoryPriority::Normal,
    }
}

/// Check if a memory matches the filter criteria
pub fn matches_memory_filter_detailed(memory: &Memory, filter: &MemoryFilter) -> bool {
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

/// Convert a database event to a Memory object
pub fn convert_db_event_to_memory(
    event: &crate::storage::shared_storage::live_query::DbEvent,
) -> Option<Memory> {
    use serde_json::Value;

    // Extract memory data from the event result
    let result = &event.result;

    // Extract basic fields
    let id = result
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;

    let content = result
        .get("content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;

    // Parse timestamps
    let created_at = result
        .get("created_at")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    // Extract metadata fields
    let metadata = result.get("metadata").cloned().unwrap_or(Value::Null);

    // Parse memory type from metadata
    let memory_type = metadata
        .get("memory_type")
        .and_then(|v| match v {
            Value::String(s) => Some(parse_memory_type(s)),
            _ => None,
        })
        .unwrap_or(MemoryType::Episodic);

    // Parse other metadata fields
    let last_accessed = metadata
        .get("last_accessed")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let access_count = metadata
        .get("access_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let priority = metadata
        .get("priority")
        .and_then(|v| match v {
            Value::String(s) => Some(parse_memory_priority(s)),
            _ => None,
        })
        .unwrap_or(MemoryPriority::Normal);

    let tags = metadata
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let source = metadata
        .get("source")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let expires_at = metadata
        .get("expires_at")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let properties = metadata
        .get("properties")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let related_memories = metadata
        .get("related_memories")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Extract embedding if present
    let embedding = result
        .get("embedding")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect()
        });

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

/// Determine why a memory matched a search query
pub fn determine_memory_match_reason(memory: &Memory, query: &str) -> String {
    let query_lower = query.to_lowercase();
    let content_lower = memory.content.to_lowercase();
    let mut reasons = Vec::new();

    if content_lower.contains(&query_lower) {
        reasons.push("content match");
    }

    for tag in &memory.tags {
        if tag.to_lowercase().contains(&query_lower) {
            reasons.push("tag match");
            break;
        }
    }

    if memory.source.to_lowercase().contains(&query_lower) {
        reasons.push("source match");
    }

    if reasons.is_empty() {
        "semantic similarity".to_string()
    } else {
        reasons.join(", ")
    }
}

/// Calculate message importance based on content and metadata
pub fn calculate_message_importance(message: &crate::messaging::types::Message) -> f64 {
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
pub fn extract_namespace_from_topic(topic: &str) -> String {
    if let Some(dot_pos) = topic.find('.') {
        topic[..dot_pos].to_string()
    } else {
        topic.to_string()
    }
}

/// Extract tags from a message for memory storage
pub fn extract_message_tags(message: &crate::messaging::types::Message) -> Vec<String> {
    let mut tags = vec![
        "message".to_string(),
        format!("sender:{}", message.sender),
        format!(
            "topic:{}",
            crate::messaging::filters::extract_topic_base(&message.topic)
        ),
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
pub fn extract_message_properties(
    message: &crate::messaging::types::Message,
) -> std::collections::HashMap<String, serde_json::Value> {
    let mut properties = std::collections::HashMap::new();
    properties.insert(
        "message_id".to_string(),
        serde_json::Value::String(message.id.as_str().to_string()),
    );
    properties.insert(
        "topic".to_string(),
        serde_json::Value::String(message.topic.clone()),
    );
    properties.insert(
        "namespace".to_string(),
        serde_json::Value::String(extract_namespace_from_topic(&message.topic)),
    );
    properties.insert(
        "sender".to_string(),
        serde_json::Value::String(message.sender.clone()),
    );
    properties.insert(
        "recipients".to_string(),
        serde_json::to_value(&message.recipients).unwrap_or_default(),
    );
    properties.insert(
        "timestamp".to_string(),
        serde_json::to_value(message.timestamp).unwrap_or_default(),
    );
    if let Some(expires_at) = &message.expires_at {
        properties.insert(
            "expires_at".to_string(),
            serde_json::to_value(expires_at).unwrap_or_default(),
        );
    }
    properties.insert(
        "headers".to_string(),
        serde_json::to_value(&message.headers).unwrap_or_default(),
    );
    properties.insert(
        "tags".to_string(),
        serde_json::to_value(&message.tags).unwrap_or_default(),
    );
    properties
}
