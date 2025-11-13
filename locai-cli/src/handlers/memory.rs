//! Memory command handlers

use crate::args::*;
use crate::commands::MemoryCommands;
use crate::context::LocaiCliContext;
use crate::output::*;
use crate::utils::*;
use colored::Colorize;
use locai::LocaiError;
use locai::memory::search_extensions::SearchMode;
use locai::storage::filters::{MemoryFilter, RelationshipFilter, SemanticSearchFilter};
use locai::storage::models::Relationship;
use reqwest;
use serde_json::{Value, json};

/// Generate query embedding using Ollama if available, otherwise use mock embedding
/// Checks OLLAMA_URL and OLLAMA_MODEL environment variables
async fn generate_query_embedding(query: &str, dimensions: usize) -> Vec<f32> {
    // Try to get embedding from Ollama if configured
    if let (Ok(ollama_url), Ok(model)) =
        (std::env::var("OLLAMA_URL"), std::env::var("OLLAMA_MODEL"))
        && let Ok(Some(embedding)) = generate_ollama_embedding(query, &model, &ollama_url).await
    {
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
async fn generate_ollama_embedding(
    text: &str,
    model: &str,
    ollama_url: &str,
) -> Result<Option<Vec<f32>>, Box<dyn std::error::Error>> {
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
        if let Some(embedding) = data.get("embedding").and_then(|e| e.as_array()) {
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
            // Parse requested mode
            let requested_mode = match args.mode.as_str() {
                "vector" => SearchMode::Vector,
                "hybrid" => SearchMode::Hybrid,
                "semantic" => SearchMode::Vector,
                "text" | "keyword" | "bm25" => SearchMode::Text,
                _ => SearchMode::Hybrid, // Default to hybrid
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
                    .map_err(|e| {
                        LocaiError::Other(format!("Invalid created_after timestamp: {}", e))
                    })?
                    .with_timezone(&chrono::Utc);
                mem_filter.created_after = Some(created_after);
            }
            if let Some(created_before_str) = args.created_before {
                let created_before = chrono::DateTime::parse_from_rfc3339(&created_before_str)
                    .map_err(|e| {
                        LocaiError::Other(format!("Invalid created_before timestamp: {}", e))
                    })?
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

            // Check if embeddings are available (Ollama configured)
            let has_ollama =
                std::env::var("OLLAMA_URL").is_ok() && std::env::var("OLLAMA_MODEL").is_ok();

            // Determine actual search mode with graceful fallback
            let (search_mode, use_hybrid_tagging) = if requested_mode == SearchMode::Hybrid {
                if has_ollama {
                    // Hybrid mode with embeddings available - run both searches separately for tagging
                    (SearchMode::Hybrid, true)
                } else {
                    // Hybrid requested but no embeddings - fallback to text-only
                    (SearchMode::Text, false)
                }
            } else {
                // Explicit mode requested (text or semantic)
                (requested_mode, false)
            };

            // Tagged result structure
            #[derive(Clone)]
            struct TaggedResult {
                memory: locai::prelude::Memory,
                score: Option<f32>,
                tags: Vec<String>, // ["text"], ["semantic"], or ["text", "semantic"]
            }

            // Perform search with tagging if hybrid mode
            let tagged_results: Vec<TaggedResult> = if use_hybrid_tagging {
                // Run both text and semantic searches separately for tagging
                let text_results = match ctx
                    .memory_manager
                    .search(
                        &args.query,
                        Some(args.limit * 2),
                        filter.clone(),
                        SearchMode::Text,
                    )
                    .await
                {
                    Ok(results) => results,
                    Err(e) => {
                        tracing::warn!("Text search failed in hybrid mode: {}", e);
                        Vec::new()
                    }
                };

                let query_embedding = generate_query_embedding(&args.query, 1024).await;
                let semantic_results = match ctx
                    .memory_manager
                    .search_with_embedding(
                        &args.query,
                        Some(&query_embedding),
                        Some(args.limit * 2),
                        filter,
                        SearchMode::Vector,
                    )
                    .await
                {
                    Ok(results) => results,
                    Err(e) => {
                        tracing::warn!("Semantic search failed in hybrid mode: {}", e);
                        Vec::new()
                    }
                };

                // Combine and tag results
                use std::collections::HashMap;
                let mut result_map: HashMap<String, TaggedResult> = HashMap::new();

                // Add text results
                for result in text_results {
                    let memory_id = result.memory.id.clone();
                    result_map.entry(memory_id).or_insert_with(|| TaggedResult {
                        memory: result.memory,
                        score: result.score,
                        tags: vec!["text".to_string()],
                    });
                }

                // Add semantic results (merge if already exists)
                for result in semantic_results {
                    let memory_id = result.memory.id.clone();
                    if let Some(existing) = result_map.get_mut(&memory_id) {
                        // Already exists from text search - add semantic tag
                        existing.tags.push("semantic".to_string());
                        // Use higher score if semantic score is better
                        if let Some(sem_score) = result.score
                            && existing.score.is_none_or(|text_score| sem_score > text_score)
                        {
                            existing.score = Some(sem_score);
                        }
                    } else {
                        // New result from semantic search only
                        result_map.insert(
                            memory_id,
                            TaggedResult {
                                memory: result.memory,
                                score: result.score,
                                tags: vec!["semantic".to_string()],
                            },
                        );
                    }
                }

                // Convert to sorted vector (by score descending)
                let mut results: Vec<TaggedResult> = result_map.into_values().collect();
                results.sort_by(|a, b| {
                    let score_a = a.score.unwrap_or(0.0);
                    let score_b = b.score.unwrap_or(0.0);
                    score_b
                        .partial_cmp(&score_a)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                results.truncate(args.limit);
                results
            } else {
                // Single mode search (text or semantic)
                let results = if matches!(search_mode, SearchMode::Vector) {
                    let query_embedding = generate_query_embedding(&args.query, 1024).await;
                    ctx.memory_manager
                        .search_with_embedding(
                            &args.query,
                            Some(&query_embedding),
                            Some(args.limit),
                            filter,
                            search_mode,
                        )
                        .await?
                } else {
                    ctx.memory_manager
                        .search(&args.query, Some(args.limit), filter, search_mode)
                        .await?
                };

                results
                    .into_iter()
                    .map(|r| TaggedResult {
                        memory: r.memory,
                        score: r.score,
                        tags: vec![
                            if search_mode == SearchMode::Vector {
                                "semantic"
                            } else {
                                "text"
                            }
                            .to_string(),
                        ],
                    })
                    .collect()
            };

            // Convert tagged results to regular results for JSON output
            let results: Vec<locai::storage::models::SearchResult> = tagged_results
                .iter()
                .map(|tr| locai::storage::models::SearchResult {
                    memory: tr.memory.clone(),
                    score: tr.score,
                })
                .collect();

            if output_format == "json" {
                // Add tags to JSON output
                let json_results: Vec<serde_json::Value> = tagged_results
                    .iter()
                    .map(|tr| {
                        let match_method = if tr.tags.len() > 1 {
                            "both"
                        } else {
                            tr.tags.first().map(|s| s.as_str()).unwrap_or("text")
                        };
                        json!({
                            "memory": tr.memory,
                            "score": tr.score,
                            "tags": tr.tags,
                            "match_method": match_method
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json_results)
                        .unwrap_or_else(|_| "[]".to_string())
                );
            } else if results.is_empty() {
                println!(
                    "{}",
                    format_info(&format!(
                        "No memories found matching '{}'",
                        args.query.color(CliColors::accent())
                    ))
                );

                // Provide helpful suggestions
                if use_hybrid_tagging {
                    println!();
                    println!("{}", "💡 Tips:".bold());
                    println!("  • Hybrid search combines text and semantic search automatically");
                    println!("  • Text search finds exact keyword matches");
                    println!("  • Semantic search finds related concepts (requires embeddings)");
                    if !has_ollama {
                        println!(
                            "  • {} Semantic search unavailable - set OLLAMA_URL and OLLAMA_MODEL to enable",
                            "⚠️".color(CliColors::warning())
                        );
                    }
                    println!("  • Try searching for different keywords or related concepts");
                } else if search_mode == SearchMode::Vector {
                    println!();
                    println!("{}", "💡 Tips for semantic search:".bold());
                    println!("  • Semantic search only finds memories with embeddings");
                    if has_ollama {
                        println!("  • Using Ollama for query embeddings");
                    } else {
                        println!(
                            "  • {} Using mock query embeddings - these don't capture semantic meaning!",
                            "⚠️".color(CliColors::warning())
                        );
                        println!("  • Set OLLAMA_URL and OLLAMA_MODEL for real semantic search");
                    }
                } else {
                    println!();
                    println!("{}", "💡 Tips:".bold());
                    println!(
                        "  • BM25 search looks for exact words - try searching for words that appear in your memories"
                    );
                    if has_ollama {
                        println!(
                            "  • Use {} for semantic search or {} for hybrid (default)",
                            "--mode semantic".color(CliColors::accent()),
                            "--mode hybrid".color(CliColors::accent())
                        );
                    }
                }
            } else {
                // Show search mode info
                let mode_info = if use_hybrid_tagging {
                    "[hybrid: text + semantic]"
                } else if search_mode == SearchMode::Vector {
                    "[semantic]"
                } else {
                    "[text]"
                };

                println!(
                    "{} {} (query: {})",
                    format_info(&format!("Found {} memories:", results.len())),
                    mode_info.color(CliColors::muted()),
                    args.query.color(CliColors::accent()).italic()
                );

                // Warn if using mock embeddings with semantic search
                if !has_ollama && (use_hybrid_tagging || search_mode == SearchMode::Vector) {
                    let avg_score: f32 =
                        tagged_results.iter().filter_map(|tr| tr.score).sum::<f32>()
                            / tagged_results.len().max(1) as f32;

                    if avg_score < 0.1 && !tagged_results.is_empty() {
                        println!();
                        println!(
                            "{}",
                            format!(
                                "⚠️  Warning: Very low similarity scores detected ({:.2} average)",
                                avg_score
                            )
                            .color(CliColors::warning())
                        );
                        println!(
                            "  This likely means you're using mock query embeddings with real stored embeddings."
                        );
                        println!("  Set OLLAMA_URL and OLLAMA_MODEL for real semantic search.");
                        println!();
                    }
                }

                // Show note if hybrid fell back to text-only
                if requested_mode == SearchMode::Hybrid && !has_ollama {
                    println!();
                    println!(
                        "{}",
                        format_info(
                            "ℹ️  Hybrid search requested but semantic search unavailable (no embeddings). Using text search only."
                        )
                    );
                    println!("  Set OLLAMA_URL and OLLAMA_MODEL to enable semantic search.");
                    println!();
                }

                // Display results with tags
                for (i, tagged_result) in tagged_results.iter().enumerate() {
                    let score = tagged_result.score.unwrap_or(0.0);
                    let (score_label, score_color) = match score {
                        s if s > 0.8 => ("Excellent", CliColors::success()),
                        s if s > 0.6 => ("Good", CliColors::info()),
                        s if s > 0.4 => ("Fair", CliColors::warning()),
                        s if s > 0.0 => ("Weak", CliColors::muted()),
                        _ => ("Very Weak", CliColors::muted()),
                    };

                    // Build tag string
                    let tag_str = if tagged_result.tags.len() > 1 {
                        format!("[{}]", tagged_result.tags.join("+"))
                    } else if let Some(tag) = tagged_result.tags.first() {
                        format!("[{}]", tag)
                    } else {
                        "[text]".to_string()
                    };

                    println!(
                        "{}. {} {} {}",
                        format!("{}", i + 1).color(CliColors::muted()),
                        tag_str.color(if tagged_result.tags.contains(&"semantic".to_string()) {
                            CliColors::info()
                        } else {
                            CliColors::muted()
                        }),
                        format!("[{} match: {:.2}]", score_label, score).color(score_color),
                        tagged_result.memory.content
                    );
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
                                    output_error(
                                        &format!("Invalid JSON properties: {}", e),
                                        output_format,
                                    );
                                    return Ok(());
                                }
                            }
                        } else {
                            Value::Null
                        };

                        let is_memory = ctx
                            .memory_manager
                            .get_memory(&create_args.target)
                            .await?
                            .is_some();
                        let is_entity = ctx
                            .memory_manager
                            .get_entity(&create_args.target)
                            .await?
                            .is_some();

                        if !is_memory && !is_entity {
                            output_error(
                                &format!(
                                    "Target '{}' not found (not a memory or entity)",
                                    create_args.target
                                ),
                                output_format,
                            );
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

                        let created = ctx
                            .memory_manager
                            .create_relationship_entity(relationship)
                            .await?;

                        if output_format == "json" {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&created)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );
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
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    println!(
                        "{}",
                        format_info(&format!(
                            "Memory Relationships: {}",
                            args.id.color(CliColors::accent())
                        ))
                    );
                    println!();
                    if !relationships.is_empty() {
                        println!(
                            "{}",
                            format_info(&format!(
                                "Outgoing Relationships ({}):",
                                relationships.len()
                            ))
                        );
                        print_relationship_list(&relationships);
                        println!();
                    }
                    if !incoming.is_empty() {
                        println!(
                            "{}",
                            format_info(&format!("Incoming Relationships ({}):", incoming.len()))
                        );
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
