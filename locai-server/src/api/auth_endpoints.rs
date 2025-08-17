//! Authentication endpoints for user signup and login

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::auth::{AuthResponse, LoginRequest, SignupRequest},
    api::auth_service::User,
    error::{ServerError, bad_request},
    state::AppState,
};

/// User data transfer object
#[derive(Debug, Serialize, ToSchema)]
pub struct UserDto {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email (if provided)
    pub email: Option<String>,
    /// User role
    pub role: String,
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        UserDto {
            id: user.id.to_string(),
            username: user.username,
            email: user.email,
            role: user.role,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

/// User creation request for admin users
#[derive(Debug, Deserialize, ToSchema)]
#[allow(dead_code)] // Used in API schema, admin functionality not yet implemented
pub struct CreateUserRequest {
    /// Username (must be unique)
    pub username: String,
    /// Password (will be hashed)
    pub password: String,
    /// Optional email
    pub email: Option<String>,
    /// User role (admin only)
    pub role: Option<String>,
}

/// User update request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    /// Updated email (optional)
    pub email: Option<String>,
    /// Updated role (admin only, optional)
    pub role: Option<String>,
}

/// User signup endpoint
#[utoipa::path(
    post,
    path = "/api/auth/signup",
    tag = "auth",
    summary = "Register a new user account",
    request_body = SignupRequest,
    responses(
        (status = 201, description = "User created successfully", body = AuthResponse),
        (status = 400, description = "Invalid request data"),
        (status = 409, description = "Username already exists"),
        (status = 403, description = "Signup disabled"),
    )
)]
pub async fn signup(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SignupRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), ServerError> {
    // Check if signup is allowed
    if !state.config.allow_signup {
        return Err(ServerError::Auth("User signup is disabled".to_string()));
    }

    // Validate input
    if request.username.trim().is_empty() {
        return Err(bad_request("Username cannot be empty"));
    }

    if request.password.len() < 8 {
        return Err(bad_request("Password must be at least 8 characters"));
    }

    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ServerError::Internal("Authentication service not available".to_string()))?;

    // Create user with viewer role by default
    let user = auth_service
        .create_user(
            &state.memory_manager,
            &request.username,
            &request.password,
            "viewer",
            request.email,
        )
        .await?;

    // Authenticate the new user to get a token
    let (token, _user, expires_at) = auth_service
        .authenticate(&state.memory_manager, &request.username, &request.password)
        .await?;

    // Return authentication response
    let auth_response = AuthResponse {
        token,
        user_id: user.id.to_string(),
        username: user.username,
        role: user.role,
        expires_at,
    };

    Ok((StatusCode::CREATED, Json(auth_response)))
}

/// User login endpoint
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    summary = "Authenticate user and get JWT token",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 400, description = "Invalid request data"),
    )
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ServerError> {
    // Validate input
    if request.username.trim().is_empty() || request.password.is_empty() {
        return Err(ServerError::Auth(
            "Username and password are required".to_string(),
        ));
    }

    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ServerError::Internal("Authentication service not available".to_string()))?;

    // Authenticate user
    let (token, user, expires_at) = auth_service
        .authenticate(&state.memory_manager, &request.username, &request.password)
        .await?;

    let auth_response = AuthResponse {
        token,
        user_id: user.id.to_string(),
        username: user.username,
        role: user.role,
        expires_at,
    };

    Ok(Json(auth_response))
}

/// List users endpoint (admin only)
#[utoipa::path(
    get,
    path = "/api/auth/users",
    tag = "auth",
    summary = "List all users (admin only)",
    responses(
        (status = 200, description = "List of users", body = Vec<UserDto>),
        (status = 403, description = "Insufficient permissions"),
    )
)]
pub async fn list_users(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<UserDto>>, ServerError> {
    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ServerError::Internal("Authentication service not available".to_string()))?;

    // Get all users
    let users = auth_service.list_users(&state.memory_manager).await?;

    let user_dtos: Vec<UserDto> = users.into_iter().map(UserDto::from).collect();

    Ok(Json(user_dtos))
}

/// Get user by ID endpoint
#[utoipa::path(
    get,
    path = "/api/auth/users/{id}",
    tag = "auth",
    summary = "Get user by ID",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User details", body = UserDto),
        (status = 404, description = "User not found"),
        (status = 403, description = "Insufficient permissions"),
    )
)]
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<UserDto>, ServerError> {
    // Parse user ID
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| ServerError::BadRequest(format!("Invalid user ID: {}", user_id)))?;

    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ServerError::Internal("Authentication service not available".to_string()))?;

    // Get user by ID
    let user = auth_service
        .get_user_by_id(&state.memory_manager, &user_uuid)
        .await?
        .ok_or_else(|| ServerError::NotFound(format!("User with ID '{}' not found", user_id)))?;

    let user_dto = UserDto::from(user);

    Ok(Json(user_dto))
}

/// Update user endpoint (admin only)
#[utoipa::path(
    put,
    path = "/api/auth/users/{id}",
    tag = "auth",
    summary = "Update user (admin only)",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserDto),
        (status = 404, description = "User not found"),
        (status = 403, description = "Insufficient permissions"),
        (status = 400, description = "Invalid request data"),
    )
)]
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<UserDto>, ServerError> {
    // Parse user ID
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| bad_request(&format!("Invalid user ID: {}", user_id)))?;

    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ServerError::Internal("Authentication service not available".to_string()))?;

    // Get existing user
    let mut user = auth_service
        .get_user_by_id(&state.memory_manager, &user_uuid)
        .await?
        .ok_or_else(|| ServerError::NotFound(format!("User with ID '{}' not found", user_id)))?;

    // Apply updates
    if let Some(email) = request.email {
        user.email = Some(email);
    }

    if let Some(role) = request.role {
        user.role = role;
    }

    user.updated_at = chrono::Utc::now();

    // Update user
    let updated_user = auth_service
        .update_user(&state.memory_manager, user)
        .await?;

    let user_dto = UserDto::from(updated_user);
    Ok(Json(user_dto))
}

/// Delete user endpoint (admin only)
#[utoipa::path(
    delete,
    path = "/api/auth/users/{id}",
    tag = "auth",
    summary = "Delete user (admin only)",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted successfully"),
        (status = 404, description = "User not found"),
        (status = 403, description = "Insufficient permissions"),
    )
)]
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<axum::http::StatusCode, ServerError> {
    // Parse user ID
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| bad_request(&format!("Invalid user ID: {}", user_id)))?;

    // Get auth service
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ServerError::Internal("Authentication service not available".to_string()))?;

    // Check if user exists
    let _user = auth_service
        .get_user_by_id(&state.memory_manager, &user_uuid)
        .await?
        .ok_or_else(|| ServerError::NotFound(format!("User with ID '{}' not found", user_id)))?;

    // Delete user
    let deleted = auth_service
        .delete_user(&state.memory_manager, &user_uuid)
        .await?;

    if !deleted {
        return Err(ServerError::Internal("Failed to delete user".to_string()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}
