//! RAG Fusion with Agentic Verification Example
//!
//! This example demonstrates an advanced RAG (Retrieval-Augmented Generation) system
//! that incorporates fusion techniques and agentic verification inspired by the
//! RewardAgent research (https://arxiv.org/html/2502.19328v1).
//!
//! Key features:
//! - Multi-query generation for diverse retrieval perspectives
//! - Reciprocal Rank Fusion (RRF) for combining retrieval results
//! - Agentic verification system with factuality and instruction-following checks
//! - Reward-based response selection and refinement
//! - Comprehensive evaluation and feedback loops
//!
//! To run this example:
//! ```bash
//! cargo run --example rag_fusion_example
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use locai::config::ConfigBuilder;
use locai::core::MemoryManager;
use locai::memory::search_extensions::SearchMode;
use locai::models::{Memory, MemoryType};
use locai::prelude::*;

// Configuration for the RAG fusion system
#[derive(Debug, Clone)]
pub struct RagFusionConfig {
    pub num_query_variants: usize,
    pub retrieval_limit_per_query: usize,
    pub final_retrieval_limit: usize,
    pub factuality_threshold: f32,
    pub instruction_following_threshold: f32,
    pub enable_verification: bool,
    pub max_refinement_iterations: usize,
}

impl Default for RagFusionConfig {
    fn default() -> Self {
        Self {
            num_query_variants: 3,
            retrieval_limit_per_query: 10,
            final_retrieval_limit: 5,
            factuality_threshold: 0.7,
            instruction_following_threshold: 0.8,
            enable_verification: true,
            max_refinement_iterations: 2,
        }
    }
}

// Represents a retrieved memory with its relevance score
#[derive(Debug, Clone)]
pub struct RetrievedMemory {
    pub memory: Memory,
    pub relevance_score: f32,
    pub source_query: String,
    pub rank: usize,
}

// Verification result for a generated response
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub factuality_score: f32,
    pub instruction_following_score: f32,
    pub overall_score: f32,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}

// Mock LLM interface for demonstration
#[async_trait::async_trait]
pub trait LlmInterface: Send + Sync {
    async fn generate_response(
        &self,
        prompt: &str,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>>;
    async fn generate_queries(
        &self,
        original_query: &str,
        num_variants: usize,
    ) -> std::result::Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;
    async fn verify_factuality(
        &self,
        response: &str,
        context: &[String],
    ) -> std::result::Result<f32, Box<dyn std::error::Error + Send + Sync>>;
    async fn verify_instruction_following(
        &self,
        response: &str,
        instruction: &str,
    ) -> std::result::Result<f32, Box<dyn std::error::Error + Send + Sync>>;
    async fn refine_response(
        &self,
        response: &str,
        issues: &[String],
        context: &[String],
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

// Mock implementation for demonstration
pub struct MockLlm;

#[async_trait::async_trait]
impl LlmInterface for MockLlm {
    async fn generate_response(
        &self,
        prompt: &str,
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate processing time
        sleep(Duration::from_millis(100)).await;

        if prompt.contains("quantum") {
            Ok("Quantum computing leverages quantum mechanical phenomena like superposition and entanglement to process information in ways that classical computers cannot. Quantum bits (qubits) can exist in multiple states simultaneously, allowing quantum computers to explore many possible solutions in parallel.".to_string())
        } else if prompt.contains("climate") {
            Ok("Climate change refers to long-term shifts in global temperatures and weather patterns. While climate variations are natural, scientific evidence shows that human activities, particularly greenhouse gas emissions, have been the dominant driver of climate change since the 1950s.".to_string())
        } else {
            Ok(format!(
                "This is a mock response to the query: {}",
                prompt.chars().take(50).collect::<String>()
            ))
        }
    }

    async fn generate_queries(
        &self,
        original_query: &str,
        num_variants: usize,
    ) -> std::result::Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut variants = vec![original_query.to_string()];

        for i in 1..num_variants {
            let variant = match i {
                1 => format!("What is {}", original_query.to_lowercase()),
                2 => format!("Explain {}", original_query.to_lowercase()),
                3 => format!("How does {} work", original_query.to_lowercase()),
                _ => format!("Tell me about {}", original_query.to_lowercase()),
            };
            variants.push(variant);
        }

        Ok(variants)
    }

    async fn verify_factuality(
        &self,
        response: &str,
        _context: &[String],
    ) -> std::result::Result<f32, Box<dyn std::error::Error + Send + Sync>> {
        // Mock factuality scoring based on response characteristics
        let score =
            if response.contains("scientific evidence") || response.contains("research shows") {
                0.9
            } else if response.contains("quantum") || response.contains("climate") {
                0.8
            } else if response.len() > 100 {
                0.7
            } else {
                0.5
            };

        Ok(score)
    }

    async fn verify_instruction_following(
        &self,
        response: &str,
        instruction: &str,
    ) -> std::result::Result<f32, Box<dyn std::error::Error + Send + Sync>> {
        // Mock instruction following scoring
        let score = if instruction.contains("explain") && response.len() > 50 {
            0.9
        } else if instruction.contains("list") && response.contains("1.") {
            0.8
        } else {
            0.7
        };

        Ok(score)
    }

    async fn refine_response(
        &self,
        response: &str,
        issues: &[String],
        _context: &[String],
    ) -> std::result::Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut refined = response.to_string();

        if issues.iter().any(|issue| issue.contains("factuality")) {
            refined
                .push_str(" [Note: This information has been verified against reliable sources.]");
        }

        if issues.iter().any(|issue| issue.contains("instruction")) {
            refined.push_str(
                " [Note: Response has been adjusted to better follow the given instructions.]",
            );
        }

        Ok(refined)
    }
}

// Main RAG Fusion system
pub struct RagFusionSystem {
    memory_manager: Arc<MemoryManager>,
    llm: Arc<dyn LlmInterface>,
    config: RagFusionConfig,
}

impl RagFusionSystem {
    pub fn new(
        memory_manager: Arc<MemoryManager>,
        llm: Arc<dyn LlmInterface>,
        config: RagFusionConfig,
    ) -> Self {
        Self {
            memory_manager,
            llm,
            config,
        }
    }

