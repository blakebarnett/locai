//! Configuration for entity extraction functionality.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{EntityType, EntityResolutionConfig, AutomaticRelationshipConfig};

/// Configuration for entity extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EntityExtractionConfig {
    /// Whether entity extraction is enabled
    pub enabled: bool,
    /// List of extractor configurations
    pub extractors: Vec<ExtractorConfig>,
    /// Minimum confidence threshold for extracted entities
    pub confidence_threshold: f32,
    /// Maximum number of entities to extract per memory (None for unlimited)
    pub max_entities_per_memory: Option<usize>,
    /// Whether to deduplicate similar entities
    pub deduplicate_entities: bool,
    /// Relationship type to use when linking memories to entities
    pub relationship_type: String,
    /// Entity resolution configuration (Phase 2)
    pub resolution: EntityResolutionConfig,
    /// Automatic relationship creation configuration (Phase 2)
    pub automatic_relationships: AutomaticRelationshipConfig,
    /// ML-specific configuration
    pub ml: MLExtractionConfig,
}

impl Default for EntityExtractionConfig {
    fn default() -> Self {
        // Use hybrid approach by default - combines rule-based + ML intelligently
        let hybrid_extractor = ExtractorConfig {
            name: "hybrid".to_string(),
            extractor_type: ExtractorType::Hybrid {
                config: HybridExtractorConfig::default(),
            },
            enabled: true,
            priority: 200, // High priority for hybrid approach
            entity_types: vec![
                // Structured data (handled by basic extractor)
                EntityType::Email,
                EntityType::Url,
                EntityType::PhoneNumber,
                EntityType::Date,
                EntityType::Time,
                EntityType::Money,
                // Named entities (handled by ML extractor)
                EntityType::Person,
                EntityType::Organization,
                EntityType::Location,
            ],
        };
        
        // Keep basic extractor as fallback
        let basic_fallback = ExtractorConfig {
            name: "basic-fallback".to_string(),
            extractor_type: ExtractorType::Regex,
            enabled: true,
            priority: 100, // Lower priority - only used if hybrid fails
            entity_types: vec![
                EntityType::Email,
                EntityType::Url,
                EntityType::PhoneNumber,
                EntityType::Date,
                EntityType::Time,
                EntityType::Money,
            ],
        };
        
        let extractors = vec![hybrid_extractor, basic_fallback];
        
        Self {
            enabled: true, // Enable by default for rich graph creation
            extractors,
            confidence_threshold: 0.15, // Lower default for better entity capture
            max_entities_per_memory: Some(50),
            deduplicate_entities: true,
            relationship_type: "mentions".to_string(),
            resolution: EntityResolutionConfig::default(),
            automatic_relationships: AutomaticRelationshipConfig::default(),
            ml: MLExtractionConfig::default(),
        }
    }
}

/// Configuration for ML-based entity extraction (Phase 3)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MLExtractionConfig {
    /// Whether ML extraction is enabled
    pub enabled: bool,
    /// Default model to use
    pub default_model: String,
    /// Model configurations
    pub models: HashMap<String, MLModelConfig>,
    /// Routing rules for multi-model extraction
    pub routing: MLRoutingConfig,
    /// Optimization settings
    pub optimization: MLOptimizationConfig,
}

impl Default for MLExtractionConfig {
    fn default() -> Self {
        let mut models = HashMap::new();
        
        // Add default DistilBERT NER configuration - better generic performance
        models.insert("distilbert-ner".to_string(), MLModelConfig {
            model_id: "elastic/distilbert-base-uncased-finetuned-conll03-english".to_string(),
            backend: MLBackend::Candle,
            device: "auto".to_string(),
            max_length: 512,
            batch_size: 8,
            confidence_threshold: 0.15, // Lower threshold for better recall
            cache_results: true,
        });
        
        Self {
            enabled: true,
            default_model: "distilbert-ner".to_string(), // Use DistilBERT as default
            models,
            routing: MLRoutingConfig { enabled: false, rules: vec![] }, // Disable routing
            optimization: MLOptimizationConfig::default(),
        }
    }
}

/// Configuration for individual ML models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLModelConfig {
    /// Model identifier (e.g., "answerdotai/ModernBERT-base")
    pub model_id: String,
    /// Backend to use for inference
    pub backend: MLBackend,
    /// Device to run on (auto, cpu, cuda:0, etc.)
    pub device: String,
    /// Maximum sequence length
    pub max_length: usize,
    /// Batch size for processing
    pub batch_size: usize,
    /// Confidence threshold for this model
    pub confidence_threshold: f32,
    /// Whether to cache results
    pub cache_results: bool,
}

/// ML backend options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MLBackend {
    /// Candle framework
    Candle,
    /// ONNX runtime
    Onnx,
    /// External API (OpenAI, Anthropic, etc.)
    External(String),
}

/// Configuration for model routing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MLRoutingConfig {
    /// Whether routing is enabled
    pub enabled: bool,
    /// Routing rules
    pub rules: Vec<MLRoutingRule>,
}

