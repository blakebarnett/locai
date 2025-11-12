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
    api::dto::{
        CreateEntityRequest, EntityDto, MemoryDto, RelationshipDto, UpdateEntityRequest,
    },
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

/// Create a relationship between entities
#[utoipa::path(
    post,
    path = "/api/entities/{id}/relationships",
    tag = "entities",
    params(
        ("id" = String, Path, description = "Source entity ID")
    ),
    request_body = CreateEntityRelationshipRequest,
    responses(
        (status = 201, description = "Relationship created successfully", body = RelationshipDto),
        (status = 404, description = "Source or target entity not found"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_entity_relationship(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonExtractor(request): JsonExtractor<CreateEntityRelationshipRequest>,
) -> ServerResult<(StatusCode, Json<RelationshipDto>)> {
    use locai::storage::models::Relationship;

    // Use id as source_id for clarity in the logic
    let source_id = id;

    // Validate that source entity exists
    let _source_entity = state
        .memory_manager
        .get_entity(&source_id)
        .await?
        .ok_or_else(|| not_found("Entity", &source_id))?;

    // Validate that target exists (can be entity OR memory)
    let target_is_entity = state
        .memory_manager
        .get_entity(&request.target_id)
        .await?
        .is_some();
    
    let target_is_memory = if !target_is_entity {
        state
            .memory_manager
            .get_memory(&request.target_id)
            .await?
            .is_some()
    } else {
        false
    };

    if !target_is_entity && !target_is_memory {
        return Err(not_found("Entity or Memory", &request.target_id));
    }

    // Create the relationship
    let now = Utc::now();
    let relationship = Relationship {
        id: uuid::Uuid::new_v4().to_string(),
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

/// Get relationships for an entity
#[utoipa::path(
    get,
    path = "/api/entities/{id}/relationships",
    tag = "entities",
    params(
        ("id" = String, Path, description = "Entity ID"),
        ("relationship_type" = Option<String>, Query, description = "Filter by relationship type"),
        ("direction" = Option<String>, Query, description = "Direction: outgoing, incoming, or both"),
    ),
    responses(
        (status = 200, description = "List of relationships", body = Vec<RelationshipDto>),
        (status = 404, description = "Entity not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_entity_relationships(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<GetEntityRelationshipsParams>,
) -> ServerResult<Json<Vec<RelationshipDto>>> {
    // Validate that the entity exists
    let _entity = state
        .memory_manager
        .get_entity(&id)
        .await?
        .ok_or_else(|| not_found("Entity", &id))?;

    // Build filter based on direction
    let direction = params.direction.as_str();
    let mut all_relationships = Vec::new();

    // Get outgoing relationships (where this entity is the source)
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

    // Get incoming relationships (where this entity is the target)
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

/// Query parameters for listing entity relationships
#[derive(Debug, Deserialize)]
pub struct GetEntityRelationshipsParams {
    /// Filter by relationship type
    pub relationship_type: Option<String>,

    /// Relationship direction: "outgoing", "incoming", or "both" (default)
    #[serde(default = "default_direction")]
    pub direction: String,
}

fn default_direction() -> String {
    "both".to_string()
}

/// Request to create a new relationship between entities (or entity→memory)
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateEntityRelationshipRequest {
    /// Type of relationship (e.g., "depends_on", "relates_to", "part_of", "contains")
    #[schema(example = "depends_on")]
    pub relationship_type: String,

    /// Target ID - can be another entity ID or a memory ID
    /// The system will automatically detect which type it is
    pub target_id: String,

    /// Properties associated with the relationship
    #[serde(default)]
    pub properties: serde_json::Value,
}
