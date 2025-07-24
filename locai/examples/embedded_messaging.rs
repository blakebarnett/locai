//! Embedded messaging example
//!
//! This example demonstrates the embedded messaging system using shared MemoryManager instances
//! for real-time intra-process communication in a D&D-like scenario.
//!
//! IMPORTANT: This demonstrates communication between multiple agents within a SINGLE process.
//! True inter-process communication requires SurrealDB server mode (planned for MSG-003).

use locai::prelude::*;
use locai::messaging::{LocaiMessaging, MessageBuilder, MessageFilter};
use serde_json::json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let _ = tracing_subscriber::fmt::init();

    println!("🎮 Locai Embedded Messaging Example");
    println!("=====================================");
    println!("📝 Note: This demonstrates intra-process communication");
    println!("   Multiple agents within the same process only");
    
    // Create shared MemoryManager instance for the campaign
    println!("📚 Creating shared campaign database...");
    let shared_memory = Arc::new(init_with_defaults().await?);
    
    // Create messaging instances for different agents (same process)
    println!("⚡ Setting up messaging instances...");
    
    let game_master = LocaiMessaging::embedded(
        shared_memory.clone(), 
        "game_master".to_string()
    ).await?;
    
    let player_elara = LocaiMessaging::embedded(
        shared_memory.clone(), 
        "player_elara".to_string()
    ).await?;
    
    let player_thorne = LocaiMessaging::embedded(
        shared_memory.clone(), 
        "player_thorne".to_string()
    ).await?;
    
    // Example 1: Basic messaging
    println!("\n🎯 Example 1: Basic Messaging");
    println!("==============================");
    
    // GM sends a narrative message
    let narrative_msg = game_master.send(
        "gm.narration",
        json!({
            "text": "As you enter the ancient temple, the air grows thick with an otherworldly presence...",
            "scene": "temple_entrance",
            "atmosphere": "mysterious"
        })
    ).await?;
    
    println!("📜 GM sent narrative: {}", narrative_msg);
    
    // Player sends an action
    let action_msg = player_elara.send(
        "character.action",
        json!({
            "character": "Elara",
            "action": "investigate",
            "target": "ancient_runes",
            "description": "Elara carefully examines the runes carved into the temple wall"
        })
    ).await?;
    
    println!("⚔️  Elara sent action: {}", action_msg);
    
    // Example 2: Targeted messaging with headers
    println!("\n🎯 Example 2: Targeted Messaging");
    println!("================================");
    
    let whisper = MessageBuilder::new(
        "character.dialogue.whisper",
        "player_elara",
        json!({
            "character": "Elara",
            "text": "Thorne, do you see those symbols? They look familiar...",
            "target": "Thorne"
        })
    )
    .recipient("player_thorne")
    .header("priority", "normal")
    .header("visibility", "private")
    .tag("roleplay")
    .tag("dialogue")
    .importance(0.7)
    .build();
    
    let whisper_id = player_elara.send_with_options(whisper).await?;
    println!("🤫 Elara whispered to Thorne: {}", whisper_id);
    
    // Example 3: Message filtering and history
    println!("\n🎯 Example 3: Message History and Filtering");
    println!("===========================================");
    
    // Add a few more messages to show filtering
    game_master.send(
        "world.event",
        json!({
            "event": "sound",
            "description": "A low rumbling echoes through the temple corridors",
            "intensity": "medium"
        })
    ).await?;
    
    player_thorne.send(
        "character.action",
        json!({
            "character": "Thorne", 
            "action": "ready_weapon",
            "weapon": "silver_sword",
            "description": "Thorne draws his silver sword, its blade gleaming in the dim light"
        })
    ).await?;
    
    // Give some time for messages to be stored
    sleep(Duration::from_millis(100)).await;
    
    // Get all messages from GM
    println!("\n📋 All messages from Game Master:");
    let gm_filter = MessageFilter::new()
        .senders(vec!["game_master".to_string()]);
    
    let gm_messages = game_master.get_message_history(Some(gm_filter), Some(10)).await?;
    for (i, msg) in gm_messages.iter().enumerate() {
        println!("  {}. [{}] {}", i + 1, msg.topic, 
            msg.content.get("text").or(msg.content.get("description"))
                .unwrap_or(&json!("No description")).as_str().unwrap_or(""));
    }
    
    // Get all character actions
    println!("\n⚔️  All character actions:");
    let action_filter = MessageFilter::new()
        .topic_patterns(vec!["*.character.action".to_string()]);
    
    let action_messages = shared_memory.get_message_history(&action_filter, Some(10)).await?;
    for (i, msg) in action_messages.iter().enumerate() {
        println!("  {}. [{}] {} - {}", i + 1, 
            msg.content.get("character").unwrap_or(&json!("Unknown")).as_str().unwrap_or("Unknown"),
            msg.content.get("action").unwrap_or(&json!("")).as_str().unwrap_or(""),
            msg.content.get("description").unwrap_or(&json!("")).as_str().unwrap_or(""));
    }
    
    // Example 4: Cross-agent relationship queries (within same process)
    println!("\n🎯 Example 4: Cross-Agent Relationships");
    println!("======================================");
    
    // Query agent interactions (this demonstrates the shared database capability)
    if let Some(memory_manager) = game_master.memory_manager() {
        let interactions = memory_manager.get_process_interactions("game_master").await?;
        println!("🔗 Game Master agent interactions: {} relationships", interactions.len());
        
        let elara_interactions = memory_manager.get_process_interactions("player_elara").await?;
        println!("🔗 Player Elara agent interactions: {} relationships", elara_interactions.len());
    }
    
    // Example 5: Message importance and tags
    println!("\n🎯 Example 5: Message Importance and Tags");
    println!("=========================================");
    
    // Send a critical event
    let critical_event = MessageBuilder::new(
        "world.event.critical",
        "game_master",
        json!({
            "event": "trap_activation",
            "description": "The floor beneath your feet suddenly gives way! A hidden pit trap opens!",
            "danger_level": "high",
            "requires_saves": true
        })
    )
    .header("priority", "urgent")
    .tag("combat")
    .tag("trap")
    .tag("critical")
    .importance(0.95)
    .expires_at(chrono::Utc::now() + chrono::Duration::minutes(5))
    .build();
    
    let critical_id = game_master.send_with_options(critical_event).await?;
    println!("🚨 Critical event sent: {}", critical_id);
    
    // Filter for high importance messages
    let important_filter = MessageFilter::new()
        .importance_range(0.8, 1.0)
        .tags(vec!["critical".to_string()]);
    
    let important_messages = game_master.get_message_history(Some(important_filter), Some(5)).await?;
    println!("⚠️  High importance messages: {}", important_messages.len());
    for msg in important_messages {
        println!("  • [{}] Importance: {:.2} - {}", 
            msg.topic,
            msg.importance.unwrap_or(0.0),
            msg.content.get("description").unwrap_or(&json!("")).as_str().unwrap_or(""));
    }
    
    // Example 6: Topic patterns and wildcards
    println!("\n🎯 Example 6: Topic Patterns and Wildcards");
    println!("==========================================");
    
    // Send various dialogue types
    player_elara.send("character.dialogue.say", json!({
        "character": "Elara",
        "text": "Everyone be careful! This place is full of traps!"
    })).await?;
    
    player_thorne.send("character.dialogue.shout", json!({
        "character": "Thorne", 
        "text": "I see the exit! This way!"
    })).await?;
    
    // Filter for all dialogue
    let dialogue_filter = MessageFilter::new()
        .topic_patterns(vec!["*.character.dialogue.*".to_string()]);
    
    let dialogue_messages = game_master.get_message_history(Some(dialogue_filter), Some(10)).await?;
    println!("💬 All dialogue messages: {}", dialogue_messages.len());
    for msg in dialogue_messages {
        let dialogue_type = msg.topic.split('.').last().unwrap_or("unknown");
        println!("  • [{}] {}: \"{}\"",
            dialogue_type,
            msg.content.get("character").unwrap_or(&json!("Unknown")).as_str().unwrap_or("Unknown"),
            msg.content.get("text").unwrap_or(&json!("")).as_str().unwrap_or(""));
    }
    
    println!("\n✅ Embedded messaging example completed!");
    println!("🎉 The messaging system successfully demonstrated:");
    println!("   • Shared MemoryManager for unified campaign data");
    println!("   • Topic-based message routing");
    println!("   • Message filtering and pattern matching");
    println!("   • Cross-agent relationship tracking (single process)");
    println!("   • Message importance and tagging");
    println!("   • Message history and retrieval");
    println!("\n📝 Note: This demonstrates intra-process communication.");
    println!("   For true inter-process communication, use SurrealDB server mode.");
    
    Ok(())
}

/// Utility function to demonstrate subscription capabilities
/// Note: This would require actual live query implementation
#[allow(dead_code)]
async fn demonstrate_subscriptions() -> Result<()> {
    println!("\n🎯 Live Subscriptions (Placeholder)");
    println!("===================================");
    
    let shared_memory = Arc::new(init_with_defaults().await?);
    let _subscriber = LocaiMessaging::embedded(shared_memory, "subscriber".to_string()).await?;
    
    // This would demonstrate live subscriptions
    // let mut stream = subscriber.subscribe("character.*").await?;
    // while let Some(message) = stream.next().await {
    //     match message {
    //         Ok(msg) => println!("📨 Received: {}", msg.topic),
    //         Err(e) => println!("❌ Error: {}", e),
    //     }
    // }
    
    println!("🔄 Live subscriptions would work here with full live query implementation");
    Ok(())
} 