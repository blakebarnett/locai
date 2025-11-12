//! Message stream types and utilities

use crate::Result;
use crate::messaging::types::Message;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::broadcast;

/// Type alias for message streams
pub type MessageStream = Pin<Box<dyn Stream<Item = Result<Message>> + Send>>;

/// Create a MessageStream from a broadcast receiver (for remote messaging)
pub fn from_broadcast_receiver(receiver: broadcast::Receiver<Message>) -> MessageStream {
    use futures::stream;

    let stream = stream::unfold(receiver, |mut rx| async move {
        match rx.recv().await {
            Ok(message) => Some((Ok(message), rx)),
            Err(broadcast::error::RecvError::Closed) => None,
            Err(broadcast::error::RecvError::Lagged(_)) => {
                // Skip lagged messages and continue
                Some((
                    Err(crate::LocaiError::Other(
                        "Message stream lagged".to_string(),
                    )),
                    rx,
                ))
            }
        }
    });

    Box::pin(stream)
}

/// Stream adapter for converting memory events to message events
pub struct MessageStreamAdapter {
    inner: Pin<Box<dyn Stream<Item = Result<Message>> + Send>>,
}

impl MessageStreamAdapter {
    /// Create a new message stream adapter
    pub fn new(stream: Pin<Box<dyn Stream<Item = Result<Message>> + Send>>) -> Self {
        Self { inner: stream }
    }

    /// Get the inner stream
    pub fn into_inner(self) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send>> {
        self.inner
    }
}

impl Stream for MessageStreamAdapter {
    type Item = Result<Message>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Utilities for working with message streams
pub mod utils {
    use super::*;
    use crate::messaging::types::MessageFilter;
    use futures::{StreamExt, TryStreamExt};

    /// Filter a message stream using a MessageFilter
    pub fn filter_stream(stream: MessageStream, filter: MessageFilter) -> MessageStream {
        let filtered = stream.try_filter(move |message| {
            let filter = filter.clone();
            futures::future::ready(matches_filter(message, &filter))
        });

        Box::pin(filtered)
    }

    /// Check if a message matches a filter
    pub fn matches_filter(message: &Message, filter: &MessageFilter) -> bool {
        // Check exact topic matches
        if let Some(topics) = &filter.topics
            && !topics.contains(&message.topic)
        {
            return false;
        }

        // Check topic patterns
        if let Some(patterns) = &filter.topic_patterns
            && !patterns
                .iter()
                .any(|pattern| matches_pattern(pattern, &message.topic))
        {
            return false;
        }

        // Check senders
        if let Some(senders) = &filter.senders
            && !senders.contains(&message.sender)
        {
            return false;
        }

        // Check recipients
        if let Some(recipients) = &filter.recipients {
            if message.recipients.is_empty() {
                // Broadcast message - matches any recipient filter
            } else if !recipients.iter().any(|r| message.recipients.contains(r)) {
                return false;
            }
        }

        // Check time range
        if let Some((start, end)) = &filter.time_range
            && (message.timestamp < *start || message.timestamp > *end)
        {
            return false;
        }

        // Check importance range
        if let Some((min, max)) = &filter.importance_range {
            if let Some(importance) = message.importance {
                if importance < *min || importance > *max {
                    return false;
                }
            } else {
                // Message has no importance score - doesn't match importance filter
                return false;
            }
        }

        // Check tags (must have all)
        if let Some(tags) = &filter.tags
            && !tags.iter().all(|tag| message.has_tag(tag))
        {
            return false;
        }

        // Check tags (must have any)
        if let Some(tags_any) = &filter.tags_any
            && !tags_any.iter().any(|tag| message.has_tag(tag))
        {
            return false;
        }

        // Check headers
        if let Some(headers) = &filter.headers {
            for (key, value) in headers {
                if message.get_header(key) != Some(value) {
                    return false;
                }
            }
        }

        // Check expiration
        if !filter.include_expired && message.is_expired() {
            return false;
        }

        // Content query filtering would need semantic search integration
        // For now, we'll do a simple contains check
        if let Some(query) = &filter.content_query {
            let content_str = message.content.to_string().to_lowercase();
            let query_lower = query.to_lowercase();
            if !content_str.contains(&query_lower) {
                return false;
            }
        }

        true
    }

