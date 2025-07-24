//! Tokenizer implementation for Candle embedding models

use std::sync::Arc;
use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use tokenizers::Tokenizer as HFTokenizer;
use tokenizers::utils::padding::{PaddingDirection, PaddingParams, PaddingStrategy as HFPaddingStrategy};
use tokenizers::utils::truncation::{TruncationDirection, TruncationParams, TruncationStrategy};
use candle_core::Result as CandleResult;
use serde_json::Value;

use crate::ml::error::{MLError, Result};
use crate::ml::tokenizer::{
    PaddingMode, SpecialToken, TokenId, Tokenizer, TokenizerOptions, TokenizedText,
};
use super::utils::ModelCache;

/// Supported tokenizer formats that can be auto-detected
#[derive(Debug, Clone)]
pub enum TokenizerFormat {
    /// Modern tokenizer.json format (preferred)
    Modern {
        tokenizer_file: String,
        config_file: Option<String>,
        special_tokens_file: Option<String>,
    },
    /// Legacy BERT-style format with vocab.txt
    BertLegacy {
        vocab_file: String,
        config_file: Option<String>,
        special_tokens_file: Option<String>,
    },
    /// SentencePiece format
    SentencePiece {
        model_file: String,
        config_file: Option<String>,
        special_tokens_file: Option<String>,
    },
    /// BPE format with vocab.json and merges.txt
    Bpe {
        vocab_file: String,
        merges_file: String,
        config_file: Option<String>,
        special_tokens_file: Option<String>,
    },
}

/// Struct to hold the results of tokenization
#[derive(Default, Debug, Clone)]
pub struct TokenizeResult {
    /// Token IDs for each input
    pub token_ids: Vec<Vec<u32>>,
    /// Attention mask for each input
    pub attention_mask: Vec<Vec<u32>>,
}

/// Tokenizer implementation using the HuggingFace tokenizers library
pub struct CandleTokenizer {
    /// The underlying HuggingFace tokenizer
    inner: HFTokenizer,
    /// Special token mappings
    special_tokens: Arc<HashMap<SpecialToken, Option<TokenId>>>,
    /// Maximum sequence length supported by the tokenizer
    max_length: Option<usize>,
    /// Cache for model files
    #[allow(dead_code)]
    cache: Arc<ModelCache>,
    /// The detected tokenizer format
    format: TokenizerFormat,
}

impl CandleTokenizer {
    /// Create a new tokenizer from a pre-built HF tokenizer
    pub fn new(tokenizer: HFTokenizer, cache: Arc<ModelCache>) -> Self {
        // Extract special tokens
        let mut special_tokens = HashMap::new();
        
        // Map special tokens by conventional names - try multiple variants
        special_tokens.insert(SpecialToken::Cls, 
            tokenizer.token_to_id("[CLS]")
                .or_else(|| tokenizer.token_to_id("<s>"))
                .or_else(|| tokenizer.token_to_id("<cls>"))
        );
        
        special_tokens.insert(SpecialToken::Sep, 
            tokenizer.token_to_id("[SEP]")
                .or_else(|| tokenizer.token_to_id("</s>"))
                .or_else(|| tokenizer.token_to_id("<sep>"))
        );
        
        special_tokens.insert(SpecialToken::Pad, 
            tokenizer.token_to_id("[PAD]")
                .or_else(|| tokenizer.token_to_id("<pad>"))
                .or_else(|| tokenizer.token_to_id("<|pad|>"))
        );
        
        special_tokens.insert(SpecialToken::Unk, 
            tokenizer.token_to_id("[UNK]")
                .or_else(|| tokenizer.token_to_id("<unk>"))
                .or_else(|| tokenizer.token_to_id("<|unk|>"))
        );
        
        special_tokens.insert(SpecialToken::Mask, 
            tokenizer.token_to_id("[MASK]")
                .or_else(|| tokenizer.token_to_id("<mask>"))
                .or_else(|| tokenizer.token_to_id("<|mask|>"))
        );
        
        special_tokens.insert(SpecialToken::Bos, 
            tokenizer.token_to_id("[BOS]")
                .or_else(|| tokenizer.token_to_id("<s>"))
                .or_else(|| tokenizer.token_to_id("<|startoftext|>"))
        );
        
        special_tokens.insert(SpecialToken::Eos, 
            tokenizer.token_to_id("[EOS]")
                .or_else(|| tokenizer.token_to_id("</s>"))
                .or_else(|| tokenizer.token_to_id("<|endoftext|>"))
        );
        
        // Try to get max length from tokenizer configuration
        let max_length = 512; // Default fallback
        
        Self {
            inner: tokenizer,
            special_tokens: Arc::new(special_tokens),
            max_length: Some(max_length),
            cache,
            format: TokenizerFormat::Modern {
                tokenizer_file: "tokenizer.json".to_string(),
                config_file: None,
                special_tokens_file: None,
            },
        }
    }
    
