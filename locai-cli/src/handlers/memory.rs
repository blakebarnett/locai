//! Memory command handlers

use crate::context::LocaiCliContext;
use crate::commands::MemoryCommands;
use crate::args::*;
use crate::output::*;
use crate::utils::*;
use colored::Colorize;
use locai::LocaiError;
use locai::memory::search_extensions::SearchMode;
use locai::storage::filters::{MemoryFilter, RelationshipFilter, SemanticSearchFilter};
use locai::storage::models::Relationship;
use serde_json::{Value, json};
use reqwest;

/// Generate query embedding using Ollama if available, otherwise use mock embedding
/// Checks OLLAMA_URL and OLLAMA_MODEL environment variables
async fn generate_query_embedding(query: &str, dimensions: usize) -> Vec<f32> {
    // Try to get embedding from Ollama if configured
    if let (Ok(ollama_url), Ok(model)) = (
        std::env::var("OLLAMA_URL"),
        std::env::var("OLLAMA_MODEL")
    )
        && let Ok(Some(embedding)) = generate_ollama_embedding(query, &model, &ollama_url).await {
        // Use Ollama embedding if dimensions match
        if embedding.len() == dimensions {
            return embedding;
        } else {
            tracing::debug!(
                "Ollama returned {} dimensions, need {}. Using mock embedding.",
                embedding.len(),
                dimensions
            );
        }
    }
    
    // Fall back to mock embedding
    generate_mock_query_embedding(query, dimensions)
}

/// Generate embedding using Ollama API
async fn generate_ollama_embedding(text: &str, model: &str, ollama_url: &str) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/embeddings", ollama_url);
    
    let payload = json!({
        "model": model,
        "prompt": text
    });
    
    let response = client
        .post(&url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;
    
    if response.status().is_success() {
        let data: Value = response.json().await?;
        if let Some(embedding) = data.get("embedding")
            .and_then(|e| e.as_array())
        {
            let vec: Vec<f32> = embedding
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            if !vec.is_empty() {
                return Ok(Some(vec));
            }
        }
    }
    
    Ok(None)
}

/// Generate a simple mock query embedding for demonstration purposes
/// This creates deterministic embeddings based on query text
/// Uses the same algorithm as quickstart mock embeddings for consistency
fn generate_mock_query_embedding(query: &str, dimensions: usize) -> Vec<f32> {
    let mut embedding = vec![0.0; dimensions];
    
    // Create deterministic values based on query content
    for (i, c) in query.chars().enumerate() {
        let idx = i % dimensions;
        let char_val = c as u32 % 255;
        embedding[idx] += (char_val as f32 / 255.0) * 0.1;
    }
    
    // Add some variation based on query length and hash
    let query_hash: u32 = query.chars().map(|c| c as u32).sum();
    for (i, val) in embedding.iter_mut().enumerate().take(dimensions) {
        *val += ((i as u32 + query_hash) % 100) as f32 / 1000.0;
    }
    
    // Normalize to unit length (common for embeddings)
    let norm: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in &mut embedding {
            *val /= norm;
        }
    }
    
    embedding
}

