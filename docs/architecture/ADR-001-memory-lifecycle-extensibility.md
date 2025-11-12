# RFC 001: Memory Lifecycle & Extensibility Enhancements

**Status**: Draft  
**Authors**: Blake Barnett & AI Assistant  
**Created**: 2025-01-24  
**Target Version**: 0.2.0

## Executive Summary

This RFC proposes enhancements to Locai to better support complex memory systems with dynamic importance tracking, flexible relationship types, and extensibility hooks. These changes maintain Locai's core philosophy as a general-purpose memory substrate while enabling domain-specific applications (AI gaming, personal assistants, knowledge management) to build sophisticated memory dynamics.

## Goals

1. **Automatic Memory Lifecycle Tracking**: Track memory vitality without manual instrumentation
2. **Flexible Relationship Registry**: Support domain-specific relationship types without core changes
3. **Extensibility Hooks**: Enable domain logic through well-defined callback points
4. **Maintain Simplicity**: Keep the default experience simple while enabling power users
5. **Backward Compatibility**: Preserve existing APIs where possible

## Non-Goals

- Domain-specific implementations (e.g., RPG mechanics, calendar systems)
- Prescriptive memory models or decay algorithms
- Built-in AI inference (users bring their own)
- Complex query languages beyond current capabilities

## Current State Analysis

### What Already Exists ✅

Locai already has excellent foundations:

**Memory Model** (`locai/src/models/memory.rs`):
```rust
pub struct Memory {
    pub id: String,
    pub content: String,
    pub memory_type: MemoryType,  // Supports Custom(String)
    pub created_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>,  // ✅ Already exists!
    pub access_count: u32,                      // ✅ Already exists!
    pub priority: MemoryPriority,
    pub tags: Vec<String>,
    pub source: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub properties: serde_json::Value,          // ✅ Flexible metadata
    pub related_memories: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}
```

**Entity Extraction Pipeline** (`locai/src/entity_extraction/`):
```rust
// ✅ Trait-based, pluggable architecture already exists
pub trait RawEntityExtractor: Send + Sync { ... }
pub trait EntityValidator: Send + Sync { ... }
pub trait EntityPostProcessor: Send + Sync { ... }
```

**Relationship Model** (`locai/src/relationships/types.rs`):
```rust
pub struct Relationship {
    pub id: String,
    pub entity_a: String,
    pub entity_b: String,
    pub relationship_type: RelationshipType,  // Currently fixed enum
    pub intensity: f32,
    pub trust_level: f32,
    pub familiarity: f32,
    pub history: Vec<RelationshipEvent>,
    pub metadata: HashMap<String, serde_json::Value>, // ✅ Flexible!
}
```

### What Needs Enhancement 🔧

1. **Memory lifecycle metadata is not automatically maintained** - Fields exist but may not update on retrieval
2. **Relationship types are a fixed enum** - No way to add custom types at runtime
3. **No hook/callback system** - Can't run custom logic on memory operations
4. **No batch operations API** - Consolidation/analysis requires many individual calls

## Proposed Enhancements

### 1. Automatic Memory Lifecycle Tracking

**Problem**: Memory metadata fields (`access_count`, `last_accessed`) exist but require manual updates.

**Proposal**: Automatically update lifecycle metadata on memory retrieval.

```rust
// In storage trait implementation
impl MemoryStore for SharedStorage {
    async fn get_memory(&self, id: &str) -> Result<Memory, StorageError> {
        let mut memory = self.fetch_memory(id).await?;
        
        // Automatic lifecycle tracking
        memory.access_count += 1;
        memory.last_accessed = Some(Utc::now());
        
        // Persist updates (lightweight, async)
        self.update_metadata(id, &memory).await?;
        
        Ok(memory)
    }
}
```

**Configuration**:
```yaml
# locai.yaml
memory:
  lifecycle_tracking:
    enabled: true  # Default: true
    update_on_read: true
    update_on_search: false  # Don't count searches as accesses
```

**Benefits**:
- Zero manual instrumentation required
- Enables forgetting curves and importance calculation
- Backward compatible (fields already exist)

---

### 2. Flexible Relationship Type Registry

**Problem**: `RelationshipType` is a fixed enum - can't add domain-specific types.

```rust
// Current - inflexible
pub enum RelationshipType {
    Friendship,
    Rivalry,
    // ... only 10 types, can't extend
}
```

**Proposal**: Dynamic relationship type system with registration.

