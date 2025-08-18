//! Graph operations API endpoints

use std::sync::Arc;

use axum::{
    Json as JsonExtractor,
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    api::dto::{
        CentralMemoryDto, EntityDto, GraphMetricsDto, GraphQueryRequest, MemoryGraphDto,
        MemoryPathDto,
    },
    error::{ServerError, ServerResult, not_found},
    state::AppState,
};

/// Get memory graph
#[utoipa::path(
    get,
    path = "/api/memories/{id}/graph",
    tag = "graph",
    params(
        ("id" = String, Path, description = "Memory ID"),
        ("depth" = Option<u8>, Query, description = "Graph traversal depth")
    ),
    responses(
        (status = 200, description = "Memory graph", body = MemoryGraphDto),
        (status = 404, description = "Memory not found"),
    )
)]
pub async fn get_memory_graph(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<GraphParams>,
) -> ServerResult<Json<MemoryGraphDto>> {
    let depth = params.depth.unwrap_or(2);

    // First check if the memory exists
    let _memory = state
        .memory_manager
        .get_memory(&id)
        .await?
        .ok_or_else(|| not_found("Memory", &id))?;

    let graph = state.memory_manager.get_memory_graph(&id, depth).await?;
    let graph_dto = MemoryGraphDto::from(graph);

    Ok(Json(graph_dto))
}

/// Get entity graph
#[utoipa::path(
    get,
    path = "/api/entities/{id}/graph",
    tag = "graph",
    params(
        ("id" = String, Path, description = "Entity ID"),
        ("depth" = Option<u8>, Query, description = "Graph traversal depth")
    ),
    responses(
        (status = 200, description = "Entity graph", body = MemoryGraphDto),
        (status = 404, description = "Entity not found"),
    )
)]
pub async fn get_entity_graph(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<GraphParams>,
) -> ServerResult<Json<MemoryGraphDto>> {
    let depth = params.depth.unwrap_or(2);

    // Check if entity exists
    let _entity = state
        .memory_manager
        .get_entity(&id)
        .await?
        .ok_or_else(|| not_found("Entity", &id))?;

    // Create a graph centered on this entity
    use locai::storage::models::MemoryGraph;
    use std::collections::HashMap;

    let mut graph = MemoryGraph {
        center_id: id.clone(),
        memories: HashMap::new(),
        relationships: Vec::new(),
    };

    // If the entity is actually a memory, get its memory graph
    if let Ok(Some(_memory)) = state.memory_manager.get_memory(&id).await {
        // This entity is also a memory, so we can get its full graph
        let memory_graph = state.memory_manager.get_memory_graph(&id, depth).await?;
        return Ok(Json(MemoryGraphDto::from(memory_graph)));
    }

    // Otherwise, find related entities and their memories
    let related_entities = state
        .memory_manager
        .find_related_entities(
            &id,
            None, // No relationship type filter
            Some("both".to_string()),
        )
        .await?;

    // Get relationships involving this entity
    let relationships = state
        .memory_manager
        .list_relationships(
            Some(locai::storage::filters::RelationshipFilter {
                source_id: Some(id.clone()),
                ..Default::default()
            }),
            None,
            None,
        )
        .await?;

    // Add relationships where this entity is the target
    let mut target_relationships = state
        .memory_manager
        .list_relationships(
            Some(locai::storage::filters::RelationshipFilter {
                target_id: Some(id.clone()),
                ..Default::default()
            }),
            None,
            None,
        )
        .await?;

    // Combine relationships
    let mut all_relationships = relationships;
    all_relationships.append(&mut target_relationships);

    // For each related entity, if it's a memory, add it to the graph
    for related_entity in related_entities {
        if let Ok(Some(memory)) = state.memory_manager.get_memory(&related_entity.id).await {
            graph.memories.insert(related_entity.id.clone(), memory);
        }
    }

    // Add relationships to the graph
    for relationship in all_relationships {
        graph.relationships.push(relationship);
    }

    let graph_dto = MemoryGraphDto::from(graph);
    Ok(Json(graph_dto))
}

