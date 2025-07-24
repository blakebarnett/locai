//! Example demonstrating BYOE (Bring Your Own Embeddings) with Cohere
//! 
//! This example shows how to use Locai with Cohere's embedding API,
//! demonstrating provider flexibility in the BYOE approach.

use locai::{Locai, MemoryBuilder};
use locai::ml::EmbeddingManager;
use anyhow::Result;

// Mock Cohere client for demonstration
// In real usage, you'd use the actual Cohere SDK
struct MockCohereClient {
    api_key: String,
}

impl MockCohereClient {
    fn new(api_key: String) -> Self {
        Self { api_key }
    }
    
    /// Generate embeddings using Cohere's embed-english-v3.0 model
    /// In real usage, this would make an HTTP request to Cohere's API
    async fn embed_text(&self, text: &str, input_type: &str) -> Result<Vec<f32>> {
        // Mock implementation - in reality you'd call:
        // POST https://api.cohere.ai/v1/embed
        // with model: "embed-english-v3.0" and input_type: "search_document" or "search_query"
        
        println!("🧠 [Mock] Generating Cohere embedding ({}): '{}'", 
                 input_type,
                 text.chars().take(50).collect::<String>());
        
        // Return a mock embedding (1024 dimensions for embed-english-v3.0)
        // In real usage, this would be the actual embedding from Cohere
        let mock_embedding: Vec<f32> = (0..1024)
            .map(|i| {
                let base = (i as f32 * 0.002 + text.len() as f32 * 0.015).cos();
                // Slightly different values based on input type
                if input_type == "search_query" {
                    base * 1.1
                } else {
                    base
                }
            })
            .collect();
        
        Ok(mock_embedding)
    }
    
    /// Embed text as a document for storage
    async fn embed_document(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_text(text, "search_document").await
    }
    
