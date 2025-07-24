# ModernBERT Named Entity Recognition with Locai

This example demonstrates how to use ModernBERT for named entity recognition (NER) with Locai's unified ModelManager architecture. It shows how entity extraction models integrate seamlessly with the same model management system used for embeddings and sentiment analysis.

## Features

- **Unified Model Management**: Uses Locai's ModelManager for consistent model lifecycle management
- **ModernBERT Integration**: Implements ModernBERT-based NER with the generic pipeline architecture
- **Consistent API**: Same model management patterns as other ML capabilities (embeddings, sentiment)
- **Resource Efficiency**: Shared caching and memory management across all ML models
- **Type Safety**: Proper trait implementation for NER models

## Architecture Overview

```rust
// Unified model management across all ML capabilities
let model_manager = ModelManagerBuilder::new()
    .cache_dir("./model_cache")
    .default_embedding_model("BAAI/bge-small-en")
    .default_ner_model("modernbert-ner")
    .build();

// All models are managed through the same interface
let embedding_model = model_manager.get_embedding_model("BAAI/bge-small-en").await?;
let ner_model = model_manager.get_ner_model("modernbert-ner").await?;
let sentiment_analyzer = SentimentAnalyzer::new(model_manager.clone()).await?;
```

## Usage Examples

### Basic Entity Extraction with ModelManager

```rust
use std::sync::Arc;
use locai::ml::{ModelManager, ModelManagerBuilder};
use modernbert_extractor::{create_unified_extraction_pipeline};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create unified model manager
    let model_manager = Arc::new(
        ModelManagerBuilder::new()
            .cache_dir("./model_cache")
            .default_ner_model("modernbert-ner")
            .build()
    );
    
    // Create extraction pipeline with model manager integration
    let pipeline = create_unified_extraction_pipeline(
        model_manager.clone(),
        "./models/modernbert-ner"
    ).await?;
    
    // Extract entities
    let text = "Apple Inc. was founded by Steve Jobs in Cupertino, California.";
    let entities = pipeline.extract(text).await?;
    
    for entity in entities {
        println!("Entity: {} | Type: {:?} | Confidence: {:.2}", 
                 entity.text, entity.entity_type, entity.confidence);
    }
    
    Ok(())
}
```

### Multi-Model Pipeline with Shared Management

```rust
use std::sync::Arc;
use locai::ml::{ModelManager, ModelManagerBuilder, SentimentAnalyzer};
use modernbert_extractor::{create_modernbert_extractor_with_manager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Single model manager for all ML capabilities
    let model_manager = Arc::new(
        ModelManagerBuilder::new()
            .cache_dir("./shared_model_cache")
            .default_embedding_model("BAAI/bge-small-en")
            .default_ner_model("modernbert-ner")
            .build()
    );
    
    let text = "I love working at Google! The company culture is amazing.";
    
    // Entity extraction using model manager
    let extractor = create_modernbert_extractor_with_manager(
        model_manager.clone(),
        "./models/modernbert-ner"
    ).await?;
    
    let entities = extractor.extract_raw(text).await?;
    println!("Entities found: {}", entities.len());
    
    // Sentiment analysis using the same model manager
    let sentiment_analyzer = SentimentAnalyzer::new(model_manager.clone()).await?;
    let sentiment = sentiment_analyzer.analyze_sentiment(text).await?;
    println!("Sentiment: {:?} (confidence: {:.2})", sentiment.label, sentiment.confidence);
    
    // Embedding generation using the same model manager
    let embedding_model = model_manager.get_embedding_model("BAAI/bge-small-en").await?;
    let embedding = embedding_model.embed_text(text, None).await?;
    println!("Embedding dimensions: {}", embedding.len());
    
    Ok(())
}
```

### Resource Management and Monitoring

```rust
use std::sync::Arc;
use locai::ml::{ModelManager, ModelManagerBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_manager = Arc::new(
        ModelManagerBuilder::new()
            .cache_dir("./model_cache")
            .build()
    );
    
    // Check available capabilities
    println!("Available ML capabilities:");
    println!("  Embeddings: {}", model_manager.has_model_type("embedding"));
    println!("  NER: {}", model_manager.has_model_type("ner"));
    println!("  Sentiment: {}", model_manager.has_model_type("sentiment"));
    
    // Load models and monitor resource usage
    let _embedding_model = model_manager.get_embedding_model("BAAI/bge-small-en").await?;
    let _ner_model = model_manager.get_ner_model("modernbert-ner").await?;
    
    // Check loaded models
    let embedding_models = model_manager.list_loaded_embedding_models();
    let ner_models = model_manager.list_loaded_ner_models();
    
    println!("Loaded models:");
    println!("  Embedding models: {:?}", embedding_models);
    println!("  NER models: {:?}", ner_models);
    
    // Cleanup when done
    model_manager.clear_models()?;
    
    Ok(())
}
```

## Model Requirements

### Local Model Setup

