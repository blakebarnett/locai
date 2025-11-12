# Batch Operations API

**Version**: 0.2.0+  
**Status**: ✅ Implemented

## Overview

The Batch Operations API allows you to execute multiple memory and relationship operations in a single HTTP request. This is essential for:

- **Bulk imports**: Create many memories at once
- **Consolidation**: Update multiple memories as part of a batch process
- **Relationship building**: Create complex relationship networks efficiently
- **Performance**: Reduce network overhead compared to individual requests

## Quick Start

### Execute a Simple Batch

```bash
curl -X POST http://localhost:3000/api/v1/batch \
  -H "Content-Type: application/json" \
  -d '{
    "operations": [
      {
        "op": "CreateMemory",
        "data": {
          "content": "This is my first batch memory",
          "memory_type": "fact",
          "priority": 1,
          "tags": ["important"]
        }
      },
      {
        "op": "CreateMemory",
        "data": {
          "content": "This is my second batch memory",
          "memory_type": "episodic",
          "priority": 2
        }
      }
    ],
    "transaction": false
  }'
```

## API Endpoint

### Endpoint: `POST /api/v1/batch`

Execute a batch of operations.

#### Request Body

```json
{
  "operations": [
    {
      "op": "CreateMemory",
      "data": { ... }
    },
    ...
  ],
  "transaction": false
}
```

**Fields**:
- `operations` (array, required): List of operations to execute
- `transaction` (boolean, optional, default: false): Execute operations as a single transaction

#### Response

```json
{
  "results": [
    {
      "operation_index": 0,
      "resource_id": "memory_abc123"
    },
    {
      "operation_index": 1,
      "resource_id": "memory_def456"
    }
  ],
  "completed": 2,
  "failed": 0,
  "transaction": false,
  "transaction_id": null
}
```

**Fields**:
- `results`: Array of operation results
- `completed`: Number of successful operations
- `failed`: Number of failed operations
- `transaction`: Whether operations were executed in transaction mode
- `transaction_id`: Optional ID for transaction tracking/debugging

## Operation Types

### CreateMemory

Create a new memory in the batch.

```json
{
  "op": "CreateMemory",
  "data": {
    "content": "Memory content",
    "memory_type": "fact",
    "priority": 1,
    "tags": ["tag1", "tag2"],
    "source": "batch_import",
    "properties": {
      "custom_field": "custom_value"
    }
  }
}
```

**Fields**:
- `content` (string, required): Memory content
- `memory_type` (string, required): Type like "fact", "episodic", "procedural", etc.
- `priority` (integer, optional): 0=Low, 1=Normal, 2=High, 3=Critical (default: 1)
- `tags` (array, optional): Array of tag strings
- `source` (string, optional): Origin of the memory
- `properties` (object, optional): Custom metadata

**Response**:
- `operation_index`: Index in operations array
- `resource_id`: ID of created memory

### UpdateMemory

Update an existing memory.

```json
{
  "op": "UpdateMemory",
  "data": {
    "id": "memory_abc123",
    "content": "Updated content",
    "priority": 2,
    "tags": ["new_tag"],
    "properties": {
      "updated_field": "new_value"
    }
  }
}
```

**Fields**:
- `id` (string, required): Memory ID to update
- `content` (string, optional): New content
- `priority` (integer, optional): New priority level
- `tags` (array, optional): Replace tags
- `properties` (object, optional): Merge with existing properties

### DeleteMemory

Delete an existing memory.

```json
{
  "op": "DeleteMemory",
  "data": {
    "id": "memory_abc123"
  }
}
```

**Fields**:
- `id` (string, required): Memory ID to delete

### CreateRelationship

Create a relationship between two memories or entities.

```json
{
  "op": "CreateRelationship",
  "data": {
    "source": "memory_or_entity_id_1",
    "target": "memory_or_entity_id_2",
    "relationship_type": "references",
    "properties": {
      "confidence": 0.95
    },
    "enforce_constraints": false
  }
}
```