    /// Embed text as a query for search
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_text(text, "search_query").await
    }
    
    /// Batch embed multiple texts as documents
    async fn embed_documents(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        println!("🧠 [Mock] Batch generating {} Cohere document embeddings", texts.len());
        
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed_document(text).await?);
        }
        
        Ok(embeddings)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 BYOE Example: Using Cohere Embeddings with Locai");
    println!("=================================================");
    
    // 1. Initialize Locai with embedded storage
    let locai = Locai::new().await?;
    println!("✅ Locai initialized with embedded SurrealDB");
    
    // 2. Initialize Cohere client
    let cohere_client = MockCohereClient::new(
        std::env::var("COHERE_API_KEY").unwrap_or_else(|_| "mock-key".to_string())
    );
    
    // 3. Initialize embedding manager for Cohere dimensions
    let embedding_manager = EmbeddingManager::with_expected_dimensions(1024);
    println!("✅ Embedding manager configured for Cohere dimensions (1024)");
    
    // 4. Create knowledge base with Cohere embeddings
    let knowledge_docs = [
        "Artificial Intelligence encompasses machine learning, deep learning, and neural networks to create intelligent systems.",
        "Sustainable energy solutions include solar panels, wind turbines, and battery storage technologies.",
        "The human brain contains approximately 86 billion neurons connected through trillions of synapses.",
        "Blockchain technology enables decentralized and secure transaction recording across distributed networks.",
        "Space exploration missions have discovered thousands of exoplanets in potentially habitable zones.",
    ];
    
    println!("\n📚 Building knowledge base with Cohere document embeddings...");
    let mut memory_ids = Vec::new();
    
    for (i, doc) in knowledge_docs.iter().enumerate() {
        // Generate document embedding using Cohere
        let embedding = cohere_client.embed_document(doc).await?;
        
        // Validate embedding
        embedding_manager.validate_embedding(&embedding)?;
        
        // Create memory with Cohere embedding
        let memory = MemoryBuilder::new()
            .content(doc)
            .source("cohere_knowledge_base")
            .tags(vec![&format!("doc_{}", i + 1), "cohere", "knowledge"])
            .embedding(embedding)
            .build();
        
        let memory_id = locai.create_memory(memory).await?;
        memory_ids.push(memory_id);
        
        println!("   ✅ Document {}: {}", i + 1, doc.chars().take(60).collect::<String>());
    }
    
    // 5. Demonstrate different search approaches
    println!("\n🔍 Search Comparison: BM25 vs. Vector (with Cohere)");
    println!("=====================================================");
    
    let queries = [
        "machine learning algorithms",
        "renewable energy technology",  
        "space and planets",
    ];
    
    for query in queries {
        println!("\n🔎 Query: '{}'", query);
        println!("   ---");
        
        // BM25 Search (always available)
        println!("   📝 BM25 Results:");
        let bm25_results = locai.search(query).await?;
        for (i, result) in bm25_results.iter().take(2).enumerate() {
            println!("      {}. [Score: {:.3}] {}", 
                     i + 1,
                     result.score.unwrap_or(0.0),
                     result.memory.content.chars().take(50).collect::<String>());
        }
        
        // Vector Search Simulation (using Cohere query embeddings)
        println!("   🧠 Vector Search (with Cohere query embedding):");
        let query_embedding = cohere_client.embed_query(query).await?;
        embedding_manager.validate_embedding(&query_embedding)?;
        
        println!("      ✅ Generated Cohere query embedding ({} dims)", query_embedding.len());
        println!("      💡 In hybrid search, this would find semantically similar content");
        println!("         even without exact keyword matches");
        
        // For demonstration, show that we could do vector similarity
        println!("      📊 Would compare with {} stored document embeddings", memory_ids.len());
    }
    
    // 6. Demonstrate embedding normalization
    println!("\n🔧 Embedding Processing with EmbeddingManager:");
    let mut test_embedding = cohere_client.embed_document("test normalization").await?;
    println!("   Original embedding magnitude: {:.6}", 
             test_embedding.iter().map(|x| x * x).sum::<f32>().sqrt());
    
    embedding_manager.normalize_embedding(&mut test_embedding)?;
    println!("   Normalized embedding magnitude: {:.6}", 
             test_embedding.iter().map(|x| x * x).sum::<f32>().sqrt());
    
    // 7. Show provider comparison
    println!("\n🏢 Provider Comparison:");
    println!("   Cohere embed-english-v3.0:");
    println!("     • Dimensions: 1024");
    println!("     • Input types: search_document, search_query, classification, clustering");
    println!("     • Optimized for: Information retrieval and semantic search");
    println!("     • Benefits: Specialized input types, good for search use cases");
    
    println!("   OpenAI text-embedding-3-small:");
    println!("     • Dimensions: 1536");
    println!("     • General purpose embedding model");
    println!("     • Benefits: High quality, widely supported");
    
    // 8. Memory statistics
    println!("\n📊 Final Statistics:");
    let all_memories = locai.list_memories(None, None, None).await?;
    let cohere_memories = all_memories.iter()
        .filter(|m| m.source == "cohere_knowledge_base")
        .count();
    
    println!("   Total memories: {}", all_memories.len());
    println!("   Cohere-embedded memories: {}", cohere_memories);
    println!("   Provider: Cohere embed-english-v3.0");
    
    // 9. Show BYOE advantages
    println!("\n🎯 BYOE with Multiple Providers:");
    println!("   ✅ Choose the best embedding model for your use case");
    println!("   ✅ Mix providers in the same system (e.g., OpenAI + Cohere)");
    println!("   ✅ Upgrade to newer models without changing Locai");
    println!("   ✅ Specialized embeddings (queries vs documents)");
    println!("   ✅ Control costs and rate limits per provider");
    
    println!("\n🎉 Cohere BYOE Example completed successfully!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cohere_embedding_types() {
        let client = MockCohereClient::new("test-key".to_string());
        
        let doc_embedding = client.embed_document("test document").await.unwrap();
        let query_embedding = client.embed_query("test query").await.unwrap();
        
        assert_eq!(doc_embedding.len(), 1024);
        assert_eq!(query_embedding.len(), 1024);
        
        // Query and document embeddings should be slightly different
        assert_ne!(doc_embedding, query_embedding);
    }
    
    #[tokio::test]
    async fn test_cohere_validation() {
        let client = MockCohereClient::new("test-key".to_string());
        let manager = EmbeddingManager::with_expected_dimensions(1024);
        
        let embedding = client.embed_document("test").await.unwrap();
        assert!(manager.validate_embedding(&embedding).is_ok());
        
        // Test wrong dimensions
        let wrong_manager = EmbeddingManager::with_expected_dimensions(1536);
        assert!(wrong_manager.validate_embedding(&embedding).is_err());
    }
} 