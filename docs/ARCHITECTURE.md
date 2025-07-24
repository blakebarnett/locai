# Architecture Overview

## System Architecture

Locai is designed as a modular memory management system for AI applications, built with a focus on performance, extensibility, and ease of use.

### Core Components

#### Locai Library (`locai`)
The core library provides:
- Memory storage and retrieval
- Entity extraction and relationship mapping
- Search functionality using BM25 full-text search
- Plugin architecture for ML models
- Unified storage interface

#### Server Component (`locai-server`)
HTTP API server providing:
- RESTful endpoints for all operations
- WebSocket support for real-time updates
- Swagger UI documentation
- Health monitoring

#### CLI Tool (`locai-cli`)
Command-line interface for:
- Memory management operations
- System administration
- Development utilities

### Storage Architecture

Locai uses a trait-based storage architecture allowing multiple backend implementations:

```rust
pub trait BaseStore: Send + Sync {
    async fn health_check(&self) -> Result<bool>;
    async fn clear(&self) -> Result<()>;
}

pub trait MemoryStore: BaseStore {
    async fn create_memory(&self, memory: Memory) -> Result<Memory>;
    async fn get_memory(&self, id: &str) -> Result<Option<Memory>>;
    async fn search_memories(&self, query: &str) -> Result<Vec<Memory>>;
}
```

Currently, SurrealDB is the primary storage backend, providing:
- Graph database capabilities for entity relationships
- Built-in BM25 full-text search
- ACID transactions
- Embedded or remote deployment options

### Entity Extraction Pipeline

The entity extraction system uses a composable pipeline architecture:

1. **Text Analysis**: Tokenization and preprocessing
2. **Entity Detection**: Configurable extractors for different entity types
3. **Relationship Discovery**: Automatic relationship inference
4. **Storage Integration**: Entities and relationships stored in graph database

### Search Architecture

Search capabilities are built on SurrealDB's native features:

1. **BM25 Full-Text Search**: Industry-standard relevance ranking
2. **Custom Analyzers**: Specialized text processing for different content types
3. **Fuzzy Matching**: Typo-tolerant search
4. **Metadata Filtering**: Structured queries on tags and properties

### Model Management

The ML model management system supports:
- Dynamic model loading
- Multiple model types (embeddings, NER, sentiment)
- Hardware acceleration (CUDA, Metal)
- Caching for performance

## Data Flow

### Memory Creation Flow

```
User Input → API Layer → Validation → Entity Extraction → Storage → Response
                                           ↓
                                    Relationship Mapping
```

### Search Flow

```
Query → Query Analysis → Strategy Selection → Index Search → Result Ranking → Response
```

## Deployment Options

### Embedded Mode
- Single binary deployment
- Uses embedded SurrealDB
- Suitable for desktop applications
- No external dependencies

### Server Mode
- Multi-user deployment
- HTTP/WebSocket API
- Horizontal scaling capability
- Remote SurrealDB support

### Cloud Deployment
- Containerized deployment
- Kubernetes support
- Distributed storage backend
- Load balancing

## Performance Considerations

### Optimization Strategies
- Lazy loading for large datasets
- Connection pooling for database access
- Parallel processing for entity extraction
- Caching for frequently accessed data

### Scalability
- Stateless API design
- Database connection pooling
- Async/await throughout
- Efficient memory usage

## Security Model

### Current State (Alpha)
- No authentication implemented
- Local deployment focus
- Trust-based access model

### Future Security Features
- JWT-based authentication
- Role-based access control
- Encrypted storage options
- API rate limiting

## Extension Points

### Custom Entity Extractors
Implement the `EntityExtractor` trait:

```rust
pub trait EntityExtractor: Send + Sync {
    fn extract_entities(&self, text: &str) -> Vec<Entity>;
}
```

### Storage Backends
Implement storage traits for new backends:
- `BaseStore`
- `MemoryStore`
- `EntityStore`
- `VectorStore`

### Search Strategies
Add custom search strategies by implementing:
- Query analyzers
- Scoring algorithms
- Result post-processing

## Future Architecture Plans

1. **Distributed Processing**: Multi-node entity extraction
2. **Event Streaming**: Kafka/Redis integration for real-time updates
3. **Plugin System**: Dynamic loading of extractors and processors
4. **Multi-tenancy**: Isolated storage per user/organization 