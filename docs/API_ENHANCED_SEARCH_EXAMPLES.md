# Enhanced Search Scoring - REST API Examples

**Version**: 0.2.0  
**Endpoint**: `GET /api/memories/search`

---

## Overview

The search endpoint supports optional lifecycle-aware scoring that combines multiple signals:
- **BM25**: Keyword relevance (standard text search)
- **Vector Similarity**: Semantic similarity (requires ML service)
- **Recency**: Time since last access (with configurable decay)
- **Access Frequency**: How often memories have been retrieved
- **Priority**: Explicit importance levels

---

## Basic Search

### Text Search (Default)

Default behavior uses BM25 text search only (for backward compatibility):

```bash
curl "http://localhost:3000/api/memories/search?q=magic+spell&limit=10&mode=text"
```

### Hybrid Search (Recommended)

**Recommended when ML service is configured.** Automatically combines text and semantic search:

```bash
curl "http://localhost:3000/api/memories/search?q=magic+spell&limit=10&mode=hybrid"
```

Hybrid search provides the best results by combining:
- **Text search**: Finds exact keyword matches
- **Semantic search**: Finds related concepts (e.g., "spell" matches "magic", "incantation")

**Note**: Falls back to text-only if ML service is not configured.

---

## Enhanced Search with Scoring

### Example 1: Recency-Focused Search

Prioritize recently accessed memories with exponential decay:

```bash
curl -G "http://localhost:3000/api/memories/search" \
  --data-urlencode "q=dragon battle" \
  --data-urlencode "limit=10" \
  --data-urlencode 'scoring={"recency_boost":2.0,"decay_function":"exponential","decay_rate":0.1}'
```

**Use Case**: RPG character memory recall where recent events are more "memorable"

---

### Example 2: Frequency-Focused Search

Boost memories that have been accessed frequently:

```bash
curl -G "http://localhost:3000/api/memories/search" \
  --data-urlencode "q=important fact" \
  --data-urlencode "limit=10" \
  --data-urlencode 'scoring={"access_boost":2.0,"recency_boost":0.5}'
```

**Use Case**: Knowledge bases where frequently referenced information is most relevant

---

### Example 3: Priority-Focused Search

Emphasize high-priority memories:

```bash
curl -G "http://localhost:3000/api/memories/search" \
  --data-urlencode "q=mission objective" \
  --data-urlencode "limit=10" \
  --data-urlencode 'scoring={"priority_boost":3.0,"recency_boost":1.0}'
```

**Use Case**: Task management systems where explicit priority matters most

---

### Example 4: Balanced Multi-Factor Scoring

Combine all factors with custom weights:

```bash
curl -G "http://localhost:3000/api/memories/search" \
  --data-urlencode "q=conversation" \
  --data-urlencode "limit=10" \
  --data-urlencode 'scoring={"bm25_weight":1.0,"recency_boost":1.5,"access_boost":1.0,"priority_boost":0.5,"decay_function":"logarithmic","decay_rate":0.05}'
```

**Use Case**: General-purpose search with balanced relevance factors

---

### Example 5: Hybrid Search + Lifecycle Scoring (Recommended)

Combine hybrid search (text + semantic) with lifecycle metadata:

```bash
curl -G "http://localhost:3000/api/memories/search" \
  --data-urlencode "q=healing magic" \
  --data-urlencode "mode=hybrid" \
  --data-urlencode "limit=10" \
  --data-urlencode 'scoring={"vector_weight":1.5,"recency_boost":1.0,"decay_function":"exponential"}'
```

**Use Case**: Semantic search with recency weighting (requires ML service configured)

---

## Decay Functions

### None
No time-based decay. All memories have equal recency weight.

```json
{"decay_function": "none"}
```

### Linear
Importance decreases linearly with age:
```
boost * max(0, 1 - age_hours * decay_rate)
```

```json
{"decay_function": "linear", "decay_rate": 0.01}
```

**Best for**: Short-term memory with hard cutoffs

### Exponential (Default)
Importance decreases exponentially (models human forgetting curve):
```
boost * exp(-decay_rate * age_hours)
```

```json
{"decay_function": "exponential", "decay_rate": 0.1}
```

**Best for**: Realistic memory degradation, RPG character memories

### Logarithmic
Slower decay than exponential:
```
boost / (1 + age_hours * decay_rate).ln()
```

```json
{"decay_function": "logarithmic", "decay_rate": 0.05}
```

**Best for**: Long-term knowledge retention, historical data

