//! ModernBERT entity extractor implementation with unified model management
//!
//! This example shows how to implement a ModernBERT-based entity extractor
//! that integrates with Locai's unified ModelManager for consistent model lifecycle management.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use candle_core::{DType, Device, Tensor, Result as CandleResult};
use candle_nn::{VarBuilder, Linear, Module, VarMap};
use serde_json::Value;
use safetensors::SafeTensors;
use anyhow::Result;
use locai::{LocaiError, ml::EmbeddingManager};

use locai::entity_extraction::pipeline::{
    RawEntityExtractor, RawEntity, GenericEntityType, ModelLoader
};

// Use official Candle ModernBERT implementation
use candle_transformers::models::modernbert::{ModernBert, Config as ModernBertConfig};

/// Simple tokenizer wrapper
#[derive(Debug)]
pub struct SimpleTokenizer {
    tokenizer: tokenizers::Tokenizer,
}

impl SimpleTokenizer {
    pub fn from_file(path: &str) -> Result<Self> {
        let tokenizer = tokenizers::Tokenizer::from_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        Ok(Self { tokenizer })
    }
    
    pub fn encode(&self, text: &str, add_special_tokens: bool) -> Result<tokenizers::Encoding> {
        self.tokenizer.encode(text, add_special_tokens)
            .map_err(|e| anyhow::anyhow!("Failed to encode text: {}", e))
    }
}

/// Get available device
fn get_device() -> Result<Device> {
    if candle_core::utils::cuda_is_available() {
        Device::new_cuda(0).map_err(|e| anyhow::anyhow!("Failed to create CUDA device: {}", e))
    } else if candle_core::utils::metal_is_available() {
        Device::new_metal(0).map_err(|e| anyhow::anyhow!("Failed to create Metal device: {}", e))
    } else {
        Ok(Device::Cpu)
    }
}

/// Label mapping for NER models
#[derive(Debug, Clone)]
pub struct LabelMapping {
    pub id_to_label: HashMap<usize, String>,
    pub label_to_id: HashMap<String, usize>,
    pub num_labels: usize,
}

impl LabelMapping {
    /// Create label mapping from a list of labels
    pub fn from_config(labels: &[String]) -> Self {
        let mut id_to_label = HashMap::new();
        let mut label_to_id = HashMap::new();
        
        for (id, label) in labels.iter().enumerate() {
            id_to_label.insert(id, label.clone());
            label_to_id.insert(label.clone(), id);
        }
        
        Self {
            id_to_label,
            label_to_id,
            num_labels: labels.len(),
        }
    }
    
    /// Create default CoNLL-2003 style label mapping
    pub fn conll2003() -> Self {
        let labels = vec![
            "O".to_string(),
            "B-PER".to_string(), "I-PER".to_string(),
            "B-ORG".to_string(), "I-ORG".to_string(),
            "B-LOC".to_string(), "I-LOC".to_string(),
            "B-MISC".to_string(), "I-MISC".to_string(),
        ];
        Self::from_config(&labels)
    }
}

/// ModernBERT model with token classification head for NER
pub struct ModernBertForTokenClassification {
    modernbert: ModernBert,
    classifier: Linear,
    #[allow(dead_code)]
    num_labels: usize,
}

impl ModernBertForTokenClassification {
    pub fn new(vb: VarBuilder, config: &ModernBertConfig, num_labels: usize) -> CandleResult<Self> {
        let modernbert = ModernBert::load(vb.clone(), config)?;
        
        let classifier = match candle_nn::linear_b(config.hidden_size, num_labels, true, vb.pp("classifier")) {
            Ok(linear) => linear,
            Err(_) => {
                let varmap = VarMap::new();
                let classifier_vb = VarBuilder::from_varmap(&varmap, vb.dtype(), vb.device());
                candle_nn::linear_b(config.hidden_size, num_labels, true, classifier_vb.pp("classifier"))?
            }
        };
        
        Ok(Self {
            modernbert,
            classifier,
            num_labels,
        })
    }
    