    /// Generate multiple query variants for diverse retrieval perspectives
    pub async fn generate_query_variants(&self, original_query: &str) -> Result<Vec<String>> {
        println!(
            "🔄 Generating {} query variants...",
            self.config.num_query_variants
        );

        let variants = self
            .llm
            .generate_queries(original_query, self.config.num_query_variants)
            .await
            .map_err(|e| LocaiError::Memory(format!("Failed to generate query variants: {}", e)))?;

        for (i, variant) in variants.iter().enumerate() {
            println!("  {}. {}", i + 1, variant);
        }

        Ok(variants)
    }

    /// Retrieve memories for each query variant
    pub async fn multi_query_retrieval(&self, queries: &[String]) -> Result<Vec<RetrievedMemory>> {
        println!("🔍 Performing multi-query retrieval...");

        let mut all_results = Vec::new();

        for (_query_idx, query) in queries.iter().enumerate() {
            println!("  Searching for: {}", query);

            let search_results = self
                .memory_manager
                .search(
                    query,
                    Some(self.config.retrieval_limit_per_query),
                    None,
                    SearchMode::Text,
                )
                .await?;

            for (rank, result) in search_results.into_iter().enumerate() {
                all_results.push(RetrievedMemory {
                    memory: result.memory,
                    relevance_score: result.score.unwrap_or(0.0),
                    source_query: query.clone(),
                    rank: rank + 1,
                });
            }
        }

        println!("  Retrieved {} total results", all_results.len());
        Ok(all_results)
    }

    /// Apply Reciprocal Rank Fusion (RRF) to combine and rank results
    pub fn apply_reciprocal_rank_fusion(
        &self,
        results: Vec<RetrievedMemory>,
    ) -> Vec<RetrievedMemory> {
        println!("🔀 Applying Reciprocal Rank Fusion...");

        let k = 60.0; // RRF parameter
        let mut score_map: HashMap<String, (RetrievedMemory, f32)> = HashMap::new();

        for result in results {
            let rrf_score = 1.0 / (k + result.rank as f32);

            match score_map.get_mut(&result.memory.id) {
                Some((_, existing_score)) => {
                    *existing_score += rrf_score;
                }
                None => {
                    score_map.insert(result.memory.id.clone(), (result, rrf_score));
                }
            }
        }

        let mut fused_results: Vec<_> = score_map.into_values().collect();
        fused_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let final_results: Vec<_> = fused_results
            .into_iter()
            .take(self.config.final_retrieval_limit)
            .enumerate()
            .map(|(new_rank, (mut result, fused_score))| {
                result.relevance_score = fused_score;
                result.rank = new_rank + 1;
                result
            })
            .collect();

        println!("  Fused to {} top results", final_results.len());
        for (i, result) in final_results.iter().enumerate() {
            println!(
                "    {}. [Score: {:.3}] {}",
                i + 1,
                result.relevance_score,
                result.memory.content.chars().take(80).collect::<String>()
            );
        }

        final_results
    }

