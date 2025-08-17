//! Relationship management API endpoints

use std::sync::Arc;

use axum::{
    Json as JsonExtractor,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use locai::storage::{filters::RelationshipFilter, models::Relationship};

use crate::{
    api::dto::{CreateRelationshipRequest, EntityDto, RelationshipDto},
    error::{ServerError, ServerResult, not_found},
    state::AppState,
    websocket::WebSocketMessage,
};

/// Query parameters for listing relationships
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListRelationshipsParams {
    /// Page number (0-based)
    #[serde(default)]
    pub page: usize,

    /// Number of items per page
    #[serde(default = "default_page_size")]
    pub size: usize,

    /// Filter by relationship type
    pub relationship_type: Option<String>,

    /// Filter by source entity ID
    pub source_id: Option<String>,

    /// Filter by target entity ID
    pub target_id: Option<String>,
}

fn default_page_size() -> usize {
    20
}

/// List relationships
#[utoipa::path(
    get,
    path = "/api/relationships",
    tag = "relationships",
    params(ListRelationshipsParams),
    responses(
        (status = 200, description = "List of relationships", body = Vec<RelationshipDto>),
    )
)]
pub async fn list_relationships(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListRelationshipsParams>,
) -> ServerResult<Json<Vec<RelationshipDto>>> {
    let memory_manager = &state.memory_manager;

    // Calculate offset from page and size
    let offset = params.page * params.size;
    let limit = params.size;

    // Build filter
    let mut filter = RelationshipFilter::default();
    if let Some(relationship_type) = params.relationship_type {
        filter.relationship_type = Some(relationship_type);
    }
    if let Some(source_id) = params.source_id {
        filter.source_id = Some(source_id);
    }
    if let Some(target_id) = params.target_id {
        filter.target_id = Some(target_id);
    }

    let relationships = memory_manager
        .list_relationships(Some(filter), Some(limit), Some(offset))
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to list relationships: {}", e)))?;

    let relationship_dtos: Vec<RelationshipDto> = relationships
        .into_iter()
        .map(RelationshipDto::from)
        .collect();

    Ok(Json(relationship_dtos))
}

/// Get a specific relationship
#[utoipa::path(
    get,
    path = "/api/relationships/{id}",
    tag = "relationships",
    params(
        ("id" = String, Path, description = "Relationship ID")
    ),
    responses(
        (status = 200, description = "Relationship details", body = RelationshipDto),
        (status = 404, description = "Relationship not found"),
    )
)]
pub async fn get_relationship(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<Json<RelationshipDto>> {
    let memory_manager = &state.memory_manager;

    let relationship = memory_manager
        .get_relationship(&id)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to get relationship: {}", e)))?
        .ok_or_else(|| not_found("Relationship", &id))?;

    Ok(Json(RelationshipDto::from(relationship)))
}

/// Create a new relationship
#[utoipa::path(
    post,
    path = "/api/relationships",
    tag = "relationships",
    request_body = CreateRelationshipRequest,
    responses(
        (status = 201, description = "Relationship created successfully", body = RelationshipDto),
        (status = 400, description = "Invalid request"),
    )
)]
pub async fn create_relationship(
    State(state): State<Arc<AppState>>,
    JsonExtractor(request): JsonExtractor<CreateRelationshipRequest>,
) -> ServerResult<(StatusCode, Json<RelationshipDto>)> {
    let memory_manager = &state.memory_manager;

    // Validate that source and target entities exist
    let source_exists = memory_manager
        .get_entity(&request.source_id)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to check source entity: {}", e)))?
        .is_some();

    if !source_exists {
        return Err(ServerError::BadRequest(format!(
            "Source entity '{}' not found",
            request.source_id
        )));
    }

    let target_exists = memory_manager
        .get_entity(&request.target_id)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to check target entity: {}", e)))?
        .is_some();

    if !target_exists {
        return Err(ServerError::BadRequest(format!(
            "Target entity '{}' not found",
            request.target_id
        )));
    }

    // Create the relationship object
    let relationship = Relationship {
        id: Uuid::new_v4().to_string(),
        source_id: request.source_id,
        target_id: request.target_id,
        relationship_type: request.relationship_type,
        properties: request.properties,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_relationship = memory_manager
        .create_relationship_entity(relationship)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to create relationship: {}", e)))?;

    let relationship_dto = RelationshipDto::from(created_relationship.clone());

    // Send WebSocket notification
    let message = WebSocketMessage::RelationshipCreated {
        relationship_id: created_relationship.id.clone(),
        source_id: created_relationship.source_id.clone(),
        target_id: created_relationship.target_id.clone(),
        relationship_type: created_relationship.relationship_type.clone(),
        properties: serde_json::to_value(&created_relationship.properties).unwrap_or_default(),
        node_id: None, // Will be set by live query system if enabled
    };
    state.broadcast_message(message);

    Ok((StatusCode::CREATED, Json(relationship_dto)))
}

/// Update a relationship
#[utoipa::path(
    put,
    path = "/api/relationships/{id}",
    tag = "relationships",
    params(
        ("id" = String, Path, description = "Relationship ID")
    ),
    request_body = UpdateRelationshipRequest,
    responses(
        (status = 200, description = "Relationship updated successfully", body = RelationshipDto),
        (status = 404, description = "Relationship not found"),
    )
)]
pub async fn update_relationship(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonExtractor(request): JsonExtractor<UpdateRelationshipRequest>,
) -> ServerResult<Json<RelationshipDto>> {
    let memory_manager = &state.memory_manager;

    // Check if relationship exists and get it
    let mut existing = memory_manager
        .get_relationship(&id)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to get relationship: {}", e)))?
        .ok_or_else(|| not_found("Relationship", &id))?;

    // Update fields if provided
    if let Some(relationship_type) = request.relationship_type {
        existing.relationship_type = relationship_type;
    }

    if let Some(properties) = request.properties {
        existing.properties = properties;
    }

    existing.updated_at = Utc::now();

    let updated_relationship = memory_manager
        .update_relationship(existing)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to update relationship: {}", e)))?;

    let relationship_dto = RelationshipDto::from(updated_relationship.clone());

    // Send WebSocket notification
    let message = WebSocketMessage::RelationshipCreated {
        relationship_id: updated_relationship.id.clone(),
        source_id: updated_relationship.source_id.clone(),
        target_id: updated_relationship.target_id.clone(),
        relationship_type: updated_relationship.relationship_type.clone(),
        properties: serde_json::to_value(&updated_relationship.properties).unwrap_or_default(),
        node_id: None, // Will be set by live query system if enabled
    };
    state.broadcast_message(message);

    Ok(Json(relationship_dto))
}

