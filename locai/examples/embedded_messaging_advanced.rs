//! Advanced embedded messaging example demonstrating production features
//!
//! This example shows how to use the production-ready embedded messaging system
//! with real-time message streaming and filtering capabilities.

use locai::prelude::*;
use locai::messaging::embedded::EmbeddedMessaging;
use locai::messaging::types::{Message, MessageFilter};
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 Starting Advanced Embedded Messaging Example");
    
    // Create memory manager using the default initialization
    let memory_manager = Arc::new(init_with_defaults().await?);
    
    // Create embedded messaging instance
    let mut messaging = EmbeddedMessaging::new(memory_manager.clone());
    
    // Initialize with live query support
    messaging.initialize().await?;
    
    println!("✅ Embedded messaging system initialized");
    
    // Demonstrate message sending and subscription
    demo_message_subscription(&messaging).await?;
    demo_filtered_messaging(&messaging).await?;
    demo_message_history(&messaging).await?;
    
    println!("🎉 Advanced embedded messaging example completed successfully!");
    
    Ok(())
}

/// Demonstrate real-time message subscription
async fn demo_message_subscription(messaging: &EmbeddedMessaging) -> Result<()> {
    println!("\n📡 Demonstrating real-time message subscription");
    
    // Create a filter for character action messages
    let filter = MessageFilter::new()
        .topic_patterns(vec!["character.*".to_string()])
        .senders(vec!["player1".to_string(), "gm".to_string()]);
    
    // Subscribe to filtered messages
    let mut message_stream = messaging.subscribe_filtered(filter).await?;
    
    // Send some messages in the background
    let messaging_clone = messaging.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let _ = messaging_clone.send_message(
            "dnd_session",
            "player1", 
            "character.action",
            json!({
                "action": "attack",
                "target": "goblin",
                "damage": 15
            })
        ).await;
        
        let _ = messaging_clone.send_message(
            "dnd_session",
            "gm",
            "character.status", 
            json!({
                "character": "Aragorn",
                "hp": 45,
                "status": "wounded"
            })
        ).await;
        
        let _ = messaging_clone.send_message(
            "dnd_session",
            "other_player",
            "character.action",
            json!({
                "action": "heal",
                "target": "Aragorn"
            })
        ).await;
    });
    
    // Listen for messages with timeout
    let mut received_count = 0;
    while received_count < 2 {
        match timeout(Duration::from_secs(2), message_stream.next()).await {
            Ok(Some(Ok(message))) => {
                println!("📨 Received message: {} from {} -> {}", 
                    message.id.as_str(), message.sender, message.topic);
                println!("   Content: {}", message.content);
                received_count += 1;
            }
            Ok(Some(Err(e))) => {
                println!("❌ Error receiving message: {}", e);
                break;
            }
            Ok(None) => {
                println!("📭 Message stream ended");
                break;
            }
            Err(_) => {
                println!("⏰ Timeout waiting for messages");
                break;
            }
        }
    }
    
    println!("✅ Received {} real-time messages", received_count);
    Ok(())
}

/// Demonstrate filtered messaging with different criteria
async fn demo_filtered_messaging(messaging: &EmbeddedMessaging) -> Result<()> {
    println!("\n🎯 Demonstrating filtered messaging");
    
    // Send messages with different priorities and tags
    let high_priority_msg = Message::new(
        "dnd_session.gm.announcement".to_string(),
        "gm".to_string(),
        json!({
            "announcement": "Initiative order has changed!",
            "round": 3
        })
    ).importance(0.9).add_tag("urgent").add_tag("combat");
    
    let normal_msg = Message::new(
        "dnd_session.player.dialogue".to_string(),
        "player2".to_string(),
        json!({
            "character": "Legolas",
            "speech": "I see movement in the trees..."
        })
    ).add_tag("roleplay");
    
    messaging.send_complete_message(high_priority_msg).await?;
    messaging.send_complete_message(normal_msg).await?;
    
    println!("✅ Sent messages with different priorities and tags");
    
    // Test different filters
    test_filter_by_importance(messaging).await?;
    test_filter_by_tags(messaging).await?;
    
    Ok(())
}

/// Test filtering by importance level
async fn test_filter_by_importance(messaging: &EmbeddedMessaging) -> Result<()> {
    println!("  🔍 Testing importance-based filtering");
    
    let filter = MessageFilter::new()
        .importance_range(0.8, 1.0); // High importance only
    
    let messages = messaging.get_message_history(Some(filter), Some(10)).await?;
    println!("  📊 Found {} high-importance messages", messages.len());
    
    for message in &messages {
        if let Some(importance) = message.importance {
            println!("    💎 Message {} has importance: {:.2}", message.id.as_str(), importance);
        }
    }
    
    Ok(())
}

/// Test filtering by tags
async fn test_filter_by_tags(messaging: &EmbeddedMessaging) -> Result<()> {
    println!("  🏷️  Testing tag-based filtering");
    
    let filter = MessageFilter::new()
        .tags(vec!["urgent".to_string()]);
    
    let messages = messaging.get_message_history(Some(filter), Some(10)).await?;
    println!("  📊 Found {} urgent messages", messages.len());
    
    for message in &messages {
        println!("    🚨 Urgent message: {} - {}", message.topic, 
            message.tags.join(", "));
    }
    
    Ok(())
}

/// Demonstrate message history retrieval
async fn demo_message_history(messaging: &EmbeddedMessaging) -> Result<()> {
    println!("\n📚 Demonstrating message history retrieval");
    
    // Send a few more messages to build history
    for i in 1..=3 {
        messaging.send_message(
            "dnd_session",
            &format!("player{}", i),
            "character.dialogue",
            json!({
                "character": format!("Character{}", i),
                "speech": format!("This is message number {}", i)
            })
        ).await?;
    }
    
    // Retrieve all message history
    let all_messages = messaging.get_message_history(None, Some(20)).await?;
    println!("📊 Total messages in history: {}", all_messages.len());
    
    // Show recent messages
    println!("📝 Recent messages:");
    for (i, message) in all_messages.iter().take(5).enumerate() {
        println!("  {}. {} from {} at {}", 
            i + 1, 
            message.topic, 
            message.sender,
            message.timestamp.format("%H:%M:%S")
        );
    }
    
    // Filter by specific sender
    let player_filter = MessageFilter::new()
        .senders(vec!["player1".to_string()]);
    
    let player_messages = messaging.get_message_history(Some(player_filter), Some(10)).await?;
    println!("📊 Messages from player1: {}", player_messages.len());
    
    Ok(())
} 