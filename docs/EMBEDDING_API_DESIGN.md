# Embedding API Design Considerations

## Overview

This document outlines critical considerations for adding user-provided embedding support to the Locai API.

## Current State

### ✅ Already Implemented

1. **Vector Record Lifecycle**: Automatically creates/updates/deletes Vector records when memory embeddings change
2. **Webhook Support**: Full Memory objects (including embeddings) are serialized in webhooks
3. **Storage Support**: Memory model supports embeddings via `with_embedding()` method
4. **Validation Infrastructure**: `EmbeddingManager` provides validation and normalization utilities

### ❌ Missing from API

1. **CreateMemoryRequest**: No `embedding` field
2. **UpdateMemoryRequest**: No `embedding` field  
3. **BatchOperation::CreateMemory**: Hardcoded to `embedding: None`
4. **API Response**: Embeddings are excluded from `MemoryDto` (by design)

## Critical Design Decisions

### 1. Dimension Requirements

**Current Constraint**: SurrealDB vector index requires 1024 dimensions (BGE-M3 compatible)

**Options**:
- **Option A (Strict)**: Enforce 1024 dimensions, reject others
  - ✅ Ensures compatibility with vector search
  - ✅ Prevents dimension mismatches
  - ❌ Limits flexibility for users with different models
  
- **Option B (Flexible)**: Allow any dimensions, validate against configured model
  - ✅ Supports multiple embedding models
  - ✅ More flexible for BYOE use cases
  - ❌ Requires dimension configuration
  - ❌ May break vector search if dimensions don't match

- **Option C (Hybrid)**: Allow 1024 or configured dimensions, warn on mismatch
  - ✅ Best of both worlds
  - ✅ Clear error messages
  - ⚠️ Requires dimension configuration

**Recommendation**: **Option A** for initial implementation (strict 1024), with clear error messages pointing to dimension requirements.

### 2. Normalization

**Current State**: Embeddings should be normalized for cosine similarity, but normalization is not automatic.

**Options**:
- **Option A**: Auto-normalize all user-provided embeddings
  - ✅ Ensures consistency
  - ✅ Prevents search issues
  - ❌ May modify user's data
  
- **Option B**: Validate normalization, reject if not normalized
  - ✅ Preserves user's exact data
  - ❌ Requires users to normalize themselves
  
- **Option C**: Optional normalization flag in request
  - ✅ Flexible
  - ⚠️ More complex API

**Recommendation**: **Option A** - Auto-normalize with a configurable option to disable.

### 3. Conflict Handling (ML Service + User Embedding) ⭐ CRITICAL

**Scenario**: ML service is configured AND user provides embedding

**Current State**: No auto-generation exists - pure BYOE approach

**Proposed Hybrid Approach**:
```
IF user provides embedding:
    → Use user's embedding (even if ML service configured)
    → Skip ML service call (performance optimization)
    → Validate and normalize user's embedding
    
ELSE IF ML service is configured:
    → Auto-generate embedding using ML service
    → Normalize generated embedding
    
ELSE:
    → No embedding (memory stored without embedding)
```

**Conflict Resolution Strategy**:
- **User-provided takes precedence**: If embedding is provided in API request, use it regardless of ML service configuration
- **No conflicts possible**: User-provided and auto-generated are mutually exclusive (one or the other, never both)
- **Performance**: Skip expensive ML service call if user provides embedding
- **Validation**: Both paths validate and normalize embeddings

**Potential Issues**:
1. **Dimension Mismatch**: User provides 768-dim, ML service generates 1024-dim
   - ✅ **Solution**: User's embedding is used as-is (user's choice)
   - ⚠️ **Warning**: May break vector search if dimensions don't match index
   - **Recommendation**: Validate dimensions match configured model (1024 for SurrealDB)

2. **Model Mismatch**: User provides embedding from Model A, ML service uses Model B
   - ✅ **Solution**: User's embedding is used (user's choice)
   - ⚠️ **Warning**: Search quality may vary if models are incompatible
   - **Recommendation**: Document that embeddings should be from compatible models