```rust
// New relationship type system
pub struct RelationshipTypeRegistry {
    types: HashMap<String, RelationshipTypeDef>,
}

pub struct RelationshipTypeDef {
    pub name: String,
    pub inverse: Option<String>,    // "knows" ↔ "known_by"
    pub symmetric: bool,             // "married_to" is symmetric
    pub transitive: bool,            // "part_of" is transitive
    pub metadata_schema: Option<serde_json::Value>,
}

// Relationship now references type by name
pub struct Relationship {
    pub relationship_type: String,  // Changed from enum to String
    // ... rest unchanged
}

impl RelationshipTypeRegistry {
    pub fn register(&mut self, def: RelationshipTypeDef) -> Result<()>;
    pub fn get(&self, name: &str) -> Option<&RelationshipTypeDef>;
    pub fn seed_common_types(&mut self);  // Loads standard types
}
```

**API Addition**:
```
POST /api/v1/config/relationship-types
{
  "name": "serves",
  "inverse": "served_by",
  "symmetric": false,
  "transitive": false,
  "metadata_schema": {
    "rank": "string",
    "since": "timestamp"
  }
}

GET /api/v1/config/relationship-types
```

**Migration Path**:
```rust
// Phase 1: Add registry alongside existing enum (0.2.0)
// Phase 2: Deprecate enum, prefer registry (0.3.0)
// Phase 3: Remove enum (1.0.0)

// During transition, auto-register enum types
impl From<OldRelationshipType> for String {
    fn from(rt: OldRelationshipType) -> String {
        match rt {
            OldRelationshipType::Friendship => "friendship".to_string(),
            // ...
        }
    }
}
```

**Benefits**:
- Domains can define their own relationship types
- Maintains backward compatibility through migration
- Enables relationship reasoning (symmetry, transitivity)

---

### 3. Memory Operation Hooks

**Problem**: No way to run custom logic during memory lifecycle events.

**Proposal**: Callback system for memory operations.

```rust
// New trait for memory hooks
#[async_trait]
pub trait MemoryHook: Send + Sync {
    /// Called after a memory is created
    async fn on_memory_created(&self, memory: &Memory) -> Result<()> {
        Ok(())
    }
    
    /// Called after a memory is accessed (read)
    async fn on_memory_accessed(&self, memory: &Memory) -> Result<()> {
        Ok(())
    }
    
    /// Called after a memory is updated
    async fn on_memory_updated(&self, old: &Memory, new: &Memory) -> Result<()> {
        Ok(())
    }
    
    /// Called before a memory is deleted (can veto)
    async fn before_memory_deleted(&self, memory: &Memory) -> Result<bool> {
        Ok(true)  // Allow deletion
    }
}

// Registration
pub struct MemoryHookRegistry {
    hooks: Vec<Box<dyn MemoryHook>>,
}

impl MemoryManager {
    pub fn register_hook(&mut self, hook: Box<dyn MemoryHook>) {
        self.hooks.register(hook);
    }
}
```

**Configuration-Based Hooks** (for REST API deployments):
```yaml
# locai.yaml
hooks:
  memory_lifecycle:
    on_created:
      - type: "webhook"
        url: "http://app-server:8000/api/memory-created"
        async: true
    
    on_accessed:
      - type: "webhook"
        url: "http://app-server:8000/api/memory-accessed"
        async: true
        batch: true  # Batch multiple accesses
        
  consolidation:
    enabled: true
    schedule: "0 0 * * *"  # Daily at midnight
    webhook: "http://app-server:8000/api/consolidate"
```

**Use Case - Game World**:
```rust
// Game implements custom hook
struct GameMemoryHook {
    entity_promoter: EntityPromoter,
}

#[async_trait]
impl MemoryHook for GameMemoryHook {
    async fn on_memory_created(&self, memory: &Memory) -> Result<()> {
        // Extract entities mentioned
        let entities = extract_entities(&memory.content).await?;
        
        // Check if any should be promoted to tracked
        for entity in entities {
            if self.entity_promoter.should_track(&entity).await? {
                self.entity_promoter.promote(entity).await?;
            }
        }
        
        Ok(())
    }
}
```

---

### 4. Batch Operations API

**Problem**: Consolidation and analysis require many individual API calls.

**Proposal**: Add batch operations endpoint.

```
POST /api/v1/batch
{
  "operations": [
    {
      "op": "update_metadata",
      "memory_id": "mem_123",
      "updates": {
        "properties": {"importance_score": 0.8}
      }
    },
    {
      "op": "create_relationship",
      "source": "entity_1",
      "target": "entity_2",
      "type": "related_to",
      "properties": {}
    }
  ],
  "transaction": true  // All or nothing
}

Response:
{
  "results": [
    {"status": "success", "id": "mem_123"},
    {"status": "success", "id": "rel_456"}
  ],
  "completed": 2,
  "failed": 0
}
```

