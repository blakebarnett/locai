# Locai Temporal Features - Usage Guide for Zera Team

**Date**: November 1, 2025  
**Status**: ✅ Implemented  
**API Version**: v1

---

## Overview

This document provides comprehensive guidance on using Locai's temporal features for the Zera Temporal Coherence System. Two key features have been implemented to support time-based memory queries and analysis:

1. **Temporal Filtering in Search** (addresses ER-001)
2. **Temporal Span in Graph API** (addresses ER-005)

---

## Feature 1: Temporal Filtering in Search API

### Endpoint

```
GET /api/memories/search
GET /api/v1/memories/search
```

### New Parameters

| Parameter | Type | Required | Format | Description |
|-----------|------|----------|--------|-------------|
| `created_after` | string | No | ISO 8601 | Filter memories created after this timestamp (inclusive) |
| `created_before` | string | No | ISO 8601 | Filter memories created before this timestamp (inclusive) |

### ISO 8601 Timestamp Format

All temporal parameters use ISO 8601 format with timezone:

```
2025-11-01T00:00:00Z        # UTC timezone
2025-11-01T14:30:00-05:00   # EST timezone
2025-11-01T14:30:00.123Z    # With milliseconds
```

### Usage Examples

#### Example 1: Get memories from the last hour

```bash
# Calculate timestamps
START_TIME="2025-11-01T09:00:00Z"
END_TIME="2025-11-01T10:00:00Z"

# Query with temporal filters
curl "http://localhost:3000/api/memories/search?q=*&created_after=${START_TIME}&created_before=${END_TIME}&limit=50"
```

#### Example 2: Get conversation memories from current session

```bash
# Session started at 8am, now it's 10am
curl "http://localhost:3000/api/memories/search?q=dialogue&memory_type=custom:conversation&created_after=2025-11-01T08:00:00Z&created_before=2025-11-01T10:00:00Z"
```

#### Example 3: Recent turn memories (last 5 turns ≈ 10 minutes)

```bash
# Get memories from the last 10 minutes
TEN_MINUTES_AGO=$(date -u -d '10 minutes ago' +"%Y-%m-%dT%H:%M:%SZ")
NOW=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

curl "http://localhost:3000/api/memories/search?q=*&created_after=${TEN_MINUTES_AGO}&created_before=${NOW}"
```

#### Example 4: All memories since session start (no end time)

```bash
# Get all memories created after session start
curl "http://localhost:3000/api/memories/search?q=*&created_after=2025-11-01T08:00:00Z"
```

#### Example 5: Combine with other filters

```bash
# Get tavern-related memories from today, tagged as important
curl "http://localhost:3000/api/memories/search?q=tavern&created_after=2025-11-01T00:00:00Z&tags=important,quest&memory_type=custom:observation"
```

### Python Integration (for Zera)

```python
from datetime import datetime, timedelta, timezone
import httpx

class LocaiClient:
    def __init__(self, base_url: str = "http://localhost:3000"):
        self.base_url = base_url
        self.client = httpx.AsyncClient()

    async def search_memories_temporal(
        self,
        query: str = "*",
        start_time: datetime | None = None,
        end_time: datetime | None = None,
        memory_type: str | None = None,
        tags: list[str] | None = None,
        limit: int = 50,
    ) -> list[dict]:
        """Search memories within a time range."""
        params = {
            "q": query,
            "limit": limit,
        }
        
        if start_time:
            params["created_after"] = start_time.isoformat()
        if end_time:
            params["created_before"] = end_time.isoformat()
        if memory_type:
            params["memory_type"] = memory_type
        if tags:
            params["tags"] = ",".join(tags)
        
        response = await self.client.get(
            f"{self.base_url}/api/memories/search",
            params=params
        )
        response.raise_for_status()
        return response.json()

# Example usage for Zera Temporal Coherence System
async def get_recent_turn_memories(session_id: str, turns: int = 5):
    """Get memories from the last N turns (assuming ~2 min per turn)."""
    locai = LocaiClient()
    
    # Calculate time range for last 5 turns
    end_time = datetime.now(timezone.utc)
    start_time = end_time - timedelta(minutes=turns * 2)
    
    memories = await locai.search_memories_temporal(
        query="*",  # All content
        start_time=start_time,
        end_time=end_time,
        memory_type="custom:conversation",
        limit=100
    )
    
    return memories

async def get_session_memories(session_start: datetime):
    """Get all memories from current session."""
    locai = LocaiClient()
    
    memories = await locai.search_memories_temporal(
        query="*",
        start_time=session_start,
        # No end_time = get everything until now
    )
    
    return memories

async def get_yesterday_events():
    """Get memories from yesterday (narrative query)."""
    locai = LocaiClient()
    
    now = datetime.now(timezone.utc)
    yesterday_start = (now - timedelta(days=1)).replace(hour=0, minute=0, second=0)
    yesterday_end = yesterday_start + timedelta(days=1)
    
    memories = await locai.search_memories_temporal(
        query="*",
        start_time=yesterday_start,
        end_time=yesterday_end,
    )
    
    return memories
```