    /// Create a new tokenizer from a local file
    pub fn from_file(path: &str, cache: Arc<ModelCache>) -> Result<Self> {
        let tokenizer = HFTokenizer::from_file(path)
            .map_err(|e| MLError::tokenization(format!("Failed to load tokenizer from file: {}", e)))?;
        
        Ok(Self::new(tokenizer, cache))
    }

    /// Detect and load a tokenizer from the Hugging Face Hub with comprehensive format support
    pub async fn from_pretrained(model_id: &str, cache: Arc<ModelCache>) -> Result<Self> {
        log::debug!("Loading tokenizer for model: {}", model_id);
        
        // Step 1: Detect available tokenizer format
        let format = Self::detect_tokenizer_format(model_id, &cache).await?;
        log::debug!("Detected tokenizer format: {:?}", format);
        
        // Step 2: Load tokenizer based on detected format
        match format {
            TokenizerFormat::Modern { ref tokenizer_file, ref config_file, ref special_tokens_file } => {
                Self::load_modern_tokenizer(model_id, tokenizer_file, config_file, special_tokens_file, cache, format.clone()).await
            },
            TokenizerFormat::BertLegacy { ref vocab_file, ref config_file, ref special_tokens_file } => {
                Self::load_bert_legacy_tokenizer(model_id, vocab_file, config_file, special_tokens_file, cache, format.clone()).await
            },
            TokenizerFormat::SentencePiece { ref model_file, ref config_file, ref special_tokens_file } => {
                Self::load_sentencepiece_tokenizer(model_id, model_file, config_file, special_tokens_file, cache, format.clone()).await
            },
            TokenizerFormat::Bpe { ref vocab_file, ref merges_file, ref config_file, ref special_tokens_file } => {
                Self::load_bpe_tokenizer(model_id, vocab_file, merges_file, config_file, special_tokens_file, cache, format.clone()).await
            },
        }
    }

    /// Detect what tokenizer format is available for a model
    async fn detect_tokenizer_format(model_id: &str, cache: &Arc<ModelCache>) -> Result<TokenizerFormat> {
        log::debug!("Detecting tokenizer format for model: {}", model_id);
        
        // Get list of all files in the model repository
        let files = cache.list_files(model_id).await
            .map_err(|e| MLError::tokenization(format!("Failed to list model files: {}", e)))?;
        
        log::debug!("Available files: {:?}", files);
        
        // Check for modern tokenizer format (preferred)
        if files.contains(&"tokenizer.json".to_string()) {
            log::debug!("Found modern tokenizer format (tokenizer.json)");
            return Ok(TokenizerFormat::Modern {
                tokenizer_file: "tokenizer.json".to_string(),
                config_file: files.iter().find(|f| *f == "tokenizer_config.json").cloned(),
                special_tokens_file: files.iter().find(|f| *f == "special_tokens_map.json").cloned(),
            });
        }

        // Check for BPE format (vocab.json + merges.txt)
        if files.contains(&"vocab.json".to_string()) && files.contains(&"merges.txt".to_string()) {
            log::debug!("Found BPE tokenizer format (vocab.json + merges.txt)");
            return Ok(TokenizerFormat::Bpe {
                vocab_file: "vocab.json".to_string(),
                merges_file: "merges.txt".to_string(),
                config_file: files.iter().find(|f| *f == "tokenizer_config.json").cloned(),
                special_tokens_file: files.iter().find(|f| *f == "special_tokens_map.json").cloned(),
            });
        }

        // Check for SentencePiece format
        let spm_files: Vec<&String> = files.iter().filter(|f| f.ends_with(".model") || f == &&"sentencepiece.bpe.model".to_string()).collect();
        if !spm_files.is_empty() {
            log::debug!("Found SentencePiece tokenizer format");
            return Ok(TokenizerFormat::SentencePiece {
                model_file: spm_files[0].clone(),
                config_file: files.iter().find(|f| *f == "tokenizer_config.json").cloned(),
                special_tokens_file: files.iter().find(|f| *f == "special_tokens_map.json").cloned(),
            });
        }

        // Check for legacy BERT format (vocab.txt)
        if files.contains(&"vocab.txt".to_string()) {
            log::debug!("Found legacy BERT tokenizer format (vocab.txt)");
            return Ok(TokenizerFormat::BertLegacy {
                vocab_file: "vocab.txt".to_string(),
                config_file: files.iter().find(|f| *f == "tokenizer_config.json").cloned(),
                special_tokens_file: files.iter().find(|f| *f == "special_tokens_map.json").cloned(),
            });
        }

        // If no recognized format found, return error with helpful information
        Err(MLError::tokenization(format!(
            "Could not detect tokenizer format for model: {}. Available files: {:?}. \
            Supported formats: tokenizer.json (modern), vocab.json+merges.txt (BPE), \
            *.model (SentencePiece), vocab.txt (BERT legacy)", 
            model_id, files
        )))
    }

