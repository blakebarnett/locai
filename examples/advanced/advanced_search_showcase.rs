//! Advanced Search Intelligence Showcase
//!
//! This example showcases Locai's advanced search intelligence capabilities
//! in realistic AI assistant scenarios. It demonstrates:
//!
//! 1. **Query Understanding**: Intent detection and strategy selection
//! 2. **Multi-Modal Search**: BM25, fuzzy, hybrid search with intelligent routing
//! 3. **Context Awareness**: Session-based conversational search
//! 4. **Search Suggestions**: Auto-completion and query refinement
//! 5. **Result Explanation**: Detailed match reasoning and provenance
//! 6. **Typo Tolerance**: Fuzzy matching for real-world user input
//! 7. **Performance**: Real-time search across large knowledge bases

use locai::storage::{
    shared_storage::{SharedStorage, SharedStorageConfig},
    traits::{MemoryStore, BaseStore},
};
use locai::models::{Memory, MemoryType, MemoryPriority};
use locai::storage::shared_storage::intelligence::{
    IntelligentSearch, SearchStrategy, QueryIntent, SuggestionType,
};
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Advanced Search Intelligence Showcase");
    println!("========================================");
    println!("Demonstrating AI Assistant Search Capabilities");
    println!();

    // Initialize Locai with search intelligence
    let storage = setup_knowledge_base().await?;
    
    // Scenario 1: AI Assistant Query Understanding
    println!("📖 Scenario 1: AI Assistant Query Understanding");
    println!("----------------------------------------------");
    demonstrate_query_understanding(&storage).await?;
    
    // Scenario 2: Conversational Search Context
    println!("\n💬 Scenario 2: Conversational Search Context");
    println!("---------------------------------------------");
    demonstrate_conversational_search(&storage).await?;
    
    // Scenario 3: Typo-Tolerant User Input
    println!("\n🔤 Scenario 3: Typo-Tolerant User Input");
    println!("---------------------------------------");
    demonstrate_typo_tolerance(&storage).await?;
    
    // Scenario 4: Intelligent Search Suggestions
    println!("\n💡 Scenario 4: Intelligent Search Suggestions");
    println!("----------------------------------------------");
    demonstrate_search_suggestions(&storage).await?;
    
    // Scenario 5: Multi-Strategy Search Fusion
    println!("\n🎯 Scenario 5: Multi-Strategy Search Fusion");
    println!("-------------------------------------------");
    demonstrate_search_fusion(&storage).await?;
    
    // Scenario 6: Knowledge Discovery
    println!("\n🔬 Scenario 6: Knowledge Discovery");
    println!("----------------------------------");
    demonstrate_knowledge_discovery(&storage).await?;

    println!("\n🎉 Advanced Search Intelligence Showcase Complete!");
    println!("\nKey Capabilities Demonstrated:");
    println!("  ✅ Natural language query understanding");
    println!("  ✅ Context-aware conversational search");
    println!("  ✅ Typo tolerance and fuzzy matching");
    println!("  ✅ Intelligent auto-completion");
    println!("  ✅ Multi-strategy result fusion");
    println!("  ✅ Detailed result explanations");
    println!("  ✅ Real-time knowledge discovery");

    Ok(())
}