---

## Scoring Configuration Reference

### Full Configuration Object

```json
{
  "bm25_weight": 1.0,           // Keyword matching weight (0.0-1.0)
  "vector_weight": 1.0,          // Semantic similarity weight (0.0-1.0)
  "recency_boost": 0.5,          // Recent memory boost factor
  "access_boost": 0.3,           // Frequent access boost factor
  "priority_boost": 0.2,         // Priority level boost factor
  "decay_function": "exponential", // none|linear|exponential|logarithmic
  "decay_rate": 0.1              // Decay speed (0.0-∞)
}
```

### Minimal Configuration

Only specify what you want to change from defaults:

```json
{
  "recency_boost": 2.0,
  "decay_function": "exponential"
}
```

All other parameters use defaults.

---

## Common Patterns

### Pattern 1: Real-Time Application
```json
{
  "recency_boost": 3.0,
  "decay_function": "exponential",
  "decay_rate": 0.2
}
```
Fast decay, heavy recency bias

### Pattern 2: Knowledge Base
```json
{
  "access_boost": 2.0,
  "priority_boost": 1.5,
  "recency_boost": 0.2
}
```
Emphasize frequently accessed and high-priority content

### Pattern 3: Long-Term Memory
```json
{
  "decay_function": "logarithmic",
  "decay_rate": 0.01,
  "access_boost": 1.0
}
```
Slow decay, stable long-term retention

### Pattern 4: Semantic Search
```json
{
  "vector_weight": 2.0,
  "bm25_weight": 0.5
}
```
Favor semantic similarity over keyword matching

---

## Error Handling

### Invalid JSON
```bash
curl -G "http://localhost:3000/api/memories/search" \
  --data-urlencode "q=test" \
  --data-urlencode 'scoring={"recency_boost":invalid}'
```

Response:
```json
{
  "error": "Invalid scoring configuration: expected value at line 1 column 26. Expected JSON like: {\"recency_boost\":2.0,\"decay_function\":\"exponential\"}"
}
```

### Missing Query
```bash
curl "http://localhost:3000/api/memories/search?scoring={}"
```

Response:
```json
{
  "error": "Missing query parameter 'q'"
}
```

---

## Tips & Best Practices

1. **Start Simple**: Begin with basic scoring and add complexity as needed
2. **URL Encoding**: Always URL-encode JSON in GET parameters
3. **Decay Rates**: 
   - 0.1 = slow decay (days/weeks)
   - 1.0 = medium decay (hours)
   - 10.0 = fast decay (minutes)
4. **Weights**: Higher weights increase influence of that factor
5. **Testing**: Use `/api/health` to verify server is running
6. **Monitoring**: Check `access_count` and `last_accessed` in results to verify scoring

---

## Advanced: Pre-Configured Profiles

The Rust library provides pre-configured profiles. To use in API, send the JSON equivalent:

### Default Profile
```json
{
  "bm25_weight": 1.0,
  "vector_weight": 1.0,
  "recency_boost": 0.5,
  "access_boost": 0.3,
  "priority_boost": 0.2,
  "decay_function": "exponential",
  "decay_rate": 0.1
}
```

### Recency-Focused Profile
```json
{
  "bm25_weight": 1.0,
  "vector_weight": 1.0,
  "recency_boost": 2.0,
  "access_boost": 0.5,
  "priority_boost": 0.3,
  "decay_function": "exponential",
  "decay_rate": 0.2
}
```

### Semantic-Focused Profile
```json
{
  "bm25_weight": 0.5,
  "vector_weight": 2.0,
  "recency_boost": 0.3,
  "access_boost": 0.2,
  "priority_boost": 0.2,
  "decay_function": "exponential",
  "decay_rate": 0.1
}
```

### Importance-Focused Profile
```json
{
  "bm25_weight": 1.0,
  "vector_weight": 1.0,
  "recency_boost": 0.2,
  "access_boost": 0.5,
  "priority_boost": 2.0,
  "decay_function": "none",
  "decay_rate": 0.0
}
```

---

## See Also

- [Enhanced Search Documentation](./guides/ENHANCED_SEARCH.md) - Full technical details
- [Lifecycle Tracking](./guides/LIFECYCLE_TRACKING.md) - How access counts and timestamps work
- [API Reference](./API.md) - Complete API documentation
- Swagger UI: `http://localhost:3000/docs` - Interactive API explorer

---

**Last Updated**: 2025-01-26  
**Version**: 0.2.0