**Fields**:
- `source` (string, required): Source memory/entity ID
- `target` (string, required): Target memory/entity ID
- `relationship_type` (string, required): Type like "references", "contains", etc.
- `properties` (object, optional): Relationship metadata
- `enforce_constraints` (boolean, optional): Apply symmetry/transitivity rules

### UpdateRelationship

Update an existing relationship.

```json
{
  "op": "UpdateRelationship",
  "data": {
    "id": "relationship_xyz789",
    "properties": {
      "confidence": 0.85,
      "last_verified": "2025-01-24T10:30:00Z"
    }
  }
}
```

**Fields**:
- `id` (string, required): Relationship ID to update
- `properties` (object, optional): Updated properties

### DeleteRelationship

Delete an existing relationship.

```json
{
  "op": "DeleteRelationship",
  "data": {
    "id": "relationship_xyz789"
  }
}
```

**Fields**:
- `id` (string, required): Relationship ID to delete

### UpdateMetadata

Update only the metadata (properties) of a memory.

```json
{
  "op": "UpdateMetadata",
  "data": {
    "memory_id": "memory_abc123",
    "metadata": {
      "importance_score": 0.8,
      "category": "scientific"
    }
  }
}
```

**Fields**:
- `memory_id` (string, required): Memory ID to update
- `metadata` (object, required): Metadata to merge with existing properties

## Execution Modes

### Sequential Execution (default)

```json
{
  "operations": [...],
  "transaction": false
}
```

- Operations execute one by one
- Partial success allowed (some operations may fail)
- Each successful operation is committed independently
- Faster for most use cases, more flexible error handling

### Transactional Execution

```json
{
  "operations": [...],
  "transaction": true
}
```

- All operations execute together (all-or-nothing)
- If any operation fails, all changes are rolled back
- Slower but guarantees consistency
- Currently limited by SurrealDB embedded transaction support

## Limits and Constraints

- **Maximum batch size**: 1000 operations per request
- **Maximum timeout**: 30 seconds for entire batch
- **Performance**: Typical throughput is 100-500 operations/second depending on operation complexity

## Error Handling

### Error Response

Failed operations include detailed error messages:

```json
{
  "results": [
    {
      "operation_index": 0,
      "resource_id": "memory_abc123"
    },
    {
      "operation_index": 1,
      "error": "Memory with id 'invalid_id' not found",
      "error_code": "NOT_FOUND"
    }
  ],
  "completed": 1,
  "failed": 1,
  "transaction": false
}
```

### Common Error Codes

| Code | Meaning | Recovery |
|------|---------|----------|
| BATCH_TOO_LARGE | Batch exceeds 1000 operations | Split into smaller batches |
| NOT_FOUND | Resource doesn't exist | Verify IDs are correct |
| VALIDATION_ERROR | Operation data is invalid | Check operation format |
| STORAGE_ERROR | Database error | Retry or contact support |

## Examples

### Example 1: Bulk Import Memories

Import a collection of historical facts:

```python
import requests

facts = [
    {"content": "The Earth orbits the Sun", "memory_type": "fact"},
    {"content": "Water freezes at 0°C", "memory_type": "fact"},
    {"content": "DNA carries genetic information", "memory_type": "fact"},
]

operations = [
    {
        "op": "CreateMemory",
        "data": {
            "content": fact["content"],
            "memory_type": fact["memory_type"],
            "tags": ["historical_facts"],
            "source": "bulk_import"
        }
    }
    for fact in facts
]

response = requests.post(
    "http://localhost:3000/api/v1/batch",
    json={"operations": operations, "transaction": False}
)

results = response.json()
print(f"Created {results['completed']} memories, {results['failed']} failed")
```

### Example 2: Consolidation Update

Mark a set of memories as consolidated and update their metadata:

```python
import requests

memory_ids = ["mem_1", "mem_2", "mem_3"]

operations = [
    {
        "op": "UpdateMetadata",
        "data": {
            "memory_id": mem_id,
            "metadata": {
                "consolidated": True,
                "consolidated_at": "2025-01-24T10:30:00Z",
                "confidence_score": 0.95
            }
        }
    }
    for mem_id in memory_ids
]

response = requests.post(
    "http://localhost:3000/api/v1/batch",
    json={"operations": operations}
)
```