/// Delete a relationship
#[utoipa::path(
    delete,
    path = "/api/relationships/{id}",
    tag = "relationships",
    params(
        ("id" = String, Path, description = "Relationship ID")
    ),
    responses(
        (status = 204, description = "Relationship deleted successfully"),
        (status = 404, description = "Relationship not found"),
    )
)]
pub async fn delete_relationship(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ServerResult<StatusCode> {
    let memory_manager = &state.memory_manager;

    // Check if relationship exists
    let _existing = memory_manager
        .get_relationship(&id)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to get relationship: {}", e)))?
        .ok_or_else(|| not_found("Relationship", &id))?;

    let deleted = memory_manager
        .delete_relationship(&id)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to delete relationship: {}", e)))?;

    if !deleted {
        return Err(not_found("Relationship", &id));
    }

    // Send WebSocket notification
    let message = WebSocketMessage::RelationshipDeleted {
        relationship_id: id.clone(),
        node_id: None, // Will be set by live query system if enabled
    };
    state.broadcast_message(message);

    Ok(StatusCode::NO_CONTENT)
}

/// Query parameters for finding related entities
#[derive(Debug, Deserialize, IntoParams)]
pub struct FindRelatedQuery {
    /// Page number (0-based)
    #[serde(default)]
    pub page: usize,

    /// Number of items per page
    #[serde(default = "default_page_size")]
    pub size: usize,

    /// Filter by relationship type
    pub relationship_type: Option<String>,

    /// Relationship direction (incoming/outgoing/both)
    pub direction: Option<String>,
}

/// Find related entities through relationships
#[utoipa::path(
    get,
    path = "/api/relationships/{id}/related",
    tag = "relationships",
    params(
        ("id" = String, Path, description = "Entity ID to find relationships for"),
        FindRelatedQuery,
    ),
    responses(
        (status = 200, description = "Related entities", body = Vec<EntityDto>),
        (status = 404, description = "Entity not found"),
    )
)]
pub async fn find_related_entities(
    State(state): State<Arc<AppState>>,
    Path(entity_id): Path<String>,
    Query(query): Query<FindRelatedQuery>,
) -> ServerResult<Json<Vec<EntityDto>>> {
    let memory_manager = &state.memory_manager;

    // Check if entity exists
    let entity_exists = memory_manager
        .get_entity(&entity_id)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to check entity: {}", e)))?
        .is_some();

    if !entity_exists {
        return Err(not_found("Entity", &entity_id));
    }

    let mut related_entities = memory_manager
        .find_related_entities(&entity_id, query.relationship_type, query.direction)
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to find related entities: {}", e)))?;

    // Apply pagination
    let offset = query.page * query.size;
    let limit = query.size;

    // Skip to the offset and take only the page size
    related_entities = related_entities
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    let entity_dtos: Vec<EntityDto> = related_entities.into_iter().map(EntityDto::from).collect();

    Ok(Json(entity_dtos))
}

/// Request to update a relationship
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateRelationshipRequest {
    /// Updated relationship type (optional)
    pub relationship_type: Option<String>,

    /// Updated properties (optional)
    pub properties: Option<serde_json::Value>,
}
