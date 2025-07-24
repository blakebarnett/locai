//! Example demonstrating the relationship enrichment callback system
//! 
//! This shows how users can provide their own sentiment analysis, emotion detection,
//! or any other enrichment logic for relationship events.

use locai::{Locai, MemoryBuilder};
use locai::relationships::RelationshipManager;
use std::collections::HashMap;
use serde_json;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize Locai with embedded storage
    let locai = Locai::new().await?;
    
    // Create a relationship manager with custom enrichment callback
    let relationship_manager = RelationshipManager::new(locai.memory_manager().clone()).await?
        .with_enrichment_callback(|action: &str, context: &str, other_entity: &str| {
            let mut enrichment = HashMap::new();
            
            println!("🔍 Enriching action: '{}' towards '{}' in context: '{}'", action, other_entity, context);
            
            // Example: Custom sentiment analysis
            let sentiment = if action.contains("help") || action.contains("support") || action.contains("praise") {
                "positive"
            } else if action.contains("attack") || action.contains("insult") || action.contains("betray") {
                "negative"
            } else {
                "neutral"
            };
            
            enrichment.insert("sentiment".to_string(), serde_json::json!(sentiment));
            
            // Example: Custom emotion detection
            let emotion = if action.contains("love") {
                "affection"
            } else if action.contains("angry") {
                "anger"
            } else if action.contains("help") {
                "compassion"
            } else {
                "neutral"
            };
            
            enrichment.insert("emotion".to_string(), serde_json::json!(emotion));
            
            // Example: Custom categorization
            let category = if action.contains("work") || action.contains("project") {
                "professional"
            } else if action.contains("help") || action.contains("support") {
                "supportive"
            } else if action.contains("fight") || action.contains("argue") {
                "conflict"
            } else {
                "social"
            };
            
            enrichment.insert("category".to_string(), serde_json::json!(category));
            
            // Example: Confidence scores
            enrichment.insert("confidence".to_string(), serde_json::json!(0.85));
            
            println!("✅ Enrichment data: {:?}", enrichment);
            enrichment
        });
    
    // Initialize relationships
    println!("🤝 Initializing relationships...");
    relationship_manager.initialize_relationship("Alice", "Bob").await?;
    
    // Process some actions with enrichment
    println!("\n📖 Processing relationship events...");
    
    relationship_manager.process_entity_action(
        "Alice", 
        "helped Bob with his work project", 
        &["Bob".to_string()], 
        "At the office during a busy deadline"
    ).await?;
    
    relationship_manager.process_entity_action(
        "Bob", 
        "thanked Alice and praised her expertise", 
        &["Alice".to_string()], 
        "After completing the project successfully"
    ).await?;
    
    relationship_manager.process_entity_action(
        "Alice", 
        "argued with Bob about the approach", 
        &["Bob".to_string()], 
        "During a heated team meeting"
    ).await?;
    
    // Get relationship summary
    println!("\n📊 Relationship Summary:");
    let summary = relationship_manager.get_relationship_summary("Alice", "Bob").await?;
    println!("{}", summary);
    
    println!("\n✨ Example complete! The enrichment callback allowed us to:");
    println!("  • Add custom sentiment analysis");
    println!("  • Detect emotions beyond just positive/negative");
    println!("  • Categorize actions by type");
    println!("  • Include confidence scores");
    println!("  • Store all this data as metadata in relationship events");
    
    Ok(())
} 