    /// Generate response using retrieved context
    pub async fn generate_response(
        &self,
        query: &str,
        context: &[RetrievedMemory],
    ) -> Result<String> {
        println!("✍️  Generating response...");

        let context_text: Vec<String> = context
            .iter()
            .map(|r| format!("Source: {}\nContent: {}", r.source_query, r.memory.content))
            .collect();

        let prompt = format!(
            "Based on the following context, please answer the question: {}\n\nContext:\n{}\n\nAnswer:",
            query,
            context_text.join("\n\n")
        );

        let response = self
            .llm
            .generate_response(&prompt)
            .await
            .map_err(|e| LocaiError::Memory(format!("Failed to generate response: {}", e)))?;

        println!(
            "  Generated response: {}",
            response.chars().take(100).collect::<String>()
        );
        Ok(response)
    }

    /// Verify response using agentic verification system
    pub async fn verify_response(
        &self,
        response: &str,
        instruction: &str,
        context: &[String],
    ) -> Result<VerificationResult> {
        if !self.config.enable_verification {
            return Ok(VerificationResult {
                factuality_score: 1.0,
                instruction_following_score: 1.0,
                overall_score: 1.0,
                issues: vec![],
                suggestions: vec![],
            });
        }

        println!("🔍 Verifying response with agentic system...");

        // Factuality verification
        let factuality_score = self
            .llm
            .verify_factuality(response, context)
            .await
            .map_err(|e| LocaiError::Memory(format!("Factuality verification failed: {}", e)))?;

        // Instruction following verification
        let instruction_following_score = self
            .llm
            .verify_instruction_following(response, instruction)
            .await
            .map_err(|e| {
                LocaiError::Memory(format!("Instruction following verification failed: {}", e))
            })?;

        // Calculate overall score (weighted average)
        let overall_score = (factuality_score * 0.6) + (instruction_following_score * 0.4);

        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        if factuality_score < self.config.factuality_threshold {
            issues.push("Low factuality score detected".to_string());
            suggestions.push("Verify claims against reliable sources".to_string());
        }

        if instruction_following_score < self.config.instruction_following_threshold {
            issues.push("Poor instruction following detected".to_string());
            suggestions.push("Ensure response directly addresses the instruction".to_string());
        }

        println!(
            "  Factuality: {:.2}, Instruction Following: {:.2}, Overall: {:.2}",
            factuality_score, instruction_following_score, overall_score
        );

        if !issues.is_empty() {
            println!("  Issues found: {:?}", issues);
        }

        Ok(VerificationResult {
            factuality_score,
            instruction_following_score,
            overall_score,
            issues,
            suggestions,
        })
    }

    /// Refine response based on verification results
    pub async fn refine_response(
        &self,
        response: &str,
        verification: &VerificationResult,
        context: &[String],
    ) -> Result<String> {
        if verification.issues.is_empty() {
            return Ok(response.to_string());
        }

        println!("🔧 Refining response based on verification feedback...");

        let refined = self
            .llm
            .refine_response(response, &verification.issues, context)
            .await
            .map_err(|e| LocaiError::Memory(format!("Response refinement failed: {}", e)))?;

        println!(
            "  Refined response: {}",
            refined.chars().take(100).collect::<String>()
        );
        Ok(refined)
    }

    /// Main RAG fusion pipeline with agentic verification
    pub async fn process_query(&self, query: &str) -> Result<String> {
        println!("\n🚀 Starting RAG Fusion pipeline for query: {}", query);

        // Step 1: Generate query variants
        let query_variants = self.generate_query_variants(query).await?;

        // Step 2: Multi-query retrieval
        let raw_results = self.multi_query_retrieval(&query_variants).await?;

        // Step 3: Apply RRF fusion
        let fused_results = self.apply_reciprocal_rank_fusion(raw_results);

        // Step 4: Generate initial response
        let mut response = self.generate_response(query, &fused_results).await?;

        // Step 5: Iterative verification and refinement
        let context_strings: Vec<String> = fused_results
            .iter()
            .map(|r| r.memory.content.clone())
            .collect();

        for iteration in 0..self.config.max_refinement_iterations {
            println!("\n🔄 Verification iteration {}", iteration + 1);

            let verification = self
                .verify_response(&response, query, &context_strings)
                .await?;

            if verification.overall_score >= 0.8 {
                println!(
                    "✅ Response quality acceptable (score: {:.2})",
                    verification.overall_score
                );
                break;
            }

            if iteration < self.config.max_refinement_iterations - 1 {
                response = self
                    .refine_response(&response, &verification, &context_strings)
                    .await?;
            }
        }

        println!("\n✅ RAG Fusion pipeline completed");
        Ok(response)
    }

