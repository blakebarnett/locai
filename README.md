# Locai

Advanced memory management system for AI agents and applications, providing persistent storage and intelligent retrieval through professional-grade BM25 search, semantic capabilities, and graph relationships.

## Features

- **BM25 Full-Text Search**: Industry-standard relevance ranking with custom analyzers  
- **BYOE (Bring Your Own Embeddings)**: Maximum provider flexibility - OpenAI, Cohere, local models, etc.
- **Graph-Based Memory**: Rich relationship mapping between memories and entities
- **Entity Extraction Pipeline**: Composable, trait-based entity extraction system
- **Live Queries**: Real-time change notifications via WebSocket
- **Dual-Use Design**: Functions as both an embeddable library and REST API service
- **Unified Storage**: SurrealDB provides graph and vector operations in a single database
- **Minimal Dependencies**: Lightweight by default, optional ML features for advanced users

## Quick Start

### As a Library

Add Locai to your Rust project:

```toml
[dependencies]
locai = "0.1.0-alpha.1"
```

Basic usage:

```rust
use locai::Locai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with defaults
    let locai = Locai::new().await?;

    // Store memories
    locai.remember("The user prefers dark mode interfaces").await?;
    locai.remember_fact("Rust was created by Mozilla Research").await?;
    
    // Search memories using BM25
    let results = locai.search("user preferences").await?;
    
    for memory in results {
        println!("Found: {}", memory.content);
    }
    
    Ok(())
}
```

### As a Service

Run the HTTP API server:

```bash
cargo run --bin locai-server
```

The server provides:
- REST API at `http://localhost:3000`
- Interactive API documentation at `/docs`
- WebSocket endpoint for live queries at `/api/ws`

### CLI Usage

```bash
# Store a memory
cargo run --bin locai-cli memory create "Important information to remember"

# Search memories  
cargo run --bin locai-cli memory search "important"

# List recent memories
cargo run --bin locai-cli memory list --limit 10
```

## BYOE (Bring Your Own Embeddings)

Locai follows a **BYOE (Bring Your Own Embeddings)** approach, focusing on its core strengths while giving you maximum flexibility for embedding providers.

### Philosophy

- **BM25 First**: Professional-grade text search that works out of the box
- **Embeddings Optional**: Vector search enhances but doesn't replace text search
- **Provider Choice**: Use any embedding service (OpenAI, Cohere, local models)
- **No Vendor Lock-in**: Switch providers without changing Locai code

### Quick Start with BYOE

```rust
// 1. BM25 search works immediately (no embeddings needed)
let locai = Locai::new().await?;
let results = locai.search("your query").await?;  // Always works!

// 2. Add embeddings for hybrid search
let embedding = your_provider.embed("text").await?;
let memory = MemoryBuilder::new()
    .content("text")
    .embedding(embedding)  // ← You provide the embedding
    .build();

locai.create_memory(memory).await?;

// 3. Hybrid search combines BM25 + vector similarity
let query_embedding = your_provider.embed("query").await?;
let results = locai.search_for("query")
    .mode(SearchMode::Hybrid)
    .with_query_embedding(query_embedding)
    .execute().await?;
```

### Supported Providers

| Provider | Model | Dimensions | Example Usage |
|----------|-------|------------|---------------|
| **OpenAI** | text-embedding-3-small | 1536 | `openai.embed(text).await?` |
| **Cohere** | embed-english-v3.0 | 1024 | `cohere.embed_document(text).await?` |
| **Azure** | text-embedding-ada-002 | 1536 | `azure_openai.embed(text).await?` |
| **Local** | fastembed, Ollama | Various | `model.embed(text)?` |
| **Custom** | Any provider | Any size | Full flexibility |

### Examples

#### OpenAI Embeddings
```rust
// examples/basic/byoe_openai_embeddings.rs
let openai_client = OpenAIClient::new(api_key);
let embedding = openai_client.embed_text("your text").await?;

let memory = MemoryBuilder::new()
    .content("your text")
    .embedding(embedding)
    .build();
```

#### Cohere Embeddings
```rust
// examples/basic/byoe_cohere_embeddings.rs
let cohere_client = CohereClient::new(api_key);
let doc_embedding = cohere_client.embed_document("document text").await?;
let query_embedding = cohere_client.embed_query("search query").await?;
```

#### Local Embeddings
```rust
// examples/hybrid_search_local.rs
let model = fastembed::TextEmbedding::try_new(Default::default())?;
let embeddings = model.embed(texts, None)?;

for (text, embedding) in texts.iter().zip(embeddings) {
    let memory = MemoryBuilder::new()
        .content(text)
        .embedding(embedding.to_vec())
        .build();
}
```

### Search Modes

#### Text Search (BM25)
- ✅ **Always available** - works without any embeddings
- ✅ **Sub-millisecond** - fastest search mode
- ✅ **Exact matches** - perfect for keyword searches
- ✅ **No infrastructure** - no embedding costs or setup

#### Vector Search
- ✅ **Semantic similarity** - understands meaning, not just keywords
- ✅ **Typo tolerance** - finds relevant content despite spelling errors
- ✅ **Cross-language** - can work across languages with right embeddings
- ⚠️ **Requires embeddings** - both documents and queries need embeddings

#### Hybrid Search
- ✅ **Best of both worlds** - combines BM25 precision with vector recall
- ✅ **Reciprocal Rank Fusion** - intelligently merges results
- ✅ **Production ready** - used by modern search engines
- ⚠️ **Higher latency** - due to dual search and embedding generation

### Cost Comparison

