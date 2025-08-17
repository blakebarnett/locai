//! Remote messaging implementation using WebSocket connections to locai-server

use crate::Result;
use crate::Result as LocaiResult;
use crate::messaging::{
    stream::MessageStream,
    types::{Message, MessageFilter, MessageId},
};
use std::{collections::HashMap, sync::Arc};

use super::{stream::from_broadcast_receiver, websocket::WebSocketClient};

/// Remote messaging interface (exported for compatibility)
pub struct RemoteMessaging;

/// Send a message via WebSocket to locai-server
pub async fn send_message(
    client: &Arc<WebSocketClient>,
    namespace: &str,
    _app_id: &str,
    topic: &str,
    content: serde_json::Value,
) -> Result<MessageId> {
    let full_topic = format!("{}.{}", namespace, topic);

    client.send_message(&full_topic, topic, content, None).await
}

/// Send a complete message with all options via WebSocket
pub async fn send_complete_message(
    client: &Arc<WebSocketClient>,
    message: Message,
) -> Result<MessageId> {
    let headers = if message.headers.is_empty() {
        None
    } else {
        Some(message.headers.clone())
    };

    client
        .send_message(&message.topic, &message.topic, message.content, headers)
        .await
}

/// Send a cross-app message via WebSocket
pub async fn send_cross_app_message(
    client: &Arc<WebSocketClient>,
    source_app: &str,
    target_app: &str,
    topic: &str,
    content: serde_json::Value,
) -> Result<MessageId> {
    let cross_app_topic = format!("app:{}:{}", target_app, topic);

    let mut headers = HashMap::new();
    headers.insert("source_app".to_string(), source_app.to_string());
    headers.insert("target_app".to_string(), target_app.to_string());
    headers.insert("cross_app".to_string(), "true".to_string());

    client
        .send_message(&cross_app_topic, topic, content, Some(headers))
        .await
}

/// Subscribe to filtered messages via WebSocket
pub async fn subscribe_filtered(
    client: &Arc<WebSocketClient>,
    filter: MessageFilter,
) -> Result<MessageStream> {
    let receiver = client.subscribe(filter).await?;
    Ok(from_broadcast_receiver(receiver))
}

/// Get message history via WebSocket
pub async fn get_message_history(
    client: &Arc<WebSocketClient>,
    filter: Option<MessageFilter>,
    limit: Option<usize>,
) -> Result<Vec<Message>> {
    client.get_message_history(filter, limit).await
}

pub async fn connect_to_remote_messaging(
    _app_id: &str,
    _config: serde_json::Value,
) -> LocaiResult<MessageStream> {
    todo!("Remote messaging not yet implemented")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[tokio::test]
    async fn test_remote_messaging() {
        let _content = json!({"action": "attack", "target": "orc"});
        let _source_app = "dnd_campaign";
        // ... rest of the test ...
    }
}
