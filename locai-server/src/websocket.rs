//! WebSocket implementation for real-time updates

use std::sync::Arc;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::state::AppState;

/// Filter for memory events in subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFilter {
    pub memory_type: Option<String>,
    pub importance_min: Option<f64>,
    pub importance_max: Option<f64>,
    pub content_contains: Option<String>,
}

/// Filter for entity events in subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityFilter {
    pub entity_type: Option<String>,
    pub properties_contains: Option<String>,
}

/// Filter for relationship events in subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipFilter {
    pub relationship_type: Option<String>,
    pub source_id: Option<String>,
    pub target_id: Option<String>,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WebSocketMessage {
    /// Memory was created
    MemoryCreated {
        memory_id: String,
        content: String,
        memory_type: String,
        metadata: serde_json::Value,
        importance: Option<f64>,
        /// Node ID to prevent echo loops in multi-instance deployments
        node_id: Option<String>,
    },

    /// Memory was updated
    MemoryUpdated {
        memory_id: String,
        content: String,
        metadata: serde_json::Value,
        importance: Option<f64>,
        node_id: Option<String>,
    },

    /// Memory was deleted
    MemoryDeleted {
        memory_id: String,
        node_id: Option<String>,
    },

    /// Relationship was created
    RelationshipCreated {
        relationship_id: String,
        source_id: String,
        target_id: String,
        relationship_type: String,
        properties: serde_json::Value,
        node_id: Option<String>,
    },

    /// Relationship was deleted
    RelationshipDeleted {
        relationship_id: String,
        node_id: Option<String>,
    },

    /// Entity was created
    EntityCreated {
        entity_id: String,
        entity_type: String,
        properties: serde_json::Value,
        node_id: Option<String>,
    },

    /// Entity was updated
    EntityUpdated {
        entity_id: String,
        entity_type: String,
        properties: serde_json::Value,
        node_id: Option<String>,
    },

    /// Entity was deleted
    EntityDeleted {
        entity_id: String,
        node_id: Option<String>,
    },

    /// Version was created
    VersionCreated {
        version_id: String,
        description: String,
        node_id: Option<String>,
    },

    /// Connection established
    Connected { connection_id: String },

    /// Client subscription request
    Subscribe {
        memory_filter: Option<MemoryFilter>,
        entity_filter: Option<EntityFilter>,
        relationship_filter: Option<RelationshipFilter>,
    },

    /// Subscription acknowledgment
    SubscriptionAck {
        filters_applied: bool,
        message: String,
    },

    /// Ping message for keepalive
    Ping,

    /// Pong response
    Pong,

    /// Error message
    Error {
        message: String,
        code: Option<String>,
    },
}

/// Handle WebSocket upgrade
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_websocket(socket: WebSocket, state: Arc<AppState>) {
    let connection_id = Uuid::new_v4();
    info!("WebSocket connection established: {}", connection_id);

    // Create a channel for this specific connection
    let (tx, mut rx) = broadcast::channel(100);

    // Add connection to state
    state.add_websocket_connection(connection_id, tx.clone());

    // Subscribe to global broadcast
    let mut global_rx = state.broadcast_tx.subscribe();

    // Split the socket
    let (mut sender, mut receiver) = socket.split();

    // Send connection established message
    let connect_msg = WebSocketMessage::Connected {
        connection_id: connection_id.to_string(),
    };

    if let Ok(msg_text) = serde_json::to_string(&connect_msg)
        && sender.send(Message::Text(msg_text.into())).await.is_err()
    {
        warn!("Failed to send connection message to {}", connection_id);
        state.remove_websocket_connection(&connection_id);
        return;
    }

    // Spawn task to handle incoming messages from client
    let state_clone = state.clone();
    let connection_id_clone = connection_id;
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!(
                        "Received WebSocket message from {}: {}",
                        connection_id_clone, text
                    );

                    // Handle ping/pong and subscription messages
                    if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                        match ws_msg {
                            WebSocketMessage::Ping => {
                                let pong = WebSocketMessage::Pong;
                                let _ = tx.send(pong);
                            }
                            WebSocketMessage::Subscribe {
                                memory_filter,
                                entity_filter,
                                relationship_filter,
                            } => {
                                // Store subscription filters for this connection
                                state_clone.set_websocket_subscription(
                                    connection_id_clone,
                                    memory_filter,
                                    entity_filter,
                                    relationship_filter,
                                );

                                // Send acknowledgment
                                let ack_msg = WebSocketMessage::SubscriptionAck {
                                    filters_applied: true,
                                    message: "Subscription filters updated successfully"
                                        .to_string(),
                                };

                                let _ = tx.send(ack_msg);
                            }
                            _ => {
                                // Handle other message types if needed
                                debug!(
                                    "Unhandled WebSocket message type from {}",
                                    connection_id_clone
                                );
                            }
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    debug!("Received binary message from {}", connection_id_clone);
                }
                Ok(Message::Close(_)) => {
                    info!(
                        "WebSocket connection closed by client: {}",
                        connection_id_clone
                    );
                    break;
                }
                Err(e) => {
                    error!("WebSocket error for {}: {}", connection_id_clone, e);
                    break;
                }
                _ => {}
            }
        }

        state_clone.remove_websocket_connection(&connection_id_clone);
    });

    // Spawn task to handle outgoing messages to client
    let outgoing_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Messages from global broadcast
                msg = global_rx.recv() => {
                    match msg {
                        Ok(ws_msg) => {
                            if let Ok(msg_text) = serde_json::to_string(&ws_msg)
                                && sender.send(Message::Text(msg_text.into())).await.is_err()
                            {
                                error!("Failed to send message to WebSocket {}", connection_id);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("Global broadcast channel closed");
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            warn!("WebSocket {} lagged behind broadcast", connection_id);
                            // Continue processing
                        }
                    }
                }

                // Messages from connection-specific channel
                msg = rx.recv() => {
                    match msg {
                        Ok(ws_msg) => {
                            if let Ok(msg_text) = serde_json::to_string(&ws_msg)
                                && sender.send(Message::Text(msg_text.into())).await.is_err()
                            {
                                error!("Failed to send message to WebSocket {}", connection_id);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("Connection-specific broadcast channel closed for {}", connection_id);
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            warn!("WebSocket {} lagged behind connection broadcast", connection_id);
                            // Continue processing
                        }
                    }
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = incoming_task => {
            debug!("Incoming task completed for {}", connection_id);
        }
        _ = outgoing_task => {
            debug!("Outgoing task completed for {}", connection_id);
        }
    }

    // Clean up
    state.remove_websocket_connection(&connection_id);
    info!("WebSocket connection closed: {}", connection_id);
}
