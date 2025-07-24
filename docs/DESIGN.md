# Locai: Advanced Memory Management for AI Systems

## Executive Summary

Locai is a sophisticated memory management system designed for AI agents and applications, providing persistent storage and intelligent retrieval through advanced BM25 search, semantic capabilities, and graph relationships. Built in Rust for performance and safety, Locai functions both as a standalone service (via REST API) and as an embeddable library for direct integration.

## Core Design Principles

- **Graph-First Memory**: Prioritize graph-based memory representation for rich relationships
- **Search Excellence**: Professional-grade BM25 search as the primary retrieval method  
- **Pipeline Architecture**: Composable pipelines for entity extraction and processing
- **Real-time Capabilities**: Live queries and messaging integration for reactive applications
- **Self-Contained**: Single process operation with local storage options
- **Performance**: Maximize throughput with Rust's safety guarantees
- **Extensibility**: Abstract database operations to allow future backend changes
- **Dual-Use**: Function effectively as both a service and an embeddable library

## Implementation Status

### ✅ Completed

- **Core Memory Management**: Full CRUD operations with rich metadata
- **Graph Storage**: SurrealDB integration for unified graph and vector operations
- **BM25 Search**: Industry-standard full-text search with custom analyzers (see [Search Architecture](SEARCH.md))
- **Entity Extraction Pipeline**: Flexible trait-based system (see [Entity Extraction](ENTITY_EXTRACTION.md))
- **Live Queries**: Real-time change notifications (see [Live Queries](LIVE_QUERIES.md))
- **HTTP API**: RESTful endpoints with WebSocket support (see [API Reference](API.md))
- **CLI Tools**: Command-line interface for operations
- **Messaging System**: Embedded and remote messaging capabilities

### 🚧 In Progress

- **Graph Visualization Frontend**: Interactive D3.js-based visualization
- **Advanced ML Integrations**: Enhanced NER models and embeddings

### 📋 Planned

- **RAG Fusion**: Query expansion and result fusion for improved search
- **Memory Consolidation**: Importance scoring and summarization
- **Full Versioning System**: Git-like branching for memory versions
- **Performance Benchmarks**: Comprehensive performance testing suite

## Crate Structure

Locai is structured as a Rust workspace with multiple crates:

```
locai/
├── Cargo.toml                   # Workspace definition
├── README.md
├── locai/                       # Core library crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs              # Public API
│   │   ├── core/               # Core memory management
│   │   ├── storage/            # Storage backends
│   │   ├── memory/             # Memory operations
│   │   ├── entity_extraction/  # Entity extraction pipeline
│   │   ├── ml/                 # Machine learning integrations
│   │   └── simple.rs           # Simplified API
├── locai-server/               # HTTP API service crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs            # Server entry point
│   │   ├── api/               # REST endpoints
│   │   └── websocket/         # Real-time features
├── locai-cli/                  # Command-line tools
│   ├── Cargo.toml
│   └── src/
│       └── main.rs            # CLI entry point
└── frontend/                   # Web UI (in development)
    ├── package.json
    └── src/                    # React/TypeScript application
```

## System Architecture

### High-Level Components

```
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│               │      │               │      │               │
│  Simple API   │──────│  Core Memory  │──────│  Storage      │
│  (Locai)      │      │  Manager      │      │  Traits       │
│               │      │               │      │               │
└───────┬───────┘      └───────┬───────┘      └───────┬───────┘
        │                      │                      │
        │                      │                      │
┌───────▼───────┐      ┌───────▼───────┐      ┌───────▼───────┐
│               │      │               │      │               │
│  HTTP API     │      │  ML Services  │      │  SurrealDB    │
│  (Optional)   │      │  (ModelMgr)   │      │  (Unified)    │
│               │      │               │      │               │
└───────────────┘      └───────┬───────┘      └───────────────┘
                              │                               
                              │                               
                       ┌──────▼────────┐                      
                       │               │                      
                       │Entity Extract │                      
                       │Pipeline       │                      
                       │               │                      
                       └───────────────┘                      
```

