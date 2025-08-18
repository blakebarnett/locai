//! Memory model representing stored information

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Memory priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPriority {
    /// Low importance memory
    Low = 0,

    /// Normal importance memory
    Normal = 1,

    /// High importance memory
    High = 2,

    /// Critical importance memory
    Critical = 3,
}

impl Default for MemoryPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Types of memories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// Conversation or dialogue memory
    Conversation,
    /// Factual knowledge memory
    Fact,
    /// Procedural/skill memory
    Procedural,
    /// Episodic/experience memory
    Episodic,
    /// Identity/self-concept memory
    Identity,
    /// World/environment memory
    World,
    /// Action/behavior memory
    Action,
    /// Event memory
    Event,
    /// Wisdom/insight memory
    Wisdom,
    /// Custom memory type
    Custom(String),
}

impl Default for MemoryType {
    fn default() -> Self {
        Self::Fact
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conversation => write!(f, "conversation"),
            Self::Fact => write!(f, "fact"),
            Self::Procedural => write!(f, "procedural"),
            Self::Episodic => write!(f, "episodic"),
            Self::Identity => write!(f, "identity"),
            Self::World => write!(f, "world"),
            Self::Action => write!(f, "action"),
            Self::Event => write!(f, "event"),
            Self::Wisdom => write!(f, "wisdom"),
            Self::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

impl MemoryType {
    /// Convert a string to a MemoryType
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "conversation" => Self::Conversation,
            "fact" => Self::Fact,
            "procedural" => Self::Procedural,
            "episodic" => Self::Episodic,
            "identity" => Self::Identity,
            "world" => Self::World,
            "action" => Self::Action,
            "event" => Self::Event,
            "wisdom" => Self::Wisdom,
            _ => {
                if let Some(stripped) = s.strip_prefix("custom:") {
                    Self::Custom(stripped.to_string())
                } else {
                    Self::Custom(s.to_string())
                }
            }
        }
    }
}

/// Core memory structure for all memory storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    /// Unique identifier for the memory
    pub id: String,

    /// The actual content of the memory
    pub content: String,

    /// Type of memory
    pub memory_type: MemoryType,

    /// When the memory was created
    pub created_at: DateTime<Utc>,

    /// When the memory was last accessed
    pub last_accessed: Option<DateTime<Utc>>,

    /// How many times the memory has been accessed
    pub access_count: u32,

    /// Priority/importance of the memory
    pub priority: MemoryPriority,

    /// Tags associated with the memory for categorization
    pub tags: Vec<String>,

    /// Source of the memory (e.g., user, agent, system)
    pub source: String,

    /// When the memory expires (if applicable)
    pub expires_at: Option<DateTime<Utc>>,

    /// Additional properties as arbitrary JSON
    pub properties: serde_json::Value,

    /// References to related memories by ID
    pub related_memories: Vec<String>,

    /// Vector embedding if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl Memory {
    /// Create a new memory with minimal information
    pub fn new(id: String, content: String, memory_type: MemoryType) -> Self {
        Self {
            id,
            content,
            memory_type,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::Normal,
            tags: Vec::new(),
            source: "unknown".to_string(),
            expires_at: None,
            properties: serde_json::json!({}),
            related_memories: Vec::new(),
            embedding: None,
        }
    }

    /// Create a builder for more complex memory creation
    pub fn builder(id: String, content: String) -> MemoryBuilder {
        MemoryBuilder::new(id, content)
    }

    /// Record an access to this memory
    pub fn record_access(&mut self) {
        self.last_accessed = Some(Utc::now());
        self.access_count += 1;
    }

    /// Add a tag to this memory
    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }

    /// Add a related memory reference
    pub fn add_related_memory(&mut self, memory_id: &str) {
        if !self.related_memories.contains(&memory_id.to_string()) {
            self.related_memories.push(memory_id.to_string());
        }
    }

    /// Set a property value
    pub fn set_property(&mut self, key: &str, value: serde_json::Value) {
        if let serde_json::Value::Object(ref mut map) = self.properties {
            map.insert(key.to_string(), value);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value);
            self.properties = serde_json::Value::Object(map);
        }
    }

    /// Set the embedding vector for this memory
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Check if this memory has an embedding
    pub fn has_embedding(&self) -> bool {
        self.embedding.is_some()
    }
}

/// Builder for creating Memory instances
pub struct MemoryBuilder {
    memory: Memory,
}

