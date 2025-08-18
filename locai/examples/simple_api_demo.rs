//! # Simple API Demo
//!
//! This example demonstrates the new simplified Locai API that makes
//! memory management straightforward and intuitive.
//!
//! Run with: cargo run --example simple_api_demo

use locai::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with a clean level for demo purposes
    use locai::config::{LogFormat, LogLevel, LoggingConfig};
    use locai::logging;
    logging::init(&LoggingConfig {
        level: LogLevel::Info,
        format: LogFormat::Default,
        file: None,
        stdout: true,
    })
    .expect("Failed to initialize logging");

    println!("🧠 Locai Simple API Demo");
    println!("========================");

    // Phase 2: Dead simple initialization
    println!("\n📝 1. Initializing Locai (testing mode)...");
    let locai = Locai::for_testing().await?;
    println!("✅ Locai initialized successfully!");

    // Basic memory operations
    println!("\n📝 2. Storing memories with simple API...");

    // Simple memory storage
    let memory_id = locai
        .remember("I learned about the new Locai API today")
        .await?;
    println!("✅ Stored episodic memory: {}", memory_id);

    // Fact storage
    let fact_id = locai
        .remember_fact("Rust is a systems programming language")
        .await?;
    println!("✅ Stored fact: {}", fact_id);

    // Conversation storage
    let conversation_id = locai
        .remember_conversation(
            "User: How does the new API work?\nBot: It's much simpler! Just use locai.remember()",
        )
        .await?;
    println!("✅ Stored conversation: {}", conversation_id);

    // Advanced memory builder (Phase 2: Unified Memory API)
    println!("\n📝 3. Using advanced memory builder...");
    let advanced_id = locai
        .remember_with("Important AI breakthrough discovered")
        .as_fact()
        .with_priority(MemoryPriority::High)
        .with_tags(&["ai", "breakthrough", "research"])
        .save()
        .await?;
    println!(
        "✅ Stored advanced memory with tags and priority: {}",
        advanced_id
    );

    // Get recent memories
    println!("\n📝 4. Retrieving recent memories...");
    let recent = locai.recent_memories(Some(3)).await?;
    println!("✅ Found {} recent memories:", recent.len());
    for (i, memory) in recent.iter().enumerate() {
        println!("   {}. {} ({})", i + 1, memory.content, memory.memory_type);
    }

    // Search operations (Phase 3: Unified Search)
    println!("\n📝 5. Testing search capabilities...");

    // Check if semantic search is available
    if locai.has_semantic_search() {
        println!("✅ Semantic search is available!");
    } else {
        println!("ℹ️  Semantic search not available (using keyword search fallback)");
    }

    // Simple search
    println!("\n   🔍 Simple search for 'Rust':");
    match locai.search("Rust").await {
        Ok(results) => {
            println!("   ✅ Found {} results:", results.len());
            for (i, result) in results.iter().enumerate() {
                println!(
                    "      {}. {} (score: {:?})",
                    i + 1,
                    result.summary(),
                    result.score
                );
            }
        }
        Err(LocaiError::NoMemoriesFound) => {
            println!("   ℹ️  No memories found for 'Rust'");
        }
        Err(e) => {
            println!("   ❌ Search error: {}", e);
        }
    }

    // Advanced search builder
    println!("\n   🔍 Advanced search for facts with 'ai' tag:");
    match locai
        .search_for("ai")
        .limit(5)
        .of_type(MemoryType::Fact)
        .with_tags(&["ai"])
        .execute()
        .await
    {
        Ok(results) => {
            println!("   ✅ Found {} AI-related facts:", results.len());
            for (i, result) in results.iter().enumerate() {
                println!("      {}. {}", i + 1, result.content);
            }
        }
        Err(LocaiError::NoMemoriesFound) => {
            println!("   ℹ️  No AI-related facts found");
        }
        Err(e) => {
            println!("   ❌ Advanced search error: {}", e);
        }
    }

    // Error handling demonstration (Phase 4: Better Error Messages)
    println!("\n📝 6. Demonstrating improved error handling...");

    // Test empty search query
    match locai.search("").await {
        Err(LocaiError::EmptySearchQuery) => {
            println!("✅ Empty search query properly caught with helpful message");
        }
        _ => println!("❌ Expected empty search query error"),
    }

    // Builder pattern demonstration
    println!("\n📝 7. Advanced configuration with builder pattern...");
    let custom_locai = Locai::builder()
        .with_memory_storage()
        .with_defaults()
        .build()
        .await?;

    let builder_test_id = custom_locai
        .remember("Builder pattern works great!")
        .await?;
    println!("✅ Builder pattern memory stored: {}", builder_test_id);

    // Access to advanced features when needed
    println!("\n📝 8. Access to advanced features...");
    let advanced_manager = locai.manager();
    let storage_metadata = advanced_manager.storage().get_metadata().await?;
    println!(
        "✅ Advanced features accessible: storage type = {}",
        storage_metadata
            .get("storage_type")
            .unwrap_or(&"unknown".into())
    );

    println!("\n🎉 Demo completed successfully!");
    println!("\nKey improvements in the new API:");
    println!("• 🚀 Dead simple initialization: Locai::new() or Locai::for_testing()");
    println!("• 📝 Unified memory API: locai.remember(), remember_fact(), remember_with()");
    println!("• 🔍 Smart search: locai.search() with automatic fallback");
    println!("• 🏗️  Builder patterns for advanced configuration and search");
    println!("• 💡 Helpful error messages with recovery suggestions");
    println!("• 🎯 Progressive disclosure: simple by default, powerful when needed");

    Ok(())
}
