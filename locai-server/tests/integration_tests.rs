use std::sync::Arc;

use locai_server::create_router;
use axum_test::TestServer;
use serde_json::{json, Value};
use tempfile::TempDir;
use http::StatusCode;

/// Helper function to create a test server with a temporary database
async fn create_test_server() -> (TestServer, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
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
    
    // Create server configuration
    let server_config = locai_server::config::ServerConfig::default();
    
    // Create AppState
    let state = Arc::new(locai_server::AppState::new(memory_manager, server_config));
    
    let app = create_router(state);
    let server = TestServer::new(app).expect("Failed to create test server");
    
    (server, temp_dir)
}

#[tokio::test]
async fn test_health_check() {
    let (server, _temp_dir) = create_test_server().await;
    
    let response = server.get("/health").await;
    
    response.assert_status_ok();
    response.assert_text("OK");
}

#[tokio::test]
async fn test_swagger_docs_available() {
    let (server, _temp_dir) = create_test_server().await;
    
    let response = server.get("/docs/").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn test_openapi_spec_available() {
    let (server, _temp_dir) = create_test_server().await;
    
    let response = server.get("/api-docs/openapi.json").await;
    response.assert_status_ok();
    
    let json: Value = response.json();
    assert_eq!(json["info"]["title"], "Locai Memory Service API");
}

mod memories {
    use super::*;

    #[tokio::test]
    async fn test_create_memory() {
        let (server, _temp_dir) = create_test_server().await;
        
        let memory_data = json!({
            "content": "This is a test memory",
            "priority": "high",
            "memory_type": "episodic"
        });
        
        let response = server
            .post("/api/memories")
            .json(&memory_data)
            .await;
        
        response.assert_status(StatusCode::CREATED);
        
        let json: Value = response.json();
        assert!(json["id"].is_string());
        assert_eq!(json["content"], "This is a test memory");
        assert_eq!(json["priority"], "High");
    }

    #[tokio::test]
    async fn test_list_memories() {
        let (server, _temp_dir) = create_test_server().await;
        
        // Create a few memories
        for i in 0..3 {
            let priority = match i {
                0 => "low",
                1 => "normal", 
                _ => "high",
            };
            
            let memory_data = json!({
                "content": format!("Test memory {}", i),
                "priority": priority,
                "memory_type": "fact"
            });
            
            server
                .post("/api/memories")
                .json(&memory_data)
                .await
                .assert_status(StatusCode::CREATED);
        }
        
        // List memories
        let response = server.get("/api/memories").await;
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
        assert!(json.as_array().unwrap().len() >= 3);
    }
}

mod entities {
    use super::*;

    #[tokio::test]
    async fn test_list_entities() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server.get("/api/entities").await;
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
    }
}

mod relationships {
    use super::*;

    #[tokio::test]
    async fn test_list_relationships() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server.get("/api/relationships").await;
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
    }
}

mod versions {
    use super::*;

    #[tokio::test]
    async fn test_list_versions() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server.get("/api/versions").await;
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
    }
}

mod graph {
    use super::*;

