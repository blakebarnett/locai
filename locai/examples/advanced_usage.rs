//! Advanced usage example for Locai

use locai::prelude::*;
use locai::models::MemoryBuilder;
use locai::models::MemoryType;
use locai::models::MemoryPriority;
use locai::storage::filters::{MemoryFilter, SortOrder};
use locai::memory::search_extensions::SearchMode;

use locai::config::ConfigBuilder;
use tracing::{info, error};

use uuid::Uuid;
use chrono::{Utc, TimeZone};
use serde_json::json;
use std::collections::HashMap;
use futures::stream::{self, StreamExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a custom configuration
    let config = ConfigBuilder::new()
        // Storage configuration
        .with_default_storage()
        .with_log_level(LogLevel::Debug)
        .build()?;
    
    // Initialize Locai
    let memory_manager = init(config).await?;
    
    info!("Locai initialized with custom configuration");
    
    // Batch insert memories
    info!("Performing batch memory insertion");
    let memories = create_batch_memories(50);
    let memory_ids = insert_memories_in_parallel(&memory_manager, memories).await?;
    
    info!("Successfully inserted {} memories", memory_ids.len());
    
    // Create complex relationships between memories
    info!("Creating memory relationships");
    create_relationships(&memory_manager, &memory_ids).await?;
    
    // Perform a complex search
    let query = "neural networks and machine learning";
    info!("Searching for: {}", query);
    
    let search_results = memory_manager.search(query, Some(10), None, SearchMode::Text).await?;
    let results: Vec<Memory> = search_results.into_iter().map(|sr| sr.memory).collect();
    
    println!("Search results for '{}' (found {} memories):", query, results.len());
    for (i, memory) in results.iter().enumerate() {
        info!("Result {}: {} (priority: {:?})", i + 1, memory.content, memory.priority);
    }
    
    // Advanced filtering
    let yesterday = Utc::now().date_naive().pred_opt().unwrap().and_hms_opt(0, 0, 0).unwrap();
    let yesterday = Utc.from_utc_datetime(&yesterday);
    
    let mut filter = MemoryFilter::default();
    filter.created_after = Some(yesterday);
    
    let filtered = memory_manager.filter_memories(
        filter,
        Some("priority"),
        Some(SortOrder::Descending),
        Some(20)
    ).await?;
    
    info!("Found {} memories from advanced filtering", filtered.len());
    
    Ok(())
}

/// Creates a batch of test memories
fn create_batch_memories(count: usize) -> Vec<Memory> {
    let topics = [
        "artificial intelligence",
        "machine learning",
        "neural networks",
        "data science",
        "computer vision",
        "natural language processing",
        "reinforcement learning",
        "deep learning",
        "robotics",
        "knowledge graphs",
    ];
    
    let mut memories = Vec::with_capacity(count);
    
    for i in 0..count {
        let topic_idx = i % topics.len();
        let priority = match i % 4 {
            0 => MemoryPriority::Low,
            1 => MemoryPriority::Normal,
            2 => MemoryPriority::High,
            _ => MemoryPriority::Critical,
        };
        
        let content = format!(
            "Important facts about {}: item {} of information series.",
            topics[topic_idx], i
        );
        
        let mut properties = HashMap::new();
        properties.insert("sequence", json!(i));
        properties.insert("category", json!(topics[topic_idx]));
        
        let memory = MemoryBuilder::new(
            Uuid::new_v4().to_string(),
            content,
        )
        .memory_type(MemoryType::Fact)
        .priority(priority)
        .source("batch-example")
        .tags(vec!["ai", topics[topic_idx]])
        .properties(properties)
        .build();
        
        memories.push(memory);
    }
    
    memories
}

/// Insert memories in parallel using batched processing
async fn insert_memories_in_parallel(
    memory_manager: &MemoryManager,
    memories: Vec<Memory>
) -> Result<Vec<String>> {
    // Process in batches of 10 concurrently
    let mut memory_ids = Vec::with_capacity(memories.len());
    
    let results = stream::iter(memories)
        .map(|memory| {
            let mm = memory_manager;
            async move {
                let id = memory.id.clone();
                mm.store_memory(memory).await.map(|_| id)
            }
        })
        .buffer_unordered(10) // Process 10 at a time
        .collect::<Vec<Result<String>>>()
        .await;
    
    for result in results {
        match result {
            Ok(id) => memory_ids.push(id),
            Err(e) => error!("Failed to insert memory: {}", e),
        }
    }
    
    Ok(memory_ids)
}

/// Create relationships between memories in a meaningful pattern
async fn create_relationships(
    memory_manager: &MemoryManager,
    memory_ids: &[String]
) -> Result<()> {
    // Create a chain of "next" relationships
    for i in 0..memory_ids.len() - 1 {
        if i % 5 == 0 {
            // Every 5th item, create a "references" relationship to the next 3 items
            for j in 1..=3 {
                if i + j < memory_ids.len() {
                    memory_manager.create_relationship(
                        &memory_ids[i],
                        &memory_ids[i + j],
                        "references"
                    ).await?;
                }
            }
        } else {
            // Otherwise create a simple "next" relationship
            memory_manager.create_relationship(
                &memory_ids[i],
                &memory_ids[i + 1],
                "next"
            ).await?;
        }
    }
    
    // Create some "related" relationships between memories with similar indices
    for i in 0..memory_ids.len() {
        let related_idx = (i + 7) % memory_ids.len();
        memory_manager.create_relationship(
            &memory_ids[i],
            &memory_ids[related_idx],
            "related"
        ).await?;
    }
    
    Ok(())
} 