## Public API Design

The library exposes multiple levels of API for different use cases:

### Simple API (Recommended)

```rust
use locai::Locai;

// Dead simple initialization
let locai = Locai::new().await?;

// Store memories
locai.remember("Important information").await?;
locai.remember_fact("The speed of light is 299,792,458 m/s").await?;

// Search with BM25
let results = locai.search("speed of light").await?;
```

### Builder Pattern

```rust
use locai::Locai;

// Custom configuration
let locai = Locai::builder()
    .with_data_dir("./custom_data")
    .with_embedding_model("BAAI/bge-small-en")
    .build().await?;

// Advanced memory creation
locai.remember_with("Important discovery")
    .as_fact()
    .with_priority(MemoryPriority::High)
    .with_tags(&["science", "breakthrough"])
    .save().await?;
```

### Direct MemoryManager API

```rust
use locai::prelude::*;

// For advanced use cases
let config = ConfigBuilder::new()
    .with_default_storage()
    .with_default_ml()
    .build()?;

let memory_manager = locai::init(config).await?;
```

See the [Architecture Overview](ARCHITECTURE.md) for detailed component descriptions.

## Feature Flags

The library uses feature flags for optional components (see [Feature Flags](FEATURES.md)):

```toml
[features]
default = ["surrealdb-embedded", "candle-embeddings"]
surrealdb-embedded = ["surrealdb", "kv-mem", "kv-rocksdb"]
surrealdb-remote = ["surrealdb", "protocol-ws", "protocol-http"]
candle-embeddings = ["candle-core", "candle-nn", "candle-transformers"]
cuda = ["candle-embeddings", "candle-core/cuda"]
```

## Core Components

### 1. Storage Architecture

The storage layer uses a trait-based design for extensibility:

```rust
pub trait BaseStore: Send + Sync {
    async fn health_check(&self) -> Result<bool>;
    async fn clear(&self) -> Result<()>;
}

pub trait MemoryStore: BaseStore {
    async fn create_memory(&self, memory: Memory) -> Result<Memory>;
    async fn get_memory(&self, id: &str) -> Result<Option<Memory>>;
    async fn update_memory(&self, memory: Memory) -> Result<Memory>;
    async fn delete_memory(&self, id: &str) -> Result<bool>;
}

pub trait EntityStore: BaseStore {
    async fn create_entity(&self, entity: Entity) -> Result<Entity>;
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>>;
}

pub trait GraphStore: MemoryStore + EntityStore + RelationshipStore {}
```

### 2. Search Architecture

Locai implements sophisticated search capabilities using SurrealDB's native BM25:

- **BM25 Full-Text Search**: Industry-standard relevance ranking
- **Custom Analyzers**: Specialized tokenization and stemming
- **Fuzzy Matching**: Typo-tolerant search
- **Temporal Search**: Time-based filtering
- **Hybrid Capabilities**: Combine text and metadata filters

See [Search Architecture](SEARCH.md) for complete details.

### 3. Entity Extraction Pipeline

The entity extraction system uses a composable pipeline:

```rust
let pipeline = EntityExtractionPipeline::builder()
    .extractor(Box::new(ml_extractor))
    .validator(Box::new(ConfidenceValidator::new(0.8)))
    .post_processor(Box::new(EntityDeduplicator::new()))
    .build()?;
```

See [Entity Extraction](ENTITY_EXTRACTION.md) for the complete architecture.

### 4. Live Query System

Real-time change notifications via WebSocket:

```javascript
const client = new LocaiLiveClient("ws://localhost:3000/api/ws");
client.subscribe({
    memory_filter: { memory_type: "episodic" }
});
```

See [Live Queries](LIVE_QUERIES.md) for implementation details.