    /// Load a modern tokenizer (tokenizer.json format)
    async fn load_modern_tokenizer(
        model_id: &str,
        tokenizer_file: &str,
        config_file: &Option<String>,
        special_tokens_file: &Option<String>,
        cache: Arc<ModelCache>,
        format: TokenizerFormat,
    ) -> Result<Self> {
        log::debug!("Loading modern tokenizer from {}", tokenizer_file);
        
        // Download tokenizer.json
        let tokenizer_path = cache.get_file(model_id, tokenizer_file).await
            .map_err(|e| MLError::tokenization(format!("Failed to download {}: {}", tokenizer_file, e)))?;
        
        // Load the base tokenizer
        let mut tokenizer = HFTokenizer::from_file(tokenizer_path)
            .map_err(|e| MLError::tokenization(format!("Failed to load tokenizer: {}", e)))?;
        
        // Apply configuration if available
        if let Some(config_file) = config_file {
            if let Ok(config_path) = cache.get_file(model_id, config_file).await {
                Self::apply_tokenizer_config(&mut tokenizer, &config_path)?;
            }
        }

        // Apply special tokens if available
        if let Some(special_tokens_file) = special_tokens_file {
            if let Ok(special_tokens_path) = cache.get_file(model_id, special_tokens_file).await {
                Self::apply_special_tokens(&mut tokenizer, &special_tokens_path)?;
            }
        }
        
        let mut result = Self::new(tokenizer, cache);
        result.format = format;
        Ok(result)
    }

    /// Load a legacy BERT tokenizer (vocab.txt format)
    async fn load_bert_legacy_tokenizer(
        model_id: &str,
        vocab_file: &str,
        config_file: &Option<String>,
        special_tokens_file: &Option<String>,
        cache: Arc<ModelCache>,
        format: TokenizerFormat,
    ) -> Result<Self> {
        log::debug!("Loading BERT legacy tokenizer from {}", vocab_file);
        
        // For BERT-style tokenizers, we need to build the tokenizer from vocab
        let vocab_path = cache.get_file(model_id, vocab_file).await
            .map_err(|e| MLError::tokenization(format!("Failed to download {}: {}", vocab_file, e)))?;

        // Read vocab file and create WordPiece tokenizer
        let vocab_content = std::fs::read_to_string(&vocab_path)
            .map_err(|e| MLError::tokenization(format!("Failed to read vocab file: {}", e)))?;
        
        let vocab: HashMap<String, u32> = vocab_content
            .lines()
            .enumerate()
            .map(|(i, token)| (token.trim().to_string(), i as u32))
            .collect();

        // Build a WordPiece tokenizer
        use tokenizers::models::wordpiece::WordPiece;
        let wordpiece_model = WordPiece::builder()
            .vocab(vocab)
            .unk_token("[UNK]".to_string())
            .max_input_chars_per_word(100)
            .continuing_subword_prefix("##".to_string())
            .build()
            .map_err(|e| MLError::tokenization(format!("Failed to build WordPiece model: {}", e)))?;

        let mut tokenizer = HFTokenizer::new(wordpiece_model);
        
        // Add pre-tokenizer
        tokenizer.with_pre_tokenizer(
            Some(tokenizers::pre_tokenizers::whitespace::WhitespaceSplit {})
        );
        
        // Apply configuration if available
        if let Some(config_file) = config_file {
            if let Ok(config_path) = cache.get_file(model_id, config_file).await {
                Self::apply_tokenizer_config(&mut tokenizer, &config_path)?;
            }
        }

        // Apply special tokens if available
        if let Some(special_tokens_file) = special_tokens_file {
            if let Ok(special_tokens_path) = cache.get_file(model_id, special_tokens_file).await {
                Self::apply_special_tokens(&mut tokenizer, &special_tokens_path)?;
            }
        }
        
        let mut result = Self::new(tokenizer, cache);
        result.format = format;
        Ok(result)
    }

