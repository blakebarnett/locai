# BYOE (Bring Your Own Embeddings) Examples

This directory contains examples demonstrating Locai's **BYOE (Bring Your Own Embeddings)** approach, which provides maximum flexibility for embedding providers while keeping Locai focused on its core strengths.

## Philosophy

Locai's core strength is **BM25 full-text search** powered by SurrealDB. Embeddings are **optional** and provided by users through external services, giving you:

- ✅ **Choice of embedding provider** (OpenAI, Cohere, Azure, etc.)
- ✅ **Cost control** - you manage embedding costs and rate limits
- ✅ **Latest models** - upgrade providers without changing Locai
- ✅ **Hybrid search** - BM25 + vector when embeddings are available
- ✅ **No vendor lock-in** - switch providers anytime

## Examples

### 1. OpenAI Embeddings (`byoe_openai_embeddings.rs`)

Demonstrates using OpenAI's `text-embedding-3-small` model:

```rust
// Generate embedding with OpenAI
let embedding = openai_client.embed_text("your text").await?;

// Create memory with embedding
let memory = MemoryBuilder::new()
    .content("your text")
    .embedding(embedding)  // ← User provides embedding
    .build();

locai.create_memory(memory).await?;
```

**Features shown:**
- OpenAI API integration (mocked)
- 1536-dimensional embeddings
- Embedding validation
- BM25 vs hybrid search comparison

### 2. Cohere Embeddings (`byoe_cohere_embeddings.rs`)

Demonstrates using Cohere's `embed-english-v3.0` model:

```rust
// Generate specialized embeddings
let doc_embedding = cohere_client.embed_document("document text").await?;
let query_embedding = cohere_client.embed_query("search query").await?;
```

**Features shown:**
- Cohere API integration (mocked)
- 1024-dimensional embeddings
- Document vs query embedding types
- Provider comparison and flexibility

### 3. Relationship Enrichment (`relationship_enrichment_callback.rs`)

Shows the replacement for sentiment analysis with flexible callbacks:

```rust
let relationship_manager = RelationshipManager::new(locai.memory_manager().clone()).await?
    .with_enrichment_callback(|action, context, entity| {
        // Your custom logic here (sentiment, emotion, etc.)
        custom_analysis(action, context, entity)
    });
```

## Quick Start

1. **Basic usage** (BM25 only):
   ```rust
   let locai = Locai::new().await?;
   let results = locai.search("your query").await?;  // Always works!
   ```

2. **With embeddings** (hybrid search):
   ```rust
   // Get embedding from your provider
   let embedding = your_embedding_provider.embed("text").await?;
   
   // Create memory with embedding
   let memory = MemoryBuilder::new()
       .content("text")
       .embedding(embedding)
       .build();
   
   locai.create_memory(memory).await?;
   ```

## Embedding Providers

### Supported (via BYOE)

| Provider | Model | Dimensions | Notes |
|----------|-------|------------|-------|
| OpenAI | text-embedding-3-small | 1536 | General purpose, high quality |
| OpenAI | text-embedding-3-large | 3072 | Highest quality, more expensive |
| Cohere | embed-english-v3.0 | 1024 | Optimized for search, input types |
| Azure OpenAI | text-embedding-ada-002 | 1536 | Enterprise-ready |
| Google | textembedding-gecko | 768 | Vertex AI integration |
| Anthropic | (via Claude) | Varies | Custom embedding extraction |
| **Any provider** | **Any model** | **Any size** | **Full flexibility** |

### Local Options (optional)

- Sentence Transformers
- Ollama embeddings  
- Custom models via candle (optional feature)

## Migration from Old ModelManager

### Before (complex):
```rust
let model_manager = ModelManagerBuilder::new()
    .cache_dir("./models")
    .default_embedding_model("BAAI/bge-small-en")
    .build();

let model = model_manager.get_embedding_model("model-id").await?;
let embedding = model.embed_text("text", None).await?;
```

### After (simple):
```rust
let embedding_manager = EmbeddingManager::new();
let embedding = your_provider.embed("text").await?;
embedding_manager.validate_embedding(&embedding)?;  // Optional
```

## Utilities

The simplified `EmbeddingManager` provides utilities:

```rust
let manager = EmbeddingManager::with_expected_dimensions(1536);

// Validate embeddings
manager.validate_embedding(&embedding)?;

// Normalize to unit length
manager.normalize_embedding(&mut embedding)?;

// Check dimensions
let dims = manager.expected_dimensions();
```

## Benefits of BYOE

1. **No Local Storage**: No gigabytes of model files
2. **No GPU Requirements**: Reduce hardware costs
3. **Provider Choice**: Use the best model for your use case
4. **Cost Control**: Manage embedding costs directly
5. **Always Current**: Use latest embedding models
6. **Hybrid Ready**: BM25 + vector when embeddings present
7. **Focused Codebase**: Locai focuses on search and memory

## Running Examples

```bash
# OpenAI example (set OPENAI_API_KEY for real usage)
cargo run --example byoe_openai_embeddings

# Cohere example (set COHERE_API_KEY for real usage)  
cargo run --example byoe_cohere_embeddings

# Relationship enrichment
cargo run --example relationship_enrichment_callback
```

## Real Integration

For production use, replace the mock clients with real SDK calls:

- **OpenAI**: Use `openai` or `async-openai` crates
- **Cohere**: Use `cohere-rust` or HTTP client
- **Azure**: Use Azure SDK or OpenAI-compatible endpoint
- **Custom**: Any HTTP client or SDK

The examples show the exact integration pattern - just replace the mock implementation with real API calls. 