### Response Format

```json
[
  {
    "id": "mem_123",
    "content": "Player investigated the tavern",
    "memory_type": "custom:observation",
    "created_at": "2025-11-01T10:05:00Z",
    "tags": ["tavern", "investigation"],
    "properties": {},
    "score": 0.95
  }
]
```

### Error Handling

**Invalid timestamp format:**
```json
{
  "error": "Invalid created_after timestamp: premature end of input. Expected ISO 8601 format like: 2025-11-01T00:00:00Z"
}
```

**Python example with error handling:**
```python
from datetime import datetime
import httpx

async def safe_temporal_search(start_time: datetime | None = None):
    try:
        locai = LocaiClient()
        return await locai.search_memories_temporal(start_time=start_time)
    except httpx.HTTPStatusError as e:
        if e.response.status_code == 400:
            print(f"Invalid request: {e.response.json()}")
            # Fall back to non-temporal search
            return await locai.search_memories_temporal()
        raise
```

---

## Feature 2: Temporal Span in Graph API

### Endpoints

```
GET /api/memories/{id}/graph
GET /api/entities/{id}/graph
```

### New Parameter

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `include_temporal_span` | boolean | No | false | Include temporal span analysis in metadata |

### Usage Examples

#### Example 1: Get memory graph with temporal span

```bash
# Get graph with temporal information
curl "http://localhost:3000/api/memories/mem_123/graph?depth=2&include_temporal_span=true"
```

#### Example 2: Without temporal span (default behavior)

```bash
# Standard graph query (backward compatible)
curl "http://localhost:3000/api/memories/mem_123/graph?depth=2"
```

### Response Format

**With `include_temporal_span=true`:**

```json
{
  "center_id": "mem_123",
  "memories": [
    {
      "id": "mem_123",
      "content": "First tavern visit",
      "created_at": "2025-11-01T10:00:00Z"
    },
    {
      "id": "mem_124",
      "content": "Spoke with bartender",
      "created_at": "2025-11-01T10:05:00Z"
    },
    {
      "id": "mem_125",
      "content": "Found mysterious note",
      "created_at": "2025-11-01T10:15:00Z"
    }
  ],
  "relationships": [
    {
      "id": "rel_1",
      "source_id": "mem_123",
      "target_id": "mem_124",
      "relationship_type": "leads_to"
    }
  ],
  "metadata": {
    "node_count": 3,
    "edge_count": 2,
    "max_depth": 2,
    "temporal_span": {
      "start": "2025-11-01T10:00:00Z",
      "end": "2025-11-01T10:15:00Z",
      "duration_days": 0,
      "duration_seconds": 900,
      "memory_count": 3
    }
  }
}
```

**Without `include_temporal_span` (default):**

```json
{
  "center_id": "mem_123",
  "memories": [...],
  "relationships": [...],
  "metadata": {
    "node_count": 3,
    "edge_count": 2,
    "max_depth": 2
    // temporal_span field is omitted
  }
}
```

### Python Integration (for Zera)

```python
from datetime import datetime

async def get_memory_community_age(memory_id: str) -> str:
    """Determine how old a memory community is."""
    locai = LocaiClient()
    
    response = await locai.client.get(
        f"{locai.base_url}/api/memories/{memory_id}/graph",
        params={
            "depth": 2,
            "include_temporal_span": True
        }
    )
    response.raise_for_status()
    graph = response.json()
    
    # Check if temporal span is included
    temporal_span = graph["metadata"].get("temporal_span")
    if not temporal_span:
        return "UNKNOWN"
    
    # Parse the end time
    latest = datetime.fromisoformat(temporal_span["end"].replace("Z", "+00:00"))
    age_hours = (datetime.now(datetime.timezone.utc) - latest).total_seconds() / 3600
    
    if age_hours < 1:
        return "THIS_TURN"
    elif age_hours < 6:
        return "THIS_SESSION"
    else:
        return "EARLIER"

async def analyze_temporal_density(memory_id: str) -> float:
    """Calculate memory density (memories per hour) for a community."""
    locai = LocaiClient()
    
    response = await locai.client.get(
        f"{locai.base_url}/api/memories/{memory_id}/graph",
        params={"depth": 2, "include_temporal_span": True}
    )
    graph = response.json()
    
    temporal_span = graph["metadata"].get("temporal_span")
    if not temporal_span or temporal_span["duration_seconds"] == 0:
        return 0.0
    
    # Calculate memories per hour
    hours = temporal_span["duration_seconds"] / 3600
    density = temporal_span["memory_count"] / max(hours, 0.01)  # Avoid division by zero
    
    return density

async def is_memory_cluster_recent(memory_id: str, threshold_hours: int = 6) -> bool:
    """Check if a memory cluster is from the current session."""
    locai = LocaiClient()
    
    response = await locai.client.get(
        f"{locai.base_url}/api/memories/{memory_id}/graph",
        params={"depth": 2, "include_temporal_span": True}
    )
    graph = response.json()
    
    temporal_span = graph["metadata"].get("temporal_span")
    if not temporal_span:
        return False
    
    latest = datetime.fromisoformat(temporal_span["end"].replace("Z", "+00:00"))
    age_hours = (datetime.now(datetime.timezone.utc) - latest).total_seconds() / 3600
    
    return age_hours < threshold_hours
```