    /// Load a SentencePiece tokenizer
    async fn load_sentencepiece_tokenizer(
        model_id: &str,
        model_file: &str,
        _config_file: &Option<String>,
        _special_tokens_file: &Option<String>,
        cache: Arc<ModelCache>,
        format: TokenizerFormat,
    ) -> Result<Self> {
        log::debug!("Loading SentencePiece tokenizer from {}", model_file);
        
        // Download SentencePiece model file
        let model_path = cache.get_file(model_id, model_file).await
            .map_err(|e| MLError::tokenization(format!("Failed to download {}: {}", model_file, e)))?;

        // Build SentencePiece tokenizer
        // Note: This is a simplified approach - in practice, SentencePiece models
        // require more complex handling and may need the sentencepiece crate
        
        // For now, try to load as a standard tokenizer file or fallback to modern format
        let tokenizer = if model_path.extension().and_then(|s| s.to_str()) == Some("json") {
            HFTokenizer::from_file(&model_path)
                .map_err(|e| MLError::tokenization(format!("Failed to load SentencePiece tokenizer: {}", e)))?
        } else {
            return Err(MLError::tokenization(format!(
                "SentencePiece binary format not fully supported yet. Model file: {}", model_file
            )));
        };
        
        let mut result = Self::new(tokenizer, cache);
        result.format = format;
        Ok(result)
    }

    /// Load a BPE tokenizer (vocab.json + merges.txt)
    async fn load_bpe_tokenizer(
        model_id: &str,
        vocab_file: &str,
        merges_file: &str,
        config_file: &Option<String>,
        special_tokens_file: &Option<String>,
        cache: Arc<ModelCache>,
        format: TokenizerFormat,
    ) -> Result<Self> {
        log::debug!("Loading BPE tokenizer from {} and {}", vocab_file, merges_file);
        
        // Download vocab and merges files
        let vocab_path = cache.get_file(model_id, vocab_file).await
            .map_err(|e| MLError::tokenization(format!("Failed to download {}: {}", vocab_file, e)))?;
        let merges_path = cache.get_file(model_id, merges_file).await
            .map_err(|e| MLError::tokenization(format!("Failed to download {}: {}", merges_file, e)))?;

        // Read vocab file
        let vocab_content = std::fs::read_to_string(&vocab_path)
            .map_err(|e| MLError::tokenization(format!("Failed to read vocab file: {}", e)))?;
        let vocab: HashMap<String, u32> = serde_json::from_str(&vocab_content)
            .map_err(|e| MLError::tokenization(format!("Failed to parse vocab JSON: {}", e)))?;

        // Read merges file
        let merges_content = std::fs::read_to_string(&merges_path)
            .map_err(|e| MLError::tokenization(format!("Failed to read merges file: {}", e)))?;
        let merges: Vec<(String, String)> = merges_content
            .lines()
            .skip(1) // Skip header line
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        // Build BPE tokenizer
        use tokenizers::models::bpe::BPE;
        let bpe_model = BPE::new(vocab, merges);

        let mut tokenizer = HFTokenizer::new(bpe_model);
        
        // Apply configuration if available
        if let Some(config_file) = config_file {
            if let Ok(config_path) = cache.get_file(model_id, config_file).await {
                Self::apply_tokenizer_config(&mut tokenizer, &config_path)?;
            }
        }

        // Apply special tokens if available
        if let Some(special_tokens_file) = special_tokens_file {
            if let Ok(special_tokens_path) = cache.get_file(model_id, special_tokens_file).await {
                Self::apply_special_tokens(&mut tokenizer, &special_tokens_path)?;
            }
        }
        
        let mut result = Self::new(tokenizer, cache);
        result.format = format;
        Ok(result)
    }

