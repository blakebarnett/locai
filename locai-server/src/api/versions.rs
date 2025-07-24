//! Version management API endpoints

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};

use crate::{
    api::dto::{CheckoutVersionRequest, CreateVersionRequest, VersionDto},
    error::{not_found, ServerError, ServerResult},
    state::AppState,
};

/// List versions
#[utoipa::path(
    get,
    path = "/api/versions",
    tag = "versions",
    responses(
        (status = 200, description = "List of versions", body = Vec<VersionDto>),
    )
)]
pub async fn list_versions(
    State(_state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<VersionDto>>> {
    Ok(Json(vec![]))
}

/// Create a new version
#[utoipa::path(
    post,
    path = "/api/versions",
    tag = "versions",
    request_body = CreateVersionRequest,
    responses(
        (status = 201, description = "Version created successfully", body = VersionDto),
    )
)]
pub async fn create_version(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<CreateVersionRequest>,
) -> ServerResult<(StatusCode, Json<VersionDto>)> {
    Err(ServerError::Internal("Not implemented".to_string()))
}

/// Checkout a version
#[utoipa::path(
    put,
    path = "/api/versions/{id}/checkout",
    tag = "versions",
    params(
        ("id" = String, Path, description = "Version ID")
    ),
    request_body = CheckoutVersionRequest,
    responses(
        (status = 200, description = "Version checked out successfully"),
        (status = 404, description = "Version not found"),
    )
)]
pub async fn checkout_version(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(_request): Json<CheckoutVersionRequest>,
) -> ServerResult<StatusCode> {
    Err(not_found("Version", &id))
} 