    /// Store a new memory in the system
    pub async fn store_memory(
        &self,
        content: &str,
        memory_type: MemoryType,
        tags: Vec<String>,
    ) -> Result<String> {
        self.memory_manager
            .add_memory_with_options(content.to_string(), |builder| {
                let mut b = builder.memory_type(memory_type);
                for tag in tags {
                    b = b.tag(tag);
                }
                b
            })
            .await
    }
}

/// Seed the memory system with sample knowledge
async fn seed_knowledge_base(memory_manager: &MemoryManager) -> Result<()> {
    println!("📚 Seeding knowledge base...");

    let knowledge_items = vec![
        (
            "Quantum computing uses quantum mechanical phenomena like superposition and entanglement to process information. Qubits can exist in multiple states simultaneously, enabling parallel computation.",
            MemoryType::Fact,
            vec!["quantum", "computing", "technology"],
        ),
        (
            "Climate change refers to long-term shifts in global temperatures and weather patterns. Human activities, particularly greenhouse gas emissions, are the primary driver since the 1950s.",
            MemoryType::Fact,
            vec!["climate", "environment", "science"],
        ),
        (
            "Machine learning is a subset of artificial intelligence that enables computers to learn and improve from experience without being explicitly programmed for every task.",
            MemoryType::Fact,
            vec!["machine learning", "AI", "technology"],
        ),
        (
            "Photosynthesis is the process by which plants convert light energy into chemical energy, producing glucose and oxygen from carbon dioxide and water.",
            MemoryType::Fact,
            vec!["biology", "plants", "science"],
        ),
        (
            "The Internet of Things (IoT) refers to the network of physical devices embedded with sensors, software, and connectivity to exchange data with other devices and systems.",
            MemoryType::Fact,
            vec!["IoT", "technology", "networking"],
        ),
        (
            "Blockchain is a distributed ledger technology that maintains a continuously growing list of records, called blocks, which are linked and secured using cryptography.",
            MemoryType::Fact,
            vec!["blockchain", "cryptocurrency", "technology"],
        ),
        (
            "Renewable energy sources include solar, wind, hydroelectric, and geothermal power. These sources are naturally replenished and produce minimal environmental impact.",
            MemoryType::Fact,
            vec!["renewable energy", "environment", "sustainability"],
        ),
        (
            "CRISPR-Cas9 is a revolutionary gene-editing technology that allows scientists to make precise changes to DNA sequences in living cells.",
            MemoryType::Fact,
            vec!["CRISPR", "genetics", "biotechnology"],
        ),
    ];

    for (content, memory_type, tags) in knowledge_items {
        let id = memory_manager
            .add_memory_with_options(content.to_string(), |builder| {
                let mut b = builder.memory_type(memory_type);
                for tag in tags {
                    b = b.tag(tag);
                }
                b
            })
            .await?;

        println!(
            "  Added: {} (ID: {})",
            content.chars().take(50).collect::<String>(),
            id
        );
    }

    println!("✅ Knowledge base seeded successfully");
    Ok(())
}

/// Demonstrate the RAG fusion system
async fn demonstrate_rag_fusion() -> Result<()> {
    println!("🎯 RAG Fusion with Agentic Verification Demo");
    println!("============================================");

    // Initialize Locai
    let config = ConfigBuilder::new()
        .with_default_storage()
        .with_default_ml()
        .with_data_dir("./data/rag_fusion_demo")
        .build()?;

    let memory_manager = Arc::new(init(config).await?);

    // Seed knowledge base
    seed_knowledge_base(&memory_manager).await?;

    // Initialize RAG fusion system
    let llm = Arc::new(MockLlm);
    let rag_config = RagFusionConfig {
        num_query_variants: 3,
        retrieval_limit_per_query: 5,
        final_retrieval_limit: 3,
        factuality_threshold: 0.7,
        instruction_following_threshold: 0.8,
        enable_verification: true,
        max_refinement_iterations: 2,
    };

    let rag_system = RagFusionSystem::new(memory_manager.clone(), llm, rag_config);

    // Test queries
    let test_queries = vec![
        "How does quantum computing work?",
        "What is climate change and what causes it?",
        "Explain machine learning in simple terms",
        "What are renewable energy sources?",
    ];

    for query in test_queries {
        println!("\n{}", "=".repeat(80));
        let response = rag_system.process_query(query).await?;
        println!("\n📝 Final Response:");
        println!("{}", response);
        println!("\n{}", "=".repeat(80));

        // Add a small delay between queries for readability
        sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    if let Err(e) = demonstrate_rag_fusion().await {
        eprintln!("❌ Demo failed: {}", e);
        std::process::exit(1);
    }

    println!("\n🎉 RAG Fusion demo completed successfully!");
    Ok(())
}
