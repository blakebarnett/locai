//! WebSocket handlers for messaging protocol

use super::MessagingServer;
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use locai::messaging::types::Message;
use locai::messaging::websocket::ServerMessage;
use serde_json;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Handle messaging WebSocket connections
pub async fn handle_messaging_websocket(socket: WebSocket, messaging_server: Arc<MessagingServer>) {
    let connection_id = Uuid::new_v4().to_string();
    info!("New messaging WebSocket connection: {}", connection_id);

    let (mut sender, mut receiver) = socket.split();

    // Connection state
    let mut authenticated = false;
    let mut app_id: Option<String> = None;
    let mut subscriptions: HashMap<String, broadcast::Receiver<locai::messaging::types::Message>> =
        HashMap::new();

    // Send connection established message
    let connect_msg = ServerMessage::AuthenticationResponse {
        success: false,
        message: "Please authenticate".to_string(),
        connection_id: connection_id.clone(),
    };

    if let Ok(msg_text) = serde_json::to_string(&connect_msg) {
        if sender.send(WsMessage::Text(msg_text.into())).await.is_err() {
            warn!("Failed to send connection message to {}", connection_id);
            return;
        }
    }

    // Message handling loop
    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg_result = receiver.next() => {
                match msg_result {
                    Some(Ok(WsMessage::Text(text))) => {
                        debug!("Received message from {}: {}", connection_id, text);

                        match serde_json::from_str::<ServerMessage>(&text) {
                            Ok(server_msg) => {
                                let response = handle_server_message(
                                    server_msg,
                                    &messaging_server,
                                    &connection_id,
                                    &mut authenticated,
                                    &mut app_id,
                                    &mut subscriptions,
                                ).await;

                                if let Some(response_msg) = response {
                                    if let Ok(response_text) = serde_json::to_string(&response_msg) {
                                        if sender.send(WsMessage::Text(response_text.into())).await.is_err() {
                                            error!("Failed to send response to {}", connection_id);
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse message from {}: {}", connection_id, e);
                                let error_msg = ServerMessage::Error {
                                    message: format!("Invalid message format: {}", e),
                                    code: Some("PARSE_ERROR".to_string()),
                                    correlation_id: None,
                                };

                                if let Ok(error_text) = serde_json::to_string(&error_msg) {
                                    let _ = sender.send(WsMessage::Text(error_text.into())).await;
                                }
                            }
                        }
                    }
                    Some(Ok(WsMessage::Ping(data))) => {
                        if sender.send(WsMessage::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        info!("WebSocket connection {} closed by client", connection_id);
                        break;
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error for {}: {}", connection_id, e);
                        break;
                    }
                    None => {
                        info!("WebSocket connection {} ended", connection_id);
                        break;
                    }
                }
            }

            // Handle messages from subscriptions
            subscription_result = receive_from_subscriptions(&mut subscriptions) => {
                match subscription_result {
                    Some((subscription_id, message)) => {
                        let incoming_msg = ServerMessage::IncomingMessage {
                            message,
                            subscription_id,
                        };

                        if let Ok(msg_text) = serde_json::to_string(&incoming_msg) {
                            if sender.send(WsMessage::Text(msg_text.into())).await.is_err() {
                                error!("Failed to forward subscription message to {}", connection_id);
                                break;
                            }
                        }
                    }
                    None => {
                        // No subscriptions or all closed
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
            }
        }
    }

    // Cleanup
    debug!("Cleaning up connection {}", connection_id);

    if let Some(_app_id) = app_id {
        if let Err(e) = messaging_server.remove_connection(&connection_id).await {
            error!("Failed to remove connection {}: {}", connection_id, e);
        }
    }

    info!("WebSocket connection {} cleanup complete", connection_id);
}

/// Handle individual server messages
async fn handle_server_message(
    msg: ServerMessage,
    messaging_server: &Arc<MessagingServer>,
    connection_id: &str,
    authenticated: &mut bool,
    app_id: &mut Option<String>,
    subscriptions: &mut HashMap<String, broadcast::Receiver<locai::messaging::types::Message>>,
) -> Option<ServerMessage> {
    match msg {
        ServerMessage::Authenticate {
            app_id: auth_app_id,
            token: _,
        } => {
            // Register connection
            if let Err(e) = messaging_server
                .register_connection(connection_id.to_string(), auth_app_id.clone())
                .await
            {
                error!("Failed to register connection: {}", e);
                return Some(ServerMessage::AuthenticationResponse {
                    success: false,
                    message: format!("Registration failed: {}", e),
                    connection_id: connection_id.to_string(),
                });
            }

            // Authenticate
            match messaging_server
                .authenticate_connection(connection_id, &auth_app_id)
                .await
            {
                Ok(success) => {
                    if success {
                        *authenticated = true;
                        *app_id = Some(auth_app_id.clone());

                        Some(ServerMessage::AuthenticationResponse {
                            success: true,
                            message: "Authentication successful".to_string(),
                            connection_id: connection_id.to_string(),
                        })
                    } else {
                        Some(ServerMessage::AuthenticationResponse {
                            success: false,
                            message: "Authentication failed".to_string(),
                            connection_id: connection_id.to_string(),
                        })
                    }
                }
                Err(e) => {
                    error!("Authentication error: {}", e);
                    Some(ServerMessage::AuthenticationResponse {
                        success: false,
                        message: format!("Authentication error: {}", e),
                        connection_id: connection_id.to_string(),
                    })
                }
            }
        }

        ServerMessage::SendMessage {
            namespace,
            topic,
            content,
            headers,
            correlation_id,
        } => {
            if !*authenticated {
                return Some(ServerMessage::Error {
                    message: "Not authenticated".to_string(),
                    code: Some("AUTH_REQUIRED".to_string()),
                    correlation_id,
                });
            }

            let sender_app = app_id.as_ref().unwrap();
            let full_topic = format!("{}.{}", namespace, topic);

            match messaging_server
                .send_message(sender_app, &full_topic, content, headers)
                .await
            {
                Ok(message_id) => Some(ServerMessage::MessageSent {
                    message_id: message_id.as_str().to_string(),
                    correlation_id,
                }),
                Err(e) => {
                    error!("Failed to send message: {}", e);
                    Some(ServerMessage::Error {
                        message: format!("Failed to send message: {}", e),
                        code: Some("SEND_ERROR".to_string()),
                        correlation_id,
                    })
                }
            }
        }

        ServerMessage::Subscribe {
            filter,
            subscription_id,
        } => {
            if !*authenticated {
                return Some(ServerMessage::Error {
                    message: "Not authenticated".to_string(),
                    code: Some("AUTH_REQUIRED".to_string()),
                    correlation_id: None,
                });
            }

            let sender_app = app_id.as_ref().unwrap();

            match messaging_server.subscribe(sender_app, filter).await {
                Ok((_sub_id, receiver)) => {
                    subscriptions.insert(subscription_id.clone(), receiver);

                    // TODO: Implement proper message forwarding to WebSocket
                    // The receiver is stored in subscriptions HashMap and should be
                    // processed in the main message loop to forward messages to the client

                    Some(ServerMessage::SubscriptionConfirmed {
                        subscription_id,
                        message: "Subscription created successfully".to_string(),
                    })
                }
                Err(e) => {
                    error!("Failed to create subscription: {}", e);
                    Some(ServerMessage::Error {
                        message: format!("Failed to create subscription: {}", e),
                        code: Some("SUBSCRIBE_ERROR".to_string()),
                        correlation_id: None,
                    })
                }
            }
        }

        ServerMessage::Unsubscribe { subscription_id } => {
            if !*authenticated {
                return Some(ServerMessage::Error {
                    message: "Not authenticated".to_string(),
                    code: Some("AUTH_REQUIRED".to_string()),
                    correlation_id: None,
                });
            }

            subscriptions.remove(&subscription_id);

            if let Err(e) = messaging_server.unsubscribe(&subscription_id).await {
                error!("Failed to unsubscribe: {}", e);
            }

            None // No response needed for unsubscribe
        }

        ServerMessage::GetMessageHistory {
            filter,
            limit,
            correlation_id,
        } => {
            if !*authenticated {
                return Some(ServerMessage::Error {
                    message: "Not authenticated".to_string(),
                    code: Some("AUTH_REQUIRED".to_string()),
                    correlation_id: Some(correlation_id),
                });
            }

            match messaging_server.get_message_history(filter, limit).await {
                Ok(messages) => Some(ServerMessage::MessageHistoryResponse {
                    messages,
                    correlation_id,
                }),
                Err(e) => {
                    error!("Failed to get message history: {}", e);
                    Some(ServerMessage::Error {
                        message: format!("Failed to get message history: {}", e),
                        code: Some("HISTORY_ERROR".to_string()),
                        correlation_id: Some(correlation_id),
                    })
                }
            }
        }

        ServerMessage::CrossAppMessage {
            source_app,
            target_app,
            topic,
            content,
            headers,
            correlation_id,
        } => {
            if !*authenticated {
                return Some(ServerMessage::Error {
                    message: "Not authenticated".to_string(),
                    code: Some("AUTH_REQUIRED".to_string()),
                    correlation_id,
                });
            }

            // Verify the source app matches the authenticated app
            let sender_app = app_id.as_ref().unwrap();
            if &source_app != sender_app {
                return Some(ServerMessage::Error {
                    message: "Source app mismatch".to_string(),
                    code: Some("AUTH_ERROR".to_string()),
                    correlation_id,
                });
            }

            let cross_app_topic = format!("app:{}:{}", target_app, topic);

            match messaging_server
                .send_message(sender_app, &cross_app_topic, content, headers)
                .await
            {
                Ok(message_id) => Some(ServerMessage::MessageSent {
                    message_id: message_id.as_str().to_string(),
                    correlation_id,
                }),
                Err(e) => {
                    error!("Failed to send cross-app message: {}", e);
                    Some(ServerMessage::Error {
                        message: format!("Failed to send cross-app message: {}", e),
                        code: Some("CROSS_APP_ERROR".to_string()),
                        correlation_id,
                    })
                }
            }
        }

        ServerMessage::Ping => Some(ServerMessage::Pong),

        _ => {
            debug!("Unhandled message type");
            None
        }
    }
}

/// Helper function to receive messages from all active subscriptions
async fn receive_from_subscriptions(
    subscriptions: &mut HashMap<String, broadcast::Receiver<Message>>,
) -> Option<(String, Message)> {
    if subscriptions.is_empty() {
        return None;
    }

    // Try to receive from each subscription
    let mut to_remove = Vec::new();

    for (subscription_id, receiver) in subscriptions.iter_mut() {
        match receiver.try_recv() {
            Ok(message) => {
                return Some((subscription_id.clone(), message));
            }
            Err(broadcast::error::TryRecvError::Empty) => {
                // No message available, continue
                continue;
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                // Subscription closed, mark for removal
                to_remove.push(subscription_id.clone());
            }
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                // Subscription lagged, but still active
                warn!("Subscription {} lagged behind", subscription_id);
                continue;
            }
        }
    }

    // Remove closed subscriptions
    for subscription_id in to_remove {
        subscriptions.remove(&subscription_id);
        debug!("Removed closed subscription: {}", subscription_id);
    }

    None
}
