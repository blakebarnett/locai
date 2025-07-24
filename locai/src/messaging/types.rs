//! Message types for the embedded messaging system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Unique identifier for messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MessageId(pub String);

impl MessageId {
    /// Create a new unique message ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
    
    /// Create a message ID from a string
    pub fn from_string(id: String) -> Self {
        Self(id)
    }
    
    /// Get the string representation of the message ID
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for MessageId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MessageId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A message in the embedded messaging system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message
    pub id: MessageId,
    /// Topic this message was sent to
    pub topic: String,
    /// ID of the sender (app_id)
    pub sender: String,
    /// List of specific recipients (empty = broadcast)
    pub recipients: Vec<String>,
    /// Message content as JSON
    pub content: serde_json::Value,
    /// Additional message headers
    pub headers: HashMap<String, String>,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Optional expiration time
    pub expires_at: Option<DateTime<Utc>>,
    /// Importance score for prioritization
    pub importance: Option<f64>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl Message {
    /// Create a new message
    pub fn new(topic: String, sender: String, content: serde_json::Value) -> Self {
        Self {
            id: MessageId::new(),
            topic,
            sender,
            recipients: vec![],
            content,
            headers: HashMap::new(),
            timestamp: Utc::now(),
            expires_at: None,
            importance: None,
            tags: vec![],
        }
    }
    
    /// Add a recipient to this message
    pub fn add_recipient<S: Into<String>>(mut self, recipient: S) -> Self {
        self.recipients.push(recipient.into());
        self
    }
    
    /// Add multiple recipients
    pub fn add_recipients<I, S>(mut self, recipients: I) -> Self 
    where 
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.recipients.extend(recipients.into_iter().map(|r| r.into()));
        self
    }
    
    /// Add a header to this message
    pub fn add_header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.headers.insert(key.into(), value.into());
        self
    }
    
    /// Set the expiration time for this message
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
    
    /// Set the importance score for this message
    pub fn importance(mut self, importance: f64) -> Self {
        self.importance = Some(importance);
        self
    }
    
    /// Add a tag to this message
    pub fn add_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(tag.into());
        self
    }
    
    /// Add multiple tags
    pub fn add_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }
    
    /// Check if this message has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
    
    /// Get a header value
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }
    
    /// Check if message has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }
}

/// Filter for selecting messages
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageFilter {
    /// Exact topic matches
    pub topics: Option<Vec<String>>,
    /// Wildcard patterns like "character.*"
    pub topic_patterns: Option<Vec<String>>,
    /// Filter by sender app IDs
    pub senders: Option<Vec<String>>,
    /// Filter by recipient app IDs
    pub recipients: Option<Vec<String>>,
    /// Filter by source app (for cross-app messaging)
    pub source_app: Option<String>,
    /// Semantic search on content
    pub content_query: Option<String>,
    /// Filter by headers (key-value pairs must match exactly)
    pub headers: Option<HashMap<String, String>>,
    /// Time range filter
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Importance range filter
    pub importance_range: Option<(f64, f64)>,
    /// Filter by tags (message must have all specified tags)
    pub tags: Option<Vec<String>>,
    /// Filter by tags (message must have any of these tags)
    pub tags_any: Option<Vec<String>>,
    /// Include expired messages (default: false)
    pub include_expired: bool,
}

impl MessageFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Filter by specific topics
    pub fn topics<I, S>(mut self, topics: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.topics = Some(topics.into_iter().map(|t| t.into()).collect());
        self
    }
    
    /// Filter by topic patterns
    pub fn topic_patterns<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.topic_patterns = Some(patterns.into_iter().map(|p| p.into()).collect());
        self
    }
    
    /// Filter by senders
    pub fn senders<I, S>(mut self, senders: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.senders = Some(senders.into_iter().map(|s| s.into()).collect());
        self
    }
    
    /// Filter by recipients
    pub fn recipients<I, S>(mut self, recipients: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.recipients = Some(recipients.into_iter().map(|r| r.into()).collect());
        self
    }
    
    /// Filter by source app (for cross-app messaging)
    pub fn source_app<S: Into<String>>(mut self, app: S) -> Self {
        self.source_app = Some(app.into());
        self
    }
    
    /// Filter by content query
    pub fn content_query<S: Into<String>>(mut self, query: S) -> Self {
        self.content_query = Some(query.into());
        self
    }
    
    /// Filter by time range
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.time_range = Some((start, end));
        self
    }
    
    /// Filter by importance range
    pub fn importance_range(mut self, min: f64, max: f64) -> Self {
        self.importance_range = Some((min, max));
        self
    }
    
    /// Filter by tags (must have all)
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = Some(tags.into_iter().map(|t| t.into()).collect());
        self
    }
    
    /// Filter by tags (must have any)
    pub fn tags_any<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags_any = Some(tags.into_iter().map(|t| t.into()).collect());
        self
    }
    
    /// Include expired messages
    pub fn include_expired(mut self, include: bool) -> Self {
        self.include_expired = include;
        self
    }
    
    /// Add a header filter
    pub fn add_header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        if self.headers.is_none() {
            self.headers = Some(HashMap::new());
        }
        self.headers.as_mut().unwrap().insert(key.into(), value.into());
        self
    }
}

