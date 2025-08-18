//! Conversation History Management Example
//!
//! This example demonstrates how to use Locai to manage conversation history
//! and maintain context across multiple turns in a conversation. It shows
//! techniques for storing, retrieving, and organizing conversational memories
//! to enable context-aware responses.
//!
//! Key features:
//! - Conversation thread management
//! - Context-aware memory retrieval
//! - Conversation summarization
//! - Memory pruning and optimization
//! - Multi-participant conversation support
//!
//! To run this example:
//! ```bash
//! cargo run --example conversation_history --features "indradb lancedb"
//! ```

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use locai::config::ConfigBuilder;
use locai::core::MemoryManager;
use locai::memory::search_extensions::SearchMode;
use locai::models::{Memory, MemoryPriority, MemoryType};
use locai::prelude::*;

// Represents a single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub conversation_id: String,
    pub participant_id: String,
    pub participant_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub message_type: MessageType,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    User,
    Assistant,
    System,
    Summary,
}

// Represents a conversation thread
#[derive(Debug, Clone)]
pub struct ConversationThread {
    pub id: String,
    pub title: String,
    pub participants: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub message_count: usize,
    pub summary: Option<String>,
}

// Configuration for conversation management
#[derive(Debug, Clone)]
pub struct ConversationConfig {
    pub max_context_messages: usize,
    pub max_memory_age_days: i64,
    pub summarization_threshold: usize,
    pub context_retrieval_limit: usize,
    pub enable_auto_summarization: bool,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            max_context_messages: 20,
            max_memory_age_days: 30,
            summarization_threshold: 50,
            context_retrieval_limit: 10,
            enable_auto_summarization: true,
        }
    }
}

// Main conversation manager
pub struct ConversationManager {
    memory_manager: Arc<MemoryManager>,
    config: ConversationConfig,
    active_conversations: HashMap<String, ConversationThread>,
}

impl ConversationManager {
    pub fn new(memory_manager: Arc<MemoryManager>, config: ConversationConfig) -> Self {
        Self {
            memory_manager,
            config,
            active_conversations: HashMap::new(),
        }
    }

    /// Create a new conversation thread
    pub async fn create_conversation(
        &mut self,
        title: &str,
        participants: Vec<String>,
    ) -> Result<String> {
        let conversation_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let thread = ConversationThread {
            id: conversation_id.clone(),
            title: title.to_string(),
            participants: participants.clone(),
            created_at: now,
            last_activity: now,
            message_count: 0,
            summary: None,
        };

        // Store conversation metadata as a memory
        let metadata_content = format!(
            "Conversation '{}' created with participants: {}",
            title,
            participants.join(", ")
        );

        self.memory_manager
            .add_memory_with_options(metadata_content, |builder| {
                builder
                    .memory_type(MemoryType::Conversation)
                    .priority(MemoryPriority::Normal)
                    .tag("conversation_metadata")
                    .tag(&conversation_id)
                    .tag("conversation_start")
            })
            .await?;

        self.active_conversations
            .insert(conversation_id.clone(), thread);

        println!(
            "📝 Created conversation '{}' with ID: {}",
            title, conversation_id
        );
        Ok(conversation_id)
    }

    /// Add a message to a conversation
    pub async fn add_message(
        &mut self,
        conversation_id: &str,
        participant_id: &str,
        participant_name: &str,
        content: &str,
        message_type: MessageType,
    ) -> Result<String> {
        let message_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let _message = ConversationMessage {
            id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            participant_id: participant_id.to_string(),
            participant_name: participant_name.to_string(),
            content: content.to_string(),
            timestamp: now,
            message_type: message_type.clone(),
            metadata: HashMap::new(),
        };

        // Store message as memory
        let memory_content = format!("{}: {}", participant_name, content);
        let memory_id = self
            .memory_manager
            .add_memory_with_options(memory_content, |builder| {
                let mut b = builder
                    .memory_type(MemoryType::Conversation)
                    .priority(MemoryPriority::Normal)
                    .tag("conversation_message")
                    .tag(conversation_id)
                    .tag(participant_id)
                    .tag(&format!("message_type_{:?}", message_type).to_lowercase());

                // Add semantic tags based on content
                if content.contains("question") || content.contains("?") {
                    b = b.tag("question");
                }
                if content.contains("important") || content.contains("urgent") {
                    b = b.tag("important");
                }

                b
            })
            .await?;

        // Update conversation thread
        if let Some(thread) = self.active_conversations.get_mut(conversation_id) {
            thread.last_activity = now;
            thread.message_count += 1;

            // Check if summarization is needed
            if self.config.enable_auto_summarization
                && thread.message_count % self.config.summarization_threshold == 0
            {
                self.summarize_conversation(conversation_id).await?;
            }
        }

        println!(
            "💬 Added message from {} to conversation {}",
            participant_name, conversation_id
        );
        Ok(memory_id)
    }