3. **Normalization**: User provides unnormalized embedding
   - ✅ **Solution**: Auto-normalize before storage (always)
   - ✅ **Benefit**: Ensures consistent cosine similarity calculations

**Implementation Logic**:
```rust
pub async fn store_memory(&self, mut memory: Memory) -> Result<String> {
    // Step 1: Check if user provided embedding
    if memory.embedding.is_none() {
        // Step 2: Auto-generate if ML service available
        if let Some(ml_service) = &self.ml_service {
            match ml_service.generate_embedding(&memory.content).await {
                Ok(embedding) => {
                    memory.embedding = Some(embedding);
                    tracing::debug!("Auto-generated embedding for memory {}", memory.id);
                }
                Err(e) => {
                    tracing::warn!("Failed to auto-generate embedding: {}", e);
                    // Continue without embedding - don't fail memory storage
                }
            }
        }
    }
    
    // Step 3: Validate and normalize embedding (if present)
    if let Some(embedding) = &mut memory.embedding {
        // Validate dimensions, NaN, etc.
        self.validate_embedding(embedding)?;
        // Normalize for cosine similarity
        self.normalize_embedding(embedding)?;
    }
    
    // Step 4: Store memory (existing logic)
    // ...
}
```

**Recommendation**: **Hybrid Approach** - Best of both worlds:
- ✅ Supports BYOE (users can provide their own)
- ✅ Supports auto-generation (convenience when ML service configured)
- ✅ No conflicts (mutually exclusive)
- ✅ Performance optimized (skip ML call if user provides)
- ✅ Backward compatible (existing BYOE code still works)

### 4. Validation Requirements

**Required Validations**:
- ✅ Non-empty vectors
- ✅ Dimension check (1024 or configured)
- ✅ Finite values (no NaN/infinity)
- ⚠️ Zero vector detection (add)
- ⚠️ Normalization check (add as warning)

**Implementation**: Use `EmbeddingManager.validate_embedding()` with dimension check.

### 5. API Response Design

**Current**: Embeddings excluded from `MemoryDto` (security/performance)

**Options**:
- **Option A**: Keep excluded (current)
  - ✅ Smaller payloads
  - ✅ Security (embeddings are large)
  - ❌ Users can't verify stored embeddings
  
- **Option B**: Include in response
  - ✅ Full transparency
  - ❌ Large payloads
  - ❌ Security concerns

- **Option C**: Optional query parameter `?include_embedding=true`
  - ✅ Flexible
  - ✅ Best of both worlds
  - ⚠️ More complex

**Recommendation**: **Option A** - Keep excluded by default. Add `?include_embedding=true` if needed later.

### 6. Batch Operations

**Current**: `BatchOperation::CreateMemory` hardcodes `embedding: None`

**Required Changes**:
- Add optional `embedding: Option<Vec<f32>>` to `BatchOperation::CreateMemory`
- Add optional `embedding: Option<Vec<f32>>` to `BatchOperation::UpdateMemory`
- Apply same validation as single operations

### 7. Additional Endpoints (Future)

Consider adding:
- `POST /api/memories/{id}/embedding` - Update only embedding
- `GET /api/memories/{id}/embedding` - Get embedding only
- `POST /api/embeddings/validate` - Validate embedding before storing

## Implementation Checklist

### Phase 1: Core Support
- [ ] Add `embedding: Option<Vec<f32>>` to `CreateMemoryRequest`
- [ ] Add `embedding: Option<Vec<f32>>` to `UpdateMemoryRequest`
- [ ] Add validation in `create_memory()` endpoint
- [ ] Add validation in `update_memory()` endpoint
- [ ] Add `embedding: Option<Vec<f32>>` to `BatchOperation::CreateMemory`
- [ ] Add `embedding: Option<Vec<f32>>` to `BatchOperation::UpdateMemory`
- [ ] Update batch executor to handle embeddings