    /// Check if a topic matches a pattern (supports wildcards)
    pub fn matches_pattern(pattern: &str, topic: &str) -> bool {
        if pattern.contains('*') {
            // Simple wildcard matching - replace * with regex .*
            // Use simple string matching for now to avoid regex dependency
            // This is a simplified implementation
            if let Some(prefix) = pattern.strip_suffix('*') {
                return topic.starts_with(prefix);
            } else if let Some(suffix) = pattern.strip_prefix('*') {
                return topic.ends_with(suffix);
            } else {
                // For more complex patterns, we'd need proper regex
                // For now, fall back to exact match
                return pattern == topic;
            }
        }

        // Exact match fallback
        pattern == topic
    }

    /// Combine multiple message streams
    pub fn merge_streams(streams: Vec<MessageStream>) -> MessageStream {
        use futures::stream::SelectAll;

        let select_all = SelectAll::new();
        let merged = streams.into_iter().fold(select_all, |mut acc, stream| {
            acc.push(stream);
            acc
        });

        Box::pin(merged)
    }

    /// Take only the first N messages from a stream
    pub fn take(stream: MessageStream, n: usize) -> MessageStream {
        Box::pin(stream.take(n))
    }

    /// Skip the first N messages from a stream
    pub fn skip(stream: MessageStream, n: usize) -> MessageStream {
        Box::pin(stream.skip(n))
    }

    /// Map messages in a stream
    pub fn map_messages<F>(stream: MessageStream, f: F) -> MessageStream
    where
        F: Fn(Message) -> Message + Send + 'static,
    {
        let mapped = stream.map_ok(f);
        Box::pin(mapped)
    }

    /// Collect messages into a vector
    pub async fn collect_messages(stream: MessageStream) -> Result<Vec<Message>> {
        stream.try_collect().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::types::{Message, MessageFilter};
    use futures::stream;
    use serde_json::json;

    #[tokio::test]
    async fn test_filter_stream() {
        let messages = vec![
            Message::new("test.topic1".to_string(), "sender1".to_string(), json!({})),
            Message::new("test.topic2".to_string(), "sender2".to_string(), json!({})),
            Message::new("test.topic1".to_string(), "sender1".to_string(), json!({})),
        ];

        let stream = Box::pin(stream::iter(messages.into_iter().map(Ok))) as MessageStream;

        let filter = MessageFilter::new().topics(vec!["test.topic1".to_string()]);

        let filtered_stream = utils::filter_stream(stream, filter);
        let collected = utils::collect_messages(filtered_stream).await.unwrap();

        assert_eq!(collected.len(), 2);
        assert!(collected.iter().all(|m| m.topic == "test.topic1"));
    }

    #[test]
    fn test_matches_filter() {
        let message = Message::new("test.topic".to_string(), "sender1".to_string(), json!({}))
            .add_tag("important")
            .importance(0.8);

        let filter = MessageFilter::new()
            .topics(vec!["test.topic".to_string()])
            .tags(vec!["important".to_string()])
            .importance_range(0.5, 1.0);

        assert!(utils::matches_filter(&message, &filter));

        let filter2 = MessageFilter::new().topics(vec!["other.topic".to_string()]);

        assert!(!utils::matches_filter(&message, &filter2));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(utils::matches_pattern("test.*", "test.topic"));
        assert!(utils::matches_pattern("test.*", "test.another.topic"));
        assert!(!utils::matches_pattern("test.*", "other.topic"));
        assert!(utils::matches_pattern("exact", "exact"));
        assert!(!utils::matches_pattern("exact", "other"));

        // Test suffix patterns
        assert!(utils::matches_pattern("*.action", "character.action"));
        assert!(!utils::matches_pattern("*.action", "character.status"));
    }
}