impl MemoryBuilder {
    /// Create a new memory builder with a specified ID
    ///
    /// Note: In most cases, you should use `new_with_content` instead to generate an ID automatically
    pub fn new(id: String, content: String) -> Self {
        Self {
            memory: Memory::new(id, content, MemoryType::Fact),
        }
    }

    /// Create a new memory builder with an auto-generated UUID
    ///
    /// This is the recommended way to create new memories
    pub fn new_with_content<S: Into<String>>(content: S) -> Self {
        Self::new(Uuid::new_v4().to_string(), content.into())
    }

    /// Create a fact memory (convenience method)
    pub fn fact<S: Into<String>>(content: S) -> Self {
        Self::new_with_content(content.into()).memory_type(MemoryType::Fact)
    }

    /// Create a conversation memory (convenience method)
    pub fn conversation<S: Into<String>>(content: S) -> Self {
        Self::new_with_content(content.into()).memory_type(MemoryType::Conversation)
    }

    /// Create an episodic memory (convenience method)
    pub fn episodic<S: Into<String>>(content: S) -> Self {
        Self::new_with_content(content.into()).memory_type(MemoryType::Episodic)
    }

    /// Create a procedural memory (convenience method)
    pub fn procedural<S: Into<String>>(content: S) -> Self {
        Self::new_with_content(content.into()).memory_type(MemoryType::Procedural)
    }

    /// Create an identity memory (convenience method)
    pub fn identity<S: Into<String>>(content: S) -> Self {
        Self::new_with_content(content.into())
            .memory_type(MemoryType::Identity)
            .priority(MemoryPriority::High)
    }

    /// Create a world knowledge memory (convenience method)
    pub fn world<S: Into<String>>(content: S) -> Self {
        Self::new_with_content(content.into()).memory_type(MemoryType::World)
    }

    /// Create an action memory (convenience method)
    pub fn action<S: Into<String>>(content: S) -> Self {
        Self::new_with_content(content.into()).memory_type(MemoryType::Action)
    }

    /// Create an event memory (convenience method)
    pub fn event<S: Into<String>>(content: S) -> Self {
        Self::new_with_content(content.into()).memory_type(MemoryType::Event)
    }

    /// Set the memory type
    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory.memory_type = memory_type;
        self
    }

    /// Set the memory priority
    pub fn priority(mut self, priority: MemoryPriority) -> Self {
        self.memory.priority = priority;
        self
    }

    /// Set memory priority to low (convenience method)
    pub fn low_priority(mut self) -> Self {
        self.memory.priority = MemoryPriority::Low;
        self
    }

    /// Set memory priority to normal (convenience method)
    pub fn normal_priority(mut self) -> Self {
        self.memory.priority = MemoryPriority::Normal;
        self
    }

    /// Set memory priority to high (convenience method)
    pub fn high_priority(mut self) -> Self {
        self.memory.priority = MemoryPriority::High;
        self
    }

    /// Set memory priority to critical (convenience method)
    pub fn critical_priority(mut self) -> Self {
        self.memory.priority = MemoryPriority::Critical;
        self
    }

    /// Set the memory source
    pub fn source<S: Into<String>>(mut self, source: S) -> Self {
        self.memory.source = source.into();
        self
    }

    /// Add tags to the memory
    pub fn tags(mut self, tags: Vec<&str>) -> Self {
        self.memory.tags = tags.iter().map(|t| t.to_string()).collect();
        self
    }

    /// Set a single tag on the memory (convenience method)
    pub fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.memory.tags.push(tag.into());
        self
    }

    /// Set the expiration date
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.memory.expires_at = Some(expires_at);
        self
    }

    /// Set properties
    pub fn properties(mut self, properties: HashMap<&str, serde_json::Value>) -> Self {
        let mut map = serde_json::Map::new();
        for (k, v) in properties {
            map.insert(k.to_string(), v);
        }
        self.memory.properties = serde_json::Value::Object(map);
        self
    }

    /// Set a single property (convenience method)
    pub fn property(mut self, key: &str, value: serde_json::Value) -> Self {
        self.memory.set_property(key, value);
        self
    }

    /// Set the embedding vector
    pub fn embedding(mut self, embedding: Vec<f32>) -> Self {
        self.memory.embedding = Some(embedding);
        self
    }

    /// Build the final Memory instance
    pub fn build(self) -> Memory {
        self.memory
    }
}