### Phase 2: Validation & Normalization
- [ ] Integrate `EmbeddingManager` validation
- [ ] Enforce 1024 dimension requirement (or configured)
- [ ] Auto-normalize embeddings before storage
- [ ] Add zero vector detection
- [ ] Add clear error messages for validation failures

### Phase 3: Documentation & Testing
- [ ] Update API documentation
- [ ] Add examples with embeddings
- [ ] Add integration tests
- [ ] Document dimension requirements
- [ ] Document normalization behavior

### Phase 4: Advanced Features (Future)
- [ ] Optional `include_embedding` query parameter
- [ ] Dedicated embedding endpoints
- [ ] Embedding validation endpoint
- [ ] Support for multiple dimension configurations

## Webhook & Lifecycle Considerations

### ✅ Already Handled

1. **Webhooks**: Memory objects include embeddings automatically (via serialization)
2. **Vector Sync**: Vector records automatically created/updated/deleted
3. **Lifecycle Events**: `memory.created`, `memory.updated` already fire for embedding changes

### ⚠️ Considerations

1. **Webhook Payload Size**: Embeddings are large (1024 floats = ~4KB). Consider:
   - Current: Full memory serialized (includes embedding)
   - Option: Add webhook config to exclude embeddings
   - Option: Add webhook config for embedding-only events

2. **Event Granularity**: Should embedding updates trigger separate events?
   - Current: `memory.updated` fires for any update
   - Option: Add `memory.embedding.updated` event
   - Recommendation: Keep current (embedding update = memory update)

## Security Considerations

1. **Payload Size**: Large embeddings could be used for DoS
   - Limit: Max embedding size validation
   - Rate limiting: Already in place

2. **Data Privacy**: Embeddings may contain sensitive information
   - Current: Excluded from API responses (good)
   - Consider: Audit logging for embedding updates

3. **Validation Costs**: Validation is CPU-intensive
   - Current: Synchronous validation
   - Consider: Async validation for large batches

## Performance Considerations

1. **Storage**: Vector records are automatically created (good)
2. **Indexing**: SurrealDB indexes embeddings automatically
3. **Search**: Vector search requires dimension match
4. **Batch Operations**: Validate all embeddings before processing

## Error Messages

Provide clear, actionable error messages:

```json
{
  "error": "Invalid embedding",
  "message": "Embedding validation failed",
  "details": {
    "dimension": {
      "expected": 1024,
      "actual": 768,
      "message": "SurrealDB vector index requires 1024 dimensions (BGE-M3 compatible)"
    },
    "normalization": {
      "norm": 0.95,
      "message": "Embedding should be normalized (norm ≈ 1.0). Auto-normalization applied."
    }
  }
}
```

## Testing Strategy

1. **Unit Tests**:
   - Validation logic
   - Normalization logic
   - Dimension checks

2. **Integration Tests**:
   - Create memory with embedding
   - Update memory embedding
   - Batch operations with embeddings
   - Vector record synchronization
   - Webhook payload verification

3. **Edge Cases**:
   - Zero vectors
   - NaN/infinity values
   - Wrong dimensions
   - Unnormalized embeddings
   - Large batch operations

## Migration Considerations

1. **Backward Compatibility**: 
   - ✅ Optional field (backward compatible)
   - ✅ Existing memories unaffected

2. **Existing Embeddings**:
   - ✅ Already stored correctly
   - ✅ Vector records already exist

3. **API Versioning**:
   - ✅ No breaking changes
   - ✅ Can add to v1 API

## Conclusion

Adding user-provided embedding support is **critical** for BYOE (Bring Your Own Embedding) use cases. The infrastructure is already in place, but API support is missing. Key considerations:

1. **Dimension enforcement** (1024 requirement)
2. **Auto-normalization** (for consistency)
3. **Validation** (comprehensive checks)
4. **Webhook support** (already works!)
5. **Vector sync** (already works!)

The main work is adding the API fields and validation logic. Lifecycle and webhook support are already handled correctly.