/// Find paths between memories
#[utoipa::path(
    get,
    path = "/api/graph/paths",
    tag = "graph",
    params(PathParams),
    responses(
        (status = 200, description = "Paths between memories", body = Vec<MemoryPathDto>),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn find_paths(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PathParams>,
) -> ServerResult<Json<Vec<MemoryPathDto>>> {
    let from_id = params
        .from
        .ok_or_else(|| ServerError::BadRequest("Missing 'from' parameter".to_string()))?;
    let to_id = params
        .to
        .ok_or_else(|| ServerError::BadRequest("Missing 'to' parameter".to_string()))?;
    let max_depth = params.max_depth.unwrap_or(5);

    let paths = state
        .memory_manager
        .find_paths(&from_id, &to_id, max_depth)
        .await?;
    let path_dtos: Vec<MemoryPathDto> = paths.into_iter().map(MemoryPathDto::from).collect();

    Ok(Json(path_dtos))
}

/// Execute graph query
#[utoipa::path(
    post,
    path = "/api/graph/query",
    tag = "graph",
    request_body = GraphQueryRequest,
    responses(
        (status = 200, description = "Query results", body = Vec<MemoryGraphDto>),
    )
)]
pub async fn query_graph(
    State(state): State<Arc<AppState>>,
    JsonExtractor(request): JsonExtractor<GraphQueryRequest>,
) -> ServerResult<Json<Vec<MemoryGraphDto>>> {
    // For now, implement a simple pattern matching system
    // In a full implementation, this would parse a graph query language

    let pattern = request.pattern.to_lowercase();
    let limit = request.limit.min(100); // Cap at 100 results

    // Simple pattern matching based on keywords
    let mut results = Vec::new();

    if pattern.contains("connected") || pattern.contains("related") {
        // Find highly connected memories
        let all_memories = state
            .memory_manager
            .filter_memories(
                locai::storage::filters::MemoryFilter::default(),
                None,
                None,
                Some(limit * 2), // Get more to filter
            )
            .await?;

        // For each memory, get its graph and check connectivity
        for memory in all_memories.into_iter().take(limit) {
            if let Ok(graph) = state.memory_manager.get_memory_graph(&memory.id, 1).await {
                // If the memory has relationships, include it
                if !graph.relationships.is_empty() {
                    results.push(MemoryGraphDto::from(graph));
                }
            }
        }
    } else if pattern.contains("isolated") || pattern.contains("orphan") {
        // Find memories with no relationships
        let all_memories = state
            .memory_manager
            .filter_memories(
                locai::storage::filters::MemoryFilter::default(),
                None,
                None,
                Some(limit * 2),
            )
            .await?;

        for memory in all_memories.into_iter().take(limit) {
            if let Ok(graph) = state.memory_manager.get_memory_graph(&memory.id, 1).await {
                // If the memory has no relationships, include it
                if graph.relationships.is_empty() && graph.memories.len() == 1 {
                    results.push(MemoryGraphDto::from(graph));
                }
            }
        }
    } else {
        // Default: semantic search on the pattern and return graphs
        let search_results = state
            .memory_manager
            .search(
                &request.pattern,
                Some(limit),
                None,
                locai::memory::search_extensions::SearchMode::Text,
            )
            .await?;

        for search_result in search_results {
            if let Ok(graph) = state
                .memory_manager
                .get_memory_graph(&search_result.memory.id, 1)
                .await
            {
                results.push(MemoryGraphDto::from(graph));
            }
        }
    }

    Ok(Json(results))
}

