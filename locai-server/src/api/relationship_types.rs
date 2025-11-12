//! Relationship Type Management API endpoints
//!
//! Provides REST API endpoints for managing dynamic relationship types,
//! including registration, validation, and metrics tracking.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use locai::relationships::{MetricsSnapshot, RelationshipTypeDef};

use crate::{
    error::{ServerError, ServerResult},
    state::AppState,
};

/// Request to register a new relationship type
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RegisterTypeRequest {
    /// Name of the relationship type (must be alphanumeric + underscore/dash)
    pub name: String,

    /// Optional inverse type name
    #[serde(default)]
    pub inverse: Option<String>,

    /// Whether this relationship is symmetric (bidirectional)
    #[serde(default)]
    pub symmetric: bool,

    /// Whether this relationship is transitive
    #[serde(default)]
    pub transitive: bool,

    /// JSON Schema for validating metadata on this relationship type
    #[serde(default)]
    pub metadata_schema: Option<serde_json::Value>,
}

/// Response containing relationship type definition
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RelationshipTypeResponse {
    pub name: String,
    pub inverse: Option<String>,
    pub symmetric: bool,
    pub transitive: bool,
    pub metadata_schema: Option<serde_json::Value>,
    pub version: u32,
    pub created_at: String,
}

impl From<RelationshipTypeDef> for RelationshipTypeResponse {
    fn from(def: RelationshipTypeDef) -> Self {
        Self {
            name: def.name,
            inverse: def.inverse,
            symmetric: def.symmetric,
            transitive: def.transitive,
            metadata_schema: def.metadata_schema,
            version: def.version,
            created_at: def.created_at.to_rfc3339(),
        }
    }
}