pub async fn handle_memory_command(
    cmd: MemoryCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        MemoryCommands::Add(args) => {
            let memory_type = parse_memory_type(&args.memory_type)?;
            let priority = parse_priority(&args.priority)?;

            let memory_id = ctx
                .memory_manager
                .add_memory_with_options(args.content, |builder| {
                    let mut b = builder.memory_type(memory_type).priority(priority);
                    for tag in args.tags {
                        b = b.tag(tag);
                    }
                    b
                })
                .await?;

            if output_format == "json" {
                let result = json!({ "memory_id": memory_id });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    format_success(&format!(
                        "Memory created with ID: {}",
                        memory_id.color(CliColors::accent()).bold()
                    ))
                );
            }
        }

        MemoryCommands::Get(args) => match ctx.memory_manager.get_memory(&args.id).await? {
            Some(memory) => {
                if output_format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&memory).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    print_memory(&memory);
                }
            }
            None => {
                println!(
                    "{}",
                    format_warning(&format!(
                        "Memory with ID '{}' not found.",
                        args.id.color(CliColors::accent())
                    ))
                );
            }
        },

        MemoryCommands::Search(args) => {
            let search_mode = match args.mode.as_str() {
                "vector" => SearchMode::Vector,
                "hybrid" => SearchMode::Hybrid,
                "semantic" => SearchMode::Vector,
                "text" | "keyword" | "bm25" => SearchMode::Text,
                _ => SearchMode::Text,
            };

            // Handle temporal filters
            let mut mem_filter = MemoryFilter::default();
            if let Some(mem_type) = args.memory_type {
                mem_filter.memory_type = Some(mem_type);
            }
            if let Some(tag) = args.tag {
                mem_filter.tags = Some(vec![tag]);
            }

            // Parse temporal filters if provided
            if let Some(created_after_str) = args.created_after {
                let created_after = chrono::DateTime::parse_from_rfc3339(&created_after_str)
                    .map_err(|e| LocaiError::Other(format!("Invalid created_after timestamp: {}", e)))?
                    .with_timezone(&chrono::Utc);
                mem_filter.created_after = Some(created_after);
            }
            if let Some(created_before_str) = args.created_before {
                let created_before = chrono::DateTime::parse_from_rfc3339(&created_before_str)
                    .map_err(|e| LocaiError::Other(format!("Invalid created_before timestamp: {}", e)))?
                    .with_timezone(&chrono::Utc);
                mem_filter.created_before = Some(created_before);
            }

            // Check if filter has any non-default values
            let has_filters = mem_filter.memory_type.is_some()
                || mem_filter.tags.is_some()
                || mem_filter.created_after.is_some()
                || mem_filter.created_before.is_some();

            let filter = if args.threshold.is_some() || has_filters {
                Some(SemanticSearchFilter {
                    similarity_threshold: args.threshold,
                    memory_filter: if has_filters { Some(mem_filter) } else { None },
                })
            } else {
                None
            };

            // For vector/semantic/hybrid search, generate query embedding
            // Try Ollama if OLLAMA_URL and OLLAMA_MODEL are set, otherwise use mock embedding
            let results = if matches!(search_mode, SearchMode::Vector | SearchMode::Hybrid) {
                // Generate query embedding (Ollama if available, otherwise mock)
                let query_embedding = generate_query_embedding(&args.query, 1024).await;
                ctx.memory_manager
                    .search_with_embedding(
                        &args.query,
                        Some(&query_embedding),
                        Some(args.limit),
                        filter,
                        search_mode,
                    )
                    .await
            } else {
                ctx.memory_manager
                    .search(&args.query, Some(args.limit), filter, search_mode)
                    .await
            };
            
            match results {
                Ok(results) => {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&results)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else if results.is_empty() {
                        println!(
                            "{}",
                            format_info(&format!(
                                "No memories found matching '{}'",
                                args.query.color(CliColors::accent())
                            ))
                        );
                        
                        // Provide helpful suggestions based on search mode
                        if matches!(search_mode, SearchMode::Vector | SearchMode::Hybrid) {
                            println!();
                            println!("{}", "💡 Tips for semantic search:".bold());
                            println!("  • Semantic search only finds memories with embeddings");
                            let has_ollama = std::env::var("OLLAMA_URL").is_ok() && std::env::var("OLLAMA_MODEL").is_ok();
                            if has_ollama {
                                println!("  • Using Ollama for query embeddings (set via OLLAMA_URL and OLLAMA_MODEL)");
                            } else {
                                println!("  • {} Using mock query embeddings - these don't capture semantic meaning!", "⚠️".color(CliColors::warning()));
                                println!("  • Mock embeddings won't match real embeddings (e.g., 'battle' won't match 'warrior')");
                                println!("  • Set OLLAMA_URL and OLLAMA_MODEL for real semantic search");
                                println!("  • Example: {} OLLAMA_URL=http://localhost:11434 OLLAMA_MODEL=nomic-embed-text locai-cli memory search \"battle\" --mode semantic", "export".color(CliColors::muted()));
                            }
                            println!("  • Quickstart creates embeddings for the first 3 memories");
                            println!("  • Try searching for words related to: {}", "warrior, John, Alice, mage, kingdom".color(CliColors::accent()));
                            println!("  • Or use text search: {}", format!("locai-cli memory search \"{}\" --mode text", args.query).color(CliColors::accent()));
                        } else if search_mode == SearchMode::Text {
                            println!();
                            println!("{}", "💡 Tips:".bold());
                            println!("  • BM25 search looks for exact words - try searching for words that appear in your memories");
                            println!("  • Example: search for '{}' or '{}' instead", "warrior".color(CliColors::accent()), "John".color(CliColors::accent()));
                            println!("  • Use {} for semantic search (if embeddings are configured)", "--mode semantic".color(CliColors::accent()));
                            println!("  • Try: {}", "locai-cli memory search \"warrior\" --mode text".color(CliColors::accent()));
                        }
                    } else {
                        println!(
                            "{} (query: {})",
                            format_info(&format!("Found {} memories:", results.len())),
                            args.query.color(CliColors::accent()).italic()
                        );
                        let has_ollama = std::env::var("OLLAMA_URL").is_ok() && std::env::var("OLLAMA_MODEL").is_ok();
                        let using_mock = !has_ollama && matches!(search_mode, SearchMode::Vector | SearchMode::Hybrid);
                        
                        if using_mock && !results.is_empty() {
                            // Check if scores are suspiciously low (likely mock vs real embedding mismatch)
                            let avg_score: f32 = results.iter()
                                .map(|r| r.score.unwrap_or(0.0))
                                .sum::<f32>() / results.len() as f32;
                            
                            if avg_score < 0.1 {
                                println!();
                                println!("{}", format!("⚠️  Warning: Very low similarity scores detected ({:.2} average)", avg_score).color(CliColors::warning()));
                                println!("  This likely means you're using mock query embeddings with real stored embeddings.");
                                println!("  Mock embeddings don't capture semantic meaning - set OLLAMA_URL and OLLAMA_MODEL for real semantic search.");
                                println!();
                            }
                        }
                        
                        for (i, result) in results.iter().enumerate() {
                            let score = result.score.unwrap_or(0.0);
                            let (score_label, score_color) = match score {
                                s if s > 0.8 => ("Excellent", CliColors::success()),
                                s if s > 0.6 => ("Good", CliColors::info()),
                                s if s > 0.4 => ("Fair", CliColors::warning()),
                                s if s > 0.0 => ("Weak", CliColors::muted()),
                                _ => ("Very Weak", CliColors::muted()),
                            };
                            println!(
                                "{}. {} {}",
                                format!("{}", i + 1).color(CliColors::muted()),
                                format!("[{} match: {:.2}]", score_label, score).color(score_color),
                                result.memory.content
                            );
                        }
                    }
                }
                Err(e) => {
                    output_error(&format!("Search failed: {}", e), output_format);
                }
            }
        }

        MemoryCommands::Delete(args) => match ctx.memory_manager.delete_memory(&args.id).await? {
            true => println!(
                "{}",
                format_success(&format!(
                    "Memory '{}' deleted successfully.",
                    args.id.color(CliColors::accent())
                ))
            ),
            false => println!(
                "{}",
                format_warning(&format!(
                    "Memory '{}' not found or could not be deleted.",
                    args.id.color(CliColors::accent())
                ))
            ),
        },

        MemoryCommands::List(args) => {
            let mut filter = MemoryFilter::default();

            if let Some(mem_type) = args.memory_type {
                filter.memory_type = Some(mem_type);
            }

            if let Some(tag) = args.tag {
                filter.tags = Some(vec![tag]);
            }

            let memories = ctx
                .memory_manager
                .filter_memories(filter, None, None, Some(args.limit))
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }

        MemoryCommands::Tag(args) => {
            match ctx.memory_manager.tag_memory(&args.id, &args.tag).await? {
                true => println!("Tag '{}' added to memory '{}'.", args.tag, args.id),
                false => println!("Failed to add tag or memory not found."),
            }
        }

        MemoryCommands::Count(args) => {
            let mut filter = MemoryFilter::default();

            if let Some(mem_type) = args.memory_type {
                filter.memory_type = Some(mem_type);
            }

            if let Some(tag) = args.tag {
                filter.tags = Some(vec![tag]);
            }

            let count = ctx.memory_manager.count_memories(Some(filter)).await?;

            if output_format == "json" {
                let result = json!({ "count": count });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!("Total memories: {}", count);
            }
        }

        MemoryCommands::Priority(args) => {
            let priority = parse_priority(&args.priority)?;
            let memories = ctx
                .memory_manager
                .get_memories_by_priority(priority, Some(args.limit))
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }

        MemoryCommands::Recent(args) => {
            let memories = ctx.memory_manager.get_recent_memories(args.limit).await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }

        MemoryCommands::Update(args) => {
            let mut memory = ctx
                .memory_manager
                .get_memory(&args.id)
                .await?
                .ok_or_else(|| LocaiError::Other(format!("Memory '{}' not found", args.id)))?;

            if let Some(content) = args.content {
                memory.content = content;
            }

            if let Some(memory_type_str) = args.memory_type {
                memory.memory_type = parse_memory_type(&memory_type_str)?;
            }

            if let Some(priority_str) = args.priority {
                memory.priority = parse_priority(&priority_str)?;
            }

            if let Some(tags) = args.tags {
                memory.tags = tags;
            }

            if let Some(properties_str) = args.properties {
                let properties: Value = serde_json::from_str(&properties_str)
                    .map_err(|e| LocaiError::Other(format!("Invalid JSON properties: {}", e)))?;
                memory.properties = properties;
            }

            let updated = ctx.memory_manager.update_memory(memory).await?;

            if output_format == "json" {
                let result = json!({
                    "success": updated,
                    "memory_id": args.id
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else if updated {
                println!(
                    "{}",
                    format_success(&format!(
                        "Memory '{}' updated successfully.",
                        args.id.color(CliColors::accent())
                    ))
                );
            } else {
                println!(
                    "{}",
                    format_warning(&format!(
                        "Memory '{}' not found or could not be updated.",
                        args.id.color(CliColors::accent())
                    ))
                );
            }
        }

        MemoryCommands::Relationships(args) => {
            if let Some(command) = args.command {
                match command {
                    MemoryRelationshipSubcommand::Create(create_args) => {
                        let properties = if let Some(props) = create_args.properties {
                            match serde_json::from_str::<Value>(&props) {
                                Ok(props) => props,
                                Err(e) => {
                                    output_error(&format!("Invalid JSON properties: {}", e), output_format);
                                    return Ok(());
                                }
                            }
                        } else {
                            Value::Null
                        };

                        let is_memory = ctx.memory_manager.get_memory(&create_args.target).await?.is_some();
                        let is_entity = ctx.memory_manager.get_entity(&create_args.target).await?.is_some();

                        if !is_memory && !is_entity {
                            output_error(&format!("Target '{}' not found (not a memory or entity)", create_args.target), output_format);
                            return Ok(());
                        }

                        let now = chrono::Utc::now();
                        let relationship = Relationship {
                            id: format!("rel:{}", uuid::Uuid::new_v4()),
                            source_id: args.id.clone(),
                            target_id: create_args.target.clone(),
                            relationship_type: create_args.relationship_type.clone(),
                            properties,
                            created_at: now,
                            updated_at: now,
                        };

                        let created = ctx.memory_manager.create_relationship_entity(relationship).await?;

                        if output_format == "json" {
                            println!("{}", serde_json::to_string_pretty(&created).unwrap_or_else(|_| "{}".to_string()));
                        } else {
                            println!(
                                "{}",
                                format_success(&format!(
                                    "Relationship '{}' created from memory '{}' to '{}'",
                                    create_args.relationship_type.color(CliColors::accent()),
                                    args.id.color(CliColors::accent()),
                                    create_args.target.color(CliColors::accent())
                                ))
                            );
                        }
                    }
                }
            } else {
                let relationships = ctx
                    .memory_manager
                    .list_relationships(
                        Some(RelationshipFilter {
                            source_id: Some(args.id.clone()),
                            ..Default::default()
                        }),
                        None,
                        None,
                    )
                    .await?;

                let incoming = ctx
                    .memory_manager
                    .list_relationships(
                        Some(RelationshipFilter {
                            target_id: Some(args.id.clone()),
                            ..Default::default()
                        }),
                        None,
                        None,
                    )
                    .await?;

                if output_format == "json" {
                    let result = json!({
                        "memory_id": args.id,
                        "outgoing": relationships,
                        "incoming": incoming,
                        "total": relationships.len() + incoming.len()
                    });
                    println!("{}", serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()));
                } else {
                    println!("{}", format_info(&format!("Memory Relationships: {}", args.id.color(CliColors::accent()))));
                    println!();
                    if !relationships.is_empty() {
                        println!("{}", format_info(&format!("Outgoing Relationships ({}):", relationships.len())));
                        print_relationship_list(&relationships);
                        println!();
                    }
                    if !incoming.is_empty() {
                        println!("{}", format_info(&format!("Incoming Relationships ({}):", incoming.len())));
                        print_relationship_list(&incoming);
                    }
                    if relationships.is_empty() && incoming.is_empty() {
                        println!("{}", format_info("No relationships found."));
                    }
                }
            }
        }
    }

    Ok(())
}