/// Builder for creating messages with fluent API
pub struct MessageBuilder {
    message: Message,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new<S1, S2>(topic: S1, sender: S2, content: serde_json::Value) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            message: Message::new(topic.into(), sender.into(), content),
        }
    }
    
    /// Add a recipient
    pub fn recipient<S: Into<String>>(mut self, recipient: S) -> Self {
        self.message = self.message.add_recipient(recipient);
        self
    }
    
    /// Add multiple recipients
    pub fn recipients<I, S>(mut self, recipients: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.message = self.message.add_recipients(recipients);
        self
    }
    
    /// Add a header
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.message = self.message.add_header(key, value);
        self
    }
    
    /// Set expiration time
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.message = self.message.expires_at(expires_at);
        self
    }
    
    /// Set importance
    pub fn importance(mut self, importance: f64) -> Self {
        self.message = self.message.importance(importance);
        self
    }
    
    /// Add a tag
    pub fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.message = self.message.add_tag(tag);
        self
    }
    
    /// Add multiple tags
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.message = self.message.add_tags(tags);
        self
    }
    
    /// Build the message
    pub fn build(self) -> Message {
        self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_creation() {
        let content = json!({"text": "Hello, world!"});
        let message = Message::new("test.topic".to_string(), "sender1".to_string(), content.clone());
        
        assert_eq!(message.topic, "test.topic");
        assert_eq!(message.sender, "sender1");
        assert_eq!(message.content, content);
        assert!(message.recipients.is_empty());
        assert!(!message.is_expired());
    }
    
    #[test]
    fn test_message_builder() {
        let content = json!({"text": "Hello, world!"});
        let message = MessageBuilder::new("test.topic", "sender1", content.clone())
            .recipient("recipient1")
            .header("priority", "high")
            .tag("test")
            .importance(0.8)
            .build();
            
        assert_eq!(message.topic, "test.topic");
        assert_eq!(message.sender, "sender1");
        assert_eq!(message.recipients, vec!["recipient1"]);
        assert_eq!(message.get_header("priority"), Some(&"high".to_string()));
        assert!(message.has_tag("test"));
        assert_eq!(message.importance, Some(0.8));
    }
    
    #[test]
    fn test_message_filter() {
        let filter = MessageFilter::new()
            .topics(vec!["topic1", "topic2"])
            .senders(vec!["sender1"])
            .importance_range(0.5, 1.0)
            .add_header("priority", "high");
            
        assert_eq!(filter.topics.as_ref().unwrap().len(), 2);
        assert_eq!(filter.senders.as_ref().unwrap()[0], "sender1");
        assert_eq!(filter.importance_range, Some((0.5, 1.0)));
        assert_eq!(filter.headers.as_ref().unwrap().get("priority"), Some(&"high".to_string()));
    }
    
    #[test]
    fn test_message_expiration() {
        let past_time = Utc::now() - chrono::Duration::hours(1);
        let future_time = Utc::now() + chrono::Duration::hours(1);
        
        let expired_message = Message::new("test".to_string(), "sender".to_string(), json!({}))
            .expires_at(past_time);
        assert!(expired_message.is_expired());
        
        let valid_message = Message::new("test".to_string(), "sender".to_string(), json!({}))
            .expires_at(future_time);
        assert!(!valid_message.is_expired());
    }
} 