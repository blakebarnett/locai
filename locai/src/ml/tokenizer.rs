//! Tokenizer interface for embedding models

use async_trait::async_trait;

use super::error::Result;

/// A token ID representing a unit of text processed by the tokenizer
pub type TokenId = u32;

/// A sequence of token IDs
pub type TokenIds = Vec<TokenId>;

/// Special token types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialToken {
    /// Beginning of sequence token
    Bos,
    /// End of sequence token
    Eos,
    /// Padding token
    Pad,
    /// Unknown token
    Unk,
    /// Mask token
    Mask,
    /// Separator token
    Sep,
    /// Classification token
    Cls,
}

/// Options for tokenization
#[derive(Debug, Clone)]
pub struct TokenizerOptions {
    /// Add special tokens like BOS/EOS
    pub add_special_tokens: bool,
    /// Truncate to max length
    pub truncation: bool,
    /// Max sequence length (if truncation is enabled)
    pub max_length: Option<usize>,
    /// Padding mode
    pub padding: PaddingMode,
}

impl Default for TokenizerOptions {
    fn default() -> Self {
        Self {
            add_special_tokens: true,
            truncation: true,
            max_length: None,
            padding: PaddingMode::None,
        }
    }
}

/// Padding mode for tokenization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingMode {
    /// No padding
    None,
    /// Pad to max length in batch
    MaxLength,
    /// Pad to specific length
    FixedLength(usize),
}

/// Tokenized text with IDs and metadata
#[derive(Debug, Clone)]
pub struct TokenizedText {
    /// The token IDs
    pub ids: TokenIds,
    /// Attention mask (1 for tokens, 0 for padding)
    pub attention_mask: Option<Vec<u8>>,
    /// Original text that was tokenized
    pub original_text: String,
}

/// Tokenizer interface for text tokenization and processing
#[async_trait]
pub trait Tokenizer: Send + Sync + 'static {
    /// Tokenize a single text
    async fn tokenize(&self, text: &str, options: Option<TokenizerOptions>) -> Result<TokenizedText>;
    
    /// Tokenize a batch of texts
    async fn tokenize_batch(&self, texts: &[String], options: Option<TokenizerOptions>) -> Result<Vec<TokenizedText>>;
    
    /// Convert token IDs back to text
    async fn decode(&self, token_ids: &[TokenId]) -> Result<String>;
    
    /// Get the vocabulary size
    fn vocab_size(&self) -> usize;
    
    /// Get the ID for a special token
    fn special_token_id(&self, token: SpecialToken) -> Option<TokenId>;
    
    /// Get the maximum sequence length supported by the tokenizer
    fn max_len(&self) -> Option<usize>;
}

/// Mock tokenizer for testing
#[cfg(test)]
pub mod mock {
    use super::*;
    
    /// A simple mock tokenizer for testing
    pub struct MockTokenizer {
        vocab_size: usize,
        max_len: usize,
    }
    
    impl MockTokenizer {
        /// Create a new mock tokenizer
        pub fn new(vocab_size: usize, max_len: usize) -> Self {
            Self { vocab_size, max_len }
        }
    }
    
    #[async_trait]
    impl Tokenizer for MockTokenizer {
        async fn tokenize(&self, text: &str, options: Option<TokenizerOptions>) -> Result<TokenizedText> {
            let options = options.unwrap_or_default();
            
            // Simple character-based tokenization for testing
            let mut ids: TokenIds = text.chars()
                .map(|c| c as u32 % self.vocab_size as u32)
                .collect();
            
            if options.add_special_tokens {
                ids.insert(0, self.special_token_id(SpecialToken::Bos).unwrap_or(0));
                ids.push(self.special_token_id(SpecialToken::Eos).unwrap_or(1));
            }
            
            let attention_mask = Some(vec![1u8; ids.len()]);
            
            Ok(TokenizedText {
                ids,
                attention_mask,
                original_text: text.to_string(),
            })
        }
        
        async fn tokenize_batch(&self, texts: &[String], options: Option<TokenizerOptions>) -> Result<Vec<TokenizedText>> {
            let mut results = Vec::with_capacity(texts.len());
            
            for text in texts {
                results.push(self.tokenize(text, options.clone()).await?);
            }
            
            Ok(results)
        }
        