**Rust API**:
```rust
pub enum BatchOperation {
    UpdateMetadata { id: String, updates: serde_json::Value },
    CreateRelationship { source: String, target: String, ... },
    DeleteMemory { id: String },
    // ...
}

impl MemoryManager {
    pub async fn batch_execute(
        &self,
        operations: Vec<BatchOperation>,
        transaction: bool
    ) -> Result<Vec<BatchResult>>;
}
```

**Benefits**:
- Efficient consolidation operations
- Reduced network overhead for remote deployments
- Transaction support for consistency

---

### 5. Enhanced Search Scoring

**Problem**: Limited control over search result ranking.

**Proposal**: Expose scoring configuration in search API.

```
POST /api/v1/memories/search
{
  "q": "wizard tower magic",
  "filters": {
    "memory_type": "knowledge",
    "tags": ["magic"]
  },
  "scoring": {
    "bm25_weight": 1.0,
    "vector_weight": 1.0,      // If embeddings present
    "recency_boost": 0.5,      // Boost recent memories
    "access_boost": 0.3,       // Boost frequently accessed
    "priority_boost": 0.2,     // Boost high priority
    "decay_function": "exponential",  // How to decay old memories
    "decay_rate": 0.1
  },
  "limit": 10
}
```

**Current State**: Basic BM25 and vector search already exist - this just exposes more knobs.

---

## Implementation Plan

### Phase 1: Foundation (0.2.0-alpha.1) - 2 weeks

**Goal**: Core infrastructure without breaking changes

1. **Automatic Lifecycle Tracking**
   - Update `get_memory()` to auto-increment access_count
   - Add configuration flag (default: enabled)
   - Write tests
   - **Estimate**: 3 days