### Example 3: Build a Relationship Network

Create a network of related memories:

```javascript
const operations = [
  // Create base memories
  {
    "op": "CreateMemory",
    "data": {
      "content": "Alice is a software engineer",
      "memory_type": "fact"
    }
  },
  {
    "op": "CreateMemory",
    "data": {
      "content": "Bob is a data scientist",
      "memory_type": "fact"
    }
  },
  // Create relationships
  {
    "op": "CreateRelationship",
    "data": {
      "source": "mem_alice",
      "target": "mem_bob",
      "relationship_type": "collaborates_with"
    }
  }
];

const response = await fetch("http://localhost:3000/api/v1/batch", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ operations, transaction: false })
});
```

## Best Practices

### 1. Batch Size

```python
# ✅ Good: Split large datasets into reasonable batches
operations_batch = all_operations[i:i+500]

# ❌ Avoid: Sending all 10,000 operations at once
```

### 2. Error Handling

```python
# ✅ Good: Check failed operations and retry
response = requests.post(url, json=batch)
if response['failed'] > 0:
    failed_ops = [r for r in response['results'] if 'error' in r]
    # Handle retries with exponential backoff
```

### 3. Sequential vs Transactional

```python
# ✅ Use sequential for independence
{"operations": ops, "transaction": false}  # Faster, failures isolated

# ✅ Use transactional for consistency
{"operations": ops, "transaction": true}   # All-or-nothing guarantee
```

### 4. Operation Ordering

```python
# ✅ Consider dependencies:
operations = [
    {"op": "CreateMemory", "data": {...}},      # Creates mem_1
    {"op": "CreateMemory", "data": {...}},      # Creates mem_2
    {"op": "CreateRelationship", "data": {      # References mem_1 and mem_2
        "source": "mem_1", 
        "target": "mem_2"
    }}
]
```

## Performance Considerations

### Throughput

Typical performance:
- **Create Memory**: 100-200 ops/sec
- **Update Memory**: 200-300 ops/sec  
- **Create Relationship**: 150-250 ops/sec
- **Mixed operations**: 100-150 ops/sec

### Optimization Tips

1. **Batch size**: 100-500 operations per batch is usually optimal
2. **Sequential mode**: Faster than transactional for independent operations
3. **Network**: Reduce request count significantly vs individual API calls
4. **Memory**: Minimal overhead for batch requests

### Example Latency

```
10 operations:   50ms
100 operations:  300ms
500 operations:  1.2s
1000 operations: 2.5s
```

## Monitoring and Debugging

### Enable Debug Logging

Set environment variable:
```bash
RUST_LOG=locai=debug,locai_server=debug
```

### Track Transaction IDs

```python
response = requests.post(url, json=batch)
if response['transaction_id']:
    print(f"Batch ID: {response['transaction_id']}")
    # Can be used for debugging multi-part batches
```

### Inspect Detailed Results

```python
for result in response['results']:
    if 'error' in result:
        print(f"Op {result['operation_index']}: {result['error']}")
        if 'error_code' in result:
            print(f"  Code: {result['error_code']}")
```

## Future Enhancements

### Planned Features

- [ ] Async batch processing with job tracking
- [ ] Batch templates for common patterns
- [ ] Rate limiting per client
- [ ] Batch operation scheduling
- [ ] Webhook callbacks for long-running batches
- [ ] Partial transaction commits

### SurrealDB Transaction Improvements

When SurrealDB embedded transaction support improves, transactional mode will provide true ACID guarantees across all operations.

## Migration from Individual Requests

### Before: Individual Requests

```python
# 1000 HTTP requests, very slow
for memory in memories:
    requests.post("/api/v1/memories", json=memory)
```

### After: Batch Request

```python
# 1 HTTP request, much faster
operations = [{"op": "CreateMemory", "data": m} for m in memories]
requests.post("/api/v1/batch", json={"operations": operations})
```

**Performance improvement**: 10-50x faster depending on network latency

## See Also

- [REST API Documentation](API.md)
- [Memory Management](DESIGN.md#memory-model)
- [Relationship Types](RELATIONSHIP_REGISTRY.md)
