//! Utility functions for Candle models

use std::path::{Path, PathBuf};
use std::sync::Arc;

use candle_core::{DType, Device, Tensor};
use candle_core::Result as CandleResult;
use hf_hub::{api::sync::Api, RepoType};
use tokio::sync::Mutex;

use crate::ml::error::{MLError, Result};

/// Get the device to use for Candle operations
pub fn get_device() -> CandleResult<Device> {
    #[cfg(feature = "cuda")]
    {
        if let Ok(device) = Device::new_cuda(0) {
            return Ok(device);
        }
    }
    
    // Default to CPU if no accelerator is available
    Ok(Device::Cpu)
}

/// Convert a Candle error to an MLError
pub fn candle_err(err: candle_core::Error) -> MLError {
    MLError::embedding(format!("Candle error: {}", err))
}

/// A cache for model files that can be shared across instances
pub struct ModelCache {
    cache_dir: PathBuf,
    hub_api: Arc<Mutex<Api>>,
}

impl ModelCache {
    /// Create a new model cache
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let cache_dir = cache_dir.into();
        Self {
            cache_dir: cache_dir.clone(),
            hub_api: Arc::new(Mutex::new(Api::new().unwrap())), // no more repo_cache method
        }
    }
    
    /// Get a file from the cache or download it
    pub async fn get_file(&self, repo: &str, filename: &str) -> Result<PathBuf> {
        let api = self.hub_api.lock().await;
        
        let repo_id = repo.to_string();
        let filename = filename.to_string();
        
        // Move api out to avoid reference to self
        let api_cloned = api.clone();
        
        // Run in a blocking task to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(move || {
            let repo_type = if repo_id.contains('/') {
                RepoType::Model
            } else {
                RepoType::Dataset
            };
            
            // Create a repo instance with the type and id
            let _repo = hf_hub::Repo::with_revision(
                repo_id.clone(),
                repo_type,
                "main".to_string(),
            );
            
            // Now use api.model or api.dataset to get files
            match repo_type {
                RepoType::Model => api_cloned.model(repo_id).get(&filename),
                RepoType::Dataset => api_cloned.dataset(repo_id).get(&filename),
                _ => Err(hf_hub::api::sync::ApiError::from(
                    std::io::Error::new(std::io::ErrorKind::Other, "Unsupported repo type")
                )),
            }
            .map_err(|e| MLError::model_loading(format!("Failed to download {}: {}", filename, e)))
        })
        .await
        .map_err(|e| MLError::model_loading(format!("Task join error: {}", e)))?;
        
        result
    }
    
    /// Check if a file exists in the cache
    pub fn file_exists(&self, path: &Path) -> bool {
        // If path is relative, check against cache_dir
        if path.is_relative() {
            self.cache_dir.join(path).exists()
        } else {
            path.exists()
        }
    }

    /// List all files in a repository
    pub async fn list_files(&self, repo: &str) -> Result<Vec<String>> {
        let api = self.hub_api.lock().await;
        
        let repo_id = repo.to_string();
        
        // Move api out to avoid reference to self
        let api_cloned = api.clone();
        
        // Run in a blocking task to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(move || {
            let repo_type = if repo_id.contains('/') {
                RepoType::Model
            } else {
                RepoType::Dataset
            };
            
            // Create a repo instance with the type and id
            let _repo = hf_hub::Repo::with_revision(
                repo_id.clone(),
                repo_type,
                "main".to_string(),
            );
            
            // Use the info() method to get repository information including files
            let info_result = match repo_type {
                RepoType::Model => api_cloned.model(repo_id.clone()).info(),
                RepoType::Dataset => api_cloned.dataset(repo_id.clone()).info(),
                _ => Err(hf_hub::api::sync::ApiError::from(
                    std::io::Error::new(std::io::ErrorKind::Other, "Unsupported repo type")
                )),
            };
            
            match info_result {
                Ok(info) => {
                    let files: Vec<String> = info.siblings.iter().map(|s| s.rfilename.clone()).collect();
                    Ok(files)
                },
                Err(e) => {
                    Err(MLError::model_loading(format!("Failed to list files: {}", e)))
                }
            }
        })
        .await
        .map_err(|e| MLError::model_loading(format!("Task join error: {}", e)))?;
        
        result
    }

    /// Get all safetensors files for a model
    pub async fn get_safetensors_files(&self, repo: &str) -> Result<Vec<PathBuf>> {
        let files = self.list_files(repo).await?;
        
        // Filter for safetensors files and download them
        let mut safetensors_files = Vec::new();
        for file in &files {
            if file.ends_with(".safetensors") {
                let path = self.get_file(repo, &file).await?;
                safetensors_files.push(path);
            }
        }
        
        Ok(safetensors_files)
    }
}

/// Normalize a tensor to unit length along the specified dimension
pub fn normalize_tensor(tensor: &Tensor, dim: usize) -> CandleResult<Tensor> {
    // Fixed sum method
    let norm = tensor.sqr()?.sum(dim)?.sqrt()?;
    // Add dimension back for broadcasting
    let norm = norm.unsqueeze(dim)?;
    // Use broadcast_div for proper broadcasting
    tensor.broadcast_div(&norm)
}

/// Convert a tensor to a vec of f32
pub fn tensor_to_vec(tensor: &Tensor) -> CandleResult<Vec<f32>> {
    let tensor = tensor.to_dtype(DType::F32)?;
    tensor.to_vec1()
}

/// Apply pooling to a sequence of token embeddings
pub fn apply_pooling(
    token_embeddings: &Tensor,
    attention_mask: Option<&Tensor>,
    strategy: crate::ml::candle::config::PoolingStrategy,
) -> CandleResult<Tensor> {
    match strategy {
        crate::ml::candle::config::PoolingStrategy::Mean => {
            if let Some(mask) = attention_mask {
                // Mean pooling with attention mask
                let mask = mask.to_dtype(token_embeddings.dtype())?;
                let mask = mask.unsqueeze(2)?; // Add embedding dimension
                let masked_embeddings = token_embeddings.broadcast_mul(&mask)?;
                // Fixed sum method
                let sum = masked_embeddings.sum(1)?;
                let counts = mask.sum(1)?;
                // Avoid division by zero by adding a small epsilon
                let epsilon = Tensor::new(1e-9f32, counts.device())?;
                let counts_nonzero = counts.broadcast_add(&epsilon)?;
                sum.broadcast_div(&counts_nonzero)
            } else {
                // Simple mean pooling without mask
                token_embeddings.mean(1)
            }
        },
        crate::ml::candle::config::PoolingStrategy::Max => {
            // Max pooling (ignores attention mask)
            token_embeddings.max(1)
        },
        crate::ml::candle::config::PoolingStrategy::Cls => {
            // Use the first token (CLS) embedding for each sequence in the batch
            // token_embeddings shape: [batch_size, seq_len, hidden_size]
            // We want to extract [:, 0, :] which is the first token for each sequence
            token_embeddings.narrow(1, 0, 1)?.squeeze(1)
        },
        crate::ml::candle::config::PoolingStrategy::Last => {
            if let Some(mask) = attention_mask {
                // Get the last token for each sequence based on attention mask
                let seq_lengths = mask.sum(1)?;
                let batch_size = token_embeddings.dim(0)?;
                let hidden_size = token_embeddings.dim(2)?;
                
                let mut last_embeddings = Vec::with_capacity(batch_size);
                for i in 0..batch_size {
                    let length = seq_lengths.get(i)?.to_scalar::<u32>()? as i64;
                    if length > 0 {
                        let idx = length - 1;
                        // Fixed indexing
                        let last_emb = token_embeddings.get(i)?.get(idx as usize)?;
                        last_embeddings.push(last_emb);
                    } else {
                        // If sequence is empty (all masks are 0), use zeros
                        let zeros = Tensor::zeros((hidden_size,), token_embeddings.dtype(), token_embeddings.device())?;
                        last_embeddings.push(zeros);
                    }
                }
                
                Tensor::stack(&last_embeddings, 0)
            } else {
                // Without mask, use the last token for all sequences
                let seq_length = token_embeddings.dim(1)? - 1;
                // Fixed indexing for slices
                token_embeddings.narrow(0, 0, token_embeddings.dim(0)?)?.narrow(1, seq_length, 1)?.squeeze(1)
            }
        }
    }
}

/// Normalize each embedding
pub fn normalize_l2(embedding: &Tensor) -> CandleResult<Tensor> {
    let norms = embedding.sqr()?.sum(1)?.sqrt()?;
    embedding.broadcast_div(&norms)
}