/// List all registered relationship types
#[utoipa::path(
    get,
    path = "/api/v1/relationship-types",
    tag = "relationship-types",
    responses(
        (status = 200, description = "List of all relationship types", body = Vec<RelationshipTypeResponse>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_relationship_types(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<RelationshipTypeResponse>>> {
    let registry = &state.relationship_type_registry;
    let types = registry.list().await;

    let responses: Vec<RelationshipTypeResponse> =
        types.into_iter().map(|def| def.into()).collect();

    Ok(Json(responses))
}

/// Get a specific relationship type by name
#[utoipa::path(
    get,
    path = "/api/v1/relationship-types/{name}",
    tag = "relationship-types",
    params(
        ("name" = String, Path, description = "Name of the relationship type")
    ),
    responses(
        (status = 200, description = "Relationship type found", body = RelationshipTypeResponse),
        (status = 404, description = "Relationship type not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_relationship_type(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> ServerResult<Json<RelationshipTypeResponse>> {
    let registry = &state.relationship_type_registry;

    match registry.get(&name).await {
        Some(type_def) => Ok(Json(type_def.into())),
        None => Err(ServerError::NotFound(format!(
            "Relationship type '{}' not found",
            name
        ))),
    }
}

/// Register a new relationship type
#[utoipa::path(
    post,
    path = "/api/v1/relationship-types",
    tag = "relationship-types",
    request_body = RegisterTypeRequest,
    responses(
        (status = 201, description = "Relationship type created", body = RelationshipTypeResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Type already exists"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn register_relationship_type(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterTypeRequest>,
) -> ServerResult<(StatusCode, Json<RelationshipTypeResponse>)> {
    let registry = &state.relationship_type_registry;

    // Create the type definition
    let mut type_def = RelationshipTypeDef::new(request.name.clone())
        .map_err(|e| ServerError::BadRequest(e.to_string()))?;

    if let Some(inverse) = request.inverse {
        type_def = type_def.with_inverse(inverse);
    }

    if request.symmetric {
        type_def = type_def.symmetric();
    }

    if request.transitive {
        type_def = type_def.transitive();
    }

    if let Some(schema) = request.metadata_schema {
        type_def = type_def.with_metadata_schema(schema);
    }

    // Try to register
    registry
        .register(type_def.clone())
        .await
        .map_err(|e| match e {
            locai::relationships::RegistryError::TypeAlreadyExists(name) => {
                ServerError::BadRequest(format!("Relationship type '{}' already exists", name))
            }
            locai::relationships::RegistryError::InvalidTypeName(msg) => {
                ServerError::BadRequest(msg)
            }
            _ => ServerError::Internal(e.to_string()),
        })?;

    Ok((StatusCode::CREATED, Json(type_def.into())))
}

/// Update an existing relationship type
#[utoipa::path(
    put,
    path = "/api/v1/relationship-types/{name}",
    tag = "relationship-types",
    params(
        ("name" = String, Path, description = "Name of the relationship type")
    ),
    request_body = RegisterTypeRequest,
    responses(
        (status = 200, description = "Relationship type updated", body = RelationshipTypeResponse),
        (status = 404, description = "Relationship type not found"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_relationship_type(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<RegisterTypeRequest>,
) -> ServerResult<Json<RelationshipTypeResponse>> {
    let registry = &state.relationship_type_registry;

    // Verify type exists
    registry
        .get(&name)
        .await
        .ok_or_else(|| ServerError::NotFound(format!("Relationship type '{}' not found", name)))?;

    // Create updated definition (keeping the same name)
    let mut type_def = RelationshipTypeDef::new(name.clone())
        .map_err(|e| ServerError::BadRequest(e.to_string()))?;

    if let Some(inverse) = request.inverse {
        type_def = type_def.with_inverse(inverse);
    }

    if request.symmetric {
        type_def = type_def.symmetric();
    }

    if request.transitive {
        type_def = type_def.transitive();
    }

    if let Some(schema) = request.metadata_schema {
        type_def = type_def.with_metadata_schema(schema);
    }

    registry
        .update(type_def.clone())
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(type_def.into()))
}

/// Delete a relationship type
#[utoipa::path(
    delete,
    path = "/api/v1/relationship-types/{name}",
    tag = "relationship-types",
    params(
        ("name" = String, Path, description = "Name of the relationship type")
    ),
    responses(
        (status = 204, description = "Relationship type deleted"),
        (status = 404, description = "Relationship type not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_relationship_type(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> ServerResult<StatusCode> {
    let registry = &state.relationship_type_registry;

    registry.delete(&name).await.map_err(|e| match e {
        locai::relationships::RegistryError::TypeNotFound(n) => {
            ServerError::NotFound(format!("Relationship type '{}' not found", n))
        }
        _ => ServerError::Internal(e.to_string()),
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Response for metrics endpoint
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetricsResponse {
    pub symmetric_relationships_created: u64,
    pub transitive_relationships_created: u64,
    pub manual_inverse_creates: u64,
    pub enforcement_requests_enabled: u64,
    pub enforcement_requests_disabled: u64,
    pub total_relationships_created: u64,
    pub enforcement_enabled_percentage: f64,
    pub symmetric_percentage: f64,
    pub transitive_percentage: f64,
    pub manual_inverse_ratio: f64,
    pub timestamp: String,
}

impl From<(MetricsSnapshot, locai::relationships::RelationshipMetrics)> for MetricsResponse {
    fn from(
        (snapshot, metrics): (MetricsSnapshot, locai::relationships::RelationshipMetrics),
    ) -> Self {
        Self {
            symmetric_relationships_created: snapshot.symmetric_relationships_created,
            transitive_relationships_created: snapshot.transitive_relationships_created,
            manual_inverse_creates: snapshot.manual_inverse_creates,
            enforcement_requests_enabled: snapshot.enforcement_requests_enabled,
            enforcement_requests_disabled: snapshot.enforcement_requests_disabled,
            total_relationships_created: snapshot.total_relationships_created,
            enforcement_enabled_percentage: metrics.enforcement_enabled_percentage(),
            symmetric_percentage: metrics.symmetric_percentage(),
            transitive_percentage: metrics.transitive_percentage(),
            manual_inverse_ratio: metrics.manual_inverse_ratio(),
            timestamp: snapshot.timestamp.to_rfc3339(),
        }
    }
}

/// Get relationship type metrics
#[utoipa::path(
    get,
    path = "/api/v1/relationship-types/metrics",
    tag = "relationship-types",
    responses(
        (status = 200, description = "Metrics snapshot", body = MetricsResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_relationship_metrics(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<MetricsResponse>> {
    let metrics = &state.relationship_metrics;
    let snapshot = metrics.export_metrics();

    let response = MetricsResponse::from((snapshot, metrics.clone()));
    Ok(Json(response))
}

/// Response indicating seeding was performed
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SeedResponse {
    pub message: String,
    pub types_seeded: usize,
}

/// Seed the registry with common relationship types
#[utoipa::path(
    post,
    path = "/api/v1/relationship-types/seed",
    tag = "relationship-types",
    responses(
        (status = 200, description = "Common types seeded", body = SeedResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn seed_common_types(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<SeedResponse>> {
    let registry = &state.relationship_type_registry;

    registry
        .seed_common_types()
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    let count = registry.count().await;

    Ok(Json(SeedResponse {
        message: "Common relationship types seeded successfully".to_string(),
        types_seeded: count,
    }))
}
