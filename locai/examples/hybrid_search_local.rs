//! Example demonstrating BYOE (Bring Your Own Embeddings) with local models
//!
//! This example shows how to use Locai with local embedding models via fastembed,
//! demonstrating cost-effective local processing in the BYOE approach.

use anyhow::Result;
use locai::ml::EmbeddingManager;
use locai::prelude::{Locai, MemoryBuilder};
use std::time::Instant;

// Mock fastembed implementation for demonstration
// In real usage, you'd use the actual fastembed crate
struct MockFastembedModel {
    model_name: String,
    dimensions: usize,
}

impl MockFastembedModel {
    /// Initialize a local embedding model via fastembed
    /// In real usage: fastembed::TextEmbedding::try_new(InitOptions::default())?
    fn new(model_name: &str) -> Result<Self> {
        println!("🔄 [Mock] Loading local embedding model: {}", model_name);

        // Mock different model dimensions
        let dimensions = match model_name {
            "BAAI/bge-small-en-v1.5" => 384,
            "sentence-transformers/all-MiniLM-L6-v2" => 384,
            "sentence-transformers/all-mpnet-base-v2" => 768,
            _ => 384,
        };

        Ok(Self {
            model_name: model_name.to_string(),
            dimensions,
        })
    }

    /// Generate embeddings for multiple texts (batch processing)
    /// In real usage: model.embed(texts, None)?
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        println!(
            "🧠 [Mock] Generating {} local embeddings with {}",
            texts.len(),
            self.model_name
        );

        let embeddings: Vec<Vec<f32>> = texts
            .iter()
            .map(|text| {
                // Mock embedding generation
                (0..self.dimensions)
                    .map(|i| {
                        let base = (i as f32 * 0.003 + text.len() as f32 * 0.02).sin();
                        // Add some text-specific variation
                        base + (text.chars().nth(i % text.len()).unwrap_or('a') as u8 as f32
                            * 0.001)
                    })
                    .collect()
            })
            .collect();

