# Live Queries

## Overview

Locai provides real-time change notifications through SurrealDB's native LIVE SELECT feature. This enables clients to receive immediate updates when data changes occur, supporting reactive user interfaces and real-time synchronization.

## Architecture

### Core Components

**LiveQueryManager**
- Manages SurrealDB LIVE SELECT queries for each table
- Handles connection lifecycle and reconnection logic
- Broadcasts events to the event router

**EventRouter**
- Converts SurrealDB change events to WebSocket messages
- Applies subscription filters
- Routes events to connected clients

**WebSocket Handler**
- Manages client connections
- Handles subscription requests
- Delivers filtered events to clients

### Event Flow

```
Database Change → SurrealDB → LIVE SELECT → LiveQueryManager → EventRouter → WebSocket → Client
```

## Configuration

### Environment Variables

```bash
# Enable live queries (default: false)
LOCAI_ENABLE_LIVE_QUERIES=true

# Event buffer size (default: 1000)
LOCAI_LIVE_QUERY_BUFFER_SIZE=1000
```

### Server Configuration

```rust
let config = ServerConfig {
    enable_live_queries: true,
    live_query_buffer_size: 1000,
    ..Default::default()
};
```

## WebSocket Protocol

### Connection

Connect to the WebSocket endpoint:
```
ws://localhost:3000/api/ws
```

### Message Format

All messages follow the structure:
```json
{
  "type": "MessageType",
  "data": { ... }
}
```

### Message Types

#### Client Messages

**Subscribe**
```json
{
  "type": "Subscribe",
  "data": {
    "memory_filter": {
      "memory_type": "episodic",
      "importance_min": 0.5
    },
    "entity_filter": {
      "entity_type": "person"
    }
  }
}
```

**Ping**
```json
{
  "type": "Ping"
}
```

#### Server Messages

**Connection Established**
```json
{
  "type": "Connected",
  "data": {
    "connection_id": "uuid-string"
  }
}
```

**Memory Events**
```json
{
  "type": "MemoryCreated",
  "data": {
    "memory_id": "memory:123",
    "content": "Memory content",
    "memory_type": "episodic",
    "metadata": {},
    "importance": 0.8,
    "node_id": "server-uuid"
  }
}
```

Additional event types:
- `MemoryUpdated`
- `MemoryDeleted`
- `EntityCreated`
- `EntityUpdated`
- `EntityDeleted`
- `RelationshipCreated`
- `RelationshipUpdated`
- `RelationshipDeleted`

## Client Implementation

### JavaScript Example

```javascript
class LocaiLiveClient {
  constructor(url) {
    this.ws = new WebSocket(url);
    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      this.handleMessage(message);
    };
  }
  
  subscribe(filters) {
    this.ws.send(JSON.stringify({
      type: "Subscribe",
      data: filters
    }));
  }
  
  handleMessage(message) {
    switch (message.type) {
      case "MemoryCreated":
        // Handle new memory
        break;
      case "EntityUpdated":
        // Handle entity update
        break;
    }
  }
}
```

### Rust Example

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};

let (ws_stream, _) = connect_async("ws://localhost:3000/api/ws").await?;
let (mut write, mut read) = ws_stream.split();

// Subscribe to events
let subscription = serde_json::json!({
    "type": "Subscribe",
    "data": {
        "memory_filter": {
            "memory_type": "episodic"
        }
    }
});

write.send(Message::Text(subscription.to_string())).await?;

// Process events
while let Some(msg) = read.next().await {
    if let Message::Text(text) = msg? {
        let event: serde_json::Value = serde_json::from_str(&text)?;
        // Handle event
    }
}
```

## Subscription Filters

### Memory Filters
- `memory_type`: Filter by memory type
- `importance_min`/`importance_max`: Filter by importance range
- `content_contains`: Text search filter

### Entity Filters
- `entity_type`: Filter by entity type
- `properties_contains`: Search entity properties

### Relationship Filters
- `relationship_type`: Filter by relationship type
- `source_id`: Filter by source entity
- `target_id`: Filter by target entity

## Multi-Instance Support

Each server instance includes a unique `node_id` in events. This enables:
- Client-side deduplication
- Load balancer awareness
- Failover handling

## Performance Considerations

### Buffer Management
- Configure buffer sizes based on expected event volume
- Monitor buffer overflow metrics
- Implement client-side rate limiting

### Connection Scaling
- Use connection pooling for multiple subscriptions
- Implement reconnection with exponential backoff
- Monitor WebSocket connection limits

### Filter Optimization
- Use specific filters to reduce event volume
- Avoid complex nested filters
- Consider client-side filtering for edge cases

## Error Handling

### Connection Errors
- Automatic reconnection with exponential backoff
- Connection state notifications
- Graceful degradation

### Subscription Errors
```json
{
  "type": "Error",
  "data": {
    "code": "INVALID_FILTER",
    "message": "Invalid filter specification"
  }
}
```

## Security

### Current Limitations (Alpha)
- No authentication on WebSocket connections
- All events visible to all clients
- No rate limiting

### Future Security Features
- JWT-based WebSocket authentication
- Event filtering based on permissions
- Rate limiting per connection
- Encrypted WebSocket connections (WSS)

## Monitoring

### Key Metrics
- Active WebSocket connections
- Events per second by type
- Buffer utilization
- Connection duration

### Debug Logging
```bash
RUST_LOG=locai_server::live_query=debug
```

## Limitations

- Maximum buffer size constraints
- No event persistence or replay
- Basic filtering capabilities only
- No event aggregation or batching

## Future Enhancements

1. **Event Replay**: Query historical events from a timestamp
2. **Advanced Filtering**: Complex filter expressions and combinations
3. **Event Batching**: Aggregate multiple events for efficiency
4. **Compression**: WebSocket compression for high-volume scenarios
5. **Metrics Integration**: Built-in Prometheus metrics 