/// Get graph metrics
#[utoipa::path(
    get,
    path = "/api/graph/metrics",
    tag = "graph",
    responses(
        (status = 200, description = "Graph metrics", body = GraphMetricsDto),
    )
)]
pub async fn get_graph_metrics(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<GraphMetricsDto>> {
    // Get counts from memory manager
    let memory_count = state.memory_manager.count_memories(None).await?;
    let relationship_count = state.memory_manager.count_relationships(None).await?;

    // Calculate basic metrics
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

    // Find central memories by getting memories with the most relationships
    let mut central_memories = Vec::new();

    // Get a sample of memories to analyze
    let sample_memories = state
        .memory_manager
        .filter_memories(
            locai::storage::filters::MemoryFilter::default(),
            None,
            None,
            Some(50), // Sample size
        )
        .await?;

    // Calculate centrality for each memory (simplified as relationship count)
    let mut memory_centrality: Vec<(String, usize, String)> = Vec::new();

    for memory in sample_memories {
        if let Ok(graph) = state.memory_manager.get_memory_graph(&memory.id, 1).await {
            let centrality_score = graph.relationships.len();
            memory_centrality.push((
                memory.id.clone(),
                centrality_score,
                memory.content.chars().take(100).collect::<String>(),
            ));
        }
    }

    // Sort by centrality and take top 5
    memory_centrality.sort_by(|a, b| b.1.cmp(&a.1));

    for (memory_id, score, content_preview) in memory_centrality.into_iter().take(5) {
        central_memories.push(CentralMemoryDto {
            memory_id,
            centrality_score: score as f64,
            content_preview,
        });
    }

    let metrics = GraphMetricsDto {
        memory_count,
        relationship_count,
        average_degree,
        density,
        connected_components: 1, // Simplified - would need graph analysis for real value
        central_memories,
    };

    Ok(Json(metrics))
}

/// Find similar structures
#[utoipa::path(
    get,
    path = "/api/graph/similar_structures",
    tag = "graph",
    params(
        ("pattern" = Option<String>, Query, description = "Pattern ID")
    ),
    responses(
        (status = 200, description = "Similar structures", body = Vec<MemoryGraphDto>),
    )
)]
pub async fn find_similar_structures(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SimilarStructuresParams>,
) -> ServerResult<Json<Vec<MemoryGraphDto>>> {
    let pattern_id = params
        .pattern
        .ok_or_else(|| ServerError::BadRequest("Missing 'pattern' parameter".to_string()))?;

    // Get the pattern memory's graph structure
    let pattern_graph = state
        .memory_manager
        .get_memory_graph(&pattern_id, 2)
        .await?;

    // Analyze the pattern structure
    let pattern_memory_count = pattern_graph.memories.len();
    let pattern_relationship_count = pattern_graph.relationships.len();
    let pattern_relationship_types: std::collections::HashSet<String> = pattern_graph
        .relationships
        .iter()
        .map(|r| r.relationship_type.clone())
        .collect();

    // Find memories with similar graph structures
    let mut similar_structures = Vec::new();

    // Get a sample of memories to compare against
    let candidate_memories = state
        .memory_manager
        .filter_memories(
            locai::storage::filters::MemoryFilter::default(),
            None,
            None,
            Some(100), // Limit candidates for performance
        )
        .await?;

    for memory in candidate_memories {
        // Skip the pattern memory itself
        if memory.id == pattern_id {
            continue;
        }

        // Get the candidate's graph structure
        if let Ok(candidate_graph) = state.memory_manager.get_memory_graph(&memory.id, 2).await {
            let candidate_memory_count = candidate_graph.memories.len();
            let candidate_relationship_count = candidate_graph.relationships.len();
            let candidate_relationship_types: std::collections::HashSet<String> = candidate_graph
                .relationships
                .iter()
                .map(|r| r.relationship_type.clone())
                .collect();

            // Calculate similarity based on structure
            let memory_count_similarity =
                if pattern_memory_count == 0 && candidate_memory_count == 0 {
                    1.0
                } else {
                    let max_count = pattern_memory_count.max(candidate_memory_count) as f64;
                    let min_count = pattern_memory_count.min(candidate_memory_count) as f64;
                    min_count / max_count
                };

            let relationship_count_similarity = if pattern_relationship_count == 0
                && candidate_relationship_count == 0
            {
                1.0
            } else {
                let max_count = pattern_relationship_count.max(candidate_relationship_count) as f64;
                let min_count = pattern_relationship_count.min(candidate_relationship_count) as f64;
                min_count / max_count
            };

            // Calculate relationship type overlap
            let common_types = pattern_relationship_types
                .intersection(&candidate_relationship_types)
                .count();
            let total_unique_types = pattern_relationship_types
                .union(&candidate_relationship_types)
                .count();

            let type_similarity = if total_unique_types == 0 {
                1.0
            } else {
                common_types as f64 / total_unique_types as f64
            };

            // Overall similarity score (weighted average)
            let similarity_score = (memory_count_similarity * 0.3)
                + (relationship_count_similarity * 0.4)
                + (type_similarity * 0.3);

            // Include if similarity is above threshold
            if similarity_score > 0.6 {
                similar_structures.push(MemoryGraphDto::from(candidate_graph));
            }
        }

        // Limit results
        if similar_structures.len() >= 10 {
            break;
        }
    }

    Ok(Json(similar_structures))
}