/// Setup a comprehensive knowledge base for demonstration
async fn setup_knowledge_base() -> Result<SharedStorage<surrealdb::engine::local::Mem>, Box<dyn std::error::Error>> {
    let config = SharedStorageConfig {
        namespace: "showcase".to_string(),
        database: "advanced_search".to_string(),
    };

    let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(()).await?;
    let storage = SharedStorage::new(client, config).await?;
    
    println!("🧠 Setting up comprehensive knowledge base...");
    
    // Clear any existing data
    storage.clear().await?;
    
    // Create a rich knowledge base covering multiple domains
    let knowledge_memories = vec![
        // AI and Machine Learning
        Memory {
            id: "ai_overview".to_string(),
            content: "Artificial Intelligence (AI) is the simulation of human intelligence in machines designed to think and act like humans. It encompasses machine learning, deep learning, neural networks, and natural language processing.".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::High,
            tags: vec!["artificial_intelligence".to_string(), "overview".to_string(), "technology".to_string()],
            source: "ai_encyclopedia".to_string(),
            expires_at: None,
            properties: json!({
                "domain": "computer_science",
                "topic": "artificial_intelligence",
                "difficulty": "beginner",
                "keywords": ["AI", "machine learning", "neural networks", "NLP"]
            }),
            related_memories: vec![],
            embedding: None,
        },
        Memory {
            id: "ml_algorithms".to_string(),
            content: "Machine learning algorithms enable computers to automatically learn and improve from experience without being explicitly programmed. Key types include supervised learning (classification, regression), unsupervised learning (clustering, dimensionality reduction), and reinforcement learning.".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::High,
            tags: vec!["machine_learning".to_string(), "algorithms".to_string(), "supervised".to_string(), "unsupervised".to_string()],
            source: "ml_textbook".to_string(),
            expires_at: None,
            properties: json!({
                "domain": "machine_learning",
                "complexity": "intermediate",
                "applications": ["prediction", "classification", "clustering"]
            }),
            related_memories: vec![],
            embedding: None,
        },
        Memory {
            id: "neural_networks_guide".to_string(),
            content: "How to design and train neural networks: 1) Define the problem and collect data 2) Choose appropriate architecture (feedforward, CNN, RNN) 3) Initialize weights randomly 4) Forward propagation 5) Calculate loss using appropriate loss function 6) Backward propagation to compute gradients 7) Update weights using optimization algorithm 8) Repeat training cycles until convergence 9) Evaluate on test data 10) Fine-tune hyperparameters".to_string(),
            memory_type: MemoryType::Procedure,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::High,
            tags: vec!["neural_networks".to_string(), "training".to_string(), "tutorial".to_string(), "deep_learning".to_string()],
            source: "deep_learning_course".to_string(),
            expires_at: None,
            properties: json!({
                "type": "step_by_step",
                "difficulty": "advanced",
                "duration": "varies",
                "tools": ["TensorFlow", "PyTorch", "Keras"]
            }),
            related_memories: vec![],
            embedding: None,
        },
        // Natural Language Processing
        Memory {
            id: "nlp_overview".to_string(),
            content: "Natural Language Processing (NLP) bridges the gap between human language and computer understanding. It includes text analysis, sentiment analysis, machine translation, question answering, and text generation using techniques like tokenization, part-of-speech tagging, named entity recognition, and transformer models.".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::High,
            tags: vec!["nlp".to_string(), "natural_language".to_string(), "text_processing".to_string(), "transformers".to_string()],
            source: "nlp_handbook".to_string(),
            expires_at: None,
            properties: json!({
                "domain": "natural_language_processing",
                "applications": ["chatbots", "translation", "sentiment_analysis", "summarization"],
                "models": ["BERT", "GPT", "T5", "RoBERTa"]
            }),
            related_memories: vec![],
            embedding: None,
        },
        Memory {
            id: "transformer_architecture".to_string(),
            content: "The Transformer architecture revolutionized natural language processing through the attention mechanism. Unlike RNNs, Transformers process sequences in parallel using self-attention to weigh the importance of different words. Key components include multi-head attention, positional encoding, feed-forward networks, and layer normalization.".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::High,
            tags: vec!["transformers".to_string(), "attention".to_string(), "architecture".to_string(), "bert".to_string(), "gpt".to_string()],
            source: "attention_is_all_you_need_paper".to_string(),
            expires_at: None,
            properties: json!({
                "year": 2017,
                "innovation": "self_attention",
                "impact": "revolutionary",
                "use_cases": ["translation", "text_generation", "question_answering"]
            }),
            related_memories: vec![],
            embedding: None,
        },
        // Quantum Computing
        Memory {
            id: "quantum_computing_intro".to_string(),
            content: "Quantum computing harnesses quantum mechanical phenomena like superposition and entanglement to process information in ways impossible for classical computers. Quantum bits (qubits) can exist in multiple states simultaneously, enabling exponential speedup for certain algorithms like Shor's algorithm for factoring and Grover's algorithm for search.".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::Medium,
            tags: vec!["quantum_computing".to_string(), "qubits".to_string(), "superposition".to_string(), "entanglement".to_string()],
            source: "quantum_physics_journal".to_string(),
            expires_at: None,
            properties: json!({
                "domain": "quantum_physics",
                "complexity": "advanced",
                "applications": ["cryptography", "optimization", "simulation"],
                "companies": ["IBM", "Google", "Rigetti"]
            }),
            related_memories: vec![],
            embedding: None,
        },
        // Computer Vision
        Memory {
            id: "computer_vision_applications".to_string(),
            content: "Computer vision enables machines to interpret and understand visual information from images and videos. Applications include object detection, facial recognition, medical image analysis, autonomous vehicles, augmented reality, and quality control in manufacturing using CNN architectures like ResNet, YOLO, and Vision Transformers.".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::Medium,
            tags: vec!["computer_vision".to_string(), "image_processing".to_string(), "cnn".to_string(), "object_detection".to_string()],
            source: "computer_vision_review".to_string(),
            expires_at: None,
            properties: json!({
                "domain": "computer_vision",
                "techniques": ["CNN", "object_detection", "segmentation", "tracking"],
                "frameworks": ["OpenCV", "TensorFlow", "PyTorch"]
            }),
            related_memories: vec![],
            embedding: None,
        },
        // Programming and Software Development
        Memory {
            id: "python_for_ai".to_string(),
            content: "Python has become the dominant language for AI and machine learning due to its simplicity and rich ecosystem. Key libraries include NumPy for numerical computing, Pandas for data manipulation, Matplotlib for visualization, Scikit-learn for traditional ML, TensorFlow and PyTorch for deep learning, and Jupyter notebooks for interactive development.".to_string(),
            memory_type: MemoryType::Fact,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            priority: MemoryPriority::Medium,
            tags: vec!["python".to_string(), "programming".to_string(), "libraries".to_string(), "data_science".to_string()],
            source: "python_ai_guide".to_string(),
            expires_at: None,
            properties: json!({
                "language": "python",
                "libraries": ["numpy", "pandas", "tensorflow", "pytorch", "scikit-learn"],
                "use_cases": ["machine_learning", "data_analysis", "research"]
            }),
            related_memories: vec![],
            embedding: None,
        },
    ];

    // Create all memories in the knowledge base
    for memory in knowledge_memories {
        storage.create_memory(memory).await?;
    }

    // Wait for indexing to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    println!("✅ Knowledge base ready with {} memories", 8);
    Ok(storage)
}