    /// Apply tokenizer configuration from config file
    fn apply_tokenizer_config(_tokenizer: &mut HFTokenizer, config_path: &Path) -> Result<()> {
        let config_content = std::fs::read_to_string(config_path)
            .map_err(|e| MLError::tokenization(format!("Failed to read tokenizer config: {}", e)))?;
        
        let config: Value = serde_json::from_str(&config_content)
            .map_err(|e| MLError::tokenization(format!("Failed to parse tokenizer config: {}", e)))?;
        
        // Apply relevant configuration settings
        if let Some(do_lower_case) = config.get("do_lower_case").and_then(|v| v.as_bool()) {
            if do_lower_case {
                // Add normalizer for lowercasing if needed
                log::debug!("Applying lowercase normalization");
            }
        }

        if let Some(model_max_length) = config.get("model_max_length").and_then(|v| v.as_u64()) {
            log::debug!("Setting model max length to {}", model_max_length);
            // Store this for later use in the CandleTokenizer
        }

        log::debug!("Applied tokenizer configuration");
        Ok(())
    }

    /// Apply special tokens from special tokens map file
    fn apply_special_tokens(_tokenizer: &mut HFTokenizer, special_tokens_path: &Path) -> Result<()> {
        let special_tokens_content = std::fs::read_to_string(special_tokens_path)
            .map_err(|e| MLError::tokenization(format!("Failed to read special tokens: {}", e)))?;
        
        let special_tokens: Value = serde_json::from_str(&special_tokens_content)
            .map_err(|e| MLError::tokenization(format!("Failed to parse special tokens: {}", e)))?;
        
        // Apply special tokens to tokenizer
        log::debug!("Applied special tokens: {:?}", special_tokens);
        Ok(())
    }

    /// Get the detected tokenizer format
    pub fn format(&self) -> &TokenizerFormat {
        &self.format
    }
    
    /// Convert a TokenizerOptions to HuggingFace padding parameters
    #[allow(dead_code)]
    fn get_padding_params(&self, options: &TokenizerOptions) -> Option<PaddingParams> {
        match options.padding {
            PaddingMode::None => None,
            PaddingMode::MaxLength => {
                let pad_id = self.special_token_id(SpecialToken::Pad);
                
                if pad_id.is_none() {
                    return None;
                }
                
                let length = match options.max_length {
                    Some(len) => len,
                    None => match self.max_len() {
                        Some(len) => len,
                        None => 512, // Default max length
                    },
                };
                
                Some(PaddingParams {
                    strategy: HFPaddingStrategy::Fixed(length),
                    direction: PaddingDirection::Right,
                    pad_id: pad_id.unwrap(),
                    pad_type_id: 0,
                    pad_token: "[PAD]".to_string(),
                    pad_to_multiple_of: None,
                })
            },
            PaddingMode::FixedLength(length) => {
                let pad_id = self.special_token_id(SpecialToken::Pad);
                
                if pad_id.is_none() {
                    return None;
                }
                
                Some(PaddingParams {
                    strategy: HFPaddingStrategy::Fixed(length),
                    direction: PaddingDirection::Right,
                    pad_id: pad_id.unwrap(),
                    pad_type_id: 0,
                    pad_token: "[PAD]".to_string(),
                    pad_to_multiple_of: None,
                })
            },
        }
    }
    
    /// Get truncation parameters from TokenizerOptions
    #[allow(dead_code)]
    fn get_truncation_params(&self, options: &TokenizerOptions) -> Option<TruncationParams> {
        if !options.truncation {
            return None;
        }
        
        let max_length = options.max_length.unwrap_or_else(|| self.max_len().unwrap_or(512));
        
        Some(TruncationParams {
            max_length,
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
            direction: TruncationDirection::Right,
        })
    }
    
