#!/bin/bash
# Generate embeddings for quickstart sample data using Ollama
# Usage: ./scripts/generate_quickstart_embeddings.sh [model] [ollama_url]
#
# Note: Not all Ollama models support embeddings. Common embedding models include:
#   - nomic-embed-text (768 dimensions)
#   - all-minilm (384 dimensions)
#   - bge-small-en-v1.5 (384 dimensions)
#   - mxbai-embed-large (1024 dimensions) - recommended for 1024-dim requirement

MODEL="${1:-nomic-embed-text}"
OLLAMA_URL="${2:-http://localhost:11434}"

echo "Generating embeddings using Ollama model: $MODEL"
echo "Ollama URL: $OLLAMA_URL"
echo ""

# Check if model supports embeddings
echo "Checking if model supports embeddings..."
TEST_RESPONSE=$(curl -s -X POST "$OLLAMA_URL/api/embeddings" \
    -H "Content-Type: application/json" \
    -d "{\"model\": \"$MODEL\", \"prompt\": \"test\"}")

if echo "$TEST_RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
    ERROR_MSG=$(echo "$TEST_RESPONSE" | jq -r '.error')
    echo "❌ Error: $ERROR_MSG" >&2
    echo "" >&2
    echo "💡 Tip: Pull an embedding model first:" >&2
    echo "   ollama pull nomic-embed-text    # 768 dimensions" >&2
    echo "   ollama pull mxbai-embed-large    # 1024 dimensions (recommended)" >&2
    echo "" >&2
    echo "Then run this script again with: ./scripts/generate_quickstart_embeddings.sh <model-name>" >&2
    exit 1
fi

echo "✓ Model supports embeddings"
echo ""

# Sample texts from quickstart
TEXTS=(
    "The protagonist is a skilled warrior named John"
    "John met Alice in the tavern last week"
    "The kingdom has been at war for three years"
)

OUTPUT_FILE="locai-cli/src/quickstart_embeddings.json"
OUTPUT_DIR=$(dirname "$OUTPUT_FILE")

# Create directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

echo "[" > "$OUTPUT_FILE"

for i in "${!TEXTS[@]}"; do
    TEXT="${TEXTS[$i]}"
    echo "Generating embedding for: $TEXT"
    
    # Call Ollama API
    RESPONSE=$(curl -s -X POST "$OLLAMA_URL/api/embeddings" \
        -H "Content-Type: application/json" \
        -d "{\"model\": \"$MODEL\", \"prompt\": \"$TEXT\"}")
    
    # Check for errors first
    if echo "$RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
        ERROR_MSG=$(echo "$RESPONSE" | jq -r '.error')
        echo "Error: $ERROR_MSG" >&2
        echo "Response: $RESPONSE" >&2
        exit 1
    fi
    
    # Extract embedding array
    EMBEDDING=$(echo "$RESPONSE" | jq -r '.embedding // empty')
    
    if [ -z "$EMBEDDING" ] || [ "$EMBEDDING" == "null" ]; then
        echo "Error: Failed to get embedding for text $((i+1))" >&2
        echo "Response: $RESPONSE" >&2
        exit 1
    fi
    
    # Create JSON entry
    echo "  {" >> "$OUTPUT_FILE"
    echo "    \"text\": $(echo "$TEXT" | jq -R .)," >> "$OUTPUT_FILE"
    echo "    \"embedding\": $EMBEDDING" >> "$OUTPUT_FILE"
    
    if [ $i -lt $((${#TEXTS[@]} - 1)) ]; then
        echo "  }," >> "$OUTPUT_FILE"
    else
        echo "  }" >> "$OUTPUT_FILE"
    fi
done

echo "]" >> "$OUTPUT_FILE"

echo ""
echo "Embeddings saved to $OUTPUT_FILE"
DIMENSIONS=$(jq -r '.[0].embedding | length' "$OUTPUT_FILE")
echo "Embedding dimensions: $DIMENSIONS"

if [ "$DIMENSIONS" != "1024" ]; then
    echo ""
    echo "⚠️  Warning: Embeddings have $DIMENSIONS dimensions, but SurrealDB requires 1024 dimensions."
    echo "   The quickstart will fall back to mock embeddings."
    echo "   Consider using a model that produces 1024-dimensional embeddings (e.g., BGE-M3 compatible models)."
fi