/// Demonstrate intelligent query understanding and intent detection
async fn demonstrate_query_understanding(storage: &SharedStorage<surrealdb::engine::local::Mem>) -> Result<(), Box<dyn std::error::Error>> {
    println!("👤 User: \"I need to understand how neural networks work\"");
    
    let query = "how do neural networks work";
    let analysis = storage.analyze_query(query).await?;
    
    println!("🧠 AI Analysis:");
    println!("   Intent: {:?} (Procedural knowledge request)", analysis.intent);
    println!("   Strategy: {:?} (Will use step-by-step guidance)", analysis.strategy);
    println!("   Confidence: {:.1}%", analysis.confidence * 100.0);
    println!("   Detected tokens: {:?}", analysis.tokens);
    
    let results = storage.intelligent_search(query, None, Some(2)).await?;
    println!("\n🎯 Search Results:");
    for (i, result) in results.iter().enumerate() {
        println!("   {}. Score: {:.3} | {}", i + 1, result.score, result.explanation.primary_reason);
        if let Some(content) = result.content.get("content").and_then(|c| c.as_str()) {
            let preview = if content.len() > 100 { 
                format!("{}...", &content[..100]) 
            } else { 
                content.to_string() 
            };
            println!("      Preview: {}", preview);
        }
    }
    
    println!("\n👤 User: \"What's the relationship between AI and machine learning?\"");
    
    let relational_query = "relationship between AI and machine learning";
    let rel_analysis = storage.analyze_query(relational_query).await?;
    
    println!("🧠 AI Analysis:");
    println!("   Intent: {:?} (Seeking connections)", rel_analysis.intent);
    println!("   Strategy: {:?} (Will explore relationships)", rel_analysis.strategy);
    
    Ok(())
}

