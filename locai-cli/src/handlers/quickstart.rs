//! Quickstart command handlers

use crate::args::QuickstartArgs;
use crate::context::LocaiCliContext;
use crate::output::*;
use colored::Colorize;
use locai::storage::filters::MemoryFilter;

/// Load pre-generated embeddings from JSON file
/// Returns a map of text -> embedding vector
fn load_quickstart_embeddings() -> std::collections::HashMap<String, Vec<f32>> {
    use std::collections::HashMap;

    // Try to load embeddings from the JSON file
    let embeddings_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("quickstart_embeddings.json");

    if let Ok(contents) = std::fs::read_to_string(&embeddings_path)
        && let Ok(data) = serde_json::from_str::<Vec<serde_json::Value>>(&contents)
    {
        let mut map = HashMap::new();
        for item in data {
            if let (Some(text), Some(embedding)) = (
                item.get("text").and_then(|v| v.as_str()),
                item.get("embedding").and_then(|v| v.as_array()),
            ) {
                let vec: Vec<f32> = embedding
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();
                if !vec.is_empty() {
                    map.insert(text.to_string(), vec);
                }
            }
        }
        return map;
    }

    // Return empty map if file doesn't exist or is invalid
    HashMap::new()
}

/// Normalize an embedding vector to unit length (L2 norm = 1.0)
/// This is required for cosine similarity to work correctly
fn normalize_embedding(mut embedding: Vec<f32>) -> Vec<f32> {
    let norm: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in &mut embedding {
            *val /= norm;
        }
    }
    embedding
}

/// Generate a simple mock embedding for demonstration purposes
/// This creates deterministic embeddings based on text content
/// Note: These are not semantically meaningful, just for demo
fn generate_mock_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut embedding = vec![0.0; dimensions];

    // Create deterministic values based on text content
    // This ensures similar texts get similar embeddings
    for (i, c) in text.chars().enumerate() {
        let idx = i % dimensions;
        let char_val = c as u32 % 255;
        embedding[idx] += (char_val as f32 / 255.0) * 0.1;
    }

    // Add some variation based on text length and hash
    let text_hash: u32 = text.chars().map(|c| c as u32).sum();
    for (i, val) in embedding.iter_mut().enumerate().take(dimensions) {
        *val += ((i as u32 + text_hash) % 100) as f32 / 1000.0;
    }

    // Normalize to unit length (common for embeddings)
    normalize_embedding(embedding)
}

