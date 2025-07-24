//! Search Mode Comparison Example
//! 
//! This example demonstrates the differences between Text (BM25), Vector, and Hybrid search modes,
//! showing performance characteristics and use cases for each approach in the BYOE pattern.

use locai::prelude::{Locai, MemoryBuilder};
use locai::memory::SearchMode;
use locai::ml::EmbeddingManager;
use anyhow::Result;
use std::time::Instant;


// Mock embedding service for comparison
struct MockEmbeddingService {
    name: String,
    dimensions: usize,
    latency_ms: u64,
}

impl MockEmbeddingService {
    fn new(name: &str, dimensions: usize, latency_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            dimensions,
            latency_ms,
        }
    }
    
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Simulate API latency
        tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;
        
        // Generate mock embedding based on text content
        let embedding: Vec<f32> = (0..self.dimensions)
            .map(|i| {
                let base = (i as f32 * 0.01 + text.len() as f32 * 0.05).cos();
                // Add text-specific variance
                let char_influence = text.chars().nth(i % text.len()).unwrap_or('a') as u8 as f32 * 0.001;
                base + char_influence
            })
            .collect();
        
        Ok(embedding)
    }
}

#[derive(Debug)]
struct SearchBenchmark {
    mode: SearchMode,
    query: String,
    duration: std::time::Duration,
    result_count: usize,
    embedding_time: Option<std::time::Duration>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 Search Mode Comparison: Text vs Vector vs Hybrid");
    println!("===================================================");
    
    // 1. Initialize Locai
    let locai = Locai::new().await?;
    println!("✅ Locai initialized");
    
    // 2. Setup embedding services for comparison
    let openai_service = MockEmbeddingService::new("OpenAI", 1536, 150);  // ~150ms latency
    let cohere_service = MockEmbeddingService::new("Cohere", 1024, 120);  // ~120ms latency
    let local_service = MockEmbeddingService::new("Local", 384, 10);      // ~10ms latency
    
    let embedding_manager = EmbeddingManager::with_expected_dimensions(1536);
    
    println!("🔧 Embedding Services Configured:");
    println!("   • OpenAI (1536d, ~150ms latency)");
    println!("   • Cohere (1024d, ~120ms latency)");  
    println!("   • Local (384d, ~10ms latency)");
    
    // 3. Create test dataset with diverse content
    let documents = [
        ("tech", "Machine learning algorithms enable computers to learn from data without explicit programming"),
        ("science", "Quantum mechanics describes the fundamental behavior of matter and energy at atomic scales"),
        ("business", "Market analysis reveals consumer preferences and drives strategic decision making"),
        ("health", "Regular exercise and balanced nutrition are essential for maintaining optimal health"),
        ("education", "Effective teaching methods adapt to different learning styles and student needs"),
        ("environment", "Climate change mitigation requires global cooperation and sustainable practices"),
        ("art", "Renaissance paintings demonstrate masterful use of perspective and light techniques"),
        ("technology", "Cloud computing provides scalable infrastructure for modern software applications"),
        ("psychology", "Cognitive behavioral therapy helps individuals modify negative thought patterns"),
        ("history", "The Industrial Revolution transformed societies through mechanization and urbanization"),
        ("physics", "Einstein's theory of relativity revolutionized our understanding of space and time"),
        ("economics", "Supply and demand dynamics determine pricing in competitive markets"),
        ("medicine", "Precision medicine uses genetic information to customize treatment approaches"),
        ("philosophy", "Existentialism explores the meaning of individual existence and personal freedom"),
        ("biology", "DNA replication ensures genetic information is accurately passed to new cells"),
    ];
    
    println!("\n📚 Creating Knowledge Base with {} Documents", documents.len());
    println!("==============================================");
    
    // 4. Create memories with OpenAI embeddings for vector search capability
    let mut memory_ids = Vec::new();
    let creation_start = Instant::now();
    
    for (i, (category, content)) in documents.iter().enumerate() {
        // Generate embedding using OpenAI service
        let embedding = openai_service.embed(content).await?;
        embedding_manager.validate_embedding(&embedding)?;
        
                 let memory = MemoryBuilder::new_with_content(*content)
             .source("benchmark_dataset")
             .tags(vec![category, "benchmark"])
             .embedding(embedding)
             .build();
        
                 let memory_id = locai.manager().store_memory(memory).await?;
        memory_ids.push(memory_id);
        
        if i < 3 {
            println!("   ✅ [{}] {}", category, content.chars().take(60).collect::<String>());
        }
    }
    
    let creation_duration = creation_start.elapsed();
    println!("   ... and {} more documents", documents.len() - 3);
    println!("✅ Knowledge base created in {:?}", creation_duration);
    