/// Demonstrate conversational search with context building
async fn demonstrate_conversational_search(storage: &SharedStorage<surrealdb::engine::local::Mem>) -> Result<(), Box<dyn std::error::Error>> {
    println!("👤 User: \"Tell me about machine learning\"");
    
    let results1 = storage.intelligent_search("machine learning", None, Some(1)).await?;
    if let Some(result) = results1.first() {
        if let Some(content) = result.content.get("content").and_then(|c| c.as_str()) {
            println!("🤖 AI: {}", &content[..200.min(content.len())]);
            if content.len() > 200 {
                println!("       ...");
            }
        }
    }
    
    println!("\n👤 User: \"How is that different from deep learning?\"");
    
    // Simulate contextual follow-up (in a real system, this would maintain conversation state)
    let context_query = "machine learning vs deep learning differences";
    let results2 = storage.intelligent_search(context_query, None, Some(1)).await?;
    
    println!("🧠 AI Context Analysis:");
    println!("   Previous topic: Machine Learning");
    println!("   Current query: Seeking comparison with deep learning");
    println!("   Search strategy: Finding discriminating features");
    
    if let Some(result) = results2.first() {
        println!("🤖 AI: Deep learning is a subset of machine learning that uses neural networks...");
        println!("       Match confidence: {:.1}%", result.score * 100.0);
    }
    
    println!("\n👤 User: \"Can you give me a practical example?\"");
    
    let example_query = "deep learning practical applications examples";
    let analysis = storage.analyze_query(example_query).await?;
    
    println!("🧠 AI Analysis:");
    println!("   Context awareness: Building on previous deep learning discussion");
    println!("   Intent: {:?} (Seeking concrete examples)", analysis.intent);
    println!("   Will search for: Applications and use cases");
    
    Ok(())
}

/// Demonstrate typo tolerance and fuzzy matching
async fn demonstrate_typo_tolerance(storage: &SharedStorage<surrealdb::engine::local::Mem>) -> Result<(), Box<dyn std::error::Error>> {
    let typo_queries = vec![
        ("machien lerning", "machine learning"),
        ("neurral netowrks", "neural networks"),
        ("quantm computng", "quantum computing"),
        ("artficial inteligence", "artificial intelligence"),
    ];
    
    for (typo_query, intended_query) in typo_queries {
        println!("👤 User types: \"{}\" (meant: \"{}\")", typo_query, intended_query);
        
        // Try fuzzy search for typo tolerance
        let fuzzy_results = storage.fuzzy_search_memories(typo_query, Some(0.3), Some(2)).await?;
        
        if !fuzzy_results.is_empty() {
            println!("🔍 Fuzzy Search Found:");
            for (memory, score) in &fuzzy_results {
                println!("   Similarity: {:.1}% | {}", score * 100.0, 
                        memory.content.chars().take(80).collect::<String>());
            }
            
            // Suggest correction
            let suggestions = storage.suggest(typo_query, None).await?;
            if !suggestions.is_empty() {
                println!("💭 AI Suggestion: Did you mean \"{}\"?", suggestions[0].suggestion);
            }
        } else {
            println!("🤖 AI: I couldn't find exact matches, but let me try some alternatives...");
            
            // Fallback to intelligent search which might handle the typos better
            let intelligent_results = storage.intelligent_search(typo_query, None, Some(1)).await?;
            if !intelligent_results.is_empty() {
                println!("   Found using intelligent search: {}", 
                        intelligent_results[0].explanation.primary_reason);
            }
        }
        println!();
    }
    
    Ok(())
}

/// Demonstrate intelligent search suggestions and auto-completion
async fn demonstrate_search_suggestions(storage: &SharedStorage<surrealdb::engine::local::Mem>) -> Result<(), Box<dyn std::error::Error>> {
    let partial_queries = vec!["mach", "neur", "trans", "quant"];
    
    for partial in partial_queries {
        println!("👤 User typing: \"{}\"", partial);
        
        let suggestions = storage.suggest(partial, None).await?;
        
        if !suggestions.is_empty() {
            println!("💡 Auto-complete suggestions:");
            for (i, suggestion) in suggestions.iter().take(3).enumerate() {
                println!("   {}. {} ({})", i + 1, suggestion.suggestion, 
                        match suggestion.suggestion_type {
                            SuggestionType::Completion => "auto-complete",
                            SuggestionType::Expansion => "topic expansion", 
                            SuggestionType::Correction => "spelling correction",
                            SuggestionType::Alternative => "alternative",
                            SuggestionType::Refinement => "refinement",
                        });
            }
        } else {
            println!("💭 No specific suggestions yet, keep typing...");
        }
        println!();
    }
    
    // Demonstrate query expansion suggestions
    println!("👤 User: \"learning\" (broad topic)");
    let broad_suggestions = storage.suggest("learning", None).await?;
    
    if !broad_suggestions.is_empty() {
        println!("🎯 Topic refinement suggestions:");
        for suggestion in broad_suggestions.iter().take(3) {
            println!("   • {}", suggestion.suggestion);
            println!("     Reason: {}", suggestion.explanation);
        }
    }
    
    Ok(())
}