pub async fn handle_quickstart_command(
    args: QuickstartArgs,
    ctx: &LocaiCliContext,
    _output_format: &str,
) -> locai::Result<()> {
    if args.cleanup {
        return cleanup_quickstart_data(ctx).await;
    }

    // Only show full intro if not using --step flag
    if args.step.is_none() {
        println!(
            "{}",
            "━━━ Locai Quick Start ━━━"
                .color(CliColors::accent())
                .bold()
        );
        println!();
        println!("Welcome to Locai! Creating sample data to help you explore.");
        println!();
    }

    let sample_memories = vec![
        (
            "The protagonist is a skilled warrior named John",
            locai::models::MemoryType::Fact,
            locai::models::MemoryPriority::High,
        ),
        (
            "John met Alice in the tavern last week",
            locai::models::MemoryType::Episodic,
            locai::models::MemoryPriority::Normal,
        ),
        (
            "The kingdom has been at war for three years",
            locai::models::MemoryType::World,
            locai::models::MemoryPriority::Normal,
        ),
        (
            "Alice is a skilled mage who studies ancient magic",
            locai::models::MemoryType::Identity,
            locai::models::MemoryPriority::Normal,
        ),
        (
            "The tavern is located in the capital city",
            locai::models::MemoryType::World,
            locai::models::MemoryPriority::Low,
        ),
    ];

    if args.step.is_none() {
        println!("{}", format_info("Creating sample memories..."));
    }
    let mut created_ids = Vec::new();

    for (content, mem_type, priority) in sample_memories {
        // Check if memory already exists (idempotent)
        let existing = ctx
            .memory_manager
            .filter_memories(
                MemoryFilter {
                    content: Some(content.to_string()),
                    ..Default::default()
                },
                None,
                None,
                Some(1),
            )
            .await?;

        let memory_index = created_ids.len();
        let id = if let Some(existing_memory) = existing.first() {
            existing_memory.id.clone()
        } else {
            ctx.memory_manager
                .add_memory_with_options(content.to_string(), |builder| {
                    builder.memory_type(mem_type).priority(priority)
                })
                .await?
        };

        // Add embeddings to first 3 memories for demonstration
        // Use pre-generated embeddings from Ollama if available, otherwise use mock embeddings
        if memory_index < 3
            && let Ok(Some(mut memory)) = ctx.memory_manager.get_memory(&id).await
        {
            // Load pre-generated embeddings (generated via scripts/generate_quickstart_embeddings.sh)
            let embeddings = load_quickstart_embeddings();

            let embedding = if let Some(emb) = embeddings.get(content) {
                // Use pre-generated embedding if available
                // Ensure it's 1024 dimensions (required by SurrealDB)
                if emb.len() == 1024 {
                    // Normalize the embedding (Ollama embeddings may not be normalized)
                    normalize_embedding(emb.clone())
                } else {
                    tracing::debug!(
                        "Pre-generated embedding has {} dimensions, need 1024. Using mock embedding.",
                        emb.len()
                    );
                    generate_mock_embedding(content, 1024)
                }
            } else {
                // Fall back to mock embeddings if pre-generated ones aren't available
                generate_mock_embedding(content, 1024)
            };

            memory.embedding = Some(embedding);

            // Update the memory with the embedding
            if let Err(e) = ctx.memory_manager.update_memory(memory).await {
                tracing::debug!("Failed to add embedding to memory {}: {}", id, e);
            }
        }

        created_ids.push(id);
    }

    if args.step.is_none() {
        let embedding_count = created_ids.len().min(3);
        // Check if pre-generated embeddings are available
        let embeddings = load_quickstart_embeddings();
        let has_real_embeddings = !embeddings.is_empty()
            && embeddings
                .values()
                .next()
                .map(|v| v.len() == 1024)
                .unwrap_or(false);
        let embedding_source = if has_real_embeddings {
            "pre-generated embeddings"
        } else {
            "mock embeddings"
        };

        if embedding_count > 0 {
            println!(
                "{}",
                format_success(&format!(
                    "✓ Created/verified {} sample memories ({} with {} for semantic search demo)",
                    created_ids.len(),
                    embedding_count,
                    embedding_source
                ))
            );
        } else {
            println!(
                "{}",
                format_success(&format!(
                    "✓ Created/verified {} sample memories",
                    created_ids.len()
                ))
            );
        }
        println!("{}", format_info("Creating sample entities..."));
    }
    let mut entity_count = 0;

    // Check if entity already exists (idempotent)
    if ctx
        .memory_manager
        .get_entity("entity:quickstart:john")
        .await?
        .is_none()
    {
        let entity1 = locai::storage::models::Entity {
            id: "entity:quickstart:john".to_string(),
            entity_type: "Person".to_string(),
            properties: serde_json::json!({"name": "John"}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        if ctx.memory_manager.create_entity(entity1).await.is_ok() {
            entity_count += 1;
        }
    } else {
        entity_count += 1;
    }

    if ctx
        .memory_manager
        .get_entity("entity:quickstart:alice")
        .await?
        .is_none()
    {
        let entity2 = locai::storage::models::Entity {
            id: "entity:quickstart:alice".to_string(),
            entity_type: "Person".to_string(),
            properties: serde_json::json!({"name": "Alice"}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        if ctx.memory_manager.create_entity(entity2).await.is_ok() {
            entity_count += 1;
        }
    } else {
        entity_count += 1;
    }

    if args.step.is_none() {
        println!(
            "{}",
            format_success(&format!(
                "✓ Created/verified {} sample entities",
                entity_count
            ))
        );
        println!("{}", format_info("Creating sample relationships..."));
    }

    // Small delay to ensure entities are fully persisted
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let mut relationship_count = 0;
    let mut skipped_count = 0;

    if created_ids.len() >= 2 {
        // Verify memories exist and are accessible before creating relationships
        let memory1 = ctx.memory_manager.get_memory(&created_ids[0]).await?;
        let memory2 = ctx.memory_manager.get_memory(&created_ids[1]).await?;

        if memory1.is_some() && memory2.is_some() {
            // Check if relationship already exists
            let filter = locai::storage::filters::RelationshipFilter {
                source_id: Some(created_ids[0].clone()),
                target_id: Some(created_ids[1].clone()),
                relationship_type: Some("references".to_string()),
                ..Default::default()
            };
            let existing = ctx
                .memory_manager
                .list_relationships(Some(filter), Some(1), None)
                .await?;
            if existing.is_empty() {
                match ctx
                    .memory_manager
                    .create_relationship(&created_ids[0], &created_ids[1], "references")
                    .await
                {
                    Ok(_) => relationship_count += 1,
                    Err(e) => {
                        // Silently skip if target not found (might be a timing issue)
                        if !e.to_string().contains("not found") {
                            // Only log unexpected errors
                            tracing::debug!("Failed to create memory-memory relationship: {}", e);
                        }
                        skipped_count += 1;
                    }
                }
            } else {
                relationship_count += 1; // Already exists
            }
        } else {
            skipped_count += 1;
        }

        // Verify entities exist before creating relationships (idempotent)
        let john_entity = ctx
            .memory_manager
            .get_entity("entity:quickstart:john")
            .await?;
        if john_entity.is_some() && memory1.is_some() {
            let filter = locai::storage::filters::RelationshipFilter {
                source_id: Some(created_ids[0].clone()),
                target_id: Some("entity:quickstart:john".to_string()),
                relationship_type: Some("has_character".to_string()),
                ..Default::default()
            };
            let existing = ctx
                .memory_manager
                .list_relationships(Some(filter), Some(1), None)
                .await?;
            if existing.is_empty() {
                match ctx
                    .memory_manager
                    .create_relationship(&created_ids[0], "entity:quickstart:john", "has_character")
                    .await
                {
                    Ok(_) => relationship_count += 1,
                    Err(e) => {
                        if !e.to_string().contains("not found") {
                            tracing::debug!("Failed to create memory-entity relationship: {}", e);
                        }
                        skipped_count += 1;
                    }
                }
            } else {
                relationship_count += 1; // Already exists
            }
        } else {
            skipped_count += 1;
        }

        let alice_entity = ctx
            .memory_manager
            .get_entity("entity:quickstart:alice")
            .await?;
        if alice_entity.is_some() && memory2.is_some() {
            let filter = locai::storage::filters::RelationshipFilter {
                source_id: Some(created_ids[1].clone()),
                target_id: Some("entity:quickstart:alice".to_string()),
                relationship_type: Some("has_character".to_string()),
                ..Default::default()
            };
            let existing = ctx
                .memory_manager
                .list_relationships(Some(filter), Some(1), None)
                .await?;
            if existing.is_empty() {
                match ctx
                    .memory_manager
                    .create_relationship(
                        &created_ids[1],
                        "entity:quickstart:alice",
                        "has_character",
                    )
                    .await
                {
                    Ok(_) => relationship_count += 1,
                    Err(e) => {
                        if !e.to_string().contains("not found") {
                            tracing::debug!("Failed to create memory-entity relationship: {}", e);
                        }
                        skipped_count += 1;
                    }
                }
            } else {
                relationship_count += 1; // Already exists
            }
        } else {
            skipped_count += 1;
        }
    }

    if args.step.is_none() {
        if relationship_count > 0 {
            let msg = if skipped_count > 0 {
                format!(
                    "✓ Created/verified {} sample relationships ({} skipped)",
                    relationship_count, skipped_count
                )
            } else {
                format!(
                    "✓ Created/verified {} sample relationships",
                    relationship_count
                )
            };
            println!("{}", format_success(&msg));
        } else {
            println!(
                "{}",
                format_warning("⚠ No relationships created (entities may not be ready yet)")
            );
        }
    }

    // Show different output based on --step flag
    match args.step {
        Some(1) => {
            println!();
            println!(
                "{}",
                "━━━ Step 1: Search ━━━".color(CliColors::accent()).bold()
            );
            println!();
            println!("Try searching for memories:");
            println!("  locai-cli memory search \"warrior\"");
            println!("  locai-cli memory search \"John\"");
            println!("  locai-cli memory search \"Alice\"");
            println!();
            println!("{}", "Note:".bold());
            println!("  • Default search uses BM25 (keyword matching)");
            println!("  • Search for words that appear in the memory content");
            println!(
                "  • First 3 memories have {} embeddings for demo",
                "mock".color(CliColors::accent())
            );
            println!(
                "  • Try semantic search: {}",
                "locai-cli memory search \"character\" --mode semantic".color(CliColors::accent())
            );
            println!();
            println!(
                "Next: {}",
                "locai-cli quickstart --step 2".color(CliColors::accent())
            );
        }
        Some(2) => {
            println!();
            println!(
                "{}",
                "━━━ Step 2: Explore ━━━".color(CliColors::accent()).bold()
            );
            println!();
            if let Some(first_id) = created_ids.first() {
                println!("View a memory:");
                println!("  locai-cli memory get {}", first_id);
                println!();
                println!("See how memories connect:");
                println!("  locai-cli graph subgraph {}", first_id);
                println!();
            }
            println!(
                "Next: {}",
                "locai-cli quickstart --step 3".color(CliColors::accent())
            );
        }
        Some(3) => {
            println!();
            println!(
                "{}",
                "━━━ Step 3: Learn More ━━━"
                    .color(CliColors::accent())
                    .bold()
            );
            println!();
            println!("Interactive tutorial:");
            println!("  locai-cli tutorial");
            println!();
            println!("Get explanations:");
            println!("  locai-cli --explain memory");
            println!("  locai-cli --explain graph");
            println!();
            println!("{}", format_success("You're all set! Happy exploring! 🚀"));
        }
        Some(n) if n > 3 => {
            println!();
            println!(
                "{}",
                format_error("Invalid step number. Use --step 1, 2, or 3.")
            );
        }
        _ => {
            // Default: Show summary and 3 key commands
            println!();
            println!("{}", format_success("✓ Sample data created!"));
            println!();
            println!("{}", "Try these 3 commands:".bold());
            println!();
            println!("  1. {}", "Search memories".color(CliColors::accent()));
            println!("     locai-cli memory search \"warrior\"");
            println!("     locai-cli memory search \"John\"");
            println!();
            println!("  2. {}", "List all memories".color(CliColors::accent()));
            println!("     locai-cli memory list");
            println!();
            if let Some(first_id) = created_ids.first() {
                println!("  3. {}", "See the graph".color(CliColors::accent()));
                println!("     locai-cli graph subgraph {}", first_id);
            }
            println!();
            println!("{}", "Next steps:".bold());
            println!(
                "  • Run interactive tutorial: {}",
                "locai-cli tutorial".color(CliColors::accent())
            );
            println!(
                "  • Learn concepts: {}",
                "locai-cli --explain memory".color(CliColors::accent())
            );
            println!(
                "  • Step-by-step guide: {}",
                "locai-cli quickstart --step 1".color(CliColors::accent())
            );
            println!(
                "  • Remove sample data: {}",
                "locai-cli quickstart --cleanup".color(CliColors::accent())
            );
            println!();
            println!("{}", "💡 About Semantic Search:".bold());
            let embeddings = load_quickstart_embeddings();
            let has_real_embeddings = !embeddings.is_empty()
                && embeddings
                    .values()
                    .next()
                    .map(|v| v.len() == 1024)
                    .unwrap_or(false);
            if has_real_embeddings {
                println!(
                    "  • First 3 memories have {} embeddings for demonstration",
                    "pre-generated".color(CliColors::accent())
                );
            } else {
                println!(
                    "  • First 3 memories have {} embeddings for demonstration",
                    "mock".color(CliColors::accent())
                );
                println!(
                    "  • To use real embeddings, run: {}",
                    "./scripts/generate_quickstart_embeddings.sh".color(CliColors::accent())
                );
            }
            println!(
                "  • Try: {}",
                "locai-cli memory search \"character\" --mode semantic".color(CliColors::accent())
            );
            println!("  • Semantic search understands meaning, not just keywords");
            println!("  • For production, use real embeddings from OpenAI, Cohere, etc.");
            println!(
                "  • See: {}",
                "locai-cli --explain search".color(CliColors::accent())
            );
        }
    }

    Ok(())
}

async fn cleanup_quickstart_data(ctx: &LocaiCliContext) -> locai::Result<()> {
    println!(
        "{}",
        "━━━ Cleaning Up Quickstart Data ━━━"
            .color(CliColors::accent())
            .bold()
    );
    println!();
    println!(
        "{}",
        format_info("Removing sample memories and entities...")
    );

    let memories = ctx
        .memory_manager
        .filter_memories(MemoryFilter::default(), None, None, Some(1000))
        .await?;

    let mut deleted_count = 0;
    for memory in memories {
        if (memory.content.contains("protagonist")
            || memory.content.contains("John met Alice")
            || memory.content.contains("kingdom has been at war")
            || memory.content.contains("Alice is a skilled mage")
            || memory.content.contains("tavern is located"))
            && ctx.memory_manager.delete_memory(&memory.id).await?
        {
            deleted_count += 1;
        }
    }

    let entities = ctx
        .memory_manager
        .list_entities(None, Some(100), None)
        .await?;

    for entity in entities {
        if entity.id.starts_with("entity:quickstart:") {
            ctx.memory_manager.delete_entity(&entity.id).await?;
        }
    }

    println!();
    println!(
        "{}",
        format_success(&format!(
            "✓ Cleaned up {} memories and quickstart entities",
            deleted_count
        ))
    );
    println!();
    println!(
        "{}",
        format_info("Sample data removed. Run 'locai-cli quickstart' again to recreate it.")
    );

    Ok(())
}