    // 5. Define test queries with different characteristics
    let test_queries = [
        ("Exact keyword", "machine learning algorithms"),           // Should match well with BM25
        ("Conceptual", "artificial intelligence and neural networks"), // Better for vector search
        ("Partial match", "quantum physics theory"),                // Hybrid might excel
        ("Business query", "market trends and consumer behavior"),   // Test domain-specific matching
        ("Medical query", "treatment approaches for patients"),      // Technical vocabulary
    ];
    
    println!("\n🔍 Search Mode Benchmarking");
    println!("============================");
    
    let mut all_benchmarks = Vec::new();
    
    for (query_type, query) in test_queries {
        println!("\n📋 Query Type: {} - '{}'", query_type, query);
        println!("   ---");
        
        // Text Search (BM25) - Always available, no embedding needed
        let text_start = Instant::now();
        let text_results = locai.search(query).await?;
        let text_duration = text_start.elapsed();
        
        let text_benchmark = SearchBenchmark {
            mode: SearchMode::Text,
            query: query.to_string(),
            duration: text_duration,
            result_count: text_results.len(),
            embedding_time: None,
        };
        
        println!("   📝 Text Search (BM25):");
        println!("      Duration: {:?}", text_duration);
        println!("      Results: {}", text_results.len());
        if let Some(first) = text_results.first() {
            println!("      Top result: [Score: {:.3}] {}", 
                     first.score,
                     first.summary().chars().take(50).collect::<String>());
        }
        
        // Vector Search - Requires query embedding
        let vector_embed_start = Instant::now();
        let query_embedding = openai_service.embed(query).await?;
        let vector_embed_duration = vector_embed_start.elapsed();
        
        let vector_search_start = Instant::now();
        let vector_results = locai.search_for(query)
            .mode(SearchMode::Vector)
            .with_query_embedding(query_embedding)
            .execute().await?;
        let vector_search_duration = vector_search_start.elapsed();
        let total_vector_duration = vector_embed_duration + vector_search_duration;
        
        let vector_benchmark = SearchBenchmark {
            mode: SearchMode::Vector,
            query: query.to_string(),
            duration: total_vector_duration,
            result_count: vector_results.len(),
            embedding_time: Some(vector_embed_duration),
        };
        
        println!("   🧠 Vector Search:");
        println!("      Embedding time: {:?}", vector_embed_duration);
        println!("      Search time: {:?}", vector_search_duration);
        println!("      Total duration: {:?}", total_vector_duration);
        println!("      Results: {}", vector_results.len());
        if let Some(first) = vector_results.first() {
            println!("      Top result: {}", 
                     first.content.chars().take(50).collect::<String>());
        }
        
        // Hybrid Search - Combines both approaches
        let hybrid_embed_start = Instant::now();
        let hybrid_query_embedding = openai_service.embed(query).await?;
        let hybrid_embed_duration = hybrid_embed_start.elapsed();
        
        let hybrid_search_start = Instant::now();
        let hybrid_results = locai.search_for(query)
            .mode(SearchMode::Hybrid)
            .with_query_embedding(hybrid_query_embedding)
            .execute().await?;
        let hybrid_search_duration = hybrid_search_start.elapsed();
        let total_hybrid_duration = hybrid_embed_duration + hybrid_search_duration;
        
        let hybrid_benchmark = SearchBenchmark {
            mode: SearchMode::Hybrid,
            query: query.to_string(),
            duration: total_hybrid_duration,
            result_count: hybrid_results.len(),
            embedding_time: Some(hybrid_embed_duration),
        };
        
        println!("   🔀 Hybrid Search (BM25 + Vector):");
        println!("      Embedding time: {:?}", hybrid_embed_duration);
        println!("      Search time: {:?}", hybrid_search_duration);
        println!("      Total duration: {:?}", total_hybrid_duration);
        println!("      Results: {}", hybrid_results.len());
        if let Some(first) = hybrid_results.first() {
            println!("      Top result: {}", 
                     first.content.chars().take(50).collect::<String>());
        }
        
        all_benchmarks.extend(vec![text_benchmark, vector_benchmark, hybrid_benchmark]);
    }
    
    // 6. Embedding Provider Comparison
    println!("\n🏁 Embedding Provider Performance Comparison");
    println!("=============================================");
    
    let test_text = "Compare embedding generation performance across providers";
    
    for service in [&openai_service, &cohere_service, &local_service] {
        let start = Instant::now();
        let _embedding = service.embed(test_text).await?;
        let duration = start.elapsed();
        
        println!("   {} ({} dims): {:?}", service.name, service.dimensions, duration);
    }
    
