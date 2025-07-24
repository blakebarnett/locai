//! Candle-based embedding model implementation
//! 
//! This module provides an implementation of the `EmbeddingModel` trait using the Candle
//! machine learning framework. It supports loading and running various embedding models
//! including sentence transformers, BERT variants, and other embedding models.
//!
//! It also provides sentiment analysis capabilities using pre-trained transformer models.
//!
//! # Examples
//!
//! ```rust,no_run
//! use std::error::Error;
//! use locai::ml::candle::{CandleModelBuilder, CandleConfig};
//! use locai::ml::embedding::{EmbeddingModel, EmbeddingOptions};
//! use locai::ml::sentiment::{SentimentAnalyzer, SentimentConfig};
//! use locai::ml::model_manager::ModelManager;
//!
//! async fn example() -> Result<(), Box<dyn Error>> {
//!     // Create an embedding model using the builder pattern
//!     let model = CandleModelBuilder::new()
//!         .with_model("BAAI/bge-small-en")
//!         .name("BGE Small English")
//!         .embedding_dim(384)
//!         .build()
//!         .await?;
//!
//!     // Generate an embedding for a single text
//!     let text = "This is a test sentence.";
//!     let embedding = model.embed_text(text, None).await?;
//!     
//!     println!("Embedding dimension: {}", embedding.len());
//!     
//!     // Use sentiment analysis
//!     let model_manager = ModelManager::new("./model_cache");
//!     let sentiment_analyzer = SentimentAnalyzer::new(model_manager).await?;
//!     let sentiment = sentiment_analyzer.analyze_sentiment("I love this!").await?;
//!     println!("Sentiment: {:?} (confidence: {:.2})", sentiment.label, sentiment.confidence);
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Feature Flags
//!
//! This module requires the `candle-embeddings` feature flag. For GPU support,
//! enable the `cuda` feature flag.

pub mod model;
pub mod builder;
pub mod config;
pub mod utils;
pub mod tokenizer;

// Re-export the main types for public API
pub use self::model::CandleEmbeddingModel;
pub use self::builder::CandleModelBuilder;
pub use self::config::{CandleConfig, PoolingStrategy};
pub use self::utils::ModelCache;
pub use self::tokenizer::CandleTokenizer;

#[cfg(feature = "candle-embeddings")]
mod tests;

#[cfg(not(feature = "candle-embeddings"))]
compile_error!("The 'candle-embeddings' feature must be enabled to use the Candle implementation."); 