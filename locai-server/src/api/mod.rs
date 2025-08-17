//! API implementation for the Locai HTTP server

use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    middleware,
    response::Json,
    routing::{delete, get, post, put},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{state::AppState, websocket::websocket_handler};

pub mod auth;
pub mod auth_endpoints;
pub mod auth_service;
pub mod dto;
pub mod entities;
pub mod graph;
pub mod memories;
pub mod relationships;
pub mod versions;

use auth::auth_middleware;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        auth_endpoints::signup,
        auth_endpoints::login,
        auth_endpoints::list_users,
        auth_endpoints::get_user,
        auth_endpoints::update_user,
        auth_endpoints::delete_user,
        memories::create_memory,
        memories::get_memory,
        memories::list_memories,
        memories::update_memory,
        memories::delete_memory,
        memories::search_memories,
        entities::list_entities,
        entities::get_entity,
        entities::create_entity,
        entities::update_entity,
        entities::delete_entity,
        entities::get_entity_memories,
        relationships::list_relationships,
        relationships::get_relationship,
        relationships::create_relationship,
        relationships::update_relationship,
        relationships::delete_relationship,
        relationships::find_related_entities,
        versions::list_versions,
        versions::create_version,
        versions::checkout_version,
        graph::get_memory_graph,
        graph::get_entity_graph,
        graph::find_paths,
        graph::query_graph,
        graph::get_graph_metrics,
        graph::find_similar_structures,
        graph::get_related_entities,
        graph::get_central_entities,
    ),
    components(
        schemas(
            auth::SignupRequest,
            auth::LoginRequest,
            auth::AuthResponse,
            auth_endpoints::UserDto,
            auth_endpoints::CreateUserRequest,
            auth_endpoints::UpdateUserRequest,
            dto::MemoryDto,
            dto::CreateMemoryRequest,
            dto::UpdateMemoryRequest,
            dto::EntityDto,
            dto::CreateEntityRequest,
            dto::UpdateEntityRequest,
            dto::RelationshipDto,
            dto::CreateRelationshipRequest,
            dto::VersionDto,
            dto::CreateVersionRequest,
            dto::CheckoutVersionRequest,
            dto::MemoryGraphDto,
            dto::MemoryPathDto,
            dto::SearchRequest,
            dto::SearchResultDto,
            dto::GraphQueryRequest,
            dto::GraphMetricsDto,
            dto::PaginationParams,
            dto::ErrorResponse,
        )
    ),
    tags(
        (name = "auth", description = "Authentication and user management endpoints"),
        (name = "memories", description = "Memory management endpoints"),
        (name = "entities", description = "Manual entity management endpoints (CRUD operations)"),
        (name = "relationships", description = "Relationship management endpoints"),
        (name = "versions", description = "Version management endpoints"),
        (name = "graph", description = "Graph operations and traversal endpoints"),
        (name = "websocket", description = "WebSocket real-time updates"),
    ),
    info(
                    title = "Locai Memory Service API",
            version = "1.0.0",
            description = "RESTful API for the Locai memory management service with graph-centric design. Provides text search by default, with vector/hybrid search available when ML service is configured.",
            contact(
                name = "Locai Team",
            url = "https://locai.dev"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "/api", description = "API base path")
    )
)]
pub struct ApiDoc;