        Ok(embeddings)
    }

    /// Generate embedding for a single text
    fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let batch_result = self.embed_batch(&[text])?;
        Ok(batch_result.into_iter().next().unwrap())
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 BYOE Example: Local Embeddings with fastembed");
    println!("===============================================");

    // 1. Initialize Locai
    let locai = Locai::new().await?;
    println!("✅ Locai initialized with embedded SurrealDB");

    // 2. Initialize local embedding model
    let model_name = "BAAI/bge-small-en-v1.5";
    let start = Instant::now();
    let embedding_model = MockFastembedModel::new(model_name)?;
    println!("✅ Local embedding model loaded in {:?}", start.elapsed());
    println!("   Model: {}", model_name);
    println!("   Dimensions: {}", embedding_model.dimensions());

    // 3. Initialize embedding manager
    let embedding_manager =
        EmbeddingManager::with_expected_dimensions(embedding_model.dimensions());

    // 4. Prepare document collection for batch processing
    let documents = [
        "Machine learning algorithms can automatically improve through experience",
        "Rust programming language provides memory safety without garbage collection",
        "Climate change mitigation requires renewable energy adoption worldwide",
        "The Renaissance period marked a cultural awakening in European history",
        "Quantum mechanics describes the behavior of matter at atomic scales",
        "Database indexing improves query performance for large datasets",
        "Neural networks are inspired by biological brain structure and function",
        "Sustainable agriculture practices help preserve soil health and biodiversity",
        "Blockchain technology enables decentralized and transparent transactions",
        "Space exploration advances our understanding of the universe",
    ];

    println!(
        "\n📚 Batch Processing {} Documents with Local Embeddings",
        documents.len()
    );
    println!("=======================================================");

    // 5. Batch generate embeddings (efficient for local processing)
    let batch_start = Instant::now();
    let embeddings = embedding_model.embed_batch(&documents)?;
    let batch_duration = batch_start.elapsed();

    println!(
        "✅ Generated {} embeddings in {:?}",
        embeddings.len(),
        batch_duration
    );
    println!(
        "   Average time per embedding: {:?}",
        batch_duration / documents.len() as u32
    );

    // 6. Create memories with local embeddings
    println!("\n💾 Storing Memories with Local Embeddings");
    let mut memory_ids = Vec::new();

    for (i, (doc, embedding)) in documents.iter().zip(embeddings.iter()).enumerate() {
        // Validate embedding
        embedding_manager.validate_embedding(embedding)?;

        // Create memory with local embedding
        let memory = MemoryBuilder::new_with_content(*doc)
            .source("local_knowledge_base")
            .tags(vec![&format!("doc_{}", i + 1), "local", "fastembed"])
            .embedding(embedding.clone())
            .build();

        let memory_id = locai.manager().store_memory(memory).await?;
        memory_ids.push(memory_id);

        if i < 3 {
            // Show first few for brevity
            println!(
                "   ✅ Memory {}: {}",
                i + 1,
                doc.chars().take(50).collect::<String>()
            );
        }
    }

    if documents.len() > 3 {
        println!("   ... and {} more memories", documents.len() - 3);
    }

    // 7. Demonstrate different search modes
    println!("\n🔍 Search Mode Comparison");
    println!("========================");

    let test_queries = [
        "artificial intelligence and neural networks",
        "sustainable development practices",
        "database optimization techniques",
    ];

    for query in test_queries {
        println!("\n🔎 Query: '{}'", query);
        println!("   ---");

        // BM25 Text Search
        let text_start = Instant::now();
        let text_results = locai.search(query).await?;
        let text_duration = text_start.elapsed();

        println!("   📝 BM25 Text Search ({:?}):", text_duration);
        for (i, result) in text_results.iter().take(2).enumerate() {
            println!(
                "      {}. [Score: {:.3}] {}",
                i + 1,
                result.score,
                result.summary().chars().take(50).collect::<String>()
            );
        }

        // Local Vector Search Preparation
        let vector_start = Instant::now();
        let query_embedding = embedding_model.embed_single(query)?;
        let vector_prep_duration = vector_start.elapsed();

        println!("   🧠 Vector Search Prep ({:?}):", vector_prep_duration);
        println!(
            "      ✅ Generated local query embedding ({} dims)",
            query_embedding.len()
        );
        println!("      💡 Ready for semantic similarity search");
        println!(
            "      📊 Would compare against {} stored embeddings",
            memory_ids.len()
        );
    }

    // 8. Performance comparison
    println!("\n⚡ Performance Analysis");
    println!("======================");

    // Single vs batch embedding generation
    let single_start = Instant::now();
    let _single_embedding = embedding_model.embed_single("Performance test text")?;
    let single_duration = single_start.elapsed();

    let batch_test_texts = ["Test 1", "Test 2", "Test 3"];
    let batch_test_start = Instant::now();
    let _batch_embeddings = embedding_model.embed_batch(&batch_test_texts)?;
    let batch_test_duration = batch_test_start.elapsed();

    println!("Single embedding generation: {:?}", single_duration);
    println!(
        "Batch embedding generation (3 texts): {:?}",
        batch_test_duration
    );
    println!(
        "Batch efficiency: {:.1}x faster per text",
        (single_duration.as_nanos() * 3) as f32 / batch_test_duration.as_nanos() as f32
    );

    // 9. Local vs Remote comparison
    println!("\n🏠 Local vs Remote Embedding Comparison");
    println!("=======================================");

    println!("Local Embeddings (fastembed):");
    println!("   ✅ No API costs");
    println!("   ✅ No rate limits");
    println!("   ✅ Private data stays local");
    println!("   ✅ Consistent performance");
    println!("   ✅ Batch processing efficient");
    println!("   ⚠️  Requires local compute resources");
    println!("   ⚠️  Model storage requirements (~100MB-1GB)");

    println!("\nRemote Embeddings (OpenAI, Cohere, etc.):");
    println!("   ✅ Latest models without local storage");
    println!("   ✅ No compute requirements");
    println!("   ✅ Automatically updated models");
    println!("   ⚠️  API costs and rate limits");
    println!("   ⚠️  Network dependency");
    println!("   ⚠️  Data leaves your infrastructure");

    // 10. Show memory statistics
    println!("\n📊 Final Statistics");
    println!("==================");
    let total_memories = locai.recent_memories(Some(1000)).await?.len();
    let memories_with_embeddings = locai
        .recent_memories(Some(1000))
        .await?
        .iter()
        .filter(|m| m.embedding.is_some())
        .count();

    println!("Total memories created: {}", total_memories);
    println!("Memories with embeddings: {}", memories_with_embeddings);
    println!("Local embedding model: {}", model_name);
    println!("Embedding dimensions: {}", embedding_model.dimensions());

    println!("\n🎉 Local BYOE Example completed successfully!");
    println!("   Next steps:");
    println!("   • Install fastembed: cargo add fastembed");
    println!("   • Replace MockFastembedModel with real fastembed calls");
    println!("   • Try different local models for your use case");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_fastembed_model() {
        let model = MockFastembedModel::new("BAAI/bge-small-en-v1.5").unwrap();
        assert_eq!(model.dimensions(), 384);

        let embedding = model.embed_single("test").unwrap();
        assert_eq!(embedding.len(), 384);

        let embeddings = model.embed_batch(&["test1", "test2"]).unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 384);
    }

    #[tokio::test]
    async fn test_embedding_validation() {
        let model = MockFastembedModel::new("sentence-transformers/all-MiniLM-L6-v2").unwrap();
        let embedding = model.embed_single("test").unwrap();

        let manager = EmbeddingManager::with_expected_dimensions(384);
        assert!(manager.validate_embedding(&embedding).is_ok());
    }
}