1. Download a ModernBERT NER model (or fine-tune your own):
```bash
# Example: Download a fine-tuned ModernBERT NER model
git clone https://huggingface.co/microsoft/modernbert-base ./models/modernbert-ner
```

2. Ensure your model directory contains:
```
./models/modernbert-ner/
├── config.json          # Model configuration
├── model.safetensors     # Model weights
├── tokenizer.json        # Tokenizer configuration
└── ...                   # Other model files
```

### Model Configuration

The extractor supports various configuration options:

```rust
let extractor = ModernBertExtractor::new(model_manager, "modernbert-ner")
    .with_max_length(512);  // Set maximum sequence length
```

## Integration Patterns

### Custom NER Model Implementation

```rust
// Note: The NERModel trait has been removed as entity extraction now uses the RawEntityExtractor trait
use std::collections::HashMap;

#[derive(Debug)]
// Legacy example - use RawEntityExtractor trait instead
struct CustomNERModel {
    model_id: String,
}

// Note: Implement RawEntityExtractor instead
impl NERModel for CustomNERModel {
    fn name(&self) -> &str {
        &self.model_id
    }
    
    fn supported_entity_types(&self) -> Vec<String> {
        vec!["PERSON".to_string(), "ORG".to_string(), "LOC".to_string()]
    }
    
    fn metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("model_type".to_string(), "custom-ner".to_string());
        metadata
    }
}
```

### Pipeline Composition

```rust
use locai::entity_extraction::{
    EntityExtractionPipeline, ConfidenceValidator, EntityMerger, EntityDeduplicator
};
use modernbert_extractor::create_modernbert_extractor_with_manager;

async fn create_production_pipeline(
    model_manager: Arc<ModelManager>
) -> Result<EntityExtractionPipeline> {
    let extractor = create_modernbert_extractor_with_manager(
        model_manager,
        "./models/modernbert-ner"
    ).await?;
    
    let pipeline = EntityExtractionPipeline::builder()
        .extractor(Box::new(extractor))
        .validator(Box::new(ConfidenceValidator::new(0.8)))  // High confidence threshold
        .post_processor(Box::new(EntityMerger::new()))       // Merge overlapping entities
        .post_processor(Box::new(EntityDeduplicator::new())) // Remove duplicates
        .build()?;
    
    Ok(pipeline)
}
```

## Performance Considerations

### Model Caching

The unified ModelManager provides automatic caching:

```rust
// First load - downloads and caches the model
let model1 = model_manager.get_ner_model("modernbert-ner").await?;

// Second load - retrieves from cache (very fast)
let model2 = model_manager.get_ner_model("modernbert-ner").await?;
```

### Memory Management

```rust
// Monitor memory usage
let loaded_models = model_manager.list_loaded_ner_models();
println!("Currently loaded NER models: {}", loaded_models.len());

// Unload specific models to free memory
model_manager.unload_ner_model("modernbert-ner")?;

// Clear all models
model_manager.clear_models()?;
```

### Batch Processing

```rust
let texts = vec![
    "Apple Inc. is based in Cupertino.",
    "Microsoft was founded by Bill Gates.",
    "Google's headquarters is in Mountain View."
];

for text in texts {
    let entities = pipeline.extract(text).await?;
    println!("Found {} entities in: {}", entities.len(), text);
}
```

## Advanced Configuration

### Multiple Model Managers

```rust
// Production setup with different cache strategies
let production_manager = ModelManagerBuilder::new()
    .cache_dir("/var/cache/locai/production")
    .default_ner_model("modernbert-large-ner")
    .build();

// Development setup with smaller models
let dev_manager = ModelManagerBuilder::new()
    .cache_dir("./dev_cache")
    .default_ner_model("distilbert-ner")
    .build();
```

### Error Handling

```rust
match model_manager.get_ner_model("modernbert-ner").await {
    Ok(model) => {
        println!("Model loaded: {}", model.name());
    }
    Err(e) => {
        eprintln!("Failed to load NER model: {}", e);
        // Fallback to alternative model or graceful degradation
    }
}
```

## Benefits of Unified Management

1. **Consistency**: Same API patterns across all ML capabilities
2. **Efficiency**: Shared caching and resource management
3. **Scalability**: Easy to add new model types and capabilities
4. **Maintainability**: Single point of configuration and monitoring
5. **Type Safety**: Compile-time checks for model compatibility

## Running the Examples

```bash
# Build with candle features
cargo build --features "candle-embeddings" --example modernbert_ner

# Run with unified model management
cargo run --features "candle-embeddings" --example modernbert_ner

# Run with debug logging
RUST_LOG=debug cargo run --features "candle-embeddings" --example modernbert_ner
```

## Integration with Other Examples

This entity extraction system works seamlessly with other Locai examples:

- **Embedding Examples**: Shared model manager for semantic search + NER
- **Sentiment Examples**: Combined sentiment analysis and entity extraction
- **RAG Systems**: Entity-aware retrieval augmented generation

See the main Locai examples directory for comprehensive integration patterns. 