```bash
# Example: 10,000 queries/day
Text Search:     $0.00/day  (no embedding costs)
Vector Search:   $2.00/day  (OpenAI text-embedding-3-small)
Hybrid Search:   $2.00/day  (same embedding costs as vector)
Local Models:    $0.00/day  (hardware/compute costs only)
```

### Performance Comparison

```bash
# Run the comparison example
cargo run --example search_mode_comparison

# Typical results:
Text Search:     ~5ms   (BM25 only)
Vector Search:   ~155ms (150ms embedding + 5ms similarity)
Hybrid Search:   ~165ms (150ms embedding + 15ms dual search)
Local Embeddings: ~15ms  (10ms embedding + 5ms similarity)
```

### Migration Guide

#### From Complex Embedding Setup
```rust
// Before: Complex model management
let model_manager = ModelManagerBuilder::new()
    .cache_dir("./models")
    .default_embedding_model("BAAI/bge-small-en")
    .build();
let model = model_manager.get_embedding_model("model-id").await?;
let embedding = model.embed_text("text", None).await?;

// After: Simple BYOE
let embedding = your_provider.embed("text").await?;
let memory = MemoryBuilder::new()
    .content("text")
    .embedding(embedding)
    .build();
```

#### From Vector-Only Systems
```rust
// BYOE gives you both vector AND text search
let results = locai.search("query").await?;  // Fast BM25
let results = locai.search_for("query")      // Semantic vector
    .mode(SearchMode::Vector)
    .with_query_embedding(embedding)
    .execute().await?;
```

### Best Practices

1. **Start with BM25**: Always works, very fast, no setup required
2. **Add embeddings gradually**: Start with key documents/queries
3. **Choose the right provider**: Balance cost, latency, and quality
4. **Use hybrid for production**: Best results for most applications
5. **Monitor costs**: Embedding APIs can add up with high query volume
6. **Consider local models**: For cost-sensitive or privacy-critical applications

Run the examples to see BYOE in action:
```bash
cargo run --example byoe_openai_embeddings
cargo run --example byoe_cohere_embeddings  
cargo run --example hybrid_search_local
cargo run --example search_mode_comparison
```

## Architecture

Locai is structured as a Rust workspace:

```
locai/
├── locai/          # Core library
├── locai-server/   # HTTP API service
├── locai-cli/      # Command-line interface
└── frontend/       # Web UI (in development)
```

### Core Components

- **Memory Manager**: Central coordinator for all memory operations
- **Storage Layer**: Trait-based abstraction with SurrealDB implementation
- **Search Engine**: BM25 full-text search with custom analyzers
- **Entity Extraction**: Pipeline architecture for flexible entity processing
- **ML Services**: Embedding generation and NER (with feature flags)

## Documentation

- [Design Document](docs/DESIGN.md) - Architecture and design decisions
- [API Reference](docs/API.md) - Complete API documentation
- [Search Architecture](docs/SEARCH.md) - BM25 search implementation details
- [Entity Extraction](docs/ENTITY_EXTRACTION.md) - Pipeline architecture
- [Live Queries](docs/LIVE_QUERIES.md) - Real-time features
- [Configuration Guide](docs/CONFIGURATION.md) - Configuration options
- [Known Limitations](KNOWN_LIMITATIONS.md) - Current limitations for alpha

## Advanced Usage

### Builder Pattern

```rust
let locai = Locai::builder()
    .with_data_dir("./custom_data")
    .with_embedding_model("BAAI/bge-small-en")
    .build()
    .await?;

// Advanced memory creation
locai.remember_with("Scientific breakthrough")
    .as_fact()
    .with_priority(MemoryPriority::High)
    .with_tags(&["science", "important"])
    .save()
    .await?;
```

### Direct API Access

```rust
// Access the underlying MemoryManager
let graph = locai.manager()
    .get_memory_graph("memory_id", 2)  // depth = 2
    .await?;

println!("Memory has {} related entities", graph.entities.len());
```

## Feature Flags

```toml
[features]
default = ["surrealdb-embedded"]                    # Minimal, fast default
surrealdb-embedded = ["surrealdb", "kv-mem", "kv-rocksdb"]  # Local database  
surrealdb-remote = ["surrealdb", "protocol-ws", "protocol-http"]  # Remote database
candle-embeddings = ["candle-core", "candle-nn", "candle-transformers"]  # Legacy local ML
cuda = ["candle-embeddings", "candle-core/cuda"]   # GPU acceleration (legacy)
http = ["axum", "tower", "tower-http"]             # HTTP API server
```

**Default is minimal**: BM25 search + SurrealDB storage only (~20 dependencies)
**Add ML features**: For advanced/legacy use cases (~100+ dependencies)

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/locai.git
cd locai

# Build all crates
cargo build --all

# Run tests
cargo test --all

# Build with all features
cargo build --all-features
```

### Running Tests

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Run with specific features
cargo test --features cuda
```

## Performance

Locai is designed for high performance:

- Sub-millisecond BM25 search queries
- Efficient memory storage with automatic indexing
- Minimal overhead for entity extraction
- Concurrent operation support

## Contributing

Contributions are welcome! Key areas include:

- Graph visualization frontend completion
- Additional entity extractors
- Performance optimizations
- Documentation improvements

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Acknowledgments

Built with:
- [SurrealDB](https://surrealdb.com/) - Unified graph and vector database
- [Candle](https://github.com/huggingface/candle) - ML inference in Rust
- [Axum](https://github.com/tokio-rs/axum) - Modern web framework
- [Tokio](https://tokio.rs/) - Async runtime 