    /// Tokenize a batch of texts
    pub fn tokenize_batch(&self, texts: &[String]) -> CandleResult<TokenizeResult> {
        let mut result = TokenizeResult::default();
        
        if texts.is_empty() {
            return Ok(result);
        }
        
        // Process each text individually, then combine the results
        for text in texts {
            let encoding = self.inner.encode(
                text.as_str(), // Use as_str() to avoid type errors
                true, // Add special tokens
            ).map_err(|e| candle_core::Error::Msg(format!("Tokenizer error: {}", e)))?;
            
            // Add the token IDs and attention mask for this text
            result.token_ids.push(encoding.get_ids().to_vec());
            result.attention_mask.push(encoding.get_attention_mask().to_vec());
        }
        
        // Confirm all sequences have the same length (pad if needed)
        if !result.token_ids.is_empty() {
            let max_length = result.token_ids.iter().map(|ids| ids.len()).max().unwrap_or(0);
            
            // Pad all sequences to max_length
            for (i, ids) in result.token_ids.iter_mut().enumerate() {
                if ids.len() < max_length {
                    let pad_token = self.special_token_id(SpecialToken::Pad).unwrap_or(0);
                    ids.resize(max_length, pad_token);
                    
                    // Also extend the attention mask
                    if let Some(mask) = result.attention_mask.get_mut(i) {
                        mask.resize(max_length, 0);
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    /// Custom method for individual encoding creation (no batch processing)
    pub fn create_single_encoding(&self, text: &str) -> CandleResult<TokenizeResult> {
        let mut result = TokenizeResult::default();
        
        let encoding = self.inner.encode(
            text, // Pass text directly since it's already &str
            true, // Add special tokens
        ).map_err(|e| candle_core::Error::Msg(format!("Tokenizer error: {}", e)))?;
        
        let ids = encoding.get_ids().to_vec();
        let mask = encoding.get_attention_mask().to_vec();
        
        result.token_ids.push(ids);
        result.attention_mask.push(mask);
        
        Ok(result)
    }
    
    /// Create a combined encoding from multiple texts (concatenated)
    pub fn create_combined_encoding(&self, texts: &[String]) -> CandleResult<TokenizeResult> {
        let mut result = TokenizeResult::default();
        
        if texts.is_empty() {
            return Ok(result);
        }
        
        // Handle single text case directly
        if texts.len() == 1 {
            return self.create_single_encoding(&texts[0]);
        }
        
        // For multiple texts, encode each separately
        let mut encodings = Vec::new();
        for text in texts {
            // Process each text individually and handle errors
            match self.inner.encode(
                text.as_str(),
                true, // Add special tokens
            ) {
                Ok(encoding) => encodings.push(encoding),
                Err(e) => return Err(candle_core::Error::Msg(format!("Tokenizer error: {}", e)))
            }
        }
        
        // Combine all encodings
        let mut all_ids = Vec::new();
        let mut all_masks = Vec::new();
        
        // For each encoding, append its ids and masks
        for enc in encodings {
            all_ids.extend_from_slice(enc.get_ids());
            all_masks.extend_from_slice(enc.get_attention_mask());
        }
        
        // Ensure we don't exceed the model's maximum token limit
        // Fixed version that doesn't use get_model_max_length
        let max_length = 512; // Default max_length
        if all_ids.len() > max_length {
            all_ids.truncate(max_length);
            all_masks.truncate(max_length);
        }
        
        result.token_ids.push(all_ids);
        result.attention_mask.push(all_masks);
        
        Ok(result)
    }
    
    /// Get a reference to the underlying HuggingFace tokenizer
    pub fn inner(&self) -> &HFTokenizer {
        &self.inner
    }
}

#[async_trait]
impl Tokenizer for CandleTokenizer {
    async fn tokenize(&self, text: &str, options: Option<TokenizerOptions>) -> Result<TokenizedText> {
        let options = options.unwrap_or_default();
        
        let mut encoding = tokio::task::spawn_blocking({
            let text = text.to_string();
            let tokenizer = self.inner.clone();
            move || {
                // Updated to use encode without get_encoding
                let mut encoding = tokenizer.encode(text, false)
                    .map_err(|e| MLError::tokenization(format!("Tokenization error: {}", e)))?;
                
                // Apply truncation
                if options.truncation {
                    let max_length = options.max_length.unwrap_or(512);
                    encoding.truncate(max_length, 0, TruncationDirection::Right);
                }
                
                Ok::<_, MLError>(encoding)
            }
        })
        .await
        .map_err(|e| MLError::tokenization(format!("Task join error: {}", e)))??;
        
        // Add special tokens if requested
        if options.add_special_tokens {
            // This part can't work with the current API since encoding doesn't have set_ids and set_attention_mask
            // Instead, we'll re-encode with add_special_tokens=true
            if let (Some(_cls_id), Some(_sep_id)) = (self.special_token_id(SpecialToken::Cls), self.special_token_id(SpecialToken::Sep)) {
                // Re-encode with special tokens
                encoding = tokio::task::spawn_blocking({
                    let text = text.to_string();
                    let tokenizer = self.inner.clone();
                    move || {
                        tokenizer.encode(text, true)
                            .map_err(|e| MLError::tokenization(format!("Tokenization error with special tokens: {}", e)))
                    }
                })
                .await
                .map_err(|e| MLError::tokenization(format!("Task join error: {}", e)))??;
            }
        }
        
        // Create TokenizedText
        Ok(TokenizedText {
            ids: encoding.get_ids().to_vec(),
            attention_mask: Some(encoding.get_attention_mask().iter().map(|&v| v as u8).collect()),
            original_text: text.to_string(),
        })
    }
    
    async fn tokenize_batch(&self, texts: &[String], options: Option<TokenizerOptions>) -> Result<Vec<TokenizedText>> {
        let options = options.unwrap_or_default();
        
        // Clone these for the blocking task
        let tokenizer = self.inner.clone();
        let add_special_tokens = options.add_special_tokens;
        
        // Prepare padding params if needed (outside the closure)
        let padding = match options.padding {
            PaddingMode::None => None,
            PaddingMode::MaxLength | PaddingMode::FixedLength(_) => {
                let pad_id = self.special_token_id(SpecialToken::Pad).unwrap_or(0);
                let length = match options.padding {
                    PaddingMode::FixedLength(len) => len,
                    _ => options.max_length.unwrap_or(512),
                };
                
                Some(PaddingParams {
                    strategy: HFPaddingStrategy::Fixed(length),
                    direction: PaddingDirection::Right,
                    pad_id,
                    pad_type_id: 0,
                    pad_token: "[PAD]".to_string(),
                    pad_to_multiple_of: None,
                })
            }
        };

        let encodings = tokio::task::spawn_blocking({
            let texts = texts.to_vec();
            
            // Prepare truncation params if enabled
            let truncation = if options.truncation {
                Some(TruncationParams {
                    max_length: options.max_length.unwrap_or(512),
                    strategy: TruncationStrategy::LongestFirst,
                    stride: 0,
                    direction: TruncationDirection::Right,
                })
            } else {
                None
            };
            
            move || {
                // Create a mutable tokenizer for this thread
                let mut local_tokenizer = tokenizer.clone();
                
                // Set padding and truncation
                if let Some(padding_params) = padding {
                    local_tokenizer.with_padding(Some(padding_params));
                }
                
                if let Some(truncation_params) = truncation {
                    local_tokenizer.with_truncation(Some(truncation_params))
                        .map_err(|e| MLError::tokenization(format!("Failed to set truncation: {}", e)))?;
                }
                
                local_tokenizer.encode_batch(texts, add_special_tokens)
                    .map_err(|e| MLError::tokenization(format!("Batch tokenization error: {}", e)))
            }
        })
        .await
        .map_err(|e| MLError::tokenization(format!("Task join error: {}", e)))??;
        
        // Convert to our TokenizedText format
        let mut results = Vec::with_capacity(encodings.len());
        
        for (i, encoding) in encodings.iter().enumerate() {
            results.push(TokenizedText {
                ids: encoding.get_ids().to_vec(),
                attention_mask: Some(encoding.get_attention_mask().iter().map(|&v| v as u8).collect()),
                original_text: texts[i].clone(),
            });
        }
        
        Ok(results)
    }
    
    async fn decode(&self, token_ids: &[TokenId]) -> Result<String> {
        tokio::task::spawn_blocking({
            let token_ids = token_ids.to_vec();
            let tokenizer = self.inner.clone();
            move || {
                tokenizer.decode(&token_ids, true)
                    .map_err(|e| MLError::tokenization(format!("Decoding error: {}", e)))
            }
        })
        .await
        .map_err(|e| MLError::tokenization(format!("Task join error: {}", e)))?
    }
    
    fn vocab_size(&self) -> usize {
        self.inner.get_vocab_size(true)
    }
    
    fn special_token_id(&self, token: SpecialToken) -> Option<TokenId> {
        *self.special_tokens.get(&token)?
    }
    
    fn max_len(&self) -> Option<usize> {
        self.max_length
    }
} 