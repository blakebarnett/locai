//! Enhanced Search Scoring Example
//!
//! This example demonstrates how to use the enhanced search scoring system
//! with configurable scoring factors for different use cases.
//!
//! The example creates a set of memories with varying:
//! - Age (recency)
//! - Access frequency
//! - Priority levels
//!
//! Then searches the same query with different scoring configurations to show
//! how the ranking changes.

use locai::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Enhanced Search Scoring Example\n");

    // Initialize Locai
    let locai = Locai::new().await?;

    println!("📝 Creating memories with varying characteristics...\n");

    // Create a fresh, frequently accessed, high-priority memory
    let memory1 = MemoryBuilder::new_with_content(
        "Wizard aldric is a powerful magic user with extensive knowledge of arcane spells"
    )
    .priority(MemoryPriority::Critical)
    .tags(vec!["wizard", "magic", "characters"])
    .source("recent_interaction")
    .build();
    let id1 = locai.remember(memory1).await?;
    println!("✓ Created: Wizard Aldric (fresh, critical priority)");

    // Create an older, moderately accessed, normal priority memory
    let memory2 = MemoryBuilder::new_with_content(
        "Wizard in fantasy world teaches about elemental spells and transmutation magic"
    )
    .priority(MemoryPriority::Normal)
    .tags(vec!["wizard", "magic", "education"])
    .source("archive")
    .build();
    let id2 = locai.remember(memory2).await?;
    println!("✓ Created: Wizard Education (older, normal priority)");

    // Create a very old, rarely accessed, low priority memory
    let memory3 = MemoryBuilder::new_with_content(
        "Historical records mention ancient wizards who discovered wizard-specific magic techniques"
    )
    .priority(MemoryPriority::Low)
    .tags(vec!["wizard", "history", "magic"])
    .source("historical_records")
    .build();
    let id3 = locai.remember(memory3).await?;
    println!("✓ Created: Historical Wizard Records (old, low priority)\n");

    println!("🎯 Searching with different scoring configurations...\n");
    println!("Query: 'wizard magic'\n");

    // Get the memories to show their characteristics
    let mem1 = locai.get_memory(&id1).await?.expect("Memory 1 not found");
    let mem2 = locai.get_memory(&id2).await?.expect("Memory 2 not found");
    let mem3 = locai.get_memory(&id3).await?.expect("Memory 3 not found");

    println!("Memory Characteristics:");
    println!("  ID1 - Priority: {:?}, Access Count: {}", mem1.priority, mem1.access_count);
    println!("  ID2 - Priority: {:?}, Access Count: {}", mem2.priority, mem2.access_count);
    println!("  ID3 - Priority: {:?}, Access Count: {}\n", mem3.priority, mem3.access_count);

    // Example 1: Default scoring (balanced)
    println!("1️⃣  DEFAULT SCORING (Balanced)");
    println!("   Configuration: BM25=1.0, Vector=1.0, Recency=0.5, Access=0.3, Priority=0.2");
    demonstrate_scoring(
        &locai,
        "wizard magic",
        Some(ScoringConfig::default()),
    ).await?;
    println!();

    // Example 2: Recency-focused (for active games)
    println!("2️⃣  RECENCY-FOCUSED (Active Games)");
    println!("   Configuration: Emphasizes recent memories");
    demonstrate_scoring(
        &locai,
        "wizard magic",
        Some(ScoringConfig::recency_focused()),
    ).await?;
    println!();

    // Example 3: Semantic-focused (when embeddings available)
    println!("3️⃣  SEMANTIC-FOCUSED (Semantic Matching)");
    println!("   Configuration: Vector weight=1.5, Text weight=0.3");
    demonstrate_scoring(
        &locai,
        "wizard magic",
        Some(ScoringConfig::semantic_focused()),
    ).await?;
    println!();

    // Example 4: Importance-focused (knowledge systems)
    println!("4️⃣  IMPORTANCE-FOCUSED (Knowledge Systems)");
    println!("   Configuration: High weight on access frequency and priority");
    demonstrate_scoring(
        &locai,
        "wizard magic",
        Some(ScoringConfig::importance_focused()),
    ).await?;
    println!();

    // Example 5: Custom scoring configuration
    println!("5️⃣  CUSTOM SCORING");
    let custom_config = ScoringConfig {
        bm25_weight: 2.0,
        vector_weight: 0.5,
        recency_boost: 0.0,    // No recency boost
        access_boost: 2.0,     // High weight on access frequency
        priority_boost: 0.5,   // Low weight on priority
        decay_function: DecayFunction::Linear,
        decay_rate: 0.05,
    };
    println!("   Configuration: Custom (Access frequency emphasis)");
    demonstrate_scoring(
        &locai,
        "wizard magic",
        Some(custom_config),
    ).await?;
    println!();

    println!("✨ Example complete!\n");
    println!("Key Takeaways:");
    println!("  • Different scoring configs produce different rankings");
    println!("  • Recency-focused configs favor fresh memories");
    println!("  • Importance-focused configs favor frequently accessed/high-priority memories");
    println!("  • Custom configs allow fine-tuned ranking for specific use cases");

    Ok(())
}

async fn demonstrate_scoring(
    locai: &Locai,
    query: &str,
    scoring: Option<ScoringConfig>,
) -> Result<()> {
    let storage = locai.manager().get_storage();
    
    // Use the enhanced search with scoring
    let results = storage
        .search_memories_with_scoring(query, scoring, Some(10))
        .await
        .map_err(|e| LocaiError::Storage(e.to_string()))?;

    for (idx, (memory, score)) in results.iter().enumerate() {
        println!(
            "   {}. Score: {:.4} - {} ({})",
            idx + 1,
            score,
            &memory.content[..memory.content.len().min(60)],
            memory.priority as i32
        );
    }

    Ok(())
}
