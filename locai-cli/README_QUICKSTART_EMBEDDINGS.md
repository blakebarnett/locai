# Quickstart Embeddings

The quickstart command uses pre-generated embeddings for the first 3 sample memories to demonstrate semantic search. These embeddings are stored in `src/quickstart_embeddings.json`.

## Generating Embeddings

To generate real embeddings using Ollama (instead of mock embeddings), run:

```bash
./scripts/generate_quickstart_embeddings.sh [model] [ollama_url]
```

**Default values:**
- Model: `nomic-embed-text` (you may want to use `mxbai-embed-large` for 1024 dimensions)
- Ollama URL: `http://localhost:11434`

**Example:**
```bash
# Using defaults
./scripts/generate_quickstart_embeddings.sh

# Custom model and URL
./scripts/generate_quickstart_embeddings.sh qwen3:7b http://localhost:11434
```

## Requirements

- Ollama running locally (or accessible at the specified URL)
- An embedding model pulled in Ollama (not all models support embeddings!)
- `jq` installed (for JSON processing)
- `curl` installed (for API calls)

## Embedding Models

**Important**: Not all Ollama models support embeddings. Language models like `qwen3:14b` do not support embeddings.

### Recommended Embedding Models:

- **mxbai-embed-large** (1024 dimensions) - **Recommended** - matches SurrealDB's 1024-dim requirement
- **nomic-embed-text** (768 dimensions) - Good general-purpose embedding model
- **all-minilm** (384 dimensions) - Smaller, faster model

### Pulling an Embedding Model:

```bash
# For 1024 dimensions (recommended for quickstart)
ollama pull mxbai-embed-large

# Or for 768 dimensions
ollama pull nomic-embed-text
```

Then run the script:
```bash
./scripts/generate_quickstart_embeddings.sh mxbai-embed-large
```

## Embedding Dimensions

The generated embeddings must be **1024 dimensions** to work with SurrealDB's vector storage (BGE-M3 compatible). If your model produces different dimensions, the quickstart will fall back to mock embeddings.

## File Format

The `quickstart_embeddings.json` file has the following structure:

```json
[
  {
    "text": "The protagonist is a skilled warrior named John",
    "embedding": [0.123, 0.456, ...]
  },
  {
    "text": "John met Alice in the tavern last week",
    "embedding": [0.789, 0.012, ...]
  },
  {
    "text": "The kingdom has been at war for three years",
    "embedding": [0.345, 0.678, ...]
  }
]
```

## Fallback Behavior

If `quickstart_embeddings.json` doesn't exist or contains invalid data, the quickstart command will automatically use mock embeddings instead. This ensures the quickstart always works, even without Ollama.

