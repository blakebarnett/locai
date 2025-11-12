# Locai

Advanced memory management system for AI agents and applications, providing persistent storage and intelligent retrieval through professional-grade BM25 search, semantic capabilities, and graph relationships.

## Features

Locai provides a complete memory management solution with BM25 full-text search, semantic search via embeddings (BYOE approach), graph-based relationship mapping, and real-time change notifications. It functions as both an embeddable library and REST API service, with SurrealDB providing unified graph and vector operations.

## Quick Start

### As a Library

Add Locai to your Rust project:

```toml
[dependencies]
locai = "0.2.1"
```

Store and search memories:

```rust
use locai::Locai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

The server provides a REST API at `http://localhost:3000` with interactive documentation at `/docs` and WebSocket support for live queries.

## BYOE (Bring Your Own Embeddings)

Locai follows a **BYOE (Bring Your Own Embeddings)** approach. BM25 text search works immediately without any setup, while embeddings are optional for enhanced semantic search. You can use any embedding provider (OpenAI, Cohere, local models) and switch providers without changing Locai code.

```rust
// BM25 search works immediately (no embeddings needed)
let results = locai.search("your query").await?;

// Add embeddings for hybrid search when needed
let embedding = your_provider.embed("text").await?;
let memory = MemoryBuilder::new()
    .content("text")
    .embedding(embedding)
    .build();
locai.create_memory(memory).await?;
```

See the [examples directory](examples/) for complete BYOE integration examples.

## Architecture

Locai is structured as a Rust workspace with core library, HTTP API server, CLI tool, and optional frontend components. The system uses a trait-based storage abstraction with SurrealDB providing graph and vector operations in a single database.

## Documentation

- [API Reference](docs/API.md) - Complete HTTP API documentation
- [User Guides](docs/guides/) - Docker, temporal features, lifecycle tracking, and more
- [Architecture](docs/ARCHITECTURE.md) - System design and components
- [Search](docs/SEARCH.md) - BM25 search implementation
- [Entity Extraction](docs/ENTITY_EXTRACTION.md) - Pipeline architecture
- [Live Queries](docs/LIVE_QUERIES.md) - Real-time features

## Development

```bash
# Build all crates
cargo build --workspace

# Run tests
make test-unit

# Run the server
cargo run --bin locai-server
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
