# Locai Relationship API Guide

**Version:** 0.1.0-alpha.1  
**Last Updated:** 2025-10-25

---

## Overview

Locai supports rich relationships between **memories** and **entities**, enabling graph-based knowledge representation. The relationship system is **flexible** and **type-agnostic** - you can create relationships between:

- Memory ↔ Memory
- Entity ↔ Entity  
- Memory ↔ Entity (cross-type)

## Core Concepts

### Memories vs Entities

| Concept | Purpose | Examples |
|---------|---------|----------|
| **Memory** | Temporal facts, observations, events | "User said X", "Character did Y", Session logs |
| **Entity** | Abstract concepts, extracted entities | Person names, locations, characters, concepts |
| **Relationship** | Directed connection between any two nodes | "has_character", "located_in", "depends_on" |

### Relationship Properties

- **source_id**: The ID of the source node (memory or entity)
- **target_id**: The ID of the target node (memory or entity)
- **relationship_type**: String describing the relationship (e.g., "has_character")
- **properties**: Arbitrary JSON metadata
- **Directionality**: Relationships are directed (A→B), but you can query bidirectionally

---

## API Endpoints

### 1. Resource-Scoped Endpoints (Recommended)

#### Create Memory Relationship

```http
POST /api/memories/{source_id}/relationships
Content-Type: application/json

{
  "target_id": "target_memory_or_entity_id",
  "relationship_type": "has_character",
  "properties": {}
}
```

**Response:** `201 Created`
```json
{
  "id": "rel_xyz",
  "source_id": "mem_abc",
  "target_id": "entity_def",
  "relationship_type": "has_character",
  "properties": {},
  "created_at": "2025-10-25T12:00:00Z",
  "updated_at": "2025-10-25T12:00:00Z"
}
```

**Target can be:**
- ✅ Another memory ID (Memory→Memory)
- ✅ An entity ID (Memory→Entity)

---

#### Get Memory Relationships

```http
GET /api/memories/{id}/relationships?direction=both&relationship_type=has_character
```

**Query Parameters:**
- `direction`: `outgoing` | `incoming` | `both` (default: `both`)
- `relationship_type`: Filter by type (optional)

**Response:** `200 OK`
```json
[
  {
    "id": "rel_1",
    "source_id": "mem_session",
    "target_id": "entity_char",
    "relationship_type": "has_character",
    "properties": {},
    "created_at": "2025-10-25T12:00:00Z",
    "updated_at": "2025-10-25T12:00:00Z"
  }
]
```

---

#### Create Entity Relationship

```http
POST /api/entities/{source_id}/relationships
Content-Type: application/json

{
  "target_id": "target_entity_or_memory_id",
  "relationship_type": "contains",
  "properties": {}
}
```

**Target can be:**
- ✅ Another entity ID (Entity→Entity)
- ✅ A memory ID (Entity→Memory)

---

#### Get Entity Relationships

```http
GET /api/entities/{id}/relationships?direction=both&relationship_type=contains
```

Same parameters as memory relationships.

---

### 2. Universal Relationship Endpoint

#### List All Relationships

```http
GET /api/relationships?source_id=X&target_id=Y&relationship_type=Z&page=0&size=20
```

**Query Parameters:**
- `page`: Page number (0-based)
- `size`: Items per page (default: 20)
- `source_id`: Filter by source
- `target_id`: Filter by target
- `relationship_type`: Filter by type

**Use Case:** Global relationship queries across the entire graph.

---

#### Get Specific Relationship

```http
GET /api/relationships/{id}
```

---

#### Update Relationship

```http
PUT /api/relationships/{id}
Content-Type: application/json

{
  "relationship_type": "updated_type",
  "properties": {"key": "value"}
}
```

---

#### Delete Relationship

```http
DELETE /api/relationships/{id}
```

---

## Common Use Cases

### 1. Game Session with Characters (Zera Example)

```python
# Create a game session memory
session_memory = await locai.create_memory({
    "content": "D&D Session: The Lost Temple",
    "memory_type": "session_profile",
    "metadata": {"date": "2025-10-25"}
})

# Create character memories
char1_memory = await locai.create_memory({
    "content": "Character: Aria the Elf",
    "memory_type": "character_profile"
})

# Link session to character (Memory→Memory)
await locai.post(f"/api/memories/{session_memory['id']}/relationships", {
    "target_id": char1_memory['id'],
    "relationship_type": "has_character"
})
```

---

### 2. Entity Extraction with Memory Links

```python
# Create a memory about a person
memory = await locai.create_memory({
    "content": "John visited Paris in 2024"
})

# Extract entities
person_entity = await locai.create_entity({
    "entity_type": "person",
    "properties": {"name": "John"}
})

location_entity = await locai.create_entity({
    "entity_type": "location",
    "properties": {"name": "Paris"}
})

# Link memory to entities (Memory→Entity)
await locai.post(f"/api/memories/{memory['id']}/relationships", {
    "target_id": person_entity['id'],
    "relationship_type": "mentions"
})

await locai.post(f"/api/memories/{memory['id']}/relationships", {
    "target_id": location_entity['id'],
    "relationship_type": "mentions"
})

# Link entities to each other (Entity→Entity)
await locai.post(f"/api/entities/{person_entity['id']}/relationships", {
    "target_id": location_entity['id'],
    "relationship_type": "visited"
})
```

