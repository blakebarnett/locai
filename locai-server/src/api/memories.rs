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
    api::dto::{CreateMemoryRequest, MemoryDto, SearchMode, SearchResultDto, UpdateMemoryRequest},
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

    /// Filter by memory type
    pub memory_type: Option<String>,

    /// Filter by priority
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

/// Search memories using semantic or keyword search
#[utoipa::path(
    get,
    path = "/api/memories/search",
    tag = "memories",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results", body = Vec<SearchResultDto>),
        (status = 400, description = "Bad request"),
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

    let semantic_filter = SemanticSearchFilter {
        similarity_threshold: params.threshold,
        memory_filter: Some(memory_filter),
    };

    // Perform search
    let search_results = state
        .memory_manager
        .search(&query, Some(limit), Some(semantic_filter), locai_mode)
        .await?;

    // Convert to DTOs
    let result_dtos: Vec<SearchResultDto> = search_results
        .into_iter()
        .map(SearchResultDto::from)
        .collect();

    Ok(Json(result_dtos))
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

    /// Filter by memory type
    pub memory_type: Option<String>,

    /// Filter by tags (comma-separated)
    pub tags: Option<String>,

    /// Filter by priority
    pub priority: Option<String>,
}
