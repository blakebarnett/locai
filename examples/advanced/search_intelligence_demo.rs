//! Search Intelligence Layer Demo
//!
//! This example demonstrates the new search intelligence capabilities of Locai,
//! including query analysis, full-text search with BM25 scoring, fuzzy matching,
//! hybrid search, and context-aware suggestions.

use locai::storage::{
    shared_storage::{SharedStorage, SharedStorageConfig},
    traits::{MemoryStore, BaseStore},
};
use locai::models::{Memory, MemoryType, MemoryPriority};
use locai::storage::shared_storage::intelligence::{IntelligentSearch, SearchStrategy, QueryIntent};
use chrono::Utc;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Search Intelligence Layer Demo");
    println!("=================================");

    // Create SharedStorage with embedded database
    let config = SharedStorageConfig {
        namespace: "demo".to_string(),
        database: "search_intelligence".to_string(),
    };

    let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(()).await?;
    let storage = SharedStorage::new(client, config).await?;

    println!("✅ Created SharedStorage with search intelligence capabilities");
    
    // Clear any existing data
    storage.clear().await?;

    // Create sample memories for testing different search features
    let sample_memories = vec![
        Memory {
            id: "mem1".to_string(),
            content: "Machine learning algorithms are becoming increasingly sophisticated in natural language processing".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::High,
            tags: vec!["ai".to_string(), "nlp".to_string(), "technology".to_string()],
            source: "research_paper".to_string(),
            expires_at: None,
            properties: json!({"topic": "artificial_intelligence"}),
            related_memories: vec![],
            embedding: None,
        },
        Memory {
            id: "mem2".to_string(),
            content: "Quantum computing could revolutionize machine learning by solving complex optimization problems".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::Medium,
            tags: vec!["quantum".to_string(), "computing".to_string(), "optimization".to_string()],
            source: "scientific_journal".to_string(),
            expires_at: None,
            properties: json!({"topic": "quantum_computing"}),
            related_memories: vec![],
            embedding: None,
        },
        Memory {
            id: "mem3".to_string(),
            content: "Deep neural networks require massive amounts of training data to achieve good performance".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::High,
            tags: vec!["deep_learning".to_string(), "neural_networks".to_string(), "training".to_string()],
            source: "ml_textbook".to_string(),
            expires_at: None,
            properties: json!({"topic": "deep_learning"}),
            related_memories: vec![],
            embedding: None,
        },
        Memory {
            id: "mem4".to_string(),
            content: "The transformer architecture has been a breakthrough in natural language understanding".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::High,
            tags: vec!["transformers".to_string(), "nlp".to_string(), "attention".to_string()],
            source: "research_paper".to_string(),
            expires_at: None,
            properties: json!({"topic": "transformers"}),
            related_memories: vec![],
            embedding: None,
        },
        Memory {
            id: "mem5".to_string(),
            content: "Quantum entanglement exhibits strange behavior that Einstein called spooky action at a distance".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::Medium,
            tags: vec!["quantum".to_string(), "physics".to_string(), "entanglement".to_string()],
            source: "physics_journal".to_string(),
            expires_at: None,
            properties: json!({"topic": "quantum_physics"}),
            related_memories: vec![],
            embedding: None,
        },
    ];

    // Create the sample memories
    println!("\n📝 Creating sample memories...");
    for memory in sample_memories {
        storage.create_memory(memory).await?;
    }

    println!("✅ Created {} sample memories", 5);

    // Demo 1: Query Analysis
    println!("\n🔍 Demo 1: Query Analysis");
    println!("------------------------");
    
    let test_queries = [
        "machine learning algorithms",
        "how do neural networks work?",
        "quantum computing and optimization",
        "what is transformer architecture?",
        "recent developments in AI",
    ];

    for query in &test_queries {
        println!("\nAnalyzing query: '{}'", query);
        match storage.analyze_query(query).await {
            Ok(analysis) => {
                println!("  Intent: {:?}", analysis.intent);
                println!("  Strategy: {:?}", analysis.strategy);
                println!("  Tokens: {:?}", analysis.tokens);
                println!("  Entities: {:?}", analysis.entities);
                println!("  Confidence: {:.2}", analysis.confidence);
            }
            Err(e) => println!("  Error: {}", e),
        }
    }

    // Demo 2: BM25 Full-Text Search with Highlighting
    println!("\n📊 Demo 2: BM25 Full-Text Search with Highlighting");
    println!("--------------------------------------------------");
    
    let search_query = "machine learning";
    println!("Searching for: '{}'", search_query);
    
    match storage.bm25_search_memories(search_query, Some(3)).await {
        Ok(results) => {
            println!("Found {} results:", results.len());
            for (i, (memory, score, highlight)) in results.iter().enumerate() {
                println!("  {}. Score: {:.3} | {}", i + 1, score, memory.content);
                println!("     Highlight: {}", highlight);
                println!("     Tags: {:?}", memory.tags);
            }
        }
        Err(e) => println!("BM25 search error: {}", e),
    }

    // Demo 3: Fuzzy Search for Typo Tolerance
    println!("\n🔤 Demo 3: Fuzzy Search for Typo Tolerance");
    println!("------------------------------------------");
    
    let fuzzy_query = "machien lerning"; // Intentional typos
    println!("Fuzzy search for: '{}'", fuzzy_query);
    
    match storage.fuzzy_search_memories(fuzzy_query, Some(0.3), Some(3)).await {
        Ok(results) => {
            println!("Found {} fuzzy matches:", results.len());
            for (i, (memory, score)) in results.iter().enumerate() {
                println!("  {}. Similarity: {:.3} | {}", i + 1, score, memory.content);
                println!("     Tags: {:?}", memory.tags);
            }
        }
        Err(e) => println!("Fuzzy search error: {}", e),
    }

    // Demo 4: Tag-based Search
    println!("\n🏷️  Demo 4: Tag-based Search");
    println!("---------------------------");
    
    let tag_search = vec!["quantum".to_string()];
    println!("Searching for tag: {:?}", tag_search);
    
    match storage.tag_search_memories(&tag_search, false, Some(5)).await {
        Ok(results) => {
            println!("Found {} memories with quantum tag:", results.len());
            for (i, memory) in results.iter().enumerate() {
                println!("  {}. {}", i + 1, memory.content);
                println!("     Tags: {:?}", memory.tags);
            }
        }
        Err(e) => println!("Tag search error: {}", e),
    }

    // Demo 5: Auto-complete Suggestions
    println!("\n💡 Demo 5: Auto-complete Suggestions");
    println!("------------------------------------");
    
    let partial_queries = ["machine", "quantum", "neural"];
    for partial in &partial_queries {
        println!("Auto-complete for: '{}'", partial);
        match storage.memory_autocomplete(partial, Some(3)).await {
            Ok(suggestions) => {
                for (i, suggestion) in suggestions.iter().enumerate() {
                    println!("  {}. {}", i + 1, suggestion);
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();
    }

    // Demo 6: Intelligent Search with Session Context
    println!("\n🧠 Demo 6: Intelligent Search with Context");
    println!("------------------------------------------");
    
    let search_queries = [
        "neural networks",
        "optimization problems", 
        "natural language",
    ];

    for query in &search_queries {
        println!("Intelligent search for: '{}'", query);
        match storage.intelligent_search(query, None, Some(2)).await {
            Ok(results) => {
                println!("Found {} intelligent results:", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!("  {}. Score: {:.3} | {}", i + 1, result.score, result.explanation.primary_reason);
                    println!("     Details: {:?}", result.explanation.details);
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();
    }

    // Demo 7: Search Suggestions
    println!("\n💭 Demo 7: Search Suggestions");
    println!("-----------------------------");
    
    let partial_queries = ["mach", "quantu", "neural"];
    for partial in &partial_queries {
        println!("Suggestions for: '{}'", partial);
        match storage.suggest(partial, None).await {
            Ok(suggestions) => {
                for (i, suggestion) in suggestions.iter().enumerate() {
                    println!("  {}. {} ({})", i + 1, suggestion.suggestion, suggestion.explanation);
                    println!("     Type: {:?}, Confidence: {:.2}", suggestion.suggestion_type, suggestion.confidence);
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();
    }

    println!("🎉 Search Intelligence Demo Complete!");
    println!("\nThe demo showcased:");
    println!("  ✅ Query analysis with intent detection");
    println!("  ✅ BM25 full-text search with highlighting");
    println!("  ✅ Fuzzy search for typo tolerance");
    println!("  ✅ Tag-based search");
    println!("  ✅ Auto-complete suggestions");
    println!("  ✅ Intelligent search with context");
    println!("  ✅ Search suggestions and refinements");

    Ok(())
} 