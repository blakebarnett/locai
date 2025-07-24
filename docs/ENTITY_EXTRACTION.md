# Entity Extraction Architecture

## Overview

Locai provides a flexible, composable pipeline architecture for entity extraction that separates generic extraction logic from domain-specific validation and processing. The system supports multiple extraction backends while maintaining a consistent API.

## Architecture

### Pipeline Components

The entity extraction system uses a three-stage pipeline:

1. **Extraction**: Raw entity detection from text
2. **Validation**: Context-aware filtering and validation
3. **Post-processing**: Deduplication, merging, and normalization

```rust
let pipeline = EntityExtractionPipeline::builder()
    .extractor(Box::new(extractor))           // Stage 1: Extract
    .validator(Box::new(validator))           // Stage 2: Validate
    .post_processor(Box::new(processor))      // Stage 3: Process
    .build()?;
```

### Core Traits

**RawEntityExtractor**
```rust
pub trait RawEntityExtractor: Send + Sync {
    async fn extract_raw(&self, text: &str) -> Result<Vec<RawEntity>>;
    fn name(&self) -> &str;
    fn supported_types(&self) -> Vec<GenericEntityType>;
}
```

**EntityValidator**
```rust
pub trait EntityValidator: Send + Sync {
    fn validate(&self, entity: &RawEntity, context: &ValidationContext) -> bool;
    fn name(&self) -> &str;
}
```

**EntityPostProcessor**
```rust
pub trait EntityPostProcessor: Send + Sync {
    fn process(&self, entities: Vec<RawEntity>) -> Vec<RawEntity>;
    fn name(&self) -> &str;
}
```

### Entity Types

The system supports both generic and custom entity types:

**Generic Types**
- `Person`: Names of people
- `Organization`: Companies, institutions
- `Location`: Geographic locations
- `Miscellaneous`: Other named entities

**Structured Types**
- `Email`: Email addresses
- `Url`: Web URLs
- `PhoneNumber`: Phone numbers
- `Date`: Date expressions
- `Time`: Time expressions
- `Money`: Monetary amounts

**Custom Types**
- Domain-specific entities via `EntityType::Custom(String)`

## Implementation Guide

### Creating Custom Extractors

Implement the `RawEntityExtractor` trait for your specific model:

```rust
use async_trait::async_trait;
use locai::entity_extraction::{RawEntityExtractor, RawEntity, GenericEntityType};

#[derive(Debug)]
struct MyCustomExtractor {
    // Your model/configuration here
}

#[async_trait]
impl RawEntityExtractor for MyCustomExtractor {
    async fn extract_raw(&self, text: &str) -> Result<Vec<RawEntity>> {
        // Your extraction logic here
        Ok(vec![])
    }

    fn name(&self) -> &str {
        "MyCustomExtractor"
    }

    fn supported_types(&self) -> Vec<GenericEntityType> {
        vec![
            GenericEntityType::Person,
            GenericEntityType::Organization,
            GenericEntityType::Location,
        ]
    }
}
```

### Building Extraction Pipelines

```rust
use locai::entity_extraction::{
    EntityExtractionPipeline, ConfidenceValidator, EntityMerger, EntityDeduplicator
};

// Create the pipeline
let pipeline = EntityExtractionPipeline::builder()
    .extractor(Box::new(my_extractor))
    .validator(Box::new(ConfidenceValidator::new(0.8)))
    .post_processor(Box::new(EntityMerger::new()))
    .post_processor(Box::new(EntityDeduplicator::new()))
    .build()?;

// Extract entities
let entities = pipeline.extract("John Smith works at Google in California.").await?;
```

### Model Loading

For models that need to be loaded from files, implement the `ModelLoader` trait:

```rust
#[async_trait]
impl ModelLoader for MyCustomExtractor {
    async fn load_model(path: &str) -> Result<Self> {
        // Load your model from the specified path
        Ok(MyCustomExtractor::new(path)?)
    }
}
```

## Validation and Post-Processing

### Built-in Validators

- **ConfidenceValidator**: Filters entities below a confidence threshold
- **LengthValidator**: Validates entity text length
- **TypeValidator**: Validates entity types

### Built-in Post-Processors

- **EntityMerger**: Merges overlapping entities
- **EntityDeduplicator**: Removes duplicate entities
- **EntityNormalizer**: Normalizes entity text

### Custom Validation

```rust
use locai::entity_extraction::{EntityValidator, ValidationContext};

#[derive(Debug)]
struct CustomValidator;

impl EntityValidator for CustomValidator {
    fn validate(&self, entity: &RawEntity, context: &ValidationContext) -> bool {
        // Your validation logic
        entity.confidence > 0.5 && entity.text.len() > 1
    }

    fn name(&self) -> &str {
        "CustomValidator"
    }
}
```

## Configuration

### Entity Extraction Config

```rust
use locai::entity_extraction::EntityExtractionConfig;

let config = EntityExtractionConfig {
    enabled: true,
    confidence_threshold: 0.8,
    max_entities_per_text: 50,
    // ... other configuration options
};
```

### Extractor-Specific Configuration

Each extractor can have its own configuration:

```rust
let config = ExtractorConfig {
    name: "my-extractor".to_string(),
    extractor_type: ExtractorType::Regex,
    enabled: true,
    priority: 128,
    entity_types: vec![EntityType::Email, EntityType::Url],
};
```

## Performance Optimization

### Batch Processing

Process multiple texts in batches for better performance:

```rust
let texts = vec!["Text 1", "Text 2", "Text 3"];
let results = futures::future::join_all(
    texts.iter().map(|text| pipeline.extract(text))
).await;
```

### Caching

Use caching for expensive model operations:

```rust
use std::collections::HashMap;
use std::sync::Mutex;

struct CachedExtractor {
    cache: Mutex<HashMap<String, Vec<RawEntity>>>,
    inner: Box<dyn RawEntityExtractor>,
}
```

## Error Handling

The system provides comprehensive error handling:

```rust
match pipeline.extract(text).await {
    Ok(entities) => {
        println!("Extracted {} entities", entities.len());
    },
    Err(e) => {
        eprintln!("Extraction failed: {}", e);
    }
}
```

## Examples

See the `examples/entity_extraction/` directory for complete examples:

- **ModernBERT NER**: Using transformer models for named entity recognition
- **Custom Pipeline**: Building custom extraction pipelines
- **Hybrid Extraction**: Combining multiple extractors

## Integration with Memory System

Entity extraction integrates seamlessly with Locai's memory system:

```rust
// Entities are automatically extracted when storing memories
let memory = locai.create_memory("John Smith called about the meeting.").await?;

// Extracted entities are linked to memories automatically
let related_memories = locai.find_memories_by_entity("John Smith").await?;
``` 