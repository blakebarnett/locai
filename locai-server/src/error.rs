//! Error handling for the Locai server

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Server error types
#[derive(Debug, Error)]
pub enum ServerError {
    /// Locai library error
    #[error("Locai error: {0}")]
    Locai(#[from] locai::LocaiError),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found error
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Bad request error
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Internal server error
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    #[allow(dead_code)]
    RateLimit,

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    #[allow(dead_code)]
    WebSocket(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic error
    #[error("{0}")]
    #[allow(dead_code)]
    Generic(String),
}

impl ServerError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ServerError::Auth(_) => StatusCode::UNAUTHORIZED,
            ServerError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Validation(_) | ServerError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ServerError::NotFound(_) => StatusCode::NOT_FOUND,
            ServerError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            ServerError::Locai(locai::LocaiError::MLNotConfigured) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error type string
    pub fn error_type(&self) -> &'static str {
        match self {
            ServerError::Locai(_) => "locai_error",
            ServerError::Auth(_) => "authentication_error",
            ServerError::Database(_) => "database_error",
            ServerError::Validation(_) => "validation_error",
            ServerError::NotFound(_) => "not_found",
            ServerError::BadRequest(_) => "bad_request",
            ServerError::Internal(_) => "internal_error",
            ServerError::RateLimit => "rate_limit_exceeded",
            ServerError::WebSocket(_) => "websocket_error",
            ServerError::Serialization(_) => "serialization_error",
            ServerError::Generic(_) => "generic_error",
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = ErrorResponse {
            error: self.error_type().to_string(),
            message: self.to_string(),
            details: None,
        };

        (status, Json(error_response)).into_response()
    }
}

/// Result type for server operations
pub type ServerResult<T> = Result<T, ServerError>;

/// Helper function to create a not found error
pub fn not_found(resource: &str, id: &str) -> ServerError {
    ServerError::NotFound(format!("{} with id '{}' not found", resource, id))
}

/// Helper function to create a validation error
#[allow(dead_code)]
pub fn validation_error(message: &str) -> ServerError {
    ServerError::Validation(message.to_string())
}

/// Helper function to create a bad request error
pub fn bad_request(message: &str) -> ServerError {
    ServerError::BadRequest(message.to_string())
}