        async fn decode(&self, token_ids: &[TokenId]) -> Result<String> {
            // Simple decoding for testing - just convert tokens to ASCII
            let chars: Vec<char> = token_ids.iter()
                .map(|&id| {
                    let clamped_id = id % 128;
                    clamped_id as u8 as char
                })
                .collect();
            
            Ok(chars.into_iter().collect())
        }
        
        fn vocab_size(&self) -> usize {
            self.vocab_size
        }
        
        fn special_token_id(&self, token: SpecialToken) -> Option<TokenId> {
            match token {
                SpecialToken::Bos => Some(0),
                SpecialToken::Eos => Some(1),
                SpecialToken::Pad => Some(2),
                SpecialToken::Unk => Some(3),
                SpecialToken::Mask => Some(4),
                SpecialToken::Sep => Some(5),
                SpecialToken::Cls => Some(6),
            }
        }
        
        fn max_len(&self) -> Option<usize> {
            Some(self.max_len)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::mock::MockTokenizer;
    
    #[tokio::test]
    async fn test_mock_tokenizer_tokenize() {
        let tokenizer = MockTokenizer::new(1000, 512);
        
        // Test with default options (add special tokens)
        let result = tokenizer.tokenize("hello", None).await.unwrap();
        
        // Should have BOS + 5 chars + EOS
        assert_eq!(result.ids.len(), 7);
        assert_eq!(result.ids[0], 0); // BOS token
        assert_eq!(result.ids[6], 1); // EOS token
        
        // Check characters
        assert_eq!(result.ids[1], 'h' as u32);
        assert_eq!(result.ids[2], 'e' as u32);
        assert_eq!(result.ids[3], 'l' as u32);
        assert_eq!(result.ids[4], 'l' as u32);
        assert_eq!(result.ids[5], 'o' as u32);
        
        // Check attention mask
        assert_eq!(result.attention_mask, Some(vec![1u8; 7]));
        
        // Original text preserved
        assert_eq!(result.original_text, "hello");
    }
    
    #[tokio::test]
    async fn test_mock_tokenizer_no_special_tokens() {
        let tokenizer = MockTokenizer::new(1000, 512);
        let options = TokenizerOptions {
            add_special_tokens: false,
            ..Default::default()
        };
        
        let result = tokenizer.tokenize("hello", Some(options)).await.unwrap();
        
        // Should have just 5 chars, no special tokens
        assert_eq!(result.ids.len(), 5);
        
        // Check characters
        assert_eq!(result.ids[0], 'h' as u32);
        assert_eq!(result.ids[1], 'e' as u32);
        assert_eq!(result.ids[2], 'l' as u32);
        assert_eq!(result.ids[3], 'l' as u32);
        assert_eq!(result.ids[4], 'o' as u32);
    }
    
    #[tokio::test]
    async fn test_mock_tokenizer_batch() {
        let tokenizer = MockTokenizer::new(1000, 512);
        let texts = vec!["hello".to_string(), "world".to_string()];
        
        let results = tokenizer.tokenize_batch(&texts, None).await.unwrap();
        
        assert_eq!(results.len(), 2);
        
        // First result
        assert_eq!(results[0].original_text, "hello");
        assert_eq!(results[0].ids.len(), 7); // BOS + 5 chars + EOS
        
        // Second result
        assert_eq!(results[1].original_text, "world");
        assert_eq!(results[1].ids.len(), 7); // BOS + 5 chars + EOS
    }
    
    #[tokio::test]
    async fn test_mock_tokenizer_decode() {
        let tokenizer = MockTokenizer::new(1000, 512);
        
        // ASCII values for 'hello'
        let tokens = vec![104, 101, 108, 108, 111];
        let decoded = tokenizer.decode(&tokens).await.unwrap();
        
        assert_eq!(decoded, "hello");
    }
    
    #[test]
    fn test_tokenizer_metadata() {
        let tokenizer = MockTokenizer::new(1000, 512);
        
        assert_eq!(tokenizer.vocab_size(), 1000);
        assert_eq!(tokenizer.max_len(), Some(512));
        
        // Test special tokens
        assert_eq!(tokenizer.special_token_id(SpecialToken::Bos), Some(0));
        assert_eq!(tokenizer.special_token_id(SpecialToken::Eos), Some(1));
        assert_eq!(tokenizer.special_token_id(SpecialToken::Pad), Some(2));
        assert_eq!(tokenizer.special_token_id(SpecialToken::Unk), Some(3));
    }
} 