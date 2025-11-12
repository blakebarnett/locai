# API Reference

## Overview

The Locai API provides RESTful HTTP endpoints for memory management, entity extraction, and search operations. The API is implemented in the `locai-server` crate and runs on port 3000 by default.

## Base URL

```
http://localhost:3000
```

## Interactive Documentation

Swagger UI documentation is available at `/docs` when the server is running.

## Endpoints

### Health Check

```
GET /api/health
GET /api/v1/health
```

Returns server health status and capabilities.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.2.1"
}
```

### Memory Operations

#### Create Memory

```
POST /api/v1/memories
```

**Request Body:**
```json
{
  "content": "The capital of France is Paris",
  "memory_type": "fact",
  "tags": ["geography", "france"],
  "source": "user_input"
}
```

**Response:**
```json
{
  "id": "memory:xyz123",
  "content": "The capital of France is Paris",
  "memory_type": "fact",
  "created_at": "2024-01-01T00:00:00Z",
  "embedding": null,
  "tags": ["geography", "france"]
}
```

#### Get Memory

```
GET /api/v1/memories/{id}
```

Retrieve a specific memory by ID.

#### Update Memory

```
PUT /api/v1/memories/{id}
```

Update an existing memory. Request body same as create.

#### Delete Memory

```
DELETE /api/v1/memories/{id}
```

Remove a memory from storage.

#### List Memories

```
GET /api/v1/memories
```

List memories with filtering and pagination.

**Query Parameters:**
- `page` (optional): Page number (0-based, default: 0)
- `size` (optional): Items per page (default: 20)
- `memory_type` (optional): Filter by memory type
- `priority` (optional): Filter by priority (Low, Normal, High, Critical)
- `tags` (optional): Comma-separated tags
- `source` (optional): Filter by source

#### Search Memories

```
GET /api/v1/memories/search?q={query}&limit={limit}
```

Search across all memories using BM25 full-text search with optional enhanced scoring.

**Query Parameters:**
- `q` (required): Search query
- `limit` (optional): Maximum results (default: 20)
- `memory_type` (optional): Filter by memory type
- `tags` (optional): Comma-separated tags
- `priority` (optional): Filter by priority
- `created_after` (optional): ISO 8601 timestamp - filter memories created after this time
- `created_before` (optional): ISO 8601 timestamp - filter memories created before this time
- `scoring` (optional): JSON-encoded scoring configuration for enhanced search (see [Enhanced Search Documentation](guides/ENHANCED_SEARCH.md))

**Example with temporal filtering:**
```bash
curl "http://localhost:3000/api/v1/memories/search?q=battle&created_after=2025-11-01T00:00:00Z&created_before=2025-11-01T23:59:59Z"
```

**Example with enhanced scoring:**
```bash
curl "http://localhost:3000/api/v1/memories/search?q=wizard&scoring=%7B%22recency_boost%22%3A2.0%2C%22decay_function%22%3A%22exponential%22%7D"
```

#### Get Memory Relationships

```
GET /api/v1/memories/{id}/relationships
```

Get all relationships for a specific memory.

**Query Parameters:**
- `direction` (optional): `outgoing`, `incoming`, or `both` (default: `both`)
- `relationship_type` (optional): Filter by relationship type

#### Create Memory Relationship

```
POST /api/v1/memories/{id}/relationships
```

Create a relationship from a memory to another memory or entity.

### Entity Operations

#### List Entities

```
GET /api/v1/entities
```

List all extracted entities with pagination.

**Query Parameters:**
- `page` (optional): Page number (0-based)
- `size` (optional): Items per page

#### Get Entity

```
GET /api/v1/entities/{id}
```

Retrieve a specific entity by ID.

#### Create Entity

```
POST /api/v1/entities
```

Create a new entity.

#### Update Entity

```
PUT /api/v1/entities/{id}
```

Update an existing entity.

#### Delete Entity

```
DELETE /api/v1/entities/{id}
```

Delete an entity.

#### Get Entity Memories

```
GET /api/v1/entities/{id}/memories
```

Get all memories that reference this entity.

#### Get Entity Relationships

```
GET /api/v1/entities/{id}/relationships
```

Get all relationships for a specific entity.

**Query Parameters:**
- `direction` (optional): `outgoing`, `incoming`, or `both` (default: `both`)
- `relationship_type` (optional): Filter by relationship type

#### Create Entity Relationship

```
POST /api/v1/entities/{id}/relationships
```

Create a relationship from an entity to another entity or memory.

### Relationship Operations

#### List Relationships

```
GET /api/v1/relationships
```

List all relationships with optional filtering.

#### Get Relationship

```
GET /api/v1/relationships/{id}
```

Retrieve a specific relationship by ID.

#### Create Relationship

```
POST /api/v1/relationships
```

Create a new relationship between any two nodes (memory or entity).

#### Update Relationship

```
PUT /api/v1/relationships/{id}
```

Update an existing relationship.

#### Delete Relationship

```
DELETE /api/v1/relationships/{id}
```

Delete a relationship.

#### Find Related Entities

```
GET /api/v1/relationships/{id}/related
```

Find entities related to the source or target of a relationship.

### Relationship Type Operations

#### List Relationship Types

```
GET /api/v1/relationship-types
```

List all registered relationship types.

#### Get Relationship Type

```
GET /api/v1/relationship-types/{name}
```

Get details for a specific relationship type.

#### Register Relationship Type

```
POST /api/v1/relationship-types
```

Register a new custom relationship type.

#### Update Relationship Type

```
PUT /api/v1/relationship-types/{name}
```

Update an existing relationship type definition.

#### Delete Relationship Type

```
DELETE /api/v1/relationship-types/{name}
```

Delete a relationship type.

#### Get Relationship Metrics

```
GET /api/v1/relationship-types/metrics
```

Get usage metrics for relationship types.

#### Seed Common Types

```
POST /api/v1/relationship-types/seed
```

Seed common relationship types (friendship, rivalry, etc.).

### Graph Operations

#### Get Memory Graph

```
GET /api/v1/memories/{id}/graph
```

Retrieve the relationship graph for a specific memory.

**Query Parameters:**
- `depth` (optional): Graph traversal depth (default: 1)
- `include_temporal_span` (optional): Include temporal span analysis (default: false)

#### Get Entity Graph

```
GET /api/v1/entities/{id}/graph
```

Retrieve the relationship graph for a specific entity.

**Query Parameters:**
- `depth` (optional): Graph traversal depth (default: 1)
- `include_temporal_span` (optional): Include temporal span analysis (default: false)

#### Find Paths

```
GET /api/v1/graph/paths
```

Find paths between two nodes in the graph.

#### Query Graph

```
POST /api/v1/graph/query
```

Execute a custom graph query.

#### Get Graph Metrics

```
GET /api/v1/graph/metrics
```

Get overall graph statistics and metrics.

### Batch Operations

#### Execute Batch

```
POST /api/v1/batch
```

Execute multiple operations in a single request. See [Batch Operations Documentation](guides/BATCH_OPERATIONS.md) for details.

### Webhook Operations

#### List Webhooks

```
GET /api/v1/webhooks
```

List all registered webhooks.

#### Get Webhook

```
GET /api/v1/webhooks/{id}
```

Get details for a specific webhook.

#### Create Webhook

```
POST /api/v1/webhooks
```

Register a new webhook for memory lifecycle events.

**Request Body:**
```json
{
  "event": "memory.created",
  "url": "https://example.com/webhooks/memory-events",
  "headers": {
    "Authorization": "Bearer token123"
  }
}
```

**Supported Events:**
- `memory.created`
- `memory.updated`
- `memory.accessed`
- `memory.deleted`

#### Update Webhook

```
PUT /api/v1/webhooks/{id}
```

Update an existing webhook configuration.

#### Delete Webhook

```
DELETE /api/v1/webhooks/{id}
```

Delete a webhook.

### Version Operations

#### List Versions

```
GET /api/v1/versions
```

List all version snapshots of the knowledge graph.

#### Create Version

```
POST /api/v1/versions
```

Create a new version snapshot.

#### Checkout Version

```
PUT /api/v1/versions/{id}/checkout
```

Restore the graph to a specific version.

### Authentication Operations

#### Sign Up

```
POST /api/v1/auth/signup
```

Register a new user account.

#### Login

```
POST /api/v1/auth/login
```

Authenticate and receive an access token.

#### List Users

```
GET /api/v1/auth/users
```

List all users (admin only).

#### Get User

```
GET /api/v1/auth/users/{id}
```

Get user details.

#### Update User

```
PUT /api/v1/auth/users/{id}
```

Update user information.

#### Delete User

```
DELETE /api/v1/auth/users/{id}
```

Delete a user account.

## Request Headers

### Content Type

All POST and PUT requests must include:
```
Content-Type: application/json
```

### Authentication

Authentication endpoints are available at `/api/v1/auth/*`. Authentication middleware can be enabled via configuration. When enabled, most endpoints require authentication tokens in the `Authorization` header:

```
Authorization: Bearer <token>
```

## Response Format

All responses follow a consistent format:

### Success Response

Responses vary by endpoint. Most endpoints return the resource directly:

```json
{
  "id": "memory:xyz123",
  "content": "...",
  ...
}
```

Error responses follow this format:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Memory not found"
  }
}
```

## Status Codes

- `200 OK`: Successful request
- `201 Created`: Resource created successfully
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Resource not found
- `500 Internal Server Error`: Server error

## Rate Limiting

Rate limiting is not currently implemented.

## WebSocket Support

WebSocket connections are available at:
- `/api/v1/ws` - General WebSocket endpoint for real-time updates
- `/api/v1/messaging/ws` - Messaging WebSocket endpoint

See the [Live Queries Documentation](LIVE_QUERIES.md) for details.

## Examples

### Creating and Searching Memories

```bash
# Create a memory
curl -X POST http://localhost:3000/api/v1/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Machine learning is a subset of artificial intelligence",
    "memory_type": "fact",
    "tags": ["ml", "ai"]
  }'

# Search for memories
curl "http://localhost:3000/api/v1/memories/search?q=machine%20learning"
```

### Working with Entities

```bash
# List all entities
curl http://localhost:3000/api/v1/entities

# Get specific entity
curl http://localhost:3000/api/v1/entities/entity:abc123

# Create an entity
curl -X POST http://localhost:3000/api/v1/entities \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Paris",
    "entity_type": "location"
  }'
```

### Working with Relationships

```bash
# Create a relationship
curl -X POST http://localhost:3000/api/v1/relationships \
  -H "Content-Type: application/json" \
  -d '{
    "source_id": "memory:abc123",
    "target_id": "entity:xyz789",
    "relationship_type": "mentions"
  }'

# Get memory graph with temporal span
curl "http://localhost:3000/api/v1/memories/memory:abc123/graph?depth=2&include_temporal_span=true"
```

### Working with Webhooks

```bash
# Register a webhook
curl -X POST http://localhost:3000/api/v1/webhooks \
  -H "Content-Type: application/json" \
  -d '{
    "event": "memory.created",
    "url": "https://example.com/webhooks/memory-events"
  }'

# List webhooks
curl http://localhost:3000/api/v1/webhooks
``` 