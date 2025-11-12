//! WebSocket client for remote messaging

use crate::{LocaiError, Result};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{RwLock, broadcast, mpsc},
    time::{interval, timeout},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message as WsMessage,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::types::{Message, MessageFilter, MessageId};

/// WebSocket message types for communication with locai-server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    /// Authentication request
    Authenticate {
        app_id: String,
        token: Option<String>,
    },

    /// Authentication response
    AuthenticationResponse {
        success: bool,
        message: String,
        connection_id: String,
    },

    /// Send a message
    SendMessage {
        namespace: String,
        topic: String,
        content: serde_json::Value,
        headers: Option<HashMap<String, String>>,
        correlation_id: Option<String>,
    },

    /// Message sent confirmation
    MessageSent {
        message_id: String,
        correlation_id: Option<String>,
    },

    /// Subscribe to messages
    Subscribe {
        filter: MessageFilter,
        subscription_id: String,
    },

    /// Subscription confirmation
    SubscriptionConfirmed {
        subscription_id: String,
        message: String,
    },

    /// Incoming message from subscription
    IncomingMessage {
        message: Message,
        subscription_id: String,
    },

    /// Unsubscribe from messages
    Unsubscribe { subscription_id: String },

    /// Cross-app message
    CrossAppMessage {
        source_app: String,
        target_app: String,
        topic: String,
        content: serde_json::Value,
        headers: Option<HashMap<String, String>>,
        correlation_id: Option<String>,
    },

    /// Get message history
    GetMessageHistory {
        filter: Option<MessageFilter>,
        limit: Option<usize>,
        correlation_id: String,
    },

    /// Message history response
    MessageHistoryResponse {
        messages: Vec<Message>,
        correlation_id: String,
    },

    /// Ping for keepalive
    Ping,

    /// Pong response
    Pong,

    /// Error message
    Error {
        message: String,
        code: Option<String>,
        correlation_id: Option<String>,
    },
}

/// Subscription information
#[derive(Debug)]
struct SubscriptionInfo {
    #[allow(dead_code)]
    filter: MessageFilter,
    sender: broadcast::Sender<Message>,
}

/// WebSocket client for remote messaging
#[derive(Debug)]
pub struct WebSocketClient {
    #[allow(dead_code)]
    connection_id: Option<String>,
    sender: mpsc::Sender<ServerMessage>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
    response_handlers: Arc<RwLock<HashMap<String, mpsc::Sender<ServerMessage>>>>,
}