/// Create the main router with all API endpoints
pub fn create_router(state: Arc<AppState>) -> Router {
    let api_router = Router::new()
        // Authentication endpoints (public, no auth middleware)
        .route("/auth/signup", post(auth_endpoints::signup))
        .route("/auth/login", post(auth_endpoints::login))
        .route("/auth/users", get(auth_endpoints::list_users))
        .route("/auth/users/{id}", get(auth_endpoints::get_user))
        .route("/auth/users/{id}", put(auth_endpoints::update_user))
        .route("/auth/users/{id}", delete(auth_endpoints::delete_user))
        // Memory endpoints
        .route("/memories", post(memories::create_memory))
        .route("/memories", get(memories::list_memories))
        .route("/memories/{id}", get(memories::get_memory))
        .route("/memories/{id}", put(memories::update_memory))
        .route("/memories/{id}", delete(memories::delete_memory))
        .route("/memories/search", get(memories::search_memories))
        // Entity endpoints
        .route("/entities", get(entities::list_entities))
        .route("/entities/{id}", get(entities::get_entity))
        .route("/entities", post(entities::create_entity))
        .route("/entities/{id}", put(entities::update_entity))
        .route("/entities/{id}", delete(entities::delete_entity))
        .route(
            "/entities/{id}/memories",
            get(entities::get_entity_memories),
        )
        // Relationship endpoints
        .route("/relationships", get(relationships::list_relationships))
        .route("/relationships", post(relationships::create_relationship))
        .route("/relationships/{id}", get(relationships::get_relationship))
        .route(
            "/relationships/{id}",
            put(relationships::update_relationship),
        )
        .route(
            "/relationships/{id}",
            delete(relationships::delete_relationship),
        )
        .route(
            "/relationships/{id}/related",
            get(relationships::find_related_entities),
        )
        // Version endpoints
        .route("/versions", get(versions::list_versions))
        .route("/versions", post(versions::create_version))
        .route("/versions/{id}/checkout", put(versions::checkout_version))
        // Graph operation endpoints
        .route("/memories/{id}/graph", get(graph::get_memory_graph))
        .route("/entities/{id}/graph", get(graph::get_entity_graph))
        .route("/graph/paths", get(graph::find_paths))
        .route("/graph/query", post(graph::query_graph))
        .route("/graph/metrics", get(graph::get_graph_metrics))
        .route(
            "/graph/similar_structures",
            get(graph::find_similar_structures),
        )
        .route(
            "/entities/{id}/related_entities",
            get(graph::get_related_entities),
        )
        .route("/entities/central", get(graph::get_central_entities))
        // WebSocket endpoints
        .route("/ws", get(websocket_handler))
        .route("/messaging/ws", get(messaging_websocket_handler))
        // Health check endpoint (with capability reporting)
        .route("/health", get(health_check))
        // Add authentication middleware if enabled
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state);

    // Main router with API prefix and documentation
    let swagger_router = SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi());

    Router::new().nest("/api", api_router).merge(swagger_router)
}

/// Health check endpoint with capability reporting
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Service health and capabilities", body = serde_json::Value)
    )
)]
async fn health_check(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let capabilities = serde_json::json!({
        "status": "OK",
        "capabilities": {
            "text_search": true,
            "vector_search": state.memory_manager.has_ml_service(),
            "hybrid_search": state.memory_manager.has_ml_service(),
            "entity_extraction": {
                "basic": state.memory_manager.config().entity_extraction.enabled,
                "ml_based": false  // Advanced ML entity extraction moved to examples
            },
            "entity_management": true,  // Manual CRUD operations for entities via API
            "memory_management": true,  // CRUD operations for memories
            "relationship_management": true,  // CRUD operations for relationships between memories/entities
            "graph_operations": true,
            "messaging": state.messaging_server.is_some(),
            "authentication": state.config.enable_auth
        },
        "search_modes": {
            "available": if state.memory_manager.has_ml_service() {
                vec!["text", "vector", "hybrid"]
            } else {
                vec!["text"]
            },
            "default": "text"
        }
    });

    Json(capabilities)
}

/// Messaging WebSocket handler
async fn messaging_websocket_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Response {
    if let Some(messaging_server) = &state.messaging_server {
        let messaging_server = messaging_server.clone();
        ws.on_upgrade(move |socket| {
            crate::messaging::handle_messaging_websocket(socket, messaging_server)
        })
    } else {
        // Return error if messaging is not enabled
        axum::response::Response::builder()
            .status(503)
            .body("Messaging service not available".into())
            .unwrap()
    }
}