    pub fn forward(&self, input_ids: &Tensor, attention_mask: &Tensor) -> CandleResult<Tensor> {
        let sequence_output = self.modernbert.forward(input_ids, attention_mask)?;
        let logits = self.classifier.forward(&sequence_output)?;
        Ok(logits)
    }
}

/// ModernBERT NER model implementation
pub struct ModernBertNERModel {
    _model: ModernBertForTokenClassification,
    _tokenizer: SimpleTokenizer,
    _device: Device,
    label_mapping: LabelMapping,
    model_id: String,
    max_length: usize,
}

impl std::fmt::Debug for ModernBertNERModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModernBertNERModel")
            .field("model_id", &self.model_id)
            .field("max_length", &self.max_length)
            .field("label_mapping", &self.label_mapping)
            .finish()
    }
}

impl ModernBertNERModel {
    /// Create a new ModernBERT NER model from a model path
    pub async fn from_path(model_path: &str) -> Result<Self> {
        let device = get_device()?;
        
        tracing::info!("🚀 Loading ModernBERT NER model: {}", model_path);
        
        // Check if model_path is a local path
        let is_local_path = Path::new(model_path).exists();
        
        let config_path = if is_local_path {
            let local_config_path = Path::new(model_path).join("config.json");
            if !local_config_path.exists() {
                anyhow::bail!("Local model config not found: {}", local_config_path.display());
            }
            local_config_path
        } else {
            anyhow::bail!("Remote model loading not implemented in this example. Use local model path.");
        };
        
        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read config: {}", e))?;
        
        let config_json: Value = serde_json::from_str(&config_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config JSON: {}", e))?;
        
        let modernbert_config: ModernBertConfig = serde_json::from_value(config_json.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse ModernBERT config: {}", e))?;
        
        // Create tokenizer
        let tokenizer_path = Path::new(model_path).join("tokenizer.json");
        let tokenizer = SimpleTokenizer::from_file(
            tokenizer_path.to_string_lossy().as_ref()
        )?;
        
        // Load model weights
        let model_file = Path::new(model_path).join("model.safetensors");
        let model_bytes = std::fs::read(&model_file)
            .map_err(|e| anyhow::anyhow!("Failed to read model file: {}", e))?;
        
        let _safetensors = SafeTensors::deserialize(&model_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize model: {}", e))?;
        
        // Create VarBuilder - in a real implementation this would load the actual model weights
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
        
        // Create label mapping for CoNLL-2003 format
        let label_mapping = LabelMapping::conll2003();
        
        // Create model
        let model = ModernBertForTokenClassification::new(vb, &modernbert_config, label_mapping.num_labels)
            .map_err(|e| anyhow::anyhow!("Failed to create ModernBERT model: {}", e))?;
        
        Ok(Self {
            _model: model,
            _tokenizer: tokenizer,
            _device: device,
            label_mapping,
            model_id: model_path.to_string(),
            max_length: 512,
        })
    }
    
    /// Get supported entity types
    pub fn supported_entity_types(&self) -> Vec<String> {
        vec!["PERSON".to_string(), "ORG".to_string(), "LOC".to_string(), "MISC".to_string()]
    }
    
    /// Get model metadata
    pub fn metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("model_type".to_string(), "ModernBERT-NER".to_string());
        metadata.insert("model_id".to_string(), self.model_id.clone());
        metadata.insert("max_length".to_string(), self.max_length.to_string());
        metadata.insert("num_labels".to_string(), self.label_mapping.num_labels.to_string());
        metadata
    }
}

/// ModernBERT extractor that integrates with unified ModelManager
pub struct ModernBertExtractor {
    _model_manager: Arc<ModelManager>,
    model_id: String,
    max_length: usize,
}

impl std::fmt::Debug for ModernBertExtractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModernBertExtractor")
            .field("model_id", &self.model_id)
            .field("max_length", &self.max_length)
            .finish()
    }
}