---

### 3. Knowledge Graph Traversal

```python
# Get all character relationships for a session
relationships = await locai.get(
    f"/api/memories/{session_id}/relationships",
    params={"relationship_type": "has_character", "direction": "outgoing"}
)

# Get all memories that mention a specific entity
entity_relationships = await locai.get(
    f"/api/entities/{entity_id}/relationships",
    params={"relationship_type": "mentions", "direction": "incoming"}
)

# Query the entire graph
all_relationships = await locai.get("/api/relationships", params={
    "relationship_type": "has_character",
    "page": 0,
    "size": 100
})
```

---

## Relationship Types

Common relationship types (you can use any string):

### Memory Relationships
- `has_character` - Session has a character
- `depends_on` - Memory depends on another
- `follows` - Temporal ordering
- `related_to` - General association

### Entity Relationships
- `part_of` - Hierarchical containment
- `located_in` - Spatial relationships
- `knows` - Social connections
- `mentions` - Cross-references

### Cross-Type (Memory↔Entity)
- `mentions` - Memory mentions entity
- `describes` - Memory describes entity
- `contains` - Entity appears in memory
- `extracted_from` - Entity extracted from memory

---

## Python Client Example

```python
from locai_client import LocaiClient

client = LocaiClient("http://localhost:3001")

# Create memories
session = await client.memories.create({
    "content": "Game Session 1",
    "memory_type": "session"
})

character = await client.memories.create({
    "content": "Character: Gandalf",
    "memory_type": "character"
})

# Create relationship
relationship = await client.memories.create_relationship(
    source_id=session['id'],
    target_id=character['id'],
    relationship_type="has_character"
)

# Get relationships
relationships = await client.memories.get_relationships(
    memory_id=session['id'],
    direction="both"
)

# Query all relationships
all_rels = await client.relationships.list(
    source_id=session['id'],
    relationship_type="has_character"
)
```

---

## Architecture Notes

### Storage Layer

Relationships are stored in **SurrealDB** using graph relations. The storage layer is **type-agnostic** - it only cares about IDs, not whether they're memories or entities.

### API Design Philosophy

1. **Resource-scoped endpoints** (`/api/memories/{id}/relationships`) - Best for creating relationships where you know the source type
2. **Universal endpoint** (`/api/relationships`) - Best for querying across the entire graph
3. **Flexible validation** - Endpoints accept any valid ID (memory or entity) as target
4. **Backward compatible** - Existing entity-only `/api/relationships POST` still works

### Performance Considerations

- Relationship queries are indexed by `source_id`, `target_id`, and `relationship_type`
- Limit results to 100 per query by default
- Use pagination for large result sets
- Consider caching frequently accessed relationships

---

## Migration Guide

### If you were using entity-only relationships:

**Before:**
```python
# Only worked with entities
POST /api/relationships
{
  "source_id": "entity_1",
  "target_id": "entity_2",
  "relationship_type": "knows"
}
```

**After (both work):**
```python
# Option 1: Keep using universal endpoint (entity→entity only)
POST /api/relationships
{
  "source_id": "entity_1",
  "target_id": "entity_2",
  "relationship_type": "knows"
}

# Option 2: Use resource-scoped endpoint (entity→entity OR entity→memory)
POST /api/entities/{entity_1}/relationships
{
  "target_id": "entity_2_or_memory",
  "relationship_type": "knows"
}
```

---

## Error Responses

### 404 Not Found
```json
{
  "error": "not_found",
  "message": "Memory or Entity 'xyz' not found"
}
```

### 400 Bad Request
```json
{
  "error": "bad_request",
  "message": "Invalid relationship type"
}
```

---

## WebSocket Notifications

When relationships are created/updated/deleted, WebSocket subscribers receive:

```json
{
  "type": "relationship_created",
  "relationship_id": "rel_xyz",
  "source_id": "mem_abc",
  "target_id": "entity_def",
  "relationship_type": "has_character",
  "properties": {},
  "timestamp": "2025-10-25T12:00:00Z"
}
```

---

## Best Practices

1. **Use descriptive relationship types** - `has_character` is better than `rel1`
2. **Store metadata in properties** - Timestamps, weights, confidence scores
3. **Query bidirectionally when needed** - Use `direction=both` to find all connections
4. **Use resource-scoped endpoints** - Clearer intent and better validation
5. **Index important relationship types** - If you query them frequently
6. **Clean up orphaned relationships** - When deleting memories/entities

---

## GraphQL Alternative (Future)

For complex graph queries, consider using the `/api/graph/query` endpoint:

```http
POST /api/graph/query
Content-Type: application/json

{
  "pattern": "memories connected to entities of type 'person'",
  "limit": 50
}
```

---

## Questions?

- **Discord**: [Locai Community](https://discord.gg/locai)
- **GitHub Issues**: [locai/issues](https://github.com/locai/locai/issues)
- **Docs**: [docs.locai.dev](https://docs.locai.dev)

