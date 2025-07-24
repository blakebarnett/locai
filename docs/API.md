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
GET /health
```

Returns server health status.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0-alpha.1"
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

#### Search Memories

```
GET /api/v1/memories/search?q={query}&limit={limit}
```

Search across all memories using BM25 full-text search.

**Query Parameters:**
- `q` (required): Search query
- `limit` (optional): Maximum results (default: 20)
- `strategy` (optional): Search strategy (auto, keyword, fuzzy)
- `min_score` (optional): Minimum relevance score

### Entity Operations

#### List Entities

```
GET /api/v1/entities
```

List all extracted entities with pagination.

#### Get Entity

```
GET /api/v1/entities/{id}
```

Retrieve a specific entity by ID.

#### Search Entities

```
GET /api/v1/entities/search?q={query}
```

Search entities by name or properties.

### Relationship Operations

#### Get Memory Graph

```
GET /api/v1/memories/{id}/graph
```

Retrieve the entity relationship graph for a specific memory.

#### Get Entity Relationships

```
GET /api/v1/entities/{id}/relationships
```

Get all relationships for a specific entity.

## Request Headers

### Content Type

All POST and PUT requests must include:
```
Content-Type: application/json
```

### Authentication

Authentication is not implemented in the alpha release. All endpoints are publicly accessible.

## Response Format

All responses follow a consistent format:

### Success Response

```json
{
  "data": { ... },
  "meta": {
    "timestamp": "2024-01-01T00:00:00Z",
    "version": "0.1.0"
  }
}
```

### Error Response

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Memory not found",
    "details": null
  },
  "meta": {
    "timestamp": "2024-01-01T00:00:00Z",
    "version": "0.1.0"
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

No rate limiting is implemented in the alpha release.

## WebSocket Support

WebSocket connections are available at `/ws` for real-time updates and streaming operations. See the WebSocket documentation for details.

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
``` 