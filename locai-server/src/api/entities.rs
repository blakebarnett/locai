//! Entity management API endpoints

use std::sync::Arc;

use axum::{
    Json as JsonExtractor,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use utoipa::IntoParams;

use chrono::Utc;
use locai::storage::{
    filters::{EntityFilter, RelationshipFilter},
    models::Entity,
};

use crate::{
    api::dto::{CreateEntityRequest, EntityDto, MemoryDto, UpdateEntityRequest},
    error::{ServerResult, not_found},
    state::AppState,
    websocket::WebSocketMessage,
};

/// List entities with filtering and pagination
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListEntitiesParams {
    /// Page number (0-based)
    #[serde(default)]
    pub page: usize,

    /// Number of items per page
    #[serde(default = "default_page_size")]
    pub size: usize,

    /// Filter by entity type
    pub entity_type: Option<String>,

    /// Filter by related entity ID
    pub related_to: Option<String>,

    /// Filter by relationship type when using related_to
    pub related_by: Option<String>,
}

fn default_page_size() -> usize {
    20
}

/// List entities
#[utoipa::path(
    get,
    path = "/api/entities",
    tag = "entities",
    params(ListEntitiesParams),
    responses(
        (status = 200, description = "List of entities", body = Vec<EntityDto>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_entities(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListEntitiesParams>,
) -> ServerResult<Json<Vec<EntityDto>>> {
    let mut filter = EntityFilter::default();

    // Apply filters
    if let Some(entity_type) = params.entity_type {
        filter.entity_type = Some(entity_type);
    }

    if let Some(related_to) = params.related_to {
        filter.related_to = Some(related_to);
    }

    if let Some(related_by) = params.related_by {
        filter.related_by = Some(related_by);
    }

    // Calculate offset for pagination
    let offset = params.page * params.size;

    // Get entities
    let entities = state
        .memory_manager
        .list_entities(Some(filter), Some(params.size), Some(offset))
        .await?;

    let entity_dtos: Vec<EntityDto> = entities.into_iter().map(EntityDto::from).collect();
    Ok(Json(entity_dtos))
}

/// Get an entity by ID
#[utoipa::path(
    get,
    path = "/api/entities/{id}",
    tag = "entities",
    params(
        ("id" = String, Path, description = "Entity ID")
    ),
    responses(
        (status = 200, description = "Entity found", body = EntityDto),
        (status = 404, description = "Entity not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_entity(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<EntityDto>> {
    let entity = state
        .memory_manager
        .get_entity(&id)
        .await?
        .ok_or_else(|| not_found("Entity", &id))?;

    let entity_dto = EntityDto::from(entity);
    Ok(Json(entity_dto))
}

/// Create a new entity
#[utoipa::path(
    post,
    path = "/api/entities",
    tag = "entities",
    request_body = CreateEntityRequest,
    responses(
        (status = 201, description = "Entity created successfully", body = EntityDto),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_entity(
    State(state): State<Arc<AppState>>,
    JsonExtractor(request): JsonExtractor<CreateEntityRequest>,
) -> ServerResult<(StatusCode, Json<EntityDto>)> {
    let now = Utc::now();

    // Create the entity
    let entity = Entity {
        id: uuid::Uuid::new_v4().to_string(),
        entity_type: request.entity_type,
        properties: request.properties,
        created_at: now,
        updated_at: now,
    };

    // Store the entity
    let created_entity = state.memory_manager.create_entity(entity).await?;

    // Broadcast WebSocket message
    let ws_message = WebSocketMessage::EntityCreated {
        entity_id: created_entity.id.clone(),
        entity_type: created_entity.entity_type.clone(),
        properties: serde_json::to_value(&created_entity.properties).unwrap_or_default(),
        node_id: None, // Will be set by live query system if enabled
    };
    state.broadcast_message(ws_message);

    let entity_dto = EntityDto::from(created_entity);
    Ok((StatusCode::CREATED, Json(entity_dto)))
}

/// Update an entity
#[utoipa::path(
    put,
    path = "/api/entities/{id}",
    tag = "entities",
    params(
        ("id" = String, Path, description = "Entity ID")
    ),
    request_body = UpdateEntityRequest,
    responses(
        (status = 200, description = "Entity updated successfully", body = EntityDto),
        (status = 404, description = "Entity not found"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_entity(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonExtractor(request): JsonExtractor<UpdateEntityRequest>,
) -> ServerResult<Json<EntityDto>> {
    // Get the existing entity
    let mut entity = state
        .memory_manager
        .get_entity(&id)
        .await?
        .ok_or_else(|| not_found("Entity", &id))?;

    // Update fields if provided
    if let Some(entity_type) = request.entity_type {
        entity.entity_type = entity_type;
    }

    if let Some(properties) = request.properties {
        entity.properties = properties;
    }

    entity.updated_at = Utc::now();

    // Update the entity
    let updated_entity = state.memory_manager.update_entity(entity).await?;

    // Broadcast WebSocket message
    let ws_message = WebSocketMessage::EntityUpdated {
        entity_id: updated_entity.id.clone(),
        entity_type: updated_entity.entity_type.clone(),
        properties: serde_json::to_value(&updated_entity.properties).unwrap_or_default(),
        node_id: None, // Will be set by live query system if enabled
    };
    state.broadcast_message(ws_message);

    let entity_dto = EntityDto::from(updated_entity);
    Ok(Json(entity_dto))
}

/// Delete an entity
#[utoipa::path(
    delete,
    path = "/api/entities/{id}",
    tag = "entities",
    params(
        ("id" = String, Path, description = "Entity ID")
    ),
    responses(
        (status = 204, description = "Entity deleted successfully"),
        (status = 404, description = "Entity not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_entity(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<StatusCode> {
    // Check if entity exists first
    let _entity = state
        .memory_manager
        .get_entity(&id)
        .await?
        .ok_or_else(|| not_found("Entity", &id))?;

    // Delete the entity
    let deleted = state.memory_manager.delete_entity(&id).await?;

    if deleted {
        // Broadcast WebSocket message
        let ws_message = WebSocketMessage::EntityDeleted {
            entity_id: id,
            node_id: None, // Will be set by live query system if enabled
        };
        state.broadcast_message(ws_message);

        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Entity", &id))
    }
}

/// Get memories related to an entity
#[utoipa::path(
    get,
    path = "/api/entities/{id}/memories",
    tag = "entities",
    params(
        ("id" = String, Path, description = "Entity ID")
    ),
    responses(
        (status = 200, description = "List of related memories", body = Vec<MemoryDto>),
        (status = 404, description = "Entity not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_entity_memories(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<Vec<MemoryDto>>> {
    // Check if entity exists
    let _entity = state
        .memory_manager
        .get_entity(&id)
        .await?
        .ok_or_else(|| not_found("Entity", &id))?;

    // Find relationships where this entity is the target (memory contains entity)
    let filter = RelationshipFilter {
        target_id: Some(id.clone()),
        relationship_type: Some("contains".to_string()),
        ..Default::default()
    };

    let relationships = state
        .memory_manager
        .list_relationships(Some(filter), None, None)
        .await?;

    // Get memories for each source (the memory that contains this entity)
    let mut memories = Vec::new();
    for relationship in relationships {
        if let Ok(Some(memory)) = state
            .memory_manager
            .get_memory(&relationship.source_id)
            .await
        {
            memories.push(MemoryDto::from(memory));
        }
    }

    Ok(Json(memories))
}
