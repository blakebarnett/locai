//! Authentication and authorization for the Locai API

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use headers::{authorization::Bearer, Authorization, HeaderMapExt};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::ServerError, state::AppState};

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User ID
    pub sub: String,
    /// Username  
    pub username: String,
    /// User role
    pub role: String,
    /// Issued at timestamp
    pub iat: usize,
    /// Expiration timestamp
    pub exp: usize,
}

/// User authentication context
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthContext {
    /// User ID
    pub user_id: Uuid,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
}

/// User signup request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignupRequest {
    /// Username (must be unique)
    pub username: String,
    /// Password (will be hashed)
    pub password: String,
    /// Optional email
    pub email: Option<String>,
}

/// User login request
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Username
    pub username: String,
    /// Password
    pub password: String,
}

/// Authentication response
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    /// JWT token
    pub token: String,
    /// User ID
    pub user_id: String,
    /// Username
    pub username: String,
    /// User role
    pub role: String,
    /// Token expiration timestamp
    pub expires_at: i64,
}

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, ServerError> {
    // Skip authentication if not enabled
    if !state.config.enable_auth {
        return Ok(next.run(request).await);
    }
    
    // Skip authentication for public endpoints
    let path = request.uri().path();
    tracing::debug!("Checking auth for path: {}", path);
    if is_public_endpoint(path) {
        tracing::debug!("Path {} is public, skipping auth", path);
        return Ok(next.run(request).await);
    }
    
    tracing::debug!("Path {} requires auth", path);
    
    // Get authorization header
    let auth_header = headers.typed_get::<Authorization<Bearer>>()
        .ok_or_else(|| ServerError::Auth("Missing authorization header".to_string()))?;
    
    // Validate and decode the JWT token
    let auth_context = validate_jwt_token(auth_header.token(), &state.config.jwt_secret)?;
    
    // Insert auth context into request extensions
    request.extensions_mut().insert(auth_context);
    
    // Continue with the request
    Ok(next.run(request).await)
}

/// Check if an endpoint is public (doesn't require authentication)
fn is_public_endpoint(path: &str) -> bool {
    matches!(path, 
        "/health" | 
        "/docs" | 
        "/api-docs" |
        "/auth/login" | 
        "/auth/signup"
    ) || path.starts_with("/docs") || path.starts_with("/api-docs")
}

/// Validate a JWT token and return the authentication context
fn validate_jwt_token(token: &str, secret: &str) -> Result<AuthContext, ServerError> {
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::default();
    
    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|e| ServerError::Auth(format!("Invalid token: {}", e)))?;
    
    let user_id = Uuid::parse_str(&token_data.claims.sub)
        .map_err(|e| ServerError::Auth(format!("Invalid user ID in token: {}", e)))?;
    
    Ok(AuthContext {
        user_id,
        username: token_data.claims.username,
        role: token_data.claims.role,
    })
}

/// Generate a JWT token for a user
pub fn generate_jwt_token(
    user_id: &Uuid,
    username: &str,
    role: &str,
    secret: &str,
    expiration_hours: u64,
) -> Result<(String, i64), ServerError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let exp = now + (expiration_hours * 3600) as usize;
    
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        iat: now,
        exp,
    };
    
    let encoding_key = EncodingKey::from_secret(secret.as_ref());
    let token = encode(&Header::default(), &claims, &encoding_key)
        .map_err(|e| ServerError::Auth(format!("Failed to generate token: {}", e)))?;
    
    Ok((token, exp as i64))
}

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String, ServerError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| ServerError::Auth(format!("Failed to hash password: {}", e)))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, ServerError> {
    bcrypt::verify(password, hash)
        .map_err(|e| ServerError::Auth(format!("Failed to verify password: {}", e)))
}

/// Extract authentication context from request extensions
#[allow(dead_code)]
pub fn get_auth_context(request: &Request) -> Option<&AuthContext> {
    request.extensions().get::<AuthContext>()
}

/// Check if user has required role
#[allow(dead_code)]
pub fn check_role_permission(auth_context: &AuthContext, required_role: &str) -> bool {
    match (auth_context.role.as_str(), required_role) {
        ("root", _) => true, // Root can do anything
        ("admin", "user") => true, // Admin can do user actions
        (user_role, required) => user_role == required,
    }
}

/// Generate a secure random root password
pub fn generate_root_password() -> String {
    use rand::Rng;
    use rand::distr::Alphanumeric;
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
} 