//! WebSocket integration tests

use std::sync::Arc;
use std::time::Duration;

use axum_test::TestServer;
use futures::{SinkExt, StreamExt};
use locai::prelude::*;
use locai_server::{AppState, create_router};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use locai_server::config::ServerConfig;
use locai_server::websocket::{MemoryFilter, WebSocketMessage};

#[tokio::test]
#[ignore] // Skip: WebSocket tests hang in test environment
async fn test_websocket_connection() {
    let app_state = create_test_app_state().await;
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    // Get the WebSocket URL
    let port = server
        .server_address()
        .map_or(8080, |addr| addr.port().unwrap_or(8080));
    let ws_url = format!("ws://localhost:{}/api/ws", port);

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
    let (_ws_sender, mut ws_receiver) = ws_stream.split();

    // Should receive a connection message
    let msg = timeout(Duration::from_secs(5), ws_receiver.next())
        .await
        .expect("Timeout waiting for connection message")
        .expect("No message received")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let ws_msg: WebSocketMessage = serde_json::from_str(&text).expect("Invalid JSON");
        match ws_msg {
            WebSocketMessage::Connected { connection_id } => {
                assert!(!connection_id.is_empty());
            }
            _ => panic!("Expected Connected message, got: {:?}", ws_msg),
        }
    } else {
        panic!("Expected text message");
    }
}

#[tokio::test]
#[ignore] // Skip: WebSocket tests hang in test environment
async fn test_websocket_subscription() {
    let app_state = create_test_app_state().await;
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    // Get the WebSocket URL
    let port = server
        .server_address()
        .map_or(8080, |addr| addr.port().unwrap_or(8080));
    let ws_url = format!("ws://localhost:{}/api/ws", port);

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Skip the connection message
    let _ = ws_receiver.next().await;

    // Send subscription message
    let subscription = WebSocketMessage::Subscribe {
        memory_filter: Some(MemoryFilter {
            memory_type: Some("test".to_string()),
            importance_min: Some(0.5),
            importance_max: None,
            content_contains: None,
        }),
        entity_filter: None,
        relationship_filter: None,
    };

    let subscription_json = serde_json::to_string(&subscription).unwrap();
    ws_sender
        .send(Message::Text(subscription_json.into()))
        .await
        .unwrap();

    // Should receive subscription acknowledgment
    let msg = timeout(Duration::from_secs(5), ws_receiver.next())
        .await
        .expect("Timeout waiting for subscription ack")
        .expect("No message received")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let ws_msg: WebSocketMessage = serde_json::from_str(&text).expect("Invalid JSON");
        match ws_msg {
            WebSocketMessage::SubscriptionAck {
                filters_applied,
                message,
            } => {
                assert!(filters_applied);
                assert!(!message.is_empty());
            }
            _ => panic!("Expected SubscriptionAck message, got: {:?}", ws_msg),
        }
    } else {
        panic!("Expected text message");
    }
}

#[tokio::test]
#[ignore] // Skip: WebSocket tests hang in test environment
async fn test_websocket_ping_pong() {
    let app_state = create_test_app_state().await;
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    // Get the WebSocket URL
    let port = server
        .server_address()
        .map_or(8080, |addr| addr.port().unwrap_or(8080));
    let ws_url = format!("ws://localhost:{}/api/ws", port);

    // Connect to WebSocket
    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Skip the connection message
    let _ = ws_receiver.next().await;

    // Send ping message
    let ping = WebSocketMessage::Ping;
    let ping_json = serde_json::to_string(&ping).unwrap();
    ws_sender
        .send(Message::Text(ping_json.into()))
        .await
        .unwrap();

    // Should receive pong response
    let msg = timeout(Duration::from_secs(5), ws_receiver.next())
        .await
        .expect("Timeout waiting for pong")
        .expect("No message received")
        .expect("WebSocket error");

    if let Message::Text(text) = msg {
        let ws_msg: WebSocketMessage = serde_json::from_str(&text).expect("Invalid JSON");
        match ws_msg {
            WebSocketMessage::Pong => {
                // Success
            }
            _ => panic!("Expected Pong message, got: {:?}", ws_msg),
        }
    } else {
        panic!("Expected text message");
    }
}

async fn create_test_app_state() -> Arc<AppState> {
    let config = ConfigBuilder::new().with_memory_storage().build().unwrap();

    let memory_manager = locai::init(config).await.unwrap();
    let mut server_config = ServerConfig::default();
    server_config.enable_auth = false; // Disable auth for testing

    Arc::new(AppState::new(memory_manager, server_config))
}
