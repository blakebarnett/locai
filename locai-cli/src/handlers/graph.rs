//! Graph command handlers

use crate::commands::GraphCommands;
use crate::context::LocaiCliContext;
use crate::output::*;
use crate::utils::*;
use colored::Colorize;
use locai::storage::filters::{MemoryFilter, RelationshipFilter};
use serde_json::{Value, json};
use std::collections::{HashSet, VecDeque};

pub async fn handle_graph_command(
    cmd: GraphCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        GraphCommands::Subgraph(args) => {
            let memory_id = resolve_memory_id(ctx, &args.id).await?;
            let graph = ctx
                .memory_manager
                .get_memory_graph(&memory_id, args.depth)
                .await?;

            if args.include_temporal_span && !graph.memories.is_empty() {
                let memories: Vec<_> = graph.memories.values().collect();
                let mut timestamps: Vec<_> = memories.iter().map(|m| m.created_at).collect();
                timestamps.sort();

                if let (Some(&start), Some(&end)) = (timestamps.first(), timestamps.last()) {
                    let duration_days = (end - start).num_days();
                    let duration_seconds = (end - start).num_seconds();

                    if output_format == "json" {
                        let mut graph_json: Value =
                            serde_json::to_value(&graph).unwrap_or_default();
                        graph_json["temporal_span"] = json!({
                            "start": start.to_rfc3339(),
                            "end": end.to_rfc3339(),
                            "duration_days": duration_days,
                            "duration_seconds": duration_seconds,
                            "memory_count": memories.len()
                        });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&graph_json)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        print_memory_graph(&graph);
                        println!();
                        println!(
                            "{}",
                            "━━━ Temporal Span ━━━".color(CliColors::accent()).bold()
                        );
                        println!(
                            "{}: {}",
                            "Start".color(CliColors::muted()),
                            start
                                .format("%Y-%m-%d %H:%M:%S UTC")
                                .to_string()
                                .color(CliColors::primary())
                        );
                        println!(
                            "{}: {}",
                            "End".color(CliColors::muted()),
                            end.format("%Y-%m-%d %H:%M:%S UTC")
                                .to_string()
                                .color(CliColors::primary())
                        );
                        println!(
                            "{}: {} days ({} seconds)",
                            "Duration".color(CliColors::muted()),
                            duration_days.to_string().color(CliColors::accent()),
                            duration_seconds.to_string().color(CliColors::muted())
                        );
                        println!(
                            "{}: {}",
                            "Memory Count".color(CliColors::muted()),
                            memories.len().to_string().color(CliColors::accent())
                        );
                    }
                    return Ok(());
                }
            }

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_graph(&graph);
            }
        }

        GraphCommands::Paths(args) => {
            let from_id = resolve_memory_id(ctx, &args.from).await?;
            let to_id = resolve_memory_id(ctx, &args.to).await?;
            let paths = ctx
                .memory_manager
                .find_paths(&from_id, &to_id, args.depth)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&paths).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_paths(&paths);
            }
        }

        GraphCommands::Connected(args) => {
            let memory_id = resolve_memory_id(ctx, &args.id).await?;

            let relationship_type = if args.relationship_type == "all" {
                None
            } else {
                Some(args.relationship_type.as_str())
            };

            let mut graph = ctx
                .memory_manager
                .get_memory_graph(&memory_id, args.depth)
                .await?;

            let mut queue = VecDeque::new();
            let mut visited = HashSet::new();
            queue.push_back((memory_id.clone(), 0u8));
            visited.insert(memory_id.clone());

            while let Some((current_id, current_depth)) = queue.pop_front() {
                if current_depth >= args.depth {
                    continue;
                }

                let filter = RelationshipFilter {
                    source_id: Some(current_id.clone()),
                    target_id: None,
                    relationship_type: relationship_type.map(|s| s.to_string()),
                    ..Default::default()
                };
                let source_rels = ctx
                    .memory_manager
                    .list_relationships(Some(filter), Some(100), None)
                    .await
                    .unwrap_or_default();

                let filter = RelationshipFilter {
                    source_id: None,
                    target_id: Some(current_id.clone()),
                    relationship_type: relationship_type.map(|s| s.to_string()),
                    ..Default::default()
                };
                let target_rels = ctx
                    .memory_manager
                    .list_relationships(Some(filter), Some(100), None)
                    .await
                    .unwrap_or_default();

                let mut all_rels = source_rels;
                all_rels.extend(target_rels);

                for rel in all_rels {
                    let other_id = if rel.source_id == current_id {
                        &rel.target_id
                    } else {
                        &rel.source_id
                    };

                    let source_is_memory = ctx
                        .memory_manager
                        .get_memory(&rel.source_id)
                        .await?
                        .is_some();
                    let target_is_memory = ctx
                        .memory_manager
                        .get_memory(&rel.target_id)
                        .await?
                        .is_some();

                    if source_is_memory && target_is_memory {
                        if !graph.relationships.iter().any(|r| r.id == rel.id) {
                            graph.add_relationship(rel.clone());
                        }

                        if !graph.memories.contains_key(other_id)
                            && let Some(memory) = ctx.memory_manager.get_memory(other_id).await?
                        {
                            graph.add_memory(memory.clone());
                        }

                        if !visited.contains(other_id) {
                            visited.insert(other_id.clone());
                            queue.push_back((other_id.clone(), current_depth + 1));
                        }
                    }
                }
            }

            if let Some(rel_type) = relationship_type {
                graph
                    .relationships
                    .retain(|r| r.relationship_type == rel_type);
            }

            if args.no_temporal {
                graph
                    .relationships
                    .retain(|r| r.relationship_type != "temporal_sequence");
            }

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_connected_memories_tree(&memory_id, &graph, args.no_temporal).await?;
            }
        }

        GraphCommands::Metrics => {
            let memory_count = ctx.memory_manager.count_memories(None).await?;
            let relationship_count = ctx.memory_manager.count_relationships(None).await?;
            let entity_count = ctx.memory_manager.count_entities(None).await?;

            let average_degree = if memory_count > 0 {
                (relationship_count as f64 * 2.0) / memory_count as f64
            } else {
                0.0
            };

            let density = if memory_count > 1 {
                relationship_count as f64 / ((memory_count * (memory_count - 1)) as f64 / 2.0)
            } else {
                0.0
            };

            let mut central_memories = Vec::new();
            let sample_memories = ctx
                .memory_manager
                .filter_memories(MemoryFilter::default(), None, None, Some(50))
                .await?;

            let mut memory_centrality: Vec<(String, usize, String)> = Vec::new();
            for memory in sample_memories {
                if let Ok(graph) = ctx.memory_manager.get_memory_graph(&memory.id, 1).await {
                    let centrality_score = graph.relationships.len();
                    memory_centrality.push((
                        memory.id.clone(),
                        centrality_score,
                        memory.content.chars().take(100).collect::<String>(),
                    ));
                }
            }

            memory_centrality.sort_by(|a, b| b.1.cmp(&a.1));
            for (memory_id, score, content_preview) in memory_centrality.into_iter().take(5) {
                central_memories.push((memory_id, score as f64, content_preview));
            }

            // Calculate connected components using BFS
            let mut connected_components = 0u64;
            let mut visited = std::collections::HashSet::new();

            // Get all memories for traversal
            let all_memories = ctx
                .memory_manager
                .filter_memories(MemoryFilter::default(), None, None, None)
                .await?;

            for memory in &all_memories {
                if visited.contains(&memory.id) {
                    continue;
                }

                // BFS to find all connected memories
                let mut queue = std::collections::VecDeque::new();
                queue.push_back(memory.id.clone());
                visited.insert(memory.id.clone());

                while let Some(current_id) = queue.pop_front() {
                    // Get relationships where this memory is source or target
                    let source_filter = RelationshipFilter {
                        source_id: Some(current_id.clone()),
                        target_id: None,
                        ..Default::default()
                    };
                    let target_filter = RelationshipFilter {
                        source_id: None,
                        target_id: Some(current_id.clone()),
                        ..Default::default()
                    };

                    let source_rels = ctx
                        .memory_manager
                        .list_relationships(Some(source_filter), None, None)
                        .await
                        .unwrap_or_default();
                    let target_rels = ctx
                        .memory_manager
                        .list_relationships(Some(target_filter), None, None)
                        .await
                        .unwrap_or_default();

                    for rel in source_rels.iter().chain(target_rels.iter()) {
                        // Check if both source and target are memories
                        // Use unwrap_or_default to handle errors gracefully
                        let source_is_memory = ctx
                            .memory_manager
                            .get_memory(&rel.source_id)
                            .await
                            .unwrap_or(None)
                            .is_some();
                        let target_is_memory = ctx
                            .memory_manager
                            .get_memory(&rel.target_id)
                            .await
                            .unwrap_or(None)
                            .is_some();

                        if source_is_memory && target_is_memory {
                            let other_id = if rel.source_id == current_id {
                                &rel.target_id
                            } else {
                                &rel.source_id
                            };

                            if !visited.contains(other_id) {
                                visited.insert(other_id.clone());
                                queue.push_back(other_id.clone());
                            }
                        }
                    }
                }

                connected_components += 1;
            }

            if output_format == "json" {
                let result = json!({
                    "memory_count": memory_count,
                    "entity_count": entity_count,
                    "relationship_count": relationship_count,
                    "average_degree": average_degree,
                    "density": density,
                    "connected_components": connected_components,
                    "central_memories": central_memories.iter().map(|(id, score, content)| json!({
                        "memory_id": id,
                        "centrality_score": score,
                        "content_preview": content
                    })).collect::<Vec<_>>()
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    "━━━ Graph Metrics ━━━".color(CliColors::accent()).bold()
                );
                println!();
                println!(
                    "{}: {}",
                    "Memories".color(CliColors::muted()),
                    memory_count.to_string().color(CliColors::accent()).bold()
                );
                println!(
                    "{}: {}",
                    "Relationships".color(CliColors::muted()),
                    relationship_count
                        .to_string()
                        .color(CliColors::accent())
                        .bold()
                );
                println!(
                    "{}: {}",
                    "Entities".color(CliColors::muted()),
                    entity_count.to_string().color(CliColors::accent()).bold()
                );
                println!(
                    "{}: {:.2}",
                    "Average Degree".color(CliColors::muted()),
                    average_degree.to_string().color(CliColors::accent())
                );
                println!(
                    "{}: {:.4}",
                    "Graph Density".color(CliColors::muted()),
                    density.to_string().color(CliColors::accent())
                );
                println!(
                    "{}: {}",
                    "Connected Components".color(CliColors::muted()),
                    connected_components.to_string().color(CliColors::accent())
                );

                if !central_memories.is_empty() {
                    println!();
                    println!(
                        "{}",
                        "Central Memories (Top 5):".color(CliColors::muted()).bold()
                    );
                    for (i, (memory_id, score, content)) in central_memories.iter().enumerate() {
                        println!(
                            "  {}. {} (score: {:.1}) - {}",
                            (i + 1).to_string().color(CliColors::muted()),
                            memory_id[..8].color(CliColors::accent()),
                            score,
                            content.color(CliColors::primary())
                        );
                    }
                }
            }
        }

        GraphCommands::Query(args) => {
            if args.pattern.trim().is_empty() {
                output_error("Query pattern cannot be empty", output_format);
                return Ok(());
            }

            let pattern = args.pattern.to_lowercase();
            let limit = args.limit.min(100);
            let mut results = Vec::new();

            if pattern.contains("connected") || pattern.contains("related") {
                let all_memories = ctx
                    .memory_manager
                    .filter_memories(MemoryFilter::default(), None, None, Some(limit * 2))
                    .await?;

                if all_memories.is_empty() {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "query": args.pattern,
                                "results": [],
                                "total_results": 0,
                                "message": "No memories found in storage"
                            }))
                            .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        println!("{}", format_info("No memories found in storage."));
                    }
                    return Ok(());
                }

                for memory in all_memories.into_iter().take(limit) {
                    match ctx.memory_manager.get_memory_graph(&memory.id, 1).await {
                        Ok(graph) => {
                            if !graph.relationships.is_empty() {
                                results.push((memory.id.clone(), graph));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to get graph for memory {}: {}", memory.id, e);
                        }
                    }
                }
            } else if pattern.contains("isolated") || pattern.contains("orphan") {
                let all_memories = ctx
                    .memory_manager
                    .filter_memories(MemoryFilter::default(), None, None, Some(limit * 2))
                    .await?;

                if all_memories.is_empty() {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "query": args.pattern,
                                "results": [],
                                "total_results": 0,
                                "message": "No memories found in storage"
                            }))
                            .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        println!("{}", format_info("No memories found in storage."));
                    }
                    return Ok(());
                }

                for memory in all_memories.into_iter().take(limit) {
                    match ctx.memory_manager.get_memory_graph(&memory.id, 1).await {
                        Ok(graph) => {
                            if graph.relationships.is_empty() && graph.memories.len() == 1 {
                                results.push((memory.id.clone(), graph));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to get graph for memory {}: {}", memory.id, e);
                        }
                    }
                }
            } else {
                let search_results = ctx
                    .memory_manager
                    .search(
                        &args.pattern,
                        Some(limit),
                        None,
                        locai::memory::search_extensions::SearchMode::Text,
                    )
                    .await?;

                for search_result in search_results {
                    match ctx
                        .memory_manager
                        .get_memory_graph(&search_result.memory.id, 1)
                        .await
                    {
                        Ok(graph) => {
                            results.push((search_result.memory.id.clone(), graph));
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to get graph for memory {}: {}",
                                search_result.memory.id,
                                e
                            );
                        }
                    }
                }
            }

            if output_format == "json" {
                let result = json!({
                    "query": args.pattern,
                    "results": results.iter().map(|(id, graph)| json!({
                        "center_id": id,
                        "memories": graph.memories.values().collect::<Vec<_>>(),
                        "relationships": graph.relationships,
                        "metadata": {
                            "total_nodes": graph.memories.len(),
                            "total_edges": graph.relationships.len()
                        }
                    })).collect::<Vec<_>>(),
                    "total_results": results.len()
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    format_info(&format!(
                        "Graph Query: \"{}\"",
                        args.pattern.color(CliColors::accent())
                    ))
                );
                if results.is_empty() {
                    println!("{}", format_info("No matching graph structures found."));
                } else {
                    println!();
                    println!("Found {} matching graph structures:", results.len());
                    println!();
                    for (i, (center_id, graph)) in results.iter().enumerate() {
                        println!(
                            "Graph {} (Center: {}):",
                            (i + 1).to_string().color(CliColors::muted()),
                            center_id[..8].color(CliColors::accent())
                        );
                        println!(
                            "  Nodes: {} memories",
                            graph.memories.len().to_string().color(CliColors::accent())
                        );
                        println!(
                            "  Edges: {} relationships",
                            graph
                                .relationships
                                .len()
                                .to_string()
                                .color(CliColors::accent())
                        );
                        if !graph.memories.is_empty() {
                            println!("  Memories:");
                            for memory in graph.memories.values().take(3) {
                                let content = if memory.content.len() > 60 {
                                    format!("{}...", &memory.content[..57])
                                } else {
                                    memory.content.clone()
                                };
                                println!(
                                    "    {} [{}] {}",
                                    "●".color(CliColors::accent()),
                                    format_memory_type(&memory.memory_type),
                                    content.color(CliColors::primary())
                                );
                            }
                            if graph.memories.len() > 3 {
                                println!(
                                    "    ... and {} more",
                                    (graph.memories.len() - 3)
                                        .to_string()
                                        .color(CliColors::muted())
                                );
                            }
                        }
                        if i < results.len() - 1 {
                            println!();
                        }
                    }
                }
            }
        }

        GraphCommands::Similar(args) => {
            // Validate pattern_id exists
            if ctx
                .memory_manager
                .get_memory(&args.pattern_id)
                .await?
                .is_none()
            {
                output_error(
                    &format!("Pattern memory '{}' not found", args.pattern_id),
                    output_format,
                );
                return Ok(());
            }

            let pattern_graph = ctx
                .memory_manager
                .get_memory_graph(&args.pattern_id, 2)
                .await?;

            let pattern_memory_count = pattern_graph.memories.len();
            let pattern_relationship_count = pattern_graph.relationships.len();
            let pattern_relationship_types: HashSet<String> = pattern_graph
                .relationships
                .iter()
                .map(|r| r.relationship_type.clone())
                .collect();

            if pattern_graph.memories.is_empty() {
                output_error(
                    &format!(
                        "Pattern memory '{}' has no graph structure",
                        args.pattern_id
                    ),
                    output_format,
                );
                return Ok(());
            }

            let mut similar_structures = Vec::new();
            let candidate_memories = ctx
                .memory_manager
                .filter_memories(MemoryFilter::default(), None, None, Some(100))
                .await?;

            if candidate_memories.is_empty() {
                if output_format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "pattern_id": args.pattern_id,
                            "results": [],
                            "total_results": 0,
                            "message": "No candidate memories found for comparison"
                        }))
                        .unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    println!(
                        "{}",
                        format_info("No candidate memories found for comparison.")
                    );
                }
                return Ok(());
            }

            for memory in candidate_memories {
                if memory.id == args.pattern_id {
                    continue;
                }

                match ctx.memory_manager.get_memory_graph(&memory.id, 2).await {
                    Ok(candidate_graph) => {
                        let candidate_memory_count = candidate_graph.memories.len();
                        let candidate_relationship_count = candidate_graph.relationships.len();
                        let candidate_relationship_types: HashSet<String> = candidate_graph
                            .relationships
                            .iter()
                            .map(|r| r.relationship_type.clone())
                            .collect();

                        let structure_similarity = if pattern_memory_count > 0
                            && candidate_memory_count > 0
                        {
                            let memory_ratio = (pattern_memory_count.min(candidate_memory_count)
                                as f64)
                                / (pattern_memory_count.max(candidate_memory_count) as f64);
                            let relationship_ratio = if pattern_relationship_count > 0
                                && candidate_relationship_count > 0
                            {
                                (pattern_relationship_count.min(candidate_relationship_count)
                                    as f64)
                                    / (pattern_relationship_count.max(candidate_relationship_count)
                                        as f64)
                            } else {
                                0.0
                            };
                            let type_overlap = pattern_relationship_types
                                .intersection(&candidate_relationship_types)
                                .count() as f64
                                / pattern_relationship_types
                                    .union(&candidate_relationship_types)
                                    .count()
                                    .max(1) as f64;

                            memory_ratio * 0.3 + relationship_ratio * 0.3 + type_overlap * 0.4
                        } else {
                            0.0
                        };

                        if structure_similarity > 0.5 {
                            similar_structures.push((
                                memory.id.clone(),
                                structure_similarity,
                                candidate_memory_count,
                                candidate_relationship_count,
                                candidate_relationship_types
                                    .intersection(&pattern_relationship_types)
                                    .cloned()
                                    .collect::<Vec<_>>(),
                            ));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to get graph for memory {}: {}", memory.id, e);
                    }
                }
            }

            similar_structures
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            similar_structures.truncate(args.limit);

            if output_format == "json" {
                let result = json!({
                    "pattern_id": args.pattern_id,
                    "results": similar_structures.iter().map(|(id, similarity, nodes, edges, types)| json!({
                        "memory_id": id,
                        "similarity": similarity,
                        "nodes": nodes,
                        "edges": edges,
                        "common_types": types
                    })).collect::<Vec<_>>()
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    format_info(&format!(
                        "Similar Structures to: {}",
                        args.pattern_id.color(CliColors::accent())
                    ))
                );
                if similar_structures.is_empty() {
                    println!("{}", format_info("No similar structures found."));
                } else {
                    println!();
                    for (i, (memory_id, similarity, nodes, edges, types)) in
                        similar_structures.iter().enumerate()
                    {
                        println!(
                            "{}. {} (Similarity: {:.2})",
                            (i + 1).to_string().color(CliColors::muted()),
                            memory_id[..8].color(CliColors::accent()),
                            similarity
                        );
                        println!("   Structure: {} memories, {} relationships", nodes, edges);
                        if !types.is_empty() {
                            println!("   Types: {}", types.join(", ").color(CliColors::info()));
                        }
                        println!();
                    }
                }
            }
        }

        GraphCommands::Entity(args) => {
            // Check if ID exists as either memory or entity
            let is_memory = ctx.memory_manager.get_memory(&args.id).await?.is_some();
            let is_entity = ctx.memory_manager.get_entity(&args.id).await?.is_some();

            if !is_memory && !is_entity {
                output_error(
                    &format!("ID '{}' not found (not a memory or entity)", args.id),
                    output_format,
                );
                return Ok(());
            }

            if is_memory {
                let graph = ctx
                    .memory_manager
                    .get_memory_graph(&args.id, args.depth)
                    .await?;

                if args.include_temporal_span && !graph.memories.is_empty() {
                    let memories: Vec<_> = graph.memories.values().collect();
                    let mut timestamps: Vec<_> = memories.iter().map(|m| m.created_at).collect();
                    timestamps.sort();

                    if let (Some(&start), Some(&end)) = (timestamps.first(), timestamps.last()) {
                        let duration_days = (end - start).num_days();
                        let duration_seconds = (end - start).num_seconds();

                        if output_format == "json" {
                            let mut graph_json: Value =
                                serde_json::to_value(&graph).unwrap_or_default();
                            graph_json["temporal_span"] = json!({
                                "start": start.to_rfc3339(),
                                "end": end.to_rfc3339(),
                                "duration_days": duration_days,
                                "duration_seconds": duration_seconds,
                                "memory_count": memories.len()
                            });
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&graph_json)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );
                        } else {
                            print_memory_graph(&graph);
                            println!();
                            println!(
                                "{}",
                                "━━━ Temporal Span ━━━".color(CliColors::accent()).bold()
                            );
                            println!(
                                "{}: {}",
                                "Start".color(CliColors::muted()),
                                start
                                    .format("%Y-%m-%d %H:%M:%S UTC")
                                    .to_string()
                                    .color(CliColors::primary())
                            );
                            println!(
                                "{}: {}",
                                "End".color(CliColors::muted()),
                                end.format("%Y-%m-%d %H:%M:%S UTC")
                                    .to_string()
                                    .color(CliColors::primary())
                            );
                            println!(
                                "{}: {} days ({} seconds)",
                                "Duration".color(CliColors::muted()),
                                duration_days.to_string().color(CliColors::accent()),
                                duration_seconds.to_string().color(CliColors::muted())
                            );
                            println!(
                                "{}: {}",
                                "Memory Count".color(CliColors::muted()),
                                memories.len().to_string().color(CliColors::accent())
                            );
                        }
                        return Ok(());
                    }
                }

                if output_format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    print_memory_graph(&graph);
                }
            } else {
                let related_entities = ctx
                    .memory_manager
                    .find_related_entities(&args.id, None, Some("both".to_string()))
                    .await?;

                let mut relationships = ctx
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

                let target_relationships = ctx
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

                relationships.extend(target_relationships);

                let filter = RelationshipFilter {
                    target_id: Some(args.id.clone()),
                    relationship_type: Some("contains".to_string()),
                    ..Default::default()
                };
                let memory_relationships = ctx
                    .memory_manager
                    .list_relationships(Some(filter), None, None)
                    .await?;

                let mut memories = Vec::new();
                for rel in memory_relationships {
                    if let Ok(Some(memory)) = ctx.memory_manager.get_memory(&rel.source_id).await {
                        memories.push(memory);
                    }
                }

                if output_format == "json" {
                    let result = json!({
                        "entity_id": args.id,
                        "related_entities": related_entities,
                        "relationships": relationships,
                        "memories": memories
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    println!(
                        "{}",
                        format_info(&format!(
                            "Entity Graph: {}",
                            args.id.color(CliColors::accent())
                        ))
                    );
                    println!();
                    println!(
                        "{}: {}",
                        "Memories".color(CliColors::muted()),
                        memories.len().to_string().color(CliColors::accent())
                    );
                    println!(
                        "{}: {}",
                        "Entities".color(CliColors::muted()),
                        related_entities
                            .len()
                            .to_string()
                            .color(CliColors::accent())
                    );
                    println!(
                        "{}: {}",
                        "Relationships".color(CliColors::muted()),
                        relationships.len().to_string().color(CliColors::accent())
                    );

                    if !memories.is_empty() {
                        println!();
                        println!("{}", "Memories:".color(CliColors::primary()).bold());
                        for memory in memories.iter().take(10) {
                            let content = if memory.content.len() > 60 {
                                format!("{}...", &memory.content[..57])
                            } else {
                                memory.content.clone()
                            };
                            println!(
                                "  {} [{}] {}",
                                "●".color(CliColors::accent()),
                                format_memory_type(&memory.memory_type),
                                content.color(CliColors::primary())
                            );
                        }
                    }

                    if !related_entities.is_empty() {
                        println!();
                        println!("{}", "Related Entities:".color(CliColors::primary()).bold());
                        for entity in related_entities.iter().take(10) {
                            println!(
                                "  {} {} ({})",
                                "◇".color(CliColors::entity()),
                                entity.entity_type.color(CliColors::entity()),
                                entity.id.color(CliColors::accent())
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
