//! Batch operations API endpoints
//!
//! This module provides REST endpoints for executing batch operations
//! on memories and relationships in bulk.

use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;
use utoipa::ToSchema;

use locai::batch::{BatchExecutor, BatchExecutorConfig, BatchOperation, BatchResponse};

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// Request to execute a batch of operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchRequest {
    /// List of operations to execute
    pub operations: Vec<serde_json::Value>,

    /// If true, execute operations as a single transaction (all or nothing)
    #[serde(default)]
    pub transaction: bool,
}

/// Execute a batch of operations
///
/// This endpoint allows you to perform multiple memory and relationship operations
/// in a single request. Operations are executed sequentially by default, or as a
/// transaction if `transaction: true` is specified.
///
/// # Example Request
///
/// ```json
/// {
///   "operations": [
///     {
///       "op": "CreateMemory",
///       "data": {
///         "content": "Batch created memory",
///         "memory_type": "fact",
///         "priority": 1
///       }
///     },
///     {
///       "op": "CreateRelationship",
///       "data": {
///         "source": "memory_1",
///         "target": "memory_2",
///         "relationship_type": "references"
///       }
///     }
///   ],
///   "transaction": false
/// }
/// ```
///
/// # Response
///
/// Returns a `BatchResponse` with results for each operation:
/// - `operation_index`: Index in the original operations array
/// - `resource_id`: ID of the created/updated/deleted resource
/// - `error`: Error message if the operation failed
///
/// # Limits
///
/// - Maximum 1000 operations per batch
/// - Default timeout: 30 seconds
///
/// # Errors
///
/// - `400 Bad Request`: Batch validation failed or exceeds size limit
/// - `500 Internal Server Error`: Storage or processing error
#[utoipa::path(
    post,
    path = "/api/v1/batch",
    tag = "batch",
    request_body = BatchRequest,
    responses(
        (status = 200, description = "Batch executed successfully", body = serde_json::Value),
        (status = 400, description = "Invalid batch request or size exceeded"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn batch_execute(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(request): axum::extract::Json<BatchRequest>,
) -> ServerResult<Json<BatchResponse>> {
    debug!(
        "Batch execute: {} operations, transaction={}",
        request.operations.len(),
        request.transaction
    );

    // Deserialize operations from serde_json::Value to BatchOperation
    let operations: Vec<BatchOperation> = request
        .operations
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    // Get storage from memory manager
    let storage = state.memory_manager.storage().clone();

    // Create executor with default config
    let config = BatchExecutorConfig::default();
    let executor = BatchExecutor::new(storage, config);

    // Execute the batch
    let response = executor
        .execute(operations, request.transaction)
        .await
        .map_err(|e| ServerError::BadRequest(format!("Batch execution failed: {}", e)))?;

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_request_serialization() {
        let request = BatchRequest {
            operations: vec![serde_json::json!({
                "op": "CreateMemory",
                "data": {
                    "content": "Test",
                    "memory_type": "fact",
                    "priority": 1
                }
            })],
            transaction: false,
        };

        let json = serde_json::to_string(&request).expect("Should serialize");
        let _deserialized: BatchRequest = serde_json::from_str(&json).expect("Should deserialize");
    }
}
