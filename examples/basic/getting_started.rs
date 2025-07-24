//! # Getting Started with Locai
//!
//! This example demonstrates the core functionality of Locai in the simplest way possible.
//! Perfect for new users who want to understand what Locai can do.
//!
//! Run with: cargo run --example getting_started

use tokio;
use locai::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Getting Started with Locai\n");

    // Step 1: Initialize Locai (dead simple!)
    println!("1️⃣ Initializing Locai...");
    let locai = Locai::new().await?;
    println!("   ✅ Locai memory system ready!\n");

    // Step 2: Store some memories
    println!("2️⃣ Storing memories...");
    
    // Simple general memory
    locai.remember("I'm learning how to use Locai today").await?;
    println!("   📝 Stored: Learning memory");
    
    // Fact with higher importance
    locai.remember_fact("Locai is a memory management system for AI").await?;
    println!("   🧠 Stored: Fact about Locai");
    
    // Conversation memory
    locai.remember_conversation(
                "User: What is Locai?\n\
         Bot: Locai is a memory system that helps AI agents remember and learn."
    ).await?;
    println!("   💬 Stored: Conversation\n");

    // Step 3: Search memories
    println!("3️⃣ Searching memories...");
    let results = locai.search("Locai").await?;
    
    println!("   🔍 Found {} memories about 'Locai':", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("      {}. {} (score: {:.2})", i+1, result.memory.content, result.score);
    }
    println!();

    // Step 4: Advanced memory storage
    println!("4️⃣ Advanced memory features...");
    locai.remember_with("This is an important technical concept")
        .as_fact()
        .with_priority(MemoryPriority::High)
        .with_tags(&["technical", "important"])
        .save().await?;
    println!("   ⭐ Stored: High-priority fact with tags\n");

    // Step 5: Show recent memories
    println!("5️⃣ Recent memories...");
    let recent = locai.recent_memories(Some(3)).await?;
    println!("   📋 Your {} most recent memories:", recent.len());
    for (i, memory) in recent.iter().enumerate() {
        println!("      {}. {}", i+1, memory.content);
    }

    println!("\n✨ That's it! You've learned the basics of Locai.");
    println!("\n💡 Next steps:");
    println!("   - Try the simple_api_demo for more features");
    println!("   - Check out examples/advanced/ for complex use cases");
    println!("   - Read the docs for configuration options");

    Ok(())
} 