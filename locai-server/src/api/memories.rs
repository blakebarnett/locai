//! Memory management API endpoints

use std::sync::Arc;

use axum::{
    Json as JsonExtractor,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use utoipa::IntoParams;

use locai::{
    memory::search_extensions::SearchMode as LocaiSearchMode,
    models::{MemoryBuilder, MemoryPriority, MemoryType},
    storage::filters::{MemoryFilter, SemanticSearchFilter},
};

use crate::{
    api::dto::{
        CreateMemoryRelationshipRequest, CreateMemoryRequest, GetMemoryRelationshipsParams,
        MemoryDto, RelationshipDto, SearchMode, SearchResultDto, ScoringConfigDto, UpdateMemoryRequest,
    },
    error::{ServerError, ServerResult, not_found},
    state::AppState,
    websocket::WebSocketMessage,
};

/// Create a new memory
#[utoipa::path(
    post,
    path = "/api/memories",
    tag = "memories",
    request_body = CreateMemoryRequest,
    responses(
        (status = 201, description = "Memory created successfully", body = MemoryDto),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_memory(
    State(state): State<Arc<AppState>>,
    JsonExtractor(request): JsonExtractor<CreateMemoryRequest>,
) -> Result<(StatusCode, Json<MemoryDto>), ServerError> {
    // Convert string types to enums
    let memory_type = MemoryType::from_str(&request.memory_type);
    let priority = match request.priority.as_str() {
        "low" => MemoryPriority::Low,
        "high" => MemoryPriority::High,
        "critical" => MemoryPriority::Critical,
        _ => MemoryPriority::Normal,
    };

    // Build the memory
    let memory = MemoryBuilder::new_with_content(request.content)
        .memory_type(memory_type)
        .priority(priority)
        .tags(request.tags.iter().map(|s| s.as_str()).collect())
        .source(request.source)
        .properties_json(request.properties)
        .build();

    // Store the memory
    let memory_id = state.memory_manager.store_memory(memory.clone()).await?;

    // Get the stored memory to return with proper ID
    let stored_memory = state
        .memory_manager
        .get_memory(&memory_id)
        .await?
        .ok_or_else(|| ServerError::Internal("Failed to retrieve stored memory".to_string()))?;

    // Broadcast WebSocket message
    let ws_message = WebSocketMessage::MemoryCreated {
        memory_id: stored_memory.id.clone(),
        content: stored_memory.content.clone(),
        memory_type: stored_memory.memory_type.to_string(),
        metadata: stored_memory.properties.clone(),
        importance: Some(match stored_memory.priority {
            locai::models::MemoryPriority::Low => 0.25,
            locai::models::MemoryPriority::Normal => 0.5,
            locai::models::MemoryPriority::High => 0.75,
            locai::models::MemoryPriority::Critical => 1.0,
        }),
        node_id: None, // Will be set by live query system if enabled
    };
    state.broadcast_message(ws_message);

    let memory_dto = MemoryDto::from(stored_memory);
    Ok((StatusCode::CREATED, Json(memory_dto)))
}

/// Get a memory by ID
#[utoipa::path(
    get,
    path = "/api/memories/{id}",
    tag = "memories",
    params(
        ("id" = String, Path, description = "Memory ID")
    ),
    responses(
        (status = 200, description = "Memory found", body = MemoryDto),
        (status = 404, description = "Memory not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<MemoryDto>> {
    let memory = state
        .memory_manager
        .get_memory(&id)
        .await?
        .ok_or_else(|| not_found("Memory", &id))?;

    let memory_dto = MemoryDto::from(memory);
    Ok(Json(memory_dto))
}

/// List memories with filtering and pagination
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListMemoriesParams {
    /// Page number (0-based)
    #[serde(default)]
    pub page: usize,

    /// Number of items per page
    #[serde(default = "default_page_size")]
    pub size: usize,

    /// Filter by memory type. For custom memory types, include the "custom:" prefix.
    /// Examples: "custom:dialogue", "custom:quest"
    #[param(example = "custom:dialogue")]
    pub memory_type: Option<String>,

    /// Filter by priority (capitalized values: "Low", "Normal", "High", "Critical")
    #[param(example = "Normal")]
    pub priority: Option<String>,

    /// Filter by tags (comma-separated)
    pub tags: Option<String>,

    /// Filter by source
    pub source: Option<String>,

    /// Filter by content (substring search)
    pub content: Option<String>,
}

fn default_page_size() -> usize {
    20
}

#[utoipa::path(
    get,
    path = "/api/memories",
    tag = "memories",
    params(ListMemoriesParams),
    responses(
        (status = 200, description = "List of memories", body = Vec<MemoryDto>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_memories(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListMemoriesParams>,
) -> ServerResult<Json<Vec<MemoryDto>>> {
    let mut filter = MemoryFilter::default();

    // Apply filters
    if let Some(memory_type) = params.memory_type {
        filter.memory_type = Some(memory_type);
    }

    if let Some(tags_str) = params.tags {
        let tags: Vec<String> = tags_str.split(',').map(|s| s.trim().to_string()).collect();
        filter.tags = Some(tags);
    }

    if let Some(content) = params.content {
        filter.content = Some(content);
    }

    if let Some(source) = params.source {
        filter.source = Some(source);
    }

    // Apply priority filter if specified
    if let Some(priority_str) = params.priority {
        let mut priority_properties = std::collections::HashMap::new();
        priority_properties.insert(
            "priority".to_string(),
            serde_json::Value::String(priority_str),
        );
        filter.properties = Some(priority_properties);
    }

    // Calculate offset for pagination
    let offset = params.page * params.size;

    // Get memories
    let memories = state
        .memory_manager
        .filter_memories(
            filter,
            None, // sort_by
            None, // sort_order
            Some(params.size),
        )
        .await?;

    // Apply manual pagination (since filter_memories doesn't support offset)
    let paginated_memories: Vec<_> = memories
        .into_iter()
        .skip(offset)
        .take(params.size)
        .map(MemoryDto::from)
        .collect();

    Ok(Json(paginated_memories))
}

/// Update a memory
#[utoipa::path(
    put,
    path = "/api/memories/{id}",
    tag = "memories",
    params(
        ("id" = String, Path, description = "Memory ID")
    ),
    request_body = UpdateMemoryRequest,
    responses(
        (status = 200, description = "Memory updated successfully", body = MemoryDto),
        (status = 404, description = "Memory not found"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonExtractor(request): JsonExtractor<UpdateMemoryRequest>,
) -> ServerResult<Json<MemoryDto>> {
    // Get the existing memory
    let mut memory = state
        .memory_manager
        .get_memory(&id)
        .await?
        .ok_or_else(|| not_found("Memory", &id))?;

    // Apply updates
    if let Some(content) = request.content {
        memory.content = content;
    }

    if let Some(memory_type_str) = request.memory_type {
        memory.memory_type = MemoryType::from_str(&memory_type_str);
    }

    if let Some(priority_str) = request.priority {
        memory.priority = match priority_str.as_str() {
            "low" => MemoryPriority::Low,
            "high" => MemoryPriority::High,
            "critical" => MemoryPriority::Critical,
            _ => MemoryPriority::Normal,
        };
    }

    if let Some(tags) = request.tags {
        memory.tags = tags;
    }

    if let Some(source) = request.source {
        memory.source = source;
    }

    if let Some(expires_at) = request.expires_at {
        memory.expires_at = Some(expires_at);
    }

    if let Some(properties) = request.properties {
        memory.properties = properties;
    }

    // Update the memory
    state.memory_manager.update_memory(memory.clone()).await?;

    // Broadcast WebSocket message
    let ws_message = WebSocketMessage::MemoryUpdated {
        memory_id: id.clone(),
        content: memory.content.clone(),
        metadata: memory.properties.clone(),
        importance: Some(match memory.priority {
            locai::models::MemoryPriority::Low => 0.25,
            locai::models::MemoryPriority::Normal => 0.5,
            locai::models::MemoryPriority::High => 0.75,
            locai::models::MemoryPriority::Critical => 1.0,
        }),
        node_id: None, // Will be set by live query system if enabled
    };
    state.broadcast_message(ws_message);

    let memory_dto = MemoryDto::from(memory);
    Ok(Json(memory_dto))
}

/// Delete a memory
#[utoipa::path(
    delete,
    path = "/api/memories/{id}",
    tag = "memories",
    params(
        ("id" = String, Path, description = "Memory ID")
    ),
    responses(
        (status = 204, description = "Memory deleted successfully"),
        (status = 404, description = "Memory not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<StatusCode> {
    // Check if memory exists
    let _memory = state
        .memory_manager
        .get_memory(&id)
        .await?
        .ok_or_else(|| not_found("Memory", &id))?;

    // Delete the memory
    let deleted = state.memory_manager.delete_memory(&id).await?;

    if !deleted {
        return Err(ServerError::Internal("Failed to delete memory".to_string()));
    }

    // Broadcast WebSocket message
    let ws_message = WebSocketMessage::MemoryDeleted {
        memory_id: id,
        node_id: None, // Will be set by live query system if enabled
    };
    state.broadcast_message(ws_message);

    Ok(StatusCode::NO_CONTENT)
}

/// Search memories using semantic or keyword search with optional lifecycle-aware scoring
///
/// Supports basic BM25 text search by default. When a scoring configuration is provided,
/// results are ranked using a combination of:
/// - BM25 keyword relevance
/// - Vector similarity (if ML service configured)
/// - Recency (time since last access)
/// - Access frequency (how often the memory has been accessed)  
/// - Priority level (explicit importance)
///
/// Supports temporal filtering via `created_after` and `created_before` parameters to search
/// memories within specific time ranges.
///
/// The scoring parameter accepts JSON configuration. For simple queries, use GET with JSON-encoded
/// query parameter. For complex scoring configs, consider POST endpoint (if implemented).
///
/// # Examples
///
/// Scoring:
/// ```text
/// GET /api/memories/search?q=spell&scoring={"recency_boost":2.0,"decay_function":"exponential"}
/// ```
///
/// Temporal filtering:
/// ```text
/// GET /api/memories/search?q=battle&created_after=2025-11-01T00:00:00Z&created_before=2025-11-01T23:59:59Z
/// ```
#[utoipa::path(
    get,
    path = "/api/memories/search",
    tag = "memories",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results with optional lifecycle-aware scoring", body = Vec<SearchResultDto>),
        (status = 400, description = "Bad request (invalid query or scoring configuration)"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn search_memories(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchResultDto>>, ServerError> {
    let query = params
        .q
        .ok_or_else(|| ServerError::BadRequest("Missing query parameter 'q'".to_string()))?;
    let limit = params.limit.unwrap_or(50);
    let mode = params.mode.unwrap_or(SearchMode::Text);

    // Validate search mode against available capabilities
    let locai_mode = match mode {
        SearchMode::Text => LocaiSearchMode::Text,
        SearchMode::Vector => {
            if !state.memory_manager.has_ml_service() {
                return Err(ServerError::BadRequest(
                    "Vector search requires ML service to be configured. Only 'text' search mode is available by default.".to_string()
                ));
            }
            LocaiSearchMode::Vector
        }
        SearchMode::Hybrid => {
            if !state.memory_manager.has_ml_service() {
                return Err(ServerError::BadRequest(
                    "Hybrid search requires ML service to be configured. Only 'text' search mode is available by default.".to_string()
                ));
            }
            LocaiSearchMode::Hybrid
        }
    };

    // Build filter
    let mut memory_filter = MemoryFilter::default();

    if let Some(memory_type) = params.memory_type {
        memory_filter.memory_type = Some(memory_type);
    }

    if let Some(tags_str) = params.tags {
        let tags: Vec<String> = tags_str.split(',').map(|s| s.trim().to_string()).collect();
        memory_filter.tags = Some(tags);
    }

    // Apply priority filter for search if specified
    if let Some(priority_str) = params.priority {
        let mut priority_properties = std::collections::HashMap::new();
        priority_properties.insert(
            "priority".to_string(),
            serde_json::Value::String(priority_str),
        );
        memory_filter.properties = Some(priority_properties);
    }

    // Apply temporal filters if specified
    if let Some(created_after_str) = params.created_after {
        match chrono::DateTime::parse_from_rfc3339(&created_after_str) {
            Ok(dt) => memory_filter.created_after = Some(dt.with_timezone(&chrono::Utc)),
            Err(e) => {
                return Err(ServerError::BadRequest(format!(
                    "Invalid created_after timestamp: {}. Expected ISO 8601 format like: 2025-11-01T00:00:00Z",
                    e
                )));
            }
        }
    }

    if let Some(created_before_str) = params.created_before {
        match chrono::DateTime::parse_from_rfc3339(&created_before_str) {
            Ok(dt) => memory_filter.created_before = Some(dt.with_timezone(&chrono::Utc)),
            Err(e) => {
                return Err(ServerError::BadRequest(format!(
                    "Invalid created_before timestamp: {}. Expected ISO 8601 format like: 2025-11-01T23:59:59Z",
                    e
                )));
            }
        }
    }

    let semantic_filter = SemanticSearchFilter {
        similarity_threshold: params.threshold,
        memory_filter: Some(memory_filter),
    };

    // Parse scoring configuration if provided
    let scoring_config = if let Some(scoring_json) = params.scoring {
        match serde_json::from_str::<ScoringConfigDto>(&scoring_json) {
            Ok(config) => Some(config.into()),
            Err(e) => {
                return Err(ServerError::BadRequest(format!(
                    "Invalid scoring configuration: {}. Expected JSON like: {{\"recency_boost\":2.0,\"decay_function\":\"exponential\"}}",
                    e
                )));
            }
        }
    } else {
        None
    };

    // Perform search (with or without scoring)
    let search_results = if let Some(scoring) = scoring_config {
        state
            .memory_manager
            .search_with_scoring(&query, Some(limit), scoring)
            .await?
    } else {
        state
            .memory_manager
            .search(&query, Some(limit), Some(semantic_filter), locai_mode)
            .await?
    };

    // Convert to DTOs
    let result_dtos: Vec<SearchResultDto> = search_results
        .into_iter()
        .map(SearchResultDto::from)
        .collect();

    Ok(Json(result_dtos))
}

/// Create a relationship between memories
#[utoipa::path(
    post,
    path = "/api/memories/{id}/relationships",
    tag = "memories",
    params(
        ("id" = String, Path, description = "Source memory ID")
    ),
    request_body = CreateMemoryRelationshipRequest,
    responses(
        (status = 201, description = "Relationship created successfully", body = RelationshipDto),
        (status = 404, description = "Source or target memory not found"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_memory_relationship(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonExtractor(request): JsonExtractor<CreateMemoryRelationshipRequest>,
) -> ServerResult<(StatusCode, Json<RelationshipDto>)> {
    use chrono::Utc;
    use locai::storage::models::Relationship;
    use uuid::Uuid;

    // Use id as source_id for clarity in the logic
    let source_id = id;

    // Validate that source memory exists
    let _source_memory = state
        .memory_manager
        .get_memory(&source_id)
        .await?
        .ok_or_else(|| not_found("Memory", &source_id))?;

    // Validate that target exists (can be memory OR entity)
    let target_is_memory = state
        .memory_manager
        .get_memory(&request.target_id)
        .await?
        .is_some();
    
    let target_is_entity = if !target_is_memory {
        state
            .memory_manager
            .get_entity(&request.target_id)
            .await?
            .is_some()
    } else {
        false
    };

    if !target_is_memory && !target_is_entity {
        return Err(not_found("Memory or Entity", &request.target_id));
    }

    // Create the relationship
    let now = Utc::now();
    let relationship = Relationship {
        id: Uuid::new_v4().to_string(),
        source_id: source_id.clone(),
        target_id: request.target_id.clone(),
        relationship_type: request.relationship_type.clone(),
        properties: request.properties,
        created_at: now,
        updated_at: now,
    };

    // Store the relationship
    let created_relationship = state
        .memory_manager
        .create_relationship_entity(relationship)
        .await?;

    // Broadcast WebSocket message
    let ws_message = WebSocketMessage::RelationshipCreated {
        relationship_id: created_relationship.id.clone(),
        source_id: created_relationship.source_id.clone(),
        target_id: created_relationship.target_id.clone(),
        relationship_type: created_relationship.relationship_type.clone(),
        properties: serde_json::to_value(&created_relationship.properties).unwrap_or_default(),
        node_id: None,
    };
    state.broadcast_message(ws_message);

    let relationship_dto = RelationshipDto::from(created_relationship);
    Ok((StatusCode::CREATED, Json(relationship_dto)))
}

/// Get relationships for a memory
#[utoipa::path(
    get,
    path = "/api/memories/{id}/relationships",
    tag = "memories",
    params(
        ("id" = String, Path, description = "Memory ID"),
        ("relationship_type" = Option<String>, Query, description = "Filter by relationship type"),
        ("direction" = Option<String>, Query, description = "Direction: outgoing, incoming, or both"),
    ),
    responses(
        (status = 200, description = "List of relationships", body = Vec<RelationshipDto>),
        (status = 404, description = "Memory not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_memory_relationships(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<GetMemoryRelationshipsParams>,
) -> ServerResult<Json<Vec<RelationshipDto>>> {
    use locai::storage::filters::RelationshipFilter;

    // Validate that the memory exists
    let _memory = state
        .memory_manager
        .get_memory(&id)
        .await?
        .ok_or_else(|| not_found("Memory", &id))?;

    // Build filter based on direction
    let direction = params.direction.as_str();
    let mut all_relationships = Vec::new();

    // Get outgoing relationships (where this memory is the source)
    if direction == "outgoing" || direction == "both" {
        let filter = RelationshipFilter {
            source_id: Some(id.clone()),
            relationship_type: params.relationship_type.clone(),
            ..Default::default()
        };

        let outgoing = state
            .memory_manager
            .list_relationships(Some(filter), Some(100), None)
            .await?;
        all_relationships.extend(outgoing);
    }

    // Get incoming relationships (where this memory is the target)
    if direction == "incoming" || direction == "both" {
        let filter = RelationshipFilter {
            target_id: Some(id.clone()),
            relationship_type: params.relationship_type,
            ..Default::default()
        };

        let incoming = state
            .memory_manager
            .list_relationships(Some(filter), Some(100), None)
            .await?;
        all_relationships.extend(incoming);
    }

    // Convert to DTOs
    let relationship_dtos: Vec<RelationshipDto> = all_relationships
        .into_iter()
        .map(RelationshipDto::from)
        .collect();

    Ok(Json(relationship_dtos))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchParams {
    /// Search query
    pub q: Option<String>,

    /// Maximum number of results
    pub limit: Option<usize>,

    /// Search mode
    pub mode: Option<SearchMode>,

    /// Similarity threshold for semantic search
    pub threshold: Option<f32>,

    /// Filter by memory type. For custom memory types, include the "custom:" prefix.
    /// 
    /// Examples:
    /// - "custom:dialogue"
    /// - "custom:quest"
    /// - "custom:observation"
    /// 
    /// Built-in types (if any) do not require a prefix.
    #[param(example = "custom:dialogue")]
    pub memory_type: Option<String>,

    /// Filter by tags (comma-separated)
    pub tags: Option<String>,

    /// Filter by priority (capitalized values: "Low", "Normal", "High", "Critical")
    #[param(example = "Normal")]
    pub priority: Option<String>,
    
    /// Optional JSON-encoded scoring configuration for enhanced search
    ///
    /// When provided, enables lifecycle-aware scoring that combines BM25, vector similarity,
    /// recency, access frequency, and priority. If omitted, uses basic BM25 scoring.
    ///
    /// Example: `{"recency_boost":2.0,"access_boost":1.5,"decay_function":"exponential"}`
    ///
    /// For complex scoring configs, consider using POST /api/memories/search with JSON body instead.
    #[param(example = r#"{"recency_boost":2.0,"decay_function":"exponential"}"#)]
    pub scoring: Option<String>,

    /// Filter by creation date - only memories created after this time (ISO 8601 format)
    ///
    /// Example: `2025-11-01T00:00:00Z`
    #[param(example = "2025-11-01T00:00:00Z")]
    pub created_after: Option<String>,

    /// Filter by creation date - only memories created before this time (ISO 8601 format)
    ///
    /// Example: `2025-11-01T23:59:59Z`
    #[param(example = "2025-11-01T23:59:59Z")]
    pub created_before: Option<String>,
}