impl WebSocketClient {
    /// Connect to locai-server WebSocket endpoint
    pub async fn connect(server_url: &str) -> Result<Self> {
        let ws_url = if server_url.starts_with("ws://") || server_url.starts_with("wss://") {
            server_url.to_string()
        } else {
            format!("ws://{}/api/ws", server_url)
        };

        info!("Connecting to locai-server at: {}", ws_url);

        let (ws_stream, _) = connect_async(&ws_url).await.map_err(|e| {
            LocaiError::Connection(format!("Failed to connect to WebSocket: {}", e))
        })?;

        let (write, read) = ws_stream.split();
        let (sender, receiver) = mpsc::channel(100);

        let subscriptions = Arc::new(RwLock::new(HashMap::new()));
        let response_handlers = Arc::new(RwLock::new(HashMap::new()));

        let client = Self {
            connection_id: None,
            sender,
            subscriptions: subscriptions.clone(),
            response_handlers: response_handlers.clone(),
        };

        // Spawn message handling tasks
        tokio::spawn(Self::writer_task(write, receiver));
        tokio::spawn(Self::reader_task(read, subscriptions, response_handlers));

        // Start keepalive task
        let sender_clone = client.sender.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if sender_clone.send(ServerMessage::Ping).await.is_err() {
                    break;
                }
            }
        });

        Ok(client)
    }

    /// Authenticate with locai-server
    pub async fn authenticate(&self, app_id: &str) -> Result<()> {
        let correlation_id = Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel(1);

        // Register response handler
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.insert(correlation_id.clone(), tx);
        }

        // Send authentication message
        let auth_msg = ServerMessage::Authenticate {
            app_id: app_id.to_string(),
            token: None, // TODO: Support authentication tokens
        };

        self.sender
            .send(auth_msg)
            .await
            .map_err(|e| LocaiError::Connection(format!("Failed to send auth message: {}", e)))?;

        // Wait for response
        match timeout(Duration::from_secs(10), rx.recv()).await {
            Ok(Some(ServerMessage::AuthenticationResponse {
                success,
                message,
                connection_id,
            })) => {
                if success {
                    info!(
                        "Successfully authenticated with connection ID: {}",
                        connection_id
                    );
                    Ok(())
                } else {
                    Err(LocaiError::Authentication(format!(
                        "Authentication failed: {}",
                        message
                    )))
                }
            }
            Ok(Some(msg)) => Err(LocaiError::Protocol(format!(
                "Unexpected auth response: {:?}",
                msg
            ))),
            Ok(None) => Err(LocaiError::Connection(
                "Auth response channel closed".to_string(),
            )),
            Err(_) => Err(LocaiError::Timeout("Authentication timeout".to_string())),
        }
    }

    /// Send a message to locai-server
    pub async fn send_message(
        &self,
        namespace: &str,
        topic: &str,
        content: serde_json::Value,
        headers: Option<HashMap<String, String>>,
    ) -> Result<MessageId> {
        let correlation_id = Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel(1);

        // Register response handler
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.insert(correlation_id.clone(), tx);
        }

        // Send message
        let msg = ServerMessage::SendMessage {
            namespace: namespace.to_string(),
            topic: topic.to_string(),
            content,
            headers,
            correlation_id: Some(correlation_id.clone()),
        };

        self.sender
            .send(msg)
            .await
            .map_err(|e| LocaiError::Connection(format!("Failed to send message: {}", e)))?;

        // Wait for confirmation
        match timeout(Duration::from_secs(10), rx.recv()).await {
            Ok(Some(ServerMessage::MessageSent { message_id, .. })) => {
                Ok(MessageId::from_string(message_id))
            }
            Ok(Some(ServerMessage::Error { message, .. })) => {
                Err(LocaiError::Other(format!("Server error: {}", message)))
            }
            Ok(Some(msg)) => Err(LocaiError::Protocol(format!(
                "Unexpected send response: {:?}",
                msg
            ))),
            Ok(None) => Err(LocaiError::Connection(
                "Send response channel closed".to_string(),
            )),
            Err(_) => Err(LocaiError::Timeout("Send timeout".to_string())),
        }
    }

    /// Subscribe to messages with a filter
    pub async fn subscribe(&self, filter: MessageFilter) -> Result<broadcast::Receiver<Message>> {
        let subscription_id = Uuid::new_v4().to_string();
        let (broadcast_tx, broadcast_rx) = broadcast::channel(100);

        // Store subscription
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(
                subscription_id.clone(),
                SubscriptionInfo {
                    filter: filter.clone(),
                    sender: broadcast_tx,
                },
            );
        }

        // Send subscribe message
        let msg = ServerMessage::Subscribe {
            filter,
            subscription_id: subscription_id.clone(),
        };

        self.sender
            .send(msg)
            .await
            .map_err(|e| LocaiError::Connection(format!("Failed to send subscribe: {}", e)))?;

        Ok(broadcast_rx)
    }

    /// Get message history
    pub async fn get_message_history(
        &self,
        filter: Option<MessageFilter>,
        limit: Option<usize>,
    ) -> Result<Vec<Message>> {
        let correlation_id = Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel(1);

        // Register response handler
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.insert(correlation_id.clone(), tx);
        }

        // Send request
        let msg = ServerMessage::GetMessageHistory {
            filter,
            limit,
            correlation_id: correlation_id.clone(),
        };

        self.sender.send(msg).await.map_err(|e| {
            LocaiError::Connection(format!("Failed to send history request: {}", e))
        })?;

        // Wait for response
        match timeout(Duration::from_secs(30), rx.recv()).await {
            Ok(Some(ServerMessage::MessageHistoryResponse { messages, .. })) => Ok(messages),
            Ok(Some(ServerMessage::Error { message, .. })) => {
                Err(LocaiError::Other(format!("Server error: {}", message)))
            }
            Ok(Some(msg)) => Err(LocaiError::Protocol(format!(
                "Unexpected history response: {:?}",
                msg
            ))),
            Ok(None) => Err(LocaiError::Connection(
                "History response channel closed".to_string(),
            )),
            Err(_) => Err(LocaiError::Timeout("History request timeout".to_string())),
        }
    }

    /// Writer task to handle outgoing messages
    async fn writer_task(
        mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>,
        mut receiver: mpsc::Receiver<ServerMessage>,
    ) {
        while let Some(msg) = receiver.recv().await {
            let json_msg = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };

            if let Err(e) = write.send(WsMessage::Text(json_msg.into())).await {
                error!("Failed to send WebSocket message: {}", e);
                break;
            }
        }
    }

    /// Reader task to handle incoming messages
    async fn reader_task(
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
        response_handlers: Arc<RwLock<HashMap<String, mpsc::Sender<ServerMessage>>>>,
    ) {
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(WsMessage::Text(text)) => {
                    debug!("Received WebSocket message: {}", text);

                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(server_msg) => {
                            Self::handle_server_message(
                                server_msg,
                                &subscriptions,
                                &response_handlers,
                            )
                            .await;
                        }
                        Err(e) => {
                            error!("Failed to parse server message: {}", e);
                        }
                    }
                }
                Ok(WsMessage::Pong(_)) => {
                    debug!("Received pong");
                }
                Ok(WsMessage::Close(_)) => {
                    info!("WebSocket connection closed by server");
                    break;
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        warn!("WebSocket reader task ended");
    }

    /// Handle incoming server messages
    async fn handle_server_message(
        msg: ServerMessage,
        subscriptions: &Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
        response_handlers: &Arc<RwLock<HashMap<String, mpsc::Sender<ServerMessage>>>>,
    ) {
        match &msg {
            ServerMessage::IncomingMessage {
                message,
                subscription_id,
            } => {
                let subs = subscriptions.read().await;
                if let Some(sub_info) = subs.get(subscription_id)
                    && let Err(e) = sub_info.sender.send(message.clone())
                {
                    debug!(
                        "Failed to broadcast message to subscription {}: {}",
                        subscription_id, e
                    );
                }
            }

            ServerMessage::AuthenticationResponse { .. }
            | ServerMessage::MessageSent { .. }
            | ServerMessage::MessageHistoryResponse { .. }
            | ServerMessage::Error { .. } => {
                // Handle response messages
                let correlation_id = match &msg {
                    ServerMessage::MessageSent { correlation_id, .. } => correlation_id.as_ref(),
                    ServerMessage::MessageHistoryResponse { correlation_id, .. } => {
                        Some(correlation_id)
                    }
                    ServerMessage::Error { correlation_id, .. } => correlation_id.as_ref(),
                    _ => None,
                };

                if let Some(corr_id) = correlation_id {
                    let mut handlers = response_handlers.write().await;
                    if let Some(sender) = handlers.remove(corr_id)
                        && let Err(e) = sender.send(msg).await
                    {
                        debug!("Failed to send response to handler: {}", e);
                    }
                }
            }

            ServerMessage::Pong => {
                debug!("Received pong from server");
            }

            _ => {
                debug!("Unhandled server message: {:?}", msg);
            }
        }
    }
}