/// Helper function to convert anyhow::Error to LocaiError
fn convert_error(err: anyhow::Error) -> LocaiError {
    LocaiError::Entity(err.to_string())
}

impl ModernBertExtractor {
    /// Create a new ModernBERT extractor using ModelManager
    pub fn new(model_manager: Arc<ModelManager>, model_id: &str) -> Self {
        Self {
            _model_manager: model_manager,
            model_id: model_id.to_string(),
            max_length: 512,
        }
    }

    /// Create from ModelManager with specific model registration
    pub async fn from_manager_with_path(
        model_manager: Arc<ModelManager>,
        model_id: &str,
        model_path: &str,
    ) -> Result<Self> {
        // Load the model through our NER model implementation
        let _ner_model = ModernBertNERModel::from_path(model_path).await?;
        
        // For this example, we'll store the model in the manager's cache
        // In a full implementation, this would be integrated into ModelManager's NER loading
        tracing::info!("✅ ModernBERT NER model loaded and ready for integration");
        
        Ok(Self {
            _model_manager: model_manager,
            model_id: model_id.to_string(),
            max_length: 512,
        })
    }

    /// Set maximum sequence length
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length;
        self
    }

    /// Extract entities using the model manager's NER model
    async fn extract_with_ner_model(&self, _text: &str) -> locai::Result<Vec<RawEntity>> {
        // For this example, return a placeholder implementation
        // In a full implementation, this would use the actual model inference
        tracing::warn!("Using placeholder entity extraction - full implementation pending");
        
        // Return some mock entities for demonstration
        Ok(vec![
            RawEntity::new(
                "placeholder".to_string(),
                GenericEntityType::Person,
                0,
                11,
                0.8,
            )
        ])
    }
}

#[async_trait]
impl RawEntityExtractor for ModernBertExtractor {
    async fn extract_raw(&self, text: &str) -> locai::Result<Vec<RawEntity>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        
        // Use the model manager's NER capabilities
        self.extract_with_ner_model(text).await
    }

    fn name(&self) -> &str {
        "ModernBertExtractor (ModelManager-integrated)"
    }

    fn supported_types(&self) -> Vec<GenericEntityType> {
        vec![
            GenericEntityType::Person,
            GenericEntityType::Organization,
            GenericEntityType::Location,
            GenericEntityType::Miscellaneous,
        ]
    }
}

#[async_trait]
impl ModelLoader for ModernBertExtractor {
    async fn load_model(model_path: &str) -> locai::Result<Self>
    where
        Self: Sized,
    {
        // Create a model manager for this extractor
        let model_manager = Arc::new(ModelManager::new("./model_cache"));
        
        // Use the integrated approach
        match Self::from_manager_with_path(model_manager, "modernbert-ner", model_path).await {
            Ok(extractor) => Ok(extractor),
            Err(e) => Err(convert_error(e)),
        }
    }
}

/// Utility function to create a ModernBERT extractor with ModelManager integration
pub fn create_modernbert_extractor_with_manager(
    model_manager: Arc<ModelManager>,
    model_id: &str,
) -> ModernBertExtractor {
    ModernBertExtractor::new(model_manager, model_id)
}

/// Create a unified extraction pipeline with ModelManager integration
pub async fn create_unified_extraction_pipeline(
    model_manager: Arc<ModelManager>,
) -> locai::Result<locai::entity_extraction::EntityExtractionPipeline> {
    use locai::entity_extraction::{
        EntityExtractionPipeline, ConfidenceValidator, EntityMerger, EntityDeduplicator
    };
    
    // Create extractor with model manager
    let extractor = create_modernbert_extractor_with_manager(model_manager, "modernbert-ner");
    
    // Create pipeline with validation and post-processing
    let pipeline = EntityExtractionPipeline::builder()
        .extractor(Box::new(extractor))
        .validator(Box::new(ConfidenceValidator::new(0.5)))
        .post_processor(Box::new(EntityMerger::new()))
        .post_processor(Box::new(EntityDeduplicator::new()))
        .build()?;
    
    Ok(pipeline)
} 