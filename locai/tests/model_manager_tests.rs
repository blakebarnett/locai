use locai::ml::model_manager::{EmbeddingManager, EmbeddingManagerBuilder};

#[test]
fn test_embedding_manager_creation() {
    let manager = EmbeddingManagerBuilder::new().build();
    
    // Basic creation should work
    assert!(manager.validate_embedding(&vec![1.0, 2.0, 3.0]).is_ok());
}

#[test]
fn test_embedding_validation() {
    let manager = EmbeddingManagerBuilder::new()
        .expected_dimensions(384)
        .build();
    
    // Valid embedding should pass
    let valid_embedding = vec![1.0; 384];
    assert!(manager.validate_embedding(&valid_embedding).is_ok());
    
    // Invalid dimension should fail
    let invalid_embedding = vec![1.0; 256];
    assert!(manager.validate_embedding(&invalid_embedding).is_err());
}

#[test]
fn test_embedding_manager_without_dimension_check() {
    let manager = EmbeddingManagerBuilder::new().build();
    
    // Without dimension validation, any size should work
    assert!(manager.validate_embedding(&vec![1.0; 128]).is_ok());
    assert!(manager.validate_embedding(&vec![1.0; 384]).is_ok());
    assert!(manager.validate_embedding(&vec![1.0; 1536]).is_ok());
}

#[test]
fn test_embedding_manager_empty_embedding() {
    let manager = EmbeddingManagerBuilder::new().build();
    
    // Empty embeddings should be rejected
    assert!(manager.validate_embedding(&vec![]).is_err());
}

#[test]
fn test_embedding_normalization() {
    let manager = EmbeddingManagerBuilder::new().build();
    
    let mut embedding = vec![3.0, 4.0, 0.0];
    assert!(manager.normalize_embedding(&mut embedding).is_ok());
    
    // Check that the normalized vector has unit length (approximately)
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((magnitude - 1.0).abs() < 1e-6);
    
    // Check actual values (3,4,0 normalized should be 0.6,0.8,0)
    assert!((embedding[0] - 0.6).abs() < 1e-6);
    assert!((embedding[1] - 0.8).abs() < 1e-6);
    assert!((embedding[2] - 0.0).abs() < 1e-6);
}

#[test]
fn test_zero_vector_normalization() {
    let manager = EmbeddingManagerBuilder::new().build();
    
    let mut embedding = vec![0.0, 0.0, 0.0];
    
    // Zero vectors should fail to normalize
    assert!(manager.normalize_embedding(&mut embedding).is_err());
}

#[test]
fn test_invalid_values() {
    let manager = EmbeddingManagerBuilder::new().build();
    
    // NaN values should be rejected
    assert!(manager.validate_embedding(&vec![1.0, f32::NAN, 3.0]).is_err());
    
    // Infinite values should be rejected  
    assert!(manager.validate_embedding(&vec![1.0, f32::INFINITY, 3.0]).is_err());
    assert!(manager.validate_embedding(&vec![1.0, f32::NEG_INFINITY, 3.0]).is_err());
}

#[test]
fn test_expected_dimensions_getter() {
    let manager_without_dims = EmbeddingManagerBuilder::new().build();
    assert_eq!(manager_without_dims.expected_dimensions(), None);
    
    let manager_with_dims = EmbeddingManagerBuilder::new()
        .expected_dimensions(512)
        .build();
    assert_eq!(manager_with_dims.expected_dimensions(), Some(512));
} 