    /// Retrieve conversation context for generating responses
    pub async fn get_conversation_context(
        &self,
        conversation_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>> {
        let limit = limit.unwrap_or(self.config.max_context_messages);

        println!("🔍 Retrieving conversation context (limit: {})", limit);

        // Search for recent messages in this conversation
        let search_query = format!("conversation_id:{}", conversation_id);
        let memories = self
            .memory_manager
            .search(&search_query, Some(limit), None, SearchMode::Text)
            .await?;

        let mut memories: Vec<Memory> = memories.into_iter().map(|r| r.memory).collect();

        // Sort by creation time (most recent first)
        memories.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        println!("  Found {} context messages", memories.len());
        Ok(memories)
    }

    /// Get relevant memories based on current conversation topic
    pub async fn get_relevant_memories(
        &self,
        conversation_id: &str,
        query: &str,
    ) -> Result<Vec<Memory>> {
        println!("🔍 Searching for relevant memories for query: {}", query);

        // First, get memories from this conversation
        let conversation_memories = self
            .get_conversation_context(conversation_id, Some(5))
            .await?;

        // Then, search for semantically similar memories across all conversations
        let relevant_memories = self
            .memory_manager
            .search(
                query,
                Some(self.config.context_retrieval_limit),
                None,
                SearchMode::Text,
            )
            .await?;

        let mut all_memories = conversation_memories;
        for result in relevant_memories {
            // Avoid duplicates
            if !all_memories.iter().any(|m| m.id == result.memory.id) {
                all_memories.push(result.memory);
            }
        }

        println!("  Found {} relevant memories", all_memories.len());
        Ok(all_memories)
    }

    /// Summarize a conversation
    pub async fn summarize_conversation(&mut self, conversation_id: &str) -> Result<String> {
        println!("📋 Summarizing conversation {}", conversation_id);

        let context = self.get_conversation_context(conversation_id, None).await?;

        if context.is_empty() {
            return Ok("No messages to summarize".to_string());
        }

        // Create a simple summary (in a real implementation, you'd use an LLM)
        let message_count = context.len();
        let participants: std::collections::HashSet<String> = context
            .iter()
            .filter_map(|m| {
                // Extract participant name from content (format: "Name: message")
                m.content.split(':').next().map(|s| s.trim().to_string())
            })
            .collect();

        let summary = format!(
            "Conversation summary: {} messages exchanged between {} participants. Topics discussed include various subjects based on message content.",
            message_count,
            participants.len()
        );

        // Store summary as a memory
        let summary_id = self
            .memory_manager
            .add_memory_with_options(summary.clone(), |builder| {
                builder
                    .memory_type(MemoryType::Conversation)
                    .priority(MemoryPriority::High)
                    .tag("conversation_summary")
                    .tag(conversation_id)
                    .tag("summary")
            })
            .await?;

        // Update conversation thread
        if let Some(thread) = self.active_conversations.get_mut(conversation_id) {
            thread.summary = Some(summary.clone());
        }

        println!("  Summary created with ID: {}", summary_id);
        Ok(summary)
    }

    /// Prune old conversation memories
    pub async fn prune_old_memories(&self) -> Result<usize> {
        println!("🧹 Pruning old conversation memories...");

        let _cutoff_date = Utc::now() - ChronoDuration::days(self.config.max_memory_age_days);

        // In a real implementation, you'd query for old memories and delete them
        // For this example, we'll just return a mock count
        let pruned_count = 0; // Placeholder

        println!("  Pruned {} old memories", pruned_count);
        Ok(pruned_count)
    }

    /// Get conversation statistics
    pub async fn get_conversation_stats(&self, conversation_id: &str) -> Result<ConversationStats> {
        let context = self.get_conversation_context(conversation_id, None).await?;

        let mut participant_counts: HashMap<String, usize> = HashMap::new();
        let mut total_words = 0;

        for memory in &context {
            // Extract participant name and count words
            if let Some((participant, message)) = memory.content.split_once(':') {
                let participant = participant.trim().to_string();
                *participant_counts.entry(participant).or_insert(0) += 1;
                total_words += message.split_whitespace().count();
            }
        }

        let thread = self.active_conversations.get(conversation_id);

        Ok(ConversationStats {
            conversation_id: conversation_id.to_string(),
            total_messages: context.len(),
            total_words,
            participant_counts,
            duration_hours: thread
                .map(|t| (t.last_activity - t.created_at).num_hours() as f64)
                .unwrap_or(0.0),
            has_summary: thread.and_then(|t| t.summary.as_ref()).is_some(),
        })
    }

    /// List all active conversations
    pub fn list_conversations(&self) -> Vec<&ConversationThread> {
        self.active_conversations.values().collect()
    }
}

#[derive(Debug)]
pub struct ConversationStats {
    pub conversation_id: String,
    pub total_messages: usize,
    pub total_words: usize,
    pub participant_counts: HashMap<String, usize>,
    pub duration_hours: f64,
    pub has_summary: bool,
}

/// Demonstrate conversation history management
async fn demonstrate_conversation_management() -> Result<()> {
    println!("🎯 Conversation History Management Demo");
    println!("=====================================");

    // Initialize Locai
    let config = ConfigBuilder::new()
        .with_default_storage()
        .with_default_ml()
        .with_data_dir("./data/conversation_demo")
        .build()?;

    let memory_manager = Arc::new(init(config).await?);

    // Initialize conversation manager
    let conv_config = ConversationConfig {
        max_context_messages: 10,
        max_memory_age_days: 7,
        summarization_threshold: 5, // Summarize after 5 messages for demo
        context_retrieval_limit: 5,
        enable_auto_summarization: true,
    };

    let mut conv_manager = ConversationManager::new(memory_manager, conv_config);

    // Create a conversation
    let conversation_id = conv_manager
        .create_conversation(
            "AI Discussion",
            vec!["user_1".to_string(), "assistant".to_string()],
        )
        .await?;

    // Simulate a conversation
    println!("\n💬 Simulating conversation...");

    let conversation_turns = vec![
        (
            "user_1",
            "Alice",
            "Hello! I'm interested in learning about artificial intelligence.",
            MessageType::User,
        ),
        (
            "assistant",
            "AI Assistant",
            "Hello Alice! I'd be happy to help you learn about AI. What specific aspects interest you most?",
            MessageType::Assistant,
        ),
        (
            "user_1",
            "Alice",
            "I'm curious about machine learning and how it differs from traditional programming.",
            MessageType::User,
        ),
        (
            "assistant",
            "AI Assistant",
            "Great question! Traditional programming involves writing explicit instructions, while machine learning allows computers to learn patterns from data.",
            MessageType::Assistant,
        ),
        (
            "user_1",
            "Alice",
            "That's fascinating! Can you give me an example of machine learning in everyday life?",
            MessageType::User,
        ),
        (
            "assistant",
            "AI Assistant",
            "Certainly! Email spam filters are a common example. They learn to identify spam by analyzing patterns in millions of emails.",
            MessageType::Assistant,
        ),
        (
            "user_1",
            "Alice",
            "I see! What about deep learning? How does that fit into machine learning?",
            MessageType::User,
        ),
    ];

    for (participant_id, participant_name, content, msg_type) in conversation_turns {
        conv_manager
            .add_message(
                &conversation_id,
                participant_id,
                participant_name,
                content,
                msg_type,
            )
            .await?;

        // Small delay for realism
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Get conversation context
    println!("\n📖 Retrieving conversation context...");
    let context = conv_manager
        .get_conversation_context(&conversation_id, Some(5))
        .await?;
    for (i, memory) in context.iter().enumerate() {
        println!("  {}. {}", i + 1, memory.content);
    }

    // Search for relevant memories
    println!("\n🔍 Searching for relevant memories about 'deep learning'...");
    let relevant = conv_manager
        .get_relevant_memories(&conversation_id, "deep learning")
        .await?;
    for (i, memory) in relevant.iter().enumerate() {
        println!("  {}. {}", i + 1, memory.content);
    }

    // Get conversation statistics
    println!("\n📊 Conversation statistics:");
    let stats = conv_manager
        .get_conversation_stats(&conversation_id)
        .await?;
    println!("  Total messages: {}", stats.total_messages);
    println!("  Total words: {}", stats.total_words);
    println!("  Duration: {:.1} hours", stats.duration_hours);
    println!("  Has summary: {}", stats.has_summary);
    println!("  Participants:");
    for (participant, count) in stats.participant_counts {
        println!("    {}: {} messages", participant, count);
    }

    // List all conversations
    println!("\n📋 Active conversations:");
    let conversations = conv_manager.list_conversations();
    for conv in conversations {
        println!(
            "  - {} (ID: {}, {} messages)",
            conv.title, conv.id, conv.message_count
        );
    }

    // Create another conversation to demonstrate multi-conversation management
    println!("\n🆕 Creating second conversation...");
    let conv2_id = conv_manager
        .create_conversation(
            "Technical Support",
            vec!["user_2".to_string(), "support_agent".to_string()],
        )
        .await?;

    conv_manager
        .add_message(
            &conv2_id,
            "user_2",
            "Bob",
            "I'm having trouble with my account login.",
            MessageType::User,
        )
        .await?;

    conv_manager
        .add_message(
            &conv2_id,
            "support_agent",
            "Support Agent",
            "I can help you with that. Can you tell me what error message you're seeing?",
            MessageType::Assistant,
        )
        .await?;

    // Show updated conversation list
    println!("\n📋 Updated conversation list:");
    let conversations = conv_manager.list_conversations();
    for conv in conversations {
        println!(
            "  - {} (ID: {}, {} messages)",
            conv.title, conv.id, conv.message_count
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    if let Err(e) = demonstrate_conversation_management().await {
        eprintln!("❌ Demo failed: {}", e);
        std::process::exit(1);
    }

    println!("\n🎉 Conversation history management demo completed successfully!");
    Ok(())
}
