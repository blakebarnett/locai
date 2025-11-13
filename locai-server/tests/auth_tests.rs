//! Tests for JWT authentication and authorization

use axum::http::StatusCode;
use axum_test::TestServer;
use jsonwebtoken::{DecodingKey, Validation, decode};
use locai_server::{
    api::auth::{Claims, generate_jwt_token},
    config::ServerConfig,
    state::AppState,
};
use std::sync::Arc;
use uuid::Uuid;

async fn create_test_server_with_auth() -> (TestServer, Arc<AppState>, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create Locai configuration for testing with memory storage
    let config = locai::config::ConfigBuilder::new()
        .with_data_dir(temp_dir.path())
        .with_memory_storage()
        .build()
        .expect("Failed to create config");

    // Initialize MemoryManager
    let memory_manager = locai::init(config)
        .await
        .expect("Failed to initialize memory manager");

    // Create server configuration with authentication enabled
    let mut server_config = ServerConfig::default();
    server_config.enable_auth = true;
    server_config.allow_signup = true;
    server_config.jwt_secret = "test-secret-key-for-jwt-token-generation".to_string();

    // Create AppState
    let mut app_state = AppState::new(memory_manager, server_config.clone());

    // Initialize auth service if auth is enabled
    if server_config.enable_auth {
        use locai_server::api::auth_service::AuthService;
        let auth_service = AuthService::new(server_config.jwt_secret.clone());
        app_state.set_auth_service(auth_service);
    }

    let state = Arc::new(app_state);
    let app = locai_server::create_router(state.clone());
    let server = TestServer::new(app).unwrap();

    (server, state, temp_dir)
}

#[tokio::test]
async fn test_jwt_token_generation() {
    let user_id = Uuid::new_v4();
    let username = "testuser";
    let role = "viewer";
    let secret = "test-secret-key";
    let expiration_hours = 24;

    let (token, expires_at) =
        generate_jwt_token(&user_id, username, role, secret, expiration_hours).unwrap();

    // Token should not be empty
    assert!(!token.is_empty());

    // Expiration should be in the future
    let now = chrono::Utc::now().timestamp();
    assert!(expires_at > now);

    // Token should be decodable
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::default();
    let token_data = decode::<Claims>(&token, &decoding_key, &validation).unwrap();

    // Verify claims
    assert_eq!(token_data.claims.sub, user_id.to_string());
    assert_eq!(token_data.claims.username, username);
    assert_eq!(token_data.claims.role, role);
}

#[tokio::test]
async fn test_jwt_token_validation() {
    let user_id = Uuid::new_v4();
    let username = "testuser";
    let role = "viewer";
    let secret = "test-secret-key";
    let expiration_hours = 24;

    let (token, _) =
        generate_jwt_token(&user_id, username, role, secret, expiration_hours).unwrap();

    // Valid token should decode successfully using jsonwebtoken directly
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::default();
    let token_data = decode::<Claims>(&token, &decoding_key, &validation).unwrap();

    assert_eq!(token_data.claims.sub, user_id.to_string());
    assert_eq!(token_data.claims.username, username);
    assert_eq!(token_data.claims.role, role);
}

#[tokio::test]
async fn test_jwt_token_validation_wrong_secret() {
    let user_id = Uuid::new_v4();
    let username = "testuser";
    let role = "viewer";
    let secret = "test-secret-key";
    let wrong_secret = "wrong-secret-key";
    let expiration_hours = 24;

    let (token, _) =
        generate_jwt_token(&user_id, username, role, secret, expiration_hours).unwrap();

    // Token with wrong secret should fail validation
    let decoding_key = DecodingKey::from_secret(wrong_secret.as_ref());
    let validation = Validation::default();
    let result = decode::<Claims>(&token, &decoding_key, &validation);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_jwt_token_expiration() {
    let user_id = Uuid::new_v4();
    let username = "testuser";
    let role = "viewer";
    let secret = "test-secret-key";

    // Create a token that expires in the past
    let now = chrono::Utc::now().timestamp() as usize;
    let exp = now - 3600; // Expired 1 hour ago

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        iat: now - 7200, // Issued 2 hours ago
        exp,
    };

    let encoding_key = jsonwebtoken::EncodingKey::from_secret(secret.as_ref());
    let token =
        jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &encoding_key).unwrap();

    // Expired token should fail validation
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::default();
    let result = decode::<Claims>(&token, &decoding_key, &validation);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_signup_endpoint() {
    let (server, _state, _temp_dir) = create_test_server_with_auth().await;

    let signup_request = serde_json::json!({
        "username": "newuser",
        "password": "password123",
        "email": "newuser@example.com"
    });

    let response = server.post("/api/auth/signup").json(&signup_request).await;

    response.assert_status(StatusCode::CREATED);

    let auth_response: serde_json::Value = response.json();
    assert!(auth_response["token"].is_string());
    assert!(!auth_response["token"].as_str().unwrap().is_empty());
    assert_eq!(auth_response["username"], "newuser");
    assert_eq!(auth_response["role"], "viewer");
    assert!(auth_response["expires_at"].is_number());
}