    #[tokio::test]
    async fn test_get_graph_metrics() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server.get("/api/graph/metrics").await;
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json["memory_count"].is_number());
        assert!(json["relationship_count"].is_number());
        assert!(json["average_degree"].is_number());
        assert!(json["density"].is_number());
        assert!(json["connected_components"].is_number());
        assert!(json["central_memories"].is_array());
    }

    #[tokio::test]
    async fn test_get_memory_graph() {
        let (server, _temp_dir) = create_test_server().await;
        
        // Create a test memory first
        let memory_data = json!({
            "content": "Test memory for graph",
            "priority": "normal",
            "memory_type": "fact"
        });
        
        let create_response = server
            .post("/api/memories")
            .json(&memory_data)
            .await;
        
        create_response.assert_status(StatusCode::CREATED);
        let memory_json: Value = create_response.json();
        let memory_id = memory_json["id"].as_str().unwrap();
        
        // Get the memory graph
        let response = server
            .get(&format!("/api/memories/{}/graph", memory_id))
            .await;
        
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert_eq!(json["center_id"], memory_id);
        assert!(json["memories"].is_array());
        assert!(json["relationships"].is_array());
        assert!(json["metadata"].is_object());
    }

    #[tokio::test]
    async fn test_get_memory_graph_with_depth() {
        let (server, _temp_dir) = create_test_server().await;
        
        // Create a test memory first
        let memory_data = json!({
            "content": "Test memory for graph with depth",
            "priority": "normal",
            "memory_type": "fact"
        });
        
        let create_response = server
            .post("/api/memories")
            .json(&memory_data)
            .await;
        
        create_response.assert_status(StatusCode::CREATED);
        let memory_json: Value = create_response.json();
        let memory_id = memory_json["id"].as_str().unwrap();
        
        // Get the memory graph with specific depth
        let response = server
            .get(&format!("/api/memories/{}/graph?depth=3", memory_id))
            .await;
        
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert_eq!(json["center_id"], memory_id);
        assert!(json["memories"].is_array());
        assert!(json["relationships"].is_array());
    }

    #[tokio::test]
    async fn test_get_memory_graph_not_found() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server
            .get("/api/memories/non-existent-id/graph")
            .await;
        
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_find_paths_missing_parameters() {
        let (server, _temp_dir) = create_test_server().await;
        
        // Test missing 'from' parameter
        let response = server
            .get("/api/graph/paths?to=some-id")
            .await;
        
        response.assert_status(StatusCode::BAD_REQUEST);
        
        // Test missing 'to' parameter
        let response = server
            .get("/api/graph/paths?from=some-id")
            .await;
        
        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_find_paths_with_valid_parameters() {
        let (server, _temp_dir) = create_test_server().await;
        
        // Create two test memories
        let memory1_data = json!({
            "content": "First test memory",
            "priority": "normal",
            "memory_type": "fact"
        });
        
        let memory2_data = json!({
            "content": "Second test memory",
            "priority": "normal",
            "memory_type": "fact"
        });
        
        let response1 = server
            .post("/api/memories")
            .json(&memory1_data)
            .await;
        response1.assert_status(StatusCode::CREATED);
        let memory1_json: Value = response1.json();
        let memory1_id = memory1_json["id"].as_str().unwrap();
        
        let response2 = server
            .post("/api/memories")
            .json(&memory2_data)
            .await;
        response2.assert_status(StatusCode::CREATED);
        let memory2_json: Value = response2.json();
        let memory2_id = memory2_json["id"].as_str().unwrap();
        
        // Find paths between the memories
        let response = server
            .get(&format!("/api/graph/paths?from={}&to={}&max_depth=3", memory1_id, memory2_id))
            .await;
        
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn test_query_graph_connected() {
        let (server, _temp_dir) = create_test_server().await;
        
        let query_data = json!({
            "pattern": "connected",
            "limit": 10
        });
        
        let response = server
            .post("/api/graph/query")
            .json(&query_data)
            .await;
        
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn test_query_graph_isolated() {
        let (server, _temp_dir) = create_test_server().await;
        
        let query_data = json!({
            "pattern": "isolated",
            "limit": 10
        });
        
        let response = server
            .post("/api/graph/query")
            .json(&query_data)
            .await;
        
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn test_query_graph_semantic() {
        let (server, _temp_dir) = create_test_server().await;
        
        // Create a test memory first
        let memory_data = json!({
            "content": "Test memory about science",
            "priority": "normal",
            "memory_type": "fact"
        });
        
        server
            .post("/api/memories")
            .json(&memory_data)
            .await
            .assert_status(StatusCode::CREATED);
        
        let query_data = json!({
            "pattern": "science",
            "limit": 10
        });
        
        let response = server
            .post("/api/graph/query")
            .json(&query_data)
            .await;
        
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn test_find_similar_structures_missing_pattern() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server
            .get("/api/graph/similar_structures")
            .await;
        
        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_find_similar_structures_with_pattern() {
        let (server, _temp_dir) = create_test_server().await;
        
        // Create a test memory first
        let memory_data = json!({
            "content": "Test memory for pattern matching",
            "priority": "normal",
            "memory_type": "fact"
        });
        
        let create_response = server
            .post("/api/memories")
            .json(&memory_data)
            .await;
        
        create_response.assert_status(StatusCode::CREATED);
        let memory_json: Value = create_response.json();
        let memory_id = memory_json["id"].as_str().unwrap();
        
        let response = server
            .get(&format!("/api/graph/similar_structures?pattern={}", memory_id))
            .await;
        
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn test_get_central_entities() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server
            .get("/api/entities/central")
            .await;
        
        response.assert_status_ok();
        
        let json: Value = response.json();
        assert!(json.is_array());
        
        // Check structure of central entities if any exist
        if let Some(entities) = json.as_array() {
            if !entities.is_empty() {
                let first_entity = &entities[0];
                assert!(first_entity["memory_id"].is_string());
                assert!(first_entity["centrality_score"].is_number());
                assert!(first_entity["content_preview"].is_string());
            }
        }
    }

    #[tokio::test]
    async fn test_get_entity_graph_not_found() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server
            .get("/api/entities/non-existent-id/graph")
            .await;
        
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_related_entities_not_found() {
        let (server, _temp_dir) = create_test_server().await;
        
        let response = server
            .get("/api/entities/non-existent-id/related_entities")
            .await;
        
        response.assert_status(StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn test_authentication_middleware() {
    let (server, _temp_dir) = create_test_server().await;
    
    // Test without API key (should work since auth is disabled in test)
    let response = server.get("/api/memories").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn test_error_handling() {
    let (server, _temp_dir) = create_test_server().await;
    
    // Test 404 for non-existent memory
    let response = server
        .get("/api/memories/non-existent-id")
        .await;
    response.assert_status(StatusCode::NOT_FOUND);
} 