    // 7. Performance Analysis
    println!("\n📊 Performance Analysis");
    println!("=======================");
    
    // Group benchmarks by search mode
    let mut text_benchmarks = Vec::new();
    let mut vector_benchmarks = Vec::new();
    let mut hybrid_benchmarks = Vec::new();
    
    for benchmark in &all_benchmarks {
        match benchmark.mode {
            SearchMode::Text => text_benchmarks.push(benchmark),
            SearchMode::Vector => vector_benchmarks.push(benchmark),
            SearchMode::Hybrid => hybrid_benchmarks.push(benchmark),
        }
    }
    
    let mode_stats = vec![
        (SearchMode::Text, text_benchmarks),
        (SearchMode::Vector, vector_benchmarks),
        (SearchMode::Hybrid, hybrid_benchmarks),
    ];
    
    for (mode, benchmarks) in mode_stats {
        let avg_duration: f64 = benchmarks.iter()
            .map(|b| b.duration.as_millis() as f64)
            .sum::<f64>() / benchmarks.len() as f64;
        
        let avg_results: f64 = benchmarks.iter()
            .map(|b| b.result_count as f64)
            .sum::<f64>() / benchmarks.len() as f64;
        
        println!("\n{:?} Search Mode:", mode);
        println!("   Average Duration: {:.1}ms", avg_duration);
        println!("   Average Results: {:.1}", avg_results);
        
        if let Some(first_with_embedding) = benchmarks.iter().find(|b| b.embedding_time.is_some()) {
            if let Some(embed_time) = first_with_embedding.embedding_time {
                println!("   Embedding Overhead: {:?}", embed_time);
            }
        }
    }
    
    // 8. Use Case Recommendations
    println!("\n💡 Search Mode Recommendations");
    println!("==============================");
    
    println!("📝 Text Search (BM25) - Best for:");
    println!("   ✅ Exact keyword matching");
    println!("   ✅ Fast response requirements (<10ms)");
    println!("   ✅ No embedding infrastructure");
    println!("   ✅ Term-based queries");
    println!("   ✅ Always available fallback");
    
    println!("\n🧠 Vector Search - Best for:");
    println!("   ✅ Semantic similarity");
    println!("   ✅ Conceptual queries");
    println!("   ✅ Cross-language matching");
    println!("   ✅ Handling typos and synonyms");
    println!("   ⚠️  Requires embedding infrastructure");
    
    println!("\n🔀 Hybrid Search - Best for:");
    println!("   ✅ Maximum recall and precision");
    println!("   ✅ Diverse query types");
    println!("   ✅ Professional search applications");
    println!("   ✅ When embedding cost is justified");
    println!("   ⚠️  Highest latency due to embedding + dual search");
    
    // 9. Cost Analysis
    println!("\n💰 Cost Considerations");
    println!("======================");
    
    let queries_per_day = 10000;
    let openai_cost_per_1k_tokens = 0.00002; // $0.00002 per 1K tokens for text-embedding-3-small
    let avg_tokens_per_query = 10;
    
    let daily_embedding_cost = (queries_per_day as f64 / 1000.0) * avg_tokens_per_query as f64 * openai_cost_per_1k_tokens;
    
    println!("Example cost analysis for {} queries/day:", queries_per_day);
    println!("   Text Search: $0.00 (no embedding costs)");
    println!("   Vector Search: ${:.4}/day (OpenAI embeddings)", daily_embedding_cost);
    println!("   Hybrid Search: ${:.4}/day (embedding cost same as vector)", daily_embedding_cost);
    println!("   Local embeddings: $0.00 (compute cost only)");
    
    println!("\n🎉 Search Mode Comparison completed!");
    println!("    Key takeaways:");
    println!("    • Text search is fastest and always available");
    println!("    • Vector search excels at semantic matching");
    println!("    • Hybrid search provides best overall results");
    println!("    • Choose based on your performance/cost/quality tradeoffs");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_embedding_services() {
        let service = MockEmbeddingService::new("Test", 100, 0);
        let embedding = service.embed("test").await.unwrap();
        assert_eq!(embedding.len(), 100);
    }
    
    #[tokio::test]
    async fn test_search_benchmark() {
        let benchmark = SearchBenchmark {
            mode: SearchMode::Text,
            query: "test".to_string(),
            duration: std::time::Duration::from_millis(10),
            result_count: 5,
            embedding_time: None,
        };
        
        assert_eq!(benchmark.mode, SearchMode::Text);
        assert_eq!(benchmark.result_count, 5);
    }
} 