#[tokio::test]
async fn test_signup_duplicate_username() {
    let (server, _state, _temp_dir) = create_test_server_with_auth().await;

    let signup_request = serde_json::json!({
        "username": "duplicate",
        "password": "password123"
    });

    // First signup should succeed
    let response1 = server.post("/api/auth/signup").json(&signup_request).await;
    response1.assert_status(StatusCode::CREATED);

    // Second signup with same username should fail (returns 400 Bad Request for validation error)
    let response2 = server.post("/api/auth/signup").json(&signup_request).await;
    response2.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_signup_validation() {
    let (server, _state, _temp_dir) = create_test_server_with_auth().await;

    // Test empty username
    let request1 = serde_json::json!({
        "username": "",
        "password": "password123"
    });
    let response1 = server.post("/api/auth/signup").json(&request1).await;
    response1.assert_status(StatusCode::BAD_REQUEST);

    // Test short password
    let request2 = serde_json::json!({
        "username": "user",
        "password": "short"
    });
    let response2 = server.post("/api/auth/signup").json(&request2).await;
    response2.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_login_endpoint() {
    let (server, _state, _temp_dir) = create_test_server_with_auth().await;

    // First create a user
    let signup_request = serde_json::json!({
        "username": "loginuser",
        "password": "password123"
    });
    let signup_response = server.post("/api/auth/signup").json(&signup_request).await;
    signup_response.assert_status(StatusCode::CREATED);

    // Then login with correct credentials
    let login_request = serde_json::json!({
        "username": "loginuser",
        "password": "password123"
    });
    let login_response = server.post("/api/auth/login").json(&login_request).await;

    login_response.assert_status(StatusCode::OK);

    let auth_response: serde_json::Value = login_response.json();
    assert!(auth_response["token"].is_string());
    assert!(!auth_response["token"].as_str().unwrap().is_empty());
    assert_eq!(auth_response["username"], "loginuser");
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let (server, _state, _temp_dir) = create_test_server_with_auth().await;

    // Create a user
    let signup_request = serde_json::json!({
        "username": "testuser",
        "password": "password123"
    });
    server.post("/api/auth/signup").json(&signup_request).await;

    // Try login with wrong password
    let login_request = serde_json::json!({
        "username": "testuser",
        "password": "wrongpassword"
    });
    let response = server.post("/api/auth/login").json(&login_request).await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_with_valid_token() {
    let (server, _state, _temp_dir) = create_test_server_with_auth().await;

    // Create a user and get token
    let signup_request = serde_json::json!({
        "username": "authtest",
        "password": "password123"
    });
    let signup_response = server.post("/api/auth/signup").json(&signup_request).await;
    signup_response.assert_status(StatusCode::CREATED);

    let auth_response: serde_json::Value = signup_response.json();
    let token = auth_response["token"].as_str().unwrap();

    // Use token to access protected endpoint
    let response = server
        .get("/api/memories")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;

    response.assert_status_ok();
}

#[tokio::test]
async fn test_auth_middleware_without_token() {
    let (server, _state, _temp_dir) = create_test_server_with_auth().await;

    // Try to access protected endpoint without token
    let response = server.get("/api/memories").await;
    // Should get 401 Unauthorized (missing auth header)
    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_public_endpoints() {
    let (server, _state, _temp_dir) = create_test_server_with_auth().await;

    // Public endpoints should work without auth
    let response = server.get("/api/health").await;
    response.assert_status_ok();

    // Auth endpoints should work without auth
    let login_request = serde_json::json!({
        "username": "publictest",
        "password": "password123"
    });
    // This will fail because user doesn't exist, but endpoint is accessible
    let response = server.post("/api/auth/login").json(&login_request).await;
    // Should get 401, not 403 (endpoint is accessible, just wrong credentials)
    assert!(response.status_code() == StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_jwt_token_claims_structure() {
    let user_id = Uuid::new_v4();
    let username = "claimstest";
    let role = "admin";
    let secret = "test-secret-key";
    let expiration_hours = 24;

    let (token, expires_at) =
        generate_jwt_token(&user_id, username, role, secret, expiration_hours).unwrap();

    // Decode and verify all claims
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::default();
    let token_data = decode::<Claims>(&token, &decoding_key, &validation).unwrap();

    assert_eq!(token_data.claims.sub, user_id.to_string());
    assert_eq!(token_data.claims.username, username);
    assert_eq!(token_data.claims.role, role);
    assert!(token_data.claims.iat > 0);
    assert!(token_data.claims.exp > token_data.claims.iat);
    assert_eq!(token_data.claims.exp as i64, expires_at);
}
