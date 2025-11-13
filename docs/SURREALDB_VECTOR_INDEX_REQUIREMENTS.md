# SurrealDB Vector Index Requirements

## Summary

Based on SurrealDB 2.3.3 documentation and testing, here are the requirements for embedding fields and vector indexes (M-Tree and HNSW):

## Field Definition

### Optional Fields (Current Implementation)
```sql
DEFINE FIELD embedding ON memory TYPE option<array<float>>;
```

**Status**: ✅ Supported by SurrealDB documentation
- Optional fields are officially supported
- You can use `WHERE embedding IS NOT NULL` to filter in queries
- Documentation examples show this pattern

### Non-Optional Fields (Alternative)
```sql
DEFINE FIELD embedding ON memory TYPE array<float>;
```

**Status**: ✅ Also supported
- Simpler schema
- No need for NULL checks in queries
- Might have better index performance

## Vector Index Requirements

### Current Index Definition (M-Tree)
```sql
DEFINE INDEX memory_embedding_mtree_idx ON memory 
    FIELDS embedding 
    MTREE DIMENSION 1024 DIST COSINE TYPE F32;
```

**Why M-Tree over HNSW?**
- M-Tree provides **exact nearest neighbor** results (better accuracy)
- HNSW provides approximate results (faster but less accurate)
- Both indexes have issues with optional fields (`option<array<float>>`)
- We have a brute-force fallback that works reliably

### Requirements:
1. **Dimension Match**: The `DIMENSION` parameter (1024) must match the actual embedding vector length
2. **Distance Metric**: `DIST COSINE` specifies cosine similarity (other options: `EUCLIDEAN`, `MANHATTAN`)
3. **HNSW Parameters**:
   - `EFC` (efConstruction): Controls index build quality (default: 150)
   - `M`: Controls graph connectivity (default: 12)

## Current Status

**Working Solution**: ✅ M-Tree index works correctly!

**Fix**: The issue was with deserialization, not the indexes themselves:
- ✅ M-Tree index works perfectly with optional fields
- ✅ Query results deserialize correctly using explicit struct fields (not `#[serde(flatten)]`)
- ✅ Follows the same pattern as BM25 search (explicitly list all fields including RecordId)

**Root Cause (Resolved)**: 
- The problem was using `#[serde(flatten)]` with `RecordId` enum when there are computed fields (`similarity_score`, `vector_distance`)
- SurrealDB's SDK requires explicit field listing when mixing RecordId fields with computed fields
- Solution: Define a struct with all fields explicitly listed (like `BM25SearchResult`)

## Current Implementation

**Solution**: M-Tree index with explicit struct deserialization
- ✅ M-Tree index works correctly with optional fields
- ✅ Explicit struct definition (like BM25 search) avoids RecordId enum issues
- ✅ Fallback to brute-force only if index returns 0 results (safety net)

**Performance**: 
- M-Tree provides exact nearest neighbor search (O(log n))
- Works efficiently with optional fields
- Brute-force fallback available as safety net

## Recommended Solutions for Production

### Option 1: Make Field Non-Optional (Best for Large Datasets)
```sql
DEFINE FIELD embedding ON memory TYPE array<float>;
```
- Enables M-Tree/HNSW indexes to work properly
- Better performance for large datasets
- Requires all memories to have embeddings (or use default empty array)

### Option 2: Keep Current Approach (Best for Flexibility)
- Keep optional fields for flexibility
- Rely on brute-force fallback
- Works well for small to medium datasets
- No schema changes needed

### Option 3: Use Separate Vector Table
- Store embeddings in the dedicated `vector` table
- Join with `memory` table when needed
- More complex but more flexible
- Allows different indexing strategies

## Testing Steps

1. **Verify Embeddings Exist**:
   ```sql
   SELECT VALUE count() FROM memory WHERE embedding IS NOT NULL;
   ```

2. **Check Embedding Dimensions**:
   ```sql
   SELECT VALUE array::len(embedding) FROM memory WHERE embedding IS NOT NULL LIMIT 1;
   ```

3. **Verify Index Exists**:
   ```sql
   INFO FOR TABLE memory;
   ```

4. **Test Simple Vector Search**:
   ```sql
   SELECT id, vector::distance::knn() AS distance
   FROM memory 
   WHERE embedding IS NOT NULL 
     AND embedding <|10|> $query_vector 
   ORDER BY distance ASC
   LIMIT 10;
   ```
   
   Note: The `<|10|>` syntax means "find 10 nearest neighbors". The number must match
   or exceed the LIMIT clause. The actual implementation uses this format with the limit
   parameter filled in dynamically.

## Next Steps

1. Add diagnostic logging to check:
   - Actual embedding dimensions
   - Index creation status
   - Query execution details

2. Test with non-optional field to verify if that's the issue

3. Check SurrealDB version-specific behavior for optional fields with HNSW indexes