/// Get related entities
#[utoipa::path(
    get,
    path = "/api/entities/{id}/related_entities",
    tag = "graph",
    params(
        ("id" = String, Path, description = "Entity ID"),
        ("relationship_type" = Option<String>, Query, description = "Relationship type filter")
    ),
    responses(
        (status = 200, description = "Related entities", body = Vec<EntityDto>),
        (status = 404, description = "Entity not found"),
    )
)]
pub async fn get_related_entities(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<RelatedEntitiesParams>,
) -> ServerResult<Json<Vec<EntityDto>>> {
    // Check if entity exists
    let _entity = state
        .memory_manager
        .get_entity(&id)
        .await?
        .ok_or_else(|| not_found("Entity", &id))?;

    // Find related entities
    let related_entities = state
        .memory_manager
        .find_related_entities(
            &id,
            params.relationship_type,
            Some("both".to_string()), // Look in both directions
        )
        .await?;

    let entity_dtos: Vec<EntityDto> = related_entities.into_iter().map(EntityDto::from).collect();

    Ok(Json(entity_dtos))
}

/// Get central entities
#[utoipa::path(
    get,
    path = "/api/entities/central",
    tag = "graph",
    responses(
        (status = 200, description = "Central entities", body = Vec<CentralMemoryDto>),
    )
)]
pub async fn get_central_entities(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<CentralMemoryDto>>> {
    // Get all entities
    let entities = state
        .memory_manager
        .list_entities(None, Some(100), None)
        .await?;

    // Calculate centrality for each entity based on relationship count
    let mut entity_centrality: Vec<(String, usize, String)> = Vec::new();

    for entity in entities {
        // Count relationships involving this entity
        let outgoing_relationships = state
            .memory_manager
            .list_relationships(
                Some(locai::storage::filters::RelationshipFilter {
                    source_id: Some(entity.id.clone()),
                    ..Default::default()
                }),
                None,
                None,
            )
            .await
            .unwrap_or_default();

        let incoming_relationships = state
            .memory_manager
            .list_relationships(
                Some(locai::storage::filters::RelationshipFilter {
                    target_id: Some(entity.id.clone()),
                    ..Default::default()
                }),
                None,
                None,
            )
            .await
            .unwrap_or_default();

        let total_relationships = outgoing_relationships.len() + incoming_relationships.len();

        // Create a content preview from entity properties
        let content_preview = if let Some(name) = entity.properties.get("name") {
            name.as_str().unwrap_or(&entity.entity_type).to_string()
        } else {
            format!(
                "{} ({})",
                entity.entity_type,
                entity.id.chars().take(8).collect::<String>()
            )
        };

        entity_centrality.push((entity.id, total_relationships, content_preview));
    }

    // Sort by centrality (relationship count) and take top entities
    entity_centrality.sort_by(|a, b| b.1.cmp(&a.1));

    let central_entities: Vec<CentralMemoryDto> = entity_centrality
        .into_iter()
        .take(10) // Top 10 most central entities
        .map(
            |(entity_id, relationship_count, content_preview)| CentralMemoryDto {
                memory_id: entity_id,
                centrality_score: relationship_count as f64,
                content_preview,
            },
        )
        .collect();

    Ok(Json(central_entities))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GraphParams {
    /// Graph traversal depth
    pub depth: Option<u8>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct PathParams {
    /// Source memory ID
    pub from: Option<String>,

    /// Target memory ID
    pub to: Option<String>,

    /// Maximum path depth
    pub max_depth: Option<u8>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SimilarStructuresParams {
    /// Pattern ID
    pub pattern: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct RelatedEntitiesParams {
    /// Relationship type filter
    pub relationship_type: Option<String>,
}