2. **Relationship Type Registry (additive)**
   - Create `RelationshipTypeRegistry` struct
   - Keep existing enum (don't break anything)
   - Add registration API
   - Seed common types from enum
   - **Estimate**: 4 days

3. **Basic Hook System**
   - Define `MemoryHook` trait
   - Implement registration
   - Add hook invocation points
   - **Estimate**: 3 days

4. **Documentation**
   - API docs
   - Migration guides
   - Examples
   - **Estimate**: 2 days

### Phase 2: REST API Integration (0.2.0-alpha.2) - 2 weeks

1. **REST Endpoints**
   - Relationship type CRUD
   - Webhook-based hooks
   - Batch operations
   - **Estimate**: 5 days

2. **Configuration System**
   - YAML/JSON config for hooks
   - Environment variable overrides
   - **Estimate**: 3 days

3. **Enhanced Search API**
   - Scoring parameters
   - Query builder enhancements
   - **Estimate**: 3 days

4. **Examples & Tests**
   - Real-world use cases
   - Integration tests
   - **Estimate**: 3 days

### Phase 3: Polish & Optimization (0.2.0) - 1 week

1. Performance tuning
2. Security review
3. Final documentation
4. Migration tooling

---

## Examples & Use Cases

### Use Case 1: AI Gaming (Zera)

**Challenge**: Track narrative entities dynamically, implement forgetting curves.

**Solution**:
```rust
// Register custom relationship types
locai.register_relationship_type(RelationshipTypeDef {
    name: "serves".to_string(),
    inverse: Some("served_by".to_string()),
    symmetric: false,
    transitive: false,
    ..Default::default()
});

// Install hook for entity promotion
locai.register_hook(Box::new(GameEntityHook::new()));

// Query with custom scoring (recency matters in active games)
let results = locai.search("wizard aldric")
    .with_scoring(ScoringConfig {
        recency_boost: 2.0,  // Recent events very important
        access_boost: 1.5,   // Frequently mentioned entities
        ..Default::default()
    })
    .execute().await?;
```

### Use Case 2: Personal AI Assistant

**Challenge**: Remember user preferences, important dates, contacts.

**Solution**:
```rust
// Register contact relationships
locai.register_relationship_type(RelationshipTypeDef {
    name: "contact_of".to_string(),
    metadata_schema: Some(json!({
        "phone": "string",
        "email": "string",
        "last_contacted": "timestamp"
    })),
    ..Default::default()
});

// Automatically track important information
locai.register_hook(Box::new(PreferenceTracker::new()));

// Find relevant context with personalization
let context = locai.search(user_query)
    .with_scoring(ScoringConfig {
        priority_boost: 3.0,  // User-marked important items
        recency_boost: 1.0,
        ..Default::default()
    })
    .execute().await?;
```

### Use Case 3: Knowledge Management

**Challenge**: Build knowledge graphs with custom relationship semantics.

**Solution**:
```rust
// Register knowledge-specific relationships
for rel_type in ["cites", "contradicts", "extends", "implements"] {
    locai.register_relationship_type(RelationshipTypeDef {
        name: rel_type.to_string(),
        transitive: rel_type == "implements",  // Inheritance
        ..Default::default()
    });
}

// Batch import from existing knowledge base
let operations = existing_kb.entries().map(|entry| {
    BatchOperation::CreateMemory { ... }
}).collect();

locai.batch_execute(operations, true).await?;
```

---

## Backward Compatibility

### Breaking Changes

**None in Phase 1** - All additions are opt-in or additive.

**Potential in Later Phases**:
- Relationship enum to string migration (with deprecation period)
- Default lifecycle tracking (can be disabled)

### Migration Path

```rust
// Old code continues to work
let rel = Relationship {
    relationship_type: RelationshipType::Friendship,  // Old enum
    // ...
};

// New code can use strings
let rel = Relationship {
    relationship_type: "custom_game_alliance".to_string(),  // New way
    // ...
};

// Both stored the same way internally
```

---

## Alternatives Considered

### Alternative 1: Keep Everything Domain-Specific

**Rejected**: Would make Locai less useful as a general library. Each domain would need to fork or wrap extensively.

### Alternative 2: Build Full ORM-Style Abstraction

**Rejected**: Too opinionated, heavyweight. Locai should be a substrate, not a framework.

### Alternative 3: Require All Extension Via External Services

**Rejected**: Makes embedded use cases (Locai as a library) difficult. Webhooks are great for services, but library users need native hooks.

---

## Open Questions

1. **Hook Performance**: Should hooks be fire-and-forget async or synchronous? 
   - **Recommendation**: Async by default, config option for sync/blocking

2. **Relationship Type Versioning**: If a type definition changes, what happens to existing relationships?
   - **Recommendation**: Store type definition version with relationship, allow migration API

3. **Batch Operation Limits**: What's a reasonable max batch size?
   - **Recommendation**: 1000 operations per batch, configurable

4. **Hook Failure Handling**: If a hook fails, does the memory operation fail?
   - **Recommendation**: Hooks can't fail memory operations (logged only), unless they veto (e.g., `before_delete`)

---

## Success Metrics

1. **Adoption**: 3+ diverse projects using enhanced features within 6 months
2. **Performance**: No > 5% regression in baseline operations
3. **Documentation**: All features have working examples
4. **Community**: Positive feedback, PRs extending the system

---

## References

- [Locai Entity Extraction Architecture](/home/bdb/git/locai/docs/ENTITY_EXTRACTION.md)
- [Locai Feature Flags](/home/bdb/git/locai/docs/FEATURES.md)
- [SurrealDB Embedded Storage](https://surrealdb.com/)
- [ConceptNet Relationship Types](https://github.com/commonsense/conceptnet5/wiki/Relations)

---

## Appendix A: API Reference

### New Configuration Types

```rust
pub struct LifecycleConfig {
    pub enabled: bool,
    pub update_on_read: bool,
    pub update_on_search: bool,
}

pub struct RelationshipTypeDef {
    pub name: String,
    pub inverse: Option<String>,
    pub symmetric: bool,
    pub transitive: bool,
    pub metadata_schema: Option<serde_json::Value>,
}

pub struct ScoringConfig {
    pub bm25_weight: f32,
    pub vector_weight: f32,
    pub recency_boost: f32,
    pub access_boost: f32,
    pub priority_boost: f32,
    pub decay_function: DecayFunction,
    pub decay_rate: f32,
}

pub enum DecayFunction {
    None,
    Linear,
    Exponential,
    Logarithmic,
}
```

### New REST Endpoints

```
# Relationship Types
POST   /api/v1/config/relationship-types
GET    /api/v1/config/relationship-types
GET    /api/v1/config/relationship-types/{name}
DELETE /api/v1/config/relationship-types/{name}

# Batch Operations
POST   /api/v1/batch

# Enhanced Search
POST   /api/v1/memories/search  (extended with scoring)
```

---

**Discussion Period**: 2 weeks from publication  
**Target Merge**: 2025-02-15  
**Implementation Start**: Upon approval