impl Default for MLRoutingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: vec![
                // Use ModernBERT for longer sequences (its strength)
                MLRoutingRule {
                    rule_type: "text_length".to_string(),
                    pattern: Some("1000-8192".to_string()),
                    model: "modernbert-ner".to_string(), // ModernBERT excels at longer sequences
                    priority: 10,
                },
                // Use DistilBERT for shorter texts (faster and often better for general NER)
                MLRoutingRule {
                    rule_type: "text_length".to_string(),
                    pattern: Some("0-999".to_string()),
                    model: "distilbert-ner".to_string(), // DistilBERT is faster and better for short texts
                    priority: 5,
                },
            ],
        }
    }
}

/// Individual routing rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLRoutingRule {
    /// Type of rule (content_type, text_length, domain, etc.)
    pub rule_type: String,
    /// Pattern to match (regex, length range, keywords)
    pub pattern: Option<String>,
    /// Model to route to
    pub model: String,
    /// Priority (higher = evaluated first)
    pub priority: u8,
}

/// Optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MLOptimizationConfig {
    /// Enable result caching
    pub enable_caching: bool,
    /// Cache size (number of entries)
    pub cache_size: usize,
    /// Enable batching
    pub enable_batching: bool,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Enable model quantization
    pub enable_quantization: bool,
}

impl Default for MLOptimizationConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_size: 1000,
            enable_batching: true,
            max_batch_size: 16,
            enable_quantization: false,
        }
    }
}

/// Configuration for hybrid entity extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HybridExtractorConfig {
    /// Enable basic (rule-based) extractor for structured data
    pub enable_basic: bool,
    /// Enable transformer-based extractor
    pub enable_transformers: bool,
    /// Transformer model to use
    pub transformer_model: String,
    /// Enable deduplication of overlapping entities
    pub enable_deduplication: bool,
    /// Confidence threshold for final entities
    pub confidence_threshold: f32,
    /// Fallback behavior when ML extractors fail
    pub fallback_to_basic: bool,
}

impl Default for HybridExtractorConfig {
    fn default() -> Self {
        Self {
            enable_basic: true,
            enable_transformers: true, // Enable by default with Candle infrastructure
            transformer_model: "elastic/distilbert-base-uncased-finetuned-conll03-english".to_string(), // Use DistilBERT for better generic NER performance
            enable_deduplication: true,
            confidence_threshold: 0.3, // Lower threshold for better recall
            fallback_to_basic: true,
        }
    }
}

/// Configuration for pipeline-based extraction (fallthrough approach)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineExtractorConfig {
    /// Stop pipeline when this confidence is reached
    pub target_confidence: f32,
    /// Maximum number of extractors to try
    pub max_attempts: usize,
    /// Whether to combine results from multiple extractors
    pub combine_results: bool,
    /// Strategy for combining results
    pub combination_strategy: CombinationStrategy,
}

impl Default for PipelineExtractorConfig {
    fn default() -> Self {
        Self {
            target_confidence: 0.9,
            max_attempts: 3,
            combine_results: true,
            combination_strategy: CombinationStrategy::HighestConfidence,
        }
    }
}

/// Strategy for combining results from multiple extractors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombinationStrategy {
    /// Take only the highest confidence extraction for each span
    HighestConfidence,
    /// Take the first extraction that meets the threshold
    FirstMatch,
    /// Combine all extractions above threshold
    AllAboveThreshold,
    /// Use voting mechanism for conflicting extractions
    Voting,
}

/// Configuration for a specific extractor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    /// Name of the extractor
    pub name: String,
    /// Type of extractor
    pub extractor_type: ExtractorType,
    /// Whether this extractor is enabled
    pub enabled: bool,
    /// Priority for this extractor (higher runs first)
    pub priority: u8,
    /// Entity types this extractor should handle
    pub entity_types: Vec<EntityType>,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            name: "basic".to_string(),
            extractor_type: ExtractorType::Regex,
            enabled: true,
            priority: 128,
            entity_types: vec![
                // Only structured data - named entities handled by ML extractor
                EntityType::Email,
                EntityType::Url,
                EntityType::PhoneNumber,
                EntityType::Date,
                EntityType::Time,
                EntityType::Money,
            ],
        }
    }
}

/// Types of entity extractors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractorType {
    /// Regular expression-based extractor (for structured data)
    Regex,
    /// spaCy NLP model
    Spacy { model: String },
    /// Hugging Face transformer model
    HuggingFace { model: String },
    /// Hybrid extractor (combines rule-based + ML)
    Hybrid {
        /// Configuration for the hybrid approach
        config: HybridExtractorConfig,
    },
    /// Pipeline extractor (fallthrough multiple extractors)
    Pipeline {
        /// List of extractors to try in order
        extractors: Vec<ExtractorConfig>,
        /// Minimum confidence to stop pipeline
        min_confidence: f32,
    },

    /// Large Language Model
    Llm { provider: String, model: String },
}

/// DistilBERT configuration optimized for generic entity recognition
/// This performs much better than ModernBERT for out-of-the-box NER tasks
/// Suitable for extracting persons, locations, organizations, and miscellaneous entities
pub fn distilbert_ner_base() -> EntityExtractionConfig {
    EntityExtractionConfig::default()
} 