### 5. Unified Storage with SurrealDB

SurrealDB provides both graph and vector operations in a single database:

```rust
// Native BM25 search
SELECT *, search::score(0) AS bm25_score
FROM memory 
WHERE content @0@ $query
ORDER BY bm25_score DESC

// Graph traversal
SELECT ->relates->entity
FROM memory:$id
WHERE relationship_type = 'mentions'

// Vector operations (when implemented)
SELECT * FROM memory 
WHERE embedding <|10,COSINE|> $query_vector
```

## Library Usage Examples

### Basic Memory Management

```rust
use locai::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize with defaults
    let locai = Locai::new().await?;
    
    // Add memories
    locai.remember("The user prefers dark mode interfaces").await?;
    locai.remember_fact("Paris is the capital of France").await?;
    
    // Search memories
    let results = locai.search("user preferences").await?;
    
    for memory in results {
        println!("Found: {}", memory.content);
    }
    
    Ok(())
}
```

### Advanced Graph Operations

```rust
// Get memory with its graph
let graph = locai.manager()
    .get_memory_graph("memory_id", 2)
    .await?;

println!("Memory has {} related entities", graph.entities.len());
```

### Entity Extraction

```rust
// Automatic entity extraction during memory creation
let memory_id = locai.remember(
    "Meeting with John Smith at Google HQ tomorrow"
).await?;

// Extracted entities: "John Smith" (Person), "Google HQ" (Organization/Location)
```

## Architectural Decisions

### Why BM25 as Primary Search?

1. **Immediate Results**: No embedding computation required
2. **Explainable**: Clear keyword matches vs opaque vector similarity
3. **Performance**: Sub-millisecond query times
4. **Storage Efficient**: No vector storage overhead
5. **Production Ready**: Battle-tested algorithm

Vector search remains available as an optional enhancement for semantic similarity use cases.

### Why Pipeline Architecture for Entity Extraction?

1. **Flexibility**: Swap extractors without changing code
2. **Composability**: Mix and match validators and processors
3. **Domain Agnostic**: Core library has no domain-specific code
4. **Extensibility**: Easy to add custom extractors

### Why SurrealDB?

1. **Unified Storage**: Graph and vector operations in one database
2. **Native BM25**: Built-in full-text search with analyzers
3. **Real-time**: Live queries out of the box
4. **Embedded Option**: Can run in-process or remote
5. **Schema Flexibility**: Adapts to changing requirements

## Development Roadmap

### Current Focus

1. **Production Hardening**: Stability and performance optimization
2. **Documentation**: Comprehensive guides and examples
3. **Graph Visualization**: Complete the frontend implementation

### Future Enhancements

1. **RAG Fusion**: Implement query expansion and result fusion
2. **Memory Consolidation**: Automatic summarization of related memories
3. **Advanced Versioning**: Git-like branching for memory evolution
4. **Multi-modal Support**: Images, audio, and video memories
5. **Distributed Operation**: Multi-node deployment capabilities

## Testing Strategy

### Current Testing

- **Unit Tests**: Comprehensive coverage of core functionality
- **Integration Tests**: End-to-end scenarios with real databases
- **Example Programs**: Demonstrate all major features

### Planned Testing

- **Performance Benchmarks**: Throughput and latency measurements
- **Load Testing**: Concurrent operation stress tests
- **Fuzzing**: Input validation robustness

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development setup and guidelines.

Key areas for contribution:
- Graph visualization frontend
- Additional entity extractors
- Performance optimizations
- Documentation improvements

## Conclusion

Locai has successfully evolved from its initial design into a sophisticated memory management system. The architecture maintains the original vision of graph-first memory while incorporating advanced features like BM25 search and real-time updates. The dual-use design (library and service) provides maximum flexibility for different deployment scenarios.

The focus on production readiness, with professional-grade search and a clean API, positions Locai as a robust solution for AI memory management needs. 