/// Utility function to calculate L2 norm, assuming Tensor is a 1D vector
pub fn normalize_l2_vec(embeddings: &[Tensor]) -> CandleResult<Vec<Tensor>> {
    let mut normalized_embeddings = Vec::with_capacity(embeddings.len());
    for embedding in embeddings {
        let normalized = normalize_l2(embedding)?;
        normalized_embeddings.push(normalized);
    }
    Ok(normalized_embeddings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{Tensor, Device};
    use crate::ml::candle::config::PoolingStrategy;
    
    #[test]
    fn test_device_selection() {
        let device = get_device();
        assert!(device.is_ok());
    }
    
    #[test]
    fn test_normalize_tensor() {
        let device = Device::Cpu;
        let tensor = Tensor::new(&[[1.0f32, 2.0, 3.0], [4.0, 5.0, 6.0]], &device).unwrap();
        
        let normalized = normalize_tensor(&tensor, 1).unwrap();
        
        // Check that each row has unit norm
        let norms = normalized.sqr().unwrap().sum(1).unwrap().sqrt().unwrap();
        let norms = norms.to_vec1::<f32>().unwrap();
        
        for norm in norms {
            assert!((norm - 1.0).abs() < 1e-5);
        }
    }
    
    #[test]
    fn test_tensor_to_vec() {
        let device = Device::Cpu;
        let values = [1.0f32, 2.0, 3.0, 4.0];
        let tensor = Tensor::new(&values, &device).unwrap();
        
        let vec = tensor_to_vec(&tensor).unwrap();
        
        assert_eq!(vec, values);
    }
    
    #[test]
    fn test_mean_pooling() -> CandleResult<()> {
        let device = get_device()?;
        let embeddings = Tensor::new(
            &[[[1.0f32, 2.0], [3.0, 4.0], [5.0, 6.0]], // Seq 1
              [[7.0, 8.0], [9.0, 10.0], [0.0, 0.0]]]   // Seq 2 (padded)
            , &device)?;
        let attention_mask = Tensor::new(
            &[[1u8, 1, 1], [1, 1, 0]], &device)?;
        
        // With mask
        let pooled = apply_pooling(&embeddings, Some(&attention_mask), PoolingStrategy::Mean)?;
        let expected = Tensor::new(&[[3.0f32, 4.0], [8.0, 9.0]], &device)?;
        assert!((pooled.sub(&expected))?.abs()?.max_all()?.to_scalar::<f32>()? < 1e-5);

        // Without mask (should average including padding for seq 2)
        let pooled_no_mask = apply_pooling(&embeddings, None, PoolingStrategy::Mean)?;
        let expected_no_mask = Tensor::new(
            &[[3.0f32, 4.0], // (1+3+5)/3, (2+4+6)/3
              [5.3333335, 6.0]] // (7+9+0)/3, (8+10+0)/3
            , &device)?;
        assert!((pooled_no_mask.sub(&expected_no_mask))?.abs()?.max_all()?.to_scalar::<f32>()? < 1e-5);
        Ok(())
    }
    
    #[test]
    fn test_max_pooling() -> CandleResult<()> {
        let device = get_device()?;
        let embeddings = Tensor::new(
            &[[[1.0f32, 2.0], [5.0, 4.0], [3.0, 6.0]], // Seq 1
              [[7.0, 10.0], [9.0, 8.0], [0.0, 0.0]]]  // Seq 2 (padded)
            , &device)?;
        // Mask is ignored for max pooling in this implementation
        let attention_mask = Tensor::new(
            &[[1u8, 1, 1], [1, 1, 0]], &device)?;
        
        let pooled = apply_pooling(&embeddings, Some(&attention_mask), PoolingStrategy::Max)?;
        let expected = Tensor::new(&[[5.0f32, 6.0], [9.0, 10.0]], &device)?;
        assert!((pooled.sub(&expected))?.abs()?.max_all()?.to_scalar::<f32>()? < 1e-5);
        Ok(())
    }
    
    #[test]
    fn test_cls_pooling() -> CandleResult<()> {
        let device = get_device()?;
        let embeddings = Tensor::new(
            &[[[1.0f32, 2.0], [3.0, 4.0], [5.0, 6.0]], // Seq 1
              [[7.0, 8.0], [9.0, 10.0], [0.0, 0.0]]]   // Seq 2
            , &device)?;
        // Mask is ignored for CLS pooling
        let attention_mask = Tensor::new(
            &[[1u8, 1, 1], [1, 1, 1]], &device)?;
            
        let pooled = apply_pooling(&embeddings, Some(&attention_mask), PoolingStrategy::Cls)?;
        // CLS pooling should take the first token of each sequence in the batch
        // For batch of 2 sequences, we expect:
        // - First sequence: [1.0, 2.0] (first token)
        // - Second sequence: [7.0, 8.0] (first token)
        let expected = Tensor::new(&[[1.0f32, 2.0], [7.0, 8.0]], &device)?;
        assert!((pooled.sub(&expected))?.abs()?.max_all()?.to_scalar::<f32>()? < 1e-5);
        Ok(())
    }
} 