### Use Cases for Zera

#### 1. Temporal Context for LLM

```python
async def build_temporal_context_for_llm(memory_id: str) -> dict:
    """Build temporal context annotations for LLM."""
    locai = LocaiClient()
    
    graph = await locai.client.get(
        f"{locai.base_url}/api/memories/{memory_id}/graph",
        params={"depth": 2, "include_temporal_span": True}
    ).json()
    
    temporal_span = graph["metadata"].get("temporal_span", {})
    
    return {
        "memory_count": temporal_span.get("memory_count", 0),
        "time_span_hours": temporal_span.get("duration_seconds", 0) / 3600,
        "recency": "recent" if temporal_span.get("duration_days", 999) < 1 else "older",
        "cluster_type": "dense" if temporal_span.get("memory_count", 0) > 5 else "sparse"
    }
```

#### 2. Session Boundary Detection

```python
async def detect_session_boundary(memory_ids: list[str]) -> bool:
    """Detect if memories span multiple sessions (>6 hour gap)."""
    locai = LocaiClient()
    
    all_timestamps = []
    
    for memory_id in memory_ids:
        graph = await locai.client.get(
            f"{locai.base_url}/api/memories/{memory_id}/graph",
            params={"depth": 1, "include_temporal_span": True}
        ).json()
        
        temporal_span = graph["metadata"].get("temporal_span")
        if temporal_span:
            all_timestamps.append(
                datetime.fromisoformat(temporal_span["start"].replace("Z", "+00:00"))
            )
            all_timestamps.append(
                datetime.fromisoformat(temporal_span["end"].replace("Z", "+00:00"))
            )
    
    if len(all_timestamps) < 2:
        return False
    
    all_timestamps.sort()
    
    # Check for gaps > 6 hours between consecutive timestamps
    for i in range(1, len(all_timestamps)):
        gap_hours = (all_timestamps[i] - all_timestamps[i-1]).total_seconds() / 3600
        if gap_hours > 6:
            return True  # Session boundary detected
    
    return False
```

---

## Combined Usage: Temporal Search + Graph Analysis

### Example: Find and Analyze Recent Memory Clusters

```python
async def find_recent_memory_clusters(hours: int = 6):
    """Find recent memory clusters and analyze their temporal characteristics."""
    locai = LocaiClient()
    
    # Step 1: Get recent memories
    start_time = datetime.now(timezone.utc) - timedelta(hours=hours)
    recent_memories = await locai.search_memories_temporal(
        query="*",
        start_time=start_time,
        limit=100
    )
    
    # Step 2: For each memory, get its graph with temporal span
    clusters = []
    for memory in recent_memories:
        graph = await locai.client.get(
            f"{locai.base_url}/api/memories/{memory['id']}/graph",
            params={"depth": 2, "include_temporal_span": True}
        ).json()
        
        temporal_span = graph["metadata"].get("temporal_span")
        if temporal_span:
            clusters.append({
                "center_id": memory["id"],
                "center_content": memory["content"],
                "cluster_size": temporal_span["memory_count"],
                "duration_minutes": temporal_span["duration_seconds"] / 60,
                "density": temporal_span["memory_count"] / max(
                    temporal_span["duration_seconds"] / 3600, 0.01
                )
            })
    
    # Sort by density (memories per hour)
    clusters.sort(key=lambda x: x["density"], reverse=True)
    return clusters
```

---

## Backward Compatibility

Both features are **fully backward compatible**:

1. **Temporal Search**: Existing search queries work unchanged. Temporal filters are optional.
2. **Graph API**: Default behavior unchanged. Temporal span only included when explicitly requested.

### Migration Path

**Old Code (still works):**
```python
# Existing code continues to work
memories = await search_memories(query="tavern")
graph = await get_memory_graph(memory_id, depth=2)
```

**New Code (with temporal features):**
```python
# Add temporal filters when needed
memories = await search_memories_temporal(
    query="tavern",
    start_time=session_start,
    end_time=datetime.now()
)

# Request temporal span when analyzing communities
graph = await get_memory_graph(
    memory_id,
    depth=2,
    include_temporal_span=True
)
```

---

## Performance Considerations

### Temporal Search Performance

- **Database**: Temporal filters use indexed `created_at` field (fast)
- **Recommendation**: Combine temporal filters with text search for best performance
- **Limitation**: No special optimization needed; standard B-tree index on timestamps

### Graph Temporal Span Performance

- **Overhead**: Minimal (simple timestamp sorting)
- **When to use**: Only when you need temporal analysis
- **Cost**: ~1ms additional processing for graphs with <100 memories

---

## OpenAPI / Swagger Documentation

The features are fully documented in Swagger UI:

```
http://localhost:3000/docs
```

**Endpoints to explore:**
- `GET /api/memories/search` - See new `created_after` and `created_before` parameters
- `GET /api/memories/{id}/graph` - See new `include_temporal_span` parameter
- `GET /api/entities/{id}/graph` - See new `include_temporal_span` parameter

---

## Testing

### Manual Testing

```bash
# 1. Create test memories with different timestamps
curl -X POST http://localhost:3000/api/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Test memory 1",
    "memory_type": "custom:test",
    "tags": ["test"]
  }'

# 2. Test temporal search
curl "http://localhost:3000/api/memories/search?q=test&created_after=2025-11-01T00:00:00Z"

# 3. Test graph with temporal span
curl "http://localhost:3000/api/memories/{id}/graph?include_temporal_span=true"
```

### Python Test Script

```python
import asyncio
from datetime import datetime, timezone, timedelta
import httpx

async def test_temporal_features():
    """Test temporal search and graph features."""
    base_url = "http://localhost:3000"
    async with httpx.AsyncClient() as client:
        # Test 1: Create memories
        print("Creating test memories...")
        memory_ids = []
        for i in range(3):
            response = await client.post(
                f"{base_url}/api/memories",
                json={
                    "content": f"Test memory {i}",
                    "memory_type": "custom:test",
                    "tags": ["temporal-test"]
                }
            )
            memory_ids.append(response.json()["id"])
            await asyncio.sleep(1)  # 1 second between memories
        
        # Test 2: Temporal search
        print("\nTesting temporal search...")
        now = datetime.now(timezone.utc)
        start = now - timedelta(minutes=5)
        
        response = await client.get(
            f"{base_url}/api/memories/search",
            params={
                "q": "test",
                "created_after": start.isoformat(),
                "tags": "temporal-test"
            }
        )
        results = response.json()
        print(f"Found {len(results)} memories in last 5 minutes")
        
        # Test 3: Graph with temporal span
        print("\nTesting graph temporal span...")
        if memory_ids:
            response = await client.get(
                f"{base_url}/api/memories/{memory_ids[0]}/graph",
                params={"depth": 2, "include_temporal_span": True}
            )
            graph = response.json()
            temporal_span = graph["metadata"].get("temporal_span")
            if temporal_span:
                print(f"Temporal span: {temporal_span['duration_seconds']} seconds")
                print(f"Memory count: {temporal_span['memory_count']}")
            else:
                print("No temporal span (expected if no connections)")

if __name__ == "__main__":
    asyncio.run(test_temporal_features())
```

---

## Summary for Zera Team

### What Changed

✅ **Implemented:**
1. Temporal filtering in search API (`created_after`, `created_before`)
2. Temporal span analysis in graph API (`include_temporal_span`)

❌ **Not Implemented** (not needed):
1. Dedicated `/api/memories/temporal_search` endpoint (use existing search instead)
2. Consolidation API (requires broader design work)
3. Temporal expression detection API (can be done client-side)

### Next Steps

1. **Update Zera integration** to use new temporal filters
2. **Test with real session data** to validate performance
3. **Monitor API usage** to see if additional temporal features are needed
4. **Consider** temporal expression parsing client-side for natural language queries

### Support

For questions or issues:
- Check Swagger docs: `http://localhost:3000/docs`
- Review this documentation
- Test with the provided Python examples

---

**Document Version**: 1.0  
**Last Updated**: November 1, 2025  
**Locai Version**: 0.2.1