/// Demonstrate multi-strategy search fusion
async fn demonstrate_search_fusion(storage: &SharedStorage<surrealdb::engine::local::Mem>) -> Result<(), Box<dyn std::error::Error>> {
    let query = "python artificial intelligence";
    
    println!("👤 User: \"{}\"", query);
    println!("🔍 Comparing different search strategies:");
    
    // BM25 Full-text search
    let bm25_results = storage.bm25_search_memories(query, Some(2)).await?;
    println!("\n📊 BM25 Full-text Search:");
    for (memory, score, highlight) in &bm25_results {
        println!("   Score: {:.3} | Tags: {:?}", score, memory.tags);
        if !highlight.is_empty() && highlight != memory.content {
            println!("   Highlight: {}", highlight.chars().take(100).collect::<String>());
        }
    }
    
    // Intelligent search (combines multiple strategies)
    let intelligent_results = storage.intelligent_search(query, None, Some(2)).await?;
    println!("\n🧠 Intelligent Search (Multi-strategy):");
    for result in &intelligent_results {
        println!("   Combined Score: {:.3} | Method: {}", result.score, result.explanation.primary_reason);
        
        // Show score breakdown
        let breakdown = &result.score_breakdown;
        if let Some(bm25) = breakdown.bm25_score {
            println!("      BM25: {:.3}", bm25);
        }
        if let Some(vector) = breakdown.vector_score {
            println!("      Vector: {:.3}", vector);
        }
        if let Some(graph) = breakdown.graph_score {
            println!("      Graph: {:.3}", graph);
        }
        
        println!("      Explanation: {:?}", result.explanation.details);
    }
    
    // Show why intelligent search might be better
    println!("\n🎯 Why Intelligent Search Excels:");
    println!("   • Combines multiple relevance signals");
    println!("   • Adapts strategy based on query type");
    println!("   • Provides detailed match explanations");
    println!("   • Normalizes scores across different methods");
    
    Ok(())
}

/// Demonstrate knowledge discovery and exploration
async fn demonstrate_knowledge_discovery(storage: &SharedStorage<surrealdb::engine::local::Mem>) -> Result<(), Box<dyn std::error::Error>> {
    println!("👤 User: \"I'm new to AI, help me explore\"");
    
    // Exploratory search
    let exploration_query = "artificial intelligence introduction overview";
    let analysis = storage.analyze_query(exploration_query).await?;
    
    println!("🧠 AI Analysis:");
    println!("   Intent: {:?} (Knowledge exploration)", analysis.intent);
    println!("   Strategy: {:?} (Broad conceptual search)", analysis.strategy);
    
    let results = storage.intelligent_search(exploration_query, None, Some(3)).await?;
    
    println!("\n📚 Knowledge Discovery Results:");
    let mut topics_found = HashMap::new();
    
    for (i, result) in results.iter().enumerate() {
        println!("   {}. {}", i + 1, result.explanation.primary_reason);
        
        // Extract topics from result metadata
        if let Some(content) = result.content.as_object() {
            for (key, value) in content {
                if key == "topic" || key == "domain" {
                    if let Some(topic) = value.as_str() {
                        *topics_found.entry(topic.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    
    if !topics_found.is_empty() {
        println!("\n🗺️  Related Topics to Explore:");
        for (topic, count) in topics_found {
            println!("   • {} (mentioned {} times)", topic.replace("_", " "), count);
        }
    }
    
    println!("\n🎓 Learning Path Suggestions:");
    println!("   1. Start with AI overview and basic concepts");
    println!("   2. Explore machine learning fundamentals");
    println!("   3. Dive into neural networks and deep learning");
    println!("   4. Specialize in areas like NLP or computer vision");
    
    // Demonstrate progressive search refinement
    println!("\n👤 User: \"Tell me more about the neural networks part\"");
    
    let refined_query = "neural networks deep learning training";
    let refined_results = storage.intelligent_search(refined_query, None, Some(2)).await?;
    
    println!("🎯 Refined Search (Building on Previous Context):");
    for result in &refined_results {
        println!("   Match: {} (confidence: {:.1}%)", 
                result.explanation.primary_reason, result.score * 100.0);
    }
    
    Ok(())
}