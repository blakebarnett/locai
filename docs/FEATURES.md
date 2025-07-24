# Feature Flags in Locai

Locai uses Cargo's feature flags system to allow conditional compilation of optional components and dependencies. This gives you the flexibility to include only the components you need, reducing binary size, compilation time, and dependency complexity.

## Available Features

### Storage Backends

- **surrealdb-embedded** - Enables the SurrealDB embedded database backend (default)
  - Provides unified graph and vector storage capabilities for memories, entities, and relationships
  - Includes RocksDB and in-memory storage engines

- **surrealdb-remote** - Enables SurrealDB remote database connections
  - Provides WebSocket and HTTP connections to remote SurrealDB instances
  - Useful for distributed deployments

### Embedding Models

- **candle-embeddings** - Enables local embedding generation using the Candle framework (default)
  - Includes dependencies for loading and running transformer models locally
  - Much larger binary size but doesn't require external API calls

- **cuda** - Enables CUDA acceleration for Candle models (requires CUDA toolkit)
  - Only meaningful when combined with `candle-embeddings`
  - Example: `--features "candle-embeddings cuda"`

- **metal** - Enables Metal acceleration for Candle models on macOS
  - Only meaningful when combined with `candle-embeddings`
  - Example: `--features "candle-embeddings metal"`

### API Services

- **http** - Enables HTTP API capabilities through Axum
  - Includes dependencies for running Locai as a service

### Debugging

- **tokio-console** - Enables Tokio console integration for async debugging

## Default Features

By default, Locai enables the `surrealdb-embedded` and `candle-embeddings` features to provide a complete, self-contained system with unified storage and local embedding generation. This ensures that users get a functional system out of the box without external dependencies.

## Feature Combinations

### Minimal Setup

For a minimal setup with just in-memory capabilities:

```
--no-default-features
```

This will give you basic memory management with in-memory storage only.

### Basic Persistent Storage

```
--features "surrealdb-embedded"
```

Enables unified graph and vector storage for persistent memory management.

### Local Embedding Generation

```
--features "candle-embeddings"
```

Enables local embedding generation without a remote service dependency.

### Full Setup (Default)

```
--features "surrealdb-embedded candle-embeddings"
```

Enables all main components for a complete deployment with unified storage and local embedding generation.

### Remote SurrealDB

```
--features "surrealdb-remote candle-embeddings"
```

Connects to a remote SurrealDB instance while maintaining local embedding generation.

### With Hardware Acceleration

```
--features "surrealdb-embedded candle-embeddings cuda"  # For CUDA
--features "surrealdb-embedded candle-embeddings metal" # For Metal
```

### HTTP API Service

```
--features "surrealdb-embedded candle-embeddings http"
```

Enables the HTTP API for serving Locai as a web service.

## Feature Compatibility Matrix

| Feature            | Compatible Features                          | Description                                       |
|--------------------|--------------------------------------------|---------------------------------------------------|
| surrealdb-embedded | candle-embeddings, http                    | Embedded SurrealDB storage backend                |
| surrealdb-remote   | candle-embeddings, http                    | Remote SurrealDB connections                       |
| candle-embeddings  | surrealdb-*, cuda, metal, http             | Local embedding generation                         |
| cuda               | candle-embeddings                          | CUDA acceleration (requires candle-embeddings)    |
| metal              | candle-embeddings                          | Metal acceleration (requires candle-embeddings)   |
| http               | surrealdb-*, candle-embeddings             | HTTP API service                                   |
| tokio-console      | All                                        | Async debugging                                    |

## Examples

Each example in the `examples` directory may require specific features to be enabled:

- **candle_embeddings.rs** - Requires `candle-embeddings` feature:
  ```
  cargo run --example candle_embeddings --features candle-embeddings
  ```

- **model_management.rs** - Requires `candle-embeddings` feature:
  ```
  cargo run --example model_management --features candle-embeddings
  ```

- **dnd_agents.rs** - Uses default features:
  ```
  cargo run --example dnd_agents --features "surrealdb-embedded candle-embeddings"
  ```

## Runtime Feature Detection

Locai provides runtime feature detection utilities to check which features are available. This is useful for client code that needs to adapt its behavior based on the available capabilities.

```rust
use locai::prelude::*;

// Get all enabled features
let features = enabled_features();
println!("Enabled features: {:?}", features);

// Check for specific features
if is_feature_enabled("candle-embeddings") {
    // Use local embedding generation
}

// Use specific helper functions
if has_local_embeddings() {
    // Local embedding generation is available
}

if has_gpu_acceleration() {
    let gpu_type = gpu_acceleration_type().unwrap();
    println!("Using GPU acceleration: {}", gpu_type);
}

if has_http_capability() {
    // Can serve HTTP requests
}
```

You can also use conditional compilation in your code:

```rust
if cfg!(feature = "candle-embeddings") {
    // Code that uses candle embeddings
} else {
    // Fallback code
}
```

## Testing with Different Feature Combinations

When developing new functionality, it's important to test with different feature combinations. This can be done locally with:

```bash
# Test with no features
cargo test --no-default-features

# Test with specific features
cargo test --features "surrealdb-embedded"
cargo test --features "candle-embeddings"
cargo test --features "surrealdb-embedded candle-embeddings http"
```

The Locai CI system will test your code with various feature combinations to ensure compatibility. 