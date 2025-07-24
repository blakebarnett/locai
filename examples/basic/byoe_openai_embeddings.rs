//! Example demonstrating BYOE (Bring Your Own Embeddings) with OpenAI
//! 
//! This example shows how to use Locai with external embedding providers like OpenAI,
//! demonstrating the flexibility of the BYOE approach for hybrid search.

use locai::{Locai, MemoryBuilder};
use locai::ml::EmbeddingManager;
use anyhow::Result;
use serde_json;

// Mock OpenAI client for demonstration
// In real usage, you'd use the actual OpenAI SDK
struct MockOpenAIClient {
    api_key: String,
}

impl MockOpenAIClient {
    fn new(api_key: String) -> Self {
        Self { api_key }
    }
    
    /// Generate embeddings using OpenAI's text-embedding-3-small model
    /// In real usage, this would make an HTTP request to OpenAI's API
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        // Mock implementation - in reality you'd call:
        // POST https://api.openai.com/v1/embeddings
        // with model: "text-embedding-3-small" and input: text
        
        println!("🤖 [Mock] Generating OpenAI embedding for: '{}'", 
                 text.chars().take(50).collect::<String>());
        
        // Return a mock embedding (1536 dimensions for text-embedding-3-small)
        // In real usage, this would be the actual embedding from OpenAI
        let mock_embedding: Vec<f32> = (0..1536)
            .map(|i| (i as f32 * 0.001 + text.len() as f32 * 0.01).sin())
            .collect();
        
        Ok(mock_embedding)
    }
    
    /// Batch embed multiple texts (more efficient)
    async fn embed_texts(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        println!("🤖 [Mock] Batch generating {} OpenAI embeddings", texts.len());
        
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed_text(text).await?);
        }
        
        Ok(embeddings)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 BYOE Example: Using OpenAI Embeddings with Locai");
    println!("================================================");
    
    // 1. Initialize Locai with embedded storage (no local models needed!)
    let locai = Locai::new().await?;
    println!("✅ Locai initialized with embedded SurrealDB");
    
    // 2. Initialize OpenAI client (use your API key)
    let openai_client = MockOpenAIClient::new(
        std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "mock-key".to_string())
    );
    
    // 3. Initialize embedding manager for validation (optional)
    let embedding_manager = EmbeddingManager::with_expected_dimensions(1536);
    println!("✅ Embedding manager configured for OpenAI dimensions (1536)");
    
    // 4. Create memories with OpenAI embeddings
    let texts = [
        "Machine learning is transforming the technology industry",
        "Rust is a systems programming language focused on safety and performance",
        "Climate change requires urgent global action and cooperation",
        "The Renaissance was a period of cultural rebirth in Europe",
        "Quantum computing could revolutionize cryptography and drug discovery",
    ];
    
    println!("\n📝 Creating memories with OpenAI embeddings...");
    let mut memory_ids = Vec::new();
    
    for (i, text) in texts.iter().enumerate() {
        // Generate embedding using OpenAI
        let embedding = openai_client.embed_text(text).await?;
        
        // Validate embedding (optional)
        embedding_manager.validate_embedding(&embedding)?;
        
        // Create memory with user-provided embedding
        let memory = MemoryBuilder::new()
            .content(text)
            .source("byoe_example")
            .tags(vec![&format!("example_{}", i + 1), "byoe", "openai"])
            .embedding(embedding)  // ← This is the key: user provides embedding
            .build();
        
        let memory_id = locai.create_memory(memory).await?;
        memory_ids.push(memory_id.clone());
        
        println!("   ✅ Memory {}: {}", i + 1, text.chars().take(50).collect::<String>());
    }
    
    // 5. Demonstrate BM25 search (works without embeddings)
    println!("\n🔍 BM25 Search (text-only, always available):");
    let bm25_results = locai.search("machine learning").await?;
    for (i, result) in bm25_results.iter().take(3).enumerate() {
        println!("   {}. [Score: {:.3}] {}", 
                 i + 1,
                 result.score.unwrap_or(0.0),
                 result.memory.content.chars().take(60).collect::<String>());
    }
    
    // 6. Demonstrate hybrid search with user-provided query embedding
    println!("\n🔍 Hybrid Search (BM25 + Vector with OpenAI query embedding):");
    
    // Generate embedding for search query
    let query = "programming languages and software development";
    let query_embedding = openai_client.embed_text(query).await?;
    embedding_manager.validate_embedding(&query_embedding)?;
    
    // For demonstration, we'll show how a hybrid search could work
    // Note: This would require the hybrid search implementation from task 003
    println!("   Query: '{}'", query);
    println!("   ✅ Generated OpenAI embedding for query ({} dimensions)", query_embedding.len());
    println!("   💡 Hybrid search would combine:");
    println!("      - BM25 text matching for '{}' ", query);
    println!("      - Vector similarity using OpenAI embeddings");
    println!("      - Reciprocal Rank Fusion to merge results");
    
    // For now, just show BM25 results
    let hybrid_results = locai.search(query).await?;
    for (i, result) in hybrid_results.iter().take(3).enumerate() {
        println!("   {}. [Score: {:.3}] {}", 
                 i + 1,
                 result.score.unwrap_or(0.0),
                 result.memory.content.chars().take(60).collect::<String>());
    }
    
    // 7. Show memory statistics
    println!("\n📊 Memory Statistics:");
    let total_memories = locai.list_memories(None, None, None).await?.len();
    let memories_with_embeddings = locai.list_memories(None, None, None).await?
        .iter()
        .filter(|m| m.embedding.is_some())
        .count();
    
    println!("   Total memories: {}", total_memories);
    println!("   Memories with embeddings: {}", memories_with_embeddings);
    println!("   Embedding provider: OpenAI text-embedding-3-small");
    
    // 8. Demonstrate cost-conscious approach
    println!("\n💰 BYOE Benefits:");
    println!("   ✅ No local model storage (saves disk space)");
    println!("   ✅ No GPU requirements (reduces hardware costs)");
    println!("   ✅ Use latest embedding models (OpenAI, Cohere, etc.)");
    println!("   ✅ BM25 search works without embeddings (always fast)");
    println!("   ✅ Hybrid search when embeddings are provided");
    println!("   ✅ You control embedding costs and quality");
    
    println!("\n🎉 BYOE Example completed successfully!");
    println!("    Next steps: Try with real OpenAI API key and other providers!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_openai_embeddings() {
        let client = MockOpenAIClient::new("test-key".to_string());
        let embedding = client.embed_text("test").await.unwrap();
        assert_eq!(embedding.len(), 1536);
        
        let manager = EmbeddingManager::with_expected_dimensions(1536);
        assert!(manager.validate_embedding(&embedding).is_ok());
    }
    
    #[tokio::test]
    async fn test_batch_embeddings() {
        let client = MockOpenAIClient::new("test-key".to_string());
        let texts = vec!["text1", "text2", "text3"];
        let embeddings = client.embed_texts(&texts).await.unwrap();
        
        assert_eq!(embeddings.len(), 3);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 1536);
        }
    }
} 