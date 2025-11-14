//! Simplified Locai API
//!
//! This module provides the simplified, user-friendly interface to Locai that makes
//! 90% of use cases require only 1-2 lines of code.

use crate::Result;
use crate::config::{ConfigBuilder, LogLevel};
use crate::core::memory_manager::MemoryManager;
use crate::memory::search_extensions::SearchMode;
use crate::models::memory::{Memory, MemoryBuilder, MemoryPriority, MemoryType};
use crate::storage::filters::SemanticSearchFilter;
use crate::storage::filters::helpers;
use std::path::Path;

/// Simplified Locai interface for easy memory management
///
/// This struct provides a simplified API that makes common operations
/// straightforward while still allowing access to advanced features when needed.
///
/// # Examples
///
/// ```rust
/// use locai::prelude::Locai;
///
/// async fn example() -> locai::Result<()> {
///     // Dead simple - everything auto-configured
///     let locai = Locai::new().await?;
///
///     // Store memories with simple API
///     locai.remember("The sky is blue").await?;
///     locai.remember_fact("Water boils at 100°C").await?;
///     
///     // Search memories
///     let results = locai.search("sky").await?;
///     
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Locai {
    manager: MemoryManager,
}

impl Locai {
    /// Create a new Locai instance with sensible defaults
    ///
    /// This initializes Locai with:
    /// - Persistent storage in the user's data directory
    /// - Default ML configuration with local embeddings
    /// - Standard logging configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     locai.remember("Hello, world!").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new() -> Result<Self> {
        let config = ConfigBuilder::defaults().build()?;
        let manager = crate::init(config).await?;
        Ok(Self { manager })
    }

    /// Create a Locai instance with a custom data directory
    ///
    /// # Arguments
    /// * `data_dir` - Path where Locai should store its data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::with_data_dir("./my_locai_data").await?;
    ///     locai.remember("Stored in custom location").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn with_data_dir(data_dir: impl AsRef<Path>) -> Result<Self> {
        let config = ConfigBuilder::new()
            .with_data_dir(data_dir)
            .with_default_storage()
            .with_default_ml()
            .with_default_logging()
            .build()?;
        let manager = crate::init(config).await?;
        Ok(Self { manager })
    }

    /// Create a Locai instance optimized for testing
    ///
    /// Uses in-memory storage for fast, isolated tests.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// #[tokio::test]
    /// async fn test_example() -> locai::Result<()> {
    ///     let locai = Locai::for_testing().await?;
    ///     locai.remember("Test memory").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn for_testing() -> Result<Self> {
        let config = ConfigBuilder::testing().build()?;
        let manager = crate::init(config).await?;
        Ok(Self { manager })
    }

    /// Create a Locai instance optimized for parallel testing
    ///
    /// Creates a completely isolated instance with unique database identifiers,
    /// allowing multiple tests to run in parallel without interference.
    /// Each call creates a separate in-memory database instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// #[tokio::test]
    /// async fn test_parallel_safe() -> locai::Result<()> {
    ///     let locai = Locai::for_testing_isolated().await?;
    ///     locai.remember("Test memory").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn for_testing_isolated() -> Result<Self> {
        use crate::storage::config::{SurrealDBConfig, SurrealDBEngine};
        use std::sync::atomic::{AtomicU64, Ordering};

        // Global counter to ensure unique database identifiers across all test instances
        static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let unique_namespace = format!("test_ns_{}", test_id);
        let unique_database = format!("test_db_{}", test_id);

        let mut config = ConfigBuilder::new()
            .with_data_dir(format!("./test_data_{}", test_id))
            .with_model_cache_dir(format!("./test_cache_{}", test_id))
            .with_default_ml()
            .with_log_level(LogLevel::Debug)
            .build()?;

        // Explicitly configure for true in-memory isolation
        config.storage.graph.surrealdb = SurrealDBConfig {
            engine: SurrealDBEngine::Memory,
            connection: "()".to_string(), // Placeholder - not used for Memory engine
            namespace: unique_namespace,
            database: unique_database,
            auth: None,
            settings: None,
        };

        let manager = crate::init(config).await?;
        Ok(Self { manager })
    }

    /// Create a Locai instance for testing with a custom test identifier
    ///
    /// Useful when you need predictable test identifiers or want to group
    /// related tests with a common prefix.
    ///
    /// # Arguments
    /// * `test_id` - A unique identifier for this test instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// #[tokio::test]
    /// async fn test_with_custom_id() -> locai::Result<()> {
    ///     let locai = Locai::for_testing_with_id("my_test_suite_001").await?;
    ///     locai.remember("Test memory").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn for_testing_with_id(test_id: &str) -> Result<Self> {
        use crate::storage::config::{SurrealDBConfig, SurrealDBEngine};

        let unique_namespace = format!("test_ns_{}", test_id);
        let unique_database = format!("test_db_{}", test_id);

        let mut config = ConfigBuilder::new()
            .with_data_dir(format!("./test_data_{}", test_id))
            .with_model_cache_dir(format!("./test_cache_{}", test_id))
            .with_default_ml()
            .with_log_level(LogLevel::Debug)
            .build()?;

        // Explicitly configure for true in-memory isolation
        config.storage.graph.surrealdb = SurrealDBConfig {
            engine: SurrealDBEngine::Memory,
            connection: "()".to_string(), // Placeholder - not used for Memory engine
            namespace: unique_namespace,
            database: unique_database,
            auth: None,
            settings: None,
        };

        let manager = crate::init(config).await?;
        Ok(Self { manager })
    }

    /// Create an advanced builder for custom configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::builder()
    ///         .with_data_dir("./custom_data")
    ///         .with_embedding_model("custom-model")
    ///         .build().await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn builder() -> LocaiBuilder {
        LocaiBuilder::new()
    }

    /// Remember something (stores as episodic memory by default)
    ///
    /// This is the simplest way to store a memory. The content will be
    /// automatically embedded for semantic search if ML is available.
    ///
    /// # Arguments
    /// * `content` - The content to remember
    ///
    /// # Returns
    /// The ID of the stored memory
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let memory_id = locai.remember("I learned something important today").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn remember(&self, content: impl Into<String>) -> Result<String> {
        let memory = MemoryBuilder::episodic(content).build();
        self.manager.store_memory(memory).await
    }

    /// Remember a fact (stores as fact memory)
    ///
    /// Facts are objective, verifiable pieces of information.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     locai.remember_fact("The capital of France is Paris").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn remember_fact(&self, content: impl Into<String>) -> Result<String> {
        self.manager.add_fact(content).await
    }

    /// Remember a conversation (stores as conversation memory)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     locai.remember_conversation("User: Hello\nBot: Hi there!").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn remember_conversation(&self, content: impl Into<String>) -> Result<String> {
        self.manager.add_conversation(content).await
    }

    /// Start building a memory with advanced options
    ///
    /// This provides access to the full memory builder API while maintaining
    /// a fluent interface.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::{Locai, MemoryPriority};
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     locai.remember_with("Important scientific discovery")
    ///         .as_fact()
    ///         .with_priority(MemoryPriority::High)
    ///         .with_tags(&["science", "breakthrough"])
    ///         .save().await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn remember_with(&self, content: impl Into<String>) -> RememberBuilder<'_> {
        RememberBuilder::new(&self.manager, content.into())
    }

    /// Universal search - searches everything intelligently
    ///
    /// This automatically searches across memories, entities, and graphs using the best
    /// available search strategy (semantic if available, otherwise keyword search).
    ///
    /// # Arguments
    /// * `query` - What to search for
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let results = locai.search("what did I learn about physics?").await?;
    ///     for result in results {
    ///         println!("Found: {}", result.summary());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn search(&self, query: &str) -> Result<Vec<crate::core::SearchResult>> {
        self.search_with_options(query, crate::core::SearchOptions::default())
            .await
    }

    /// Search with customization options
    ///
    /// This method provides advanced search capabilities with customizable options
    /// including search strategy, result filtering, and more.
    ///
    /// # Arguments
    /// * `query` - What to search for
    /// * `options` - Search options controlling behavior
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::*;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let options = SearchOptions {
    ///         limit: 5,
    ///         strategy: SearchStrategy::Semantic,
    ///         include_types: SearchTypeFilter::memories_only(),
    ///         ..Default::default()
    ///     };
    ///     let results = locai.search_with_options("physics", options).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_with_options(
        &self,
        query: &str,
        options: crate::core::SearchOptions,
    ) -> Result<Vec<crate::core::SearchResult>> {
        use crate::memory::search_extensions::{SearchMode, UniversalSearchOptions};
        use crate::storage::filters::SemanticSearchFilter;

        // Convert SearchOptions to UniversalSearchOptions
        let universal_options = UniversalSearchOptions {
            include_memories: options.include_types.memories,
            include_entities: options.include_types.entities,
            include_graphs: options.include_types.graphs,
            graph_depth: options.graph_depth,
            memory_type_filter: None, // TODO: Add memory type filtering to SearchOptions
            entity_type_filter: None, // TODO: Add entity type filtering to SearchOptions
            similarity_threshold: options.min_score,
            expand_with_relations: options.include_context,
        };

        // Handle different search strategies
        let results = match options.strategy {
            crate::core::SearchStrategy::Auto => {
                // Use universal search which automatically determines the best approach
                self.manager
                    .universal_search(query, Some(options.limit), Some(universal_options))
                    .await?
            }
            crate::core::SearchStrategy::Semantic => {
                // Force vector search mode (for backward compatibility)
                if options.include_types.memories {
                    let filter = SemanticSearchFilter {
                        similarity_threshold: options.min_score,
                        memory_filter: None,
                    };
                    let search_results = self
                        .manager
                        .search(query, Some(options.limit), Some(filter), SearchMode::Vector)
                        .await?;
                    search_results
                        .into_iter()
                        .map(
                            |sr| crate::memory::search_extensions::UniversalSearchResult::Memory {
                                memory: sr.memory,
                                score: sr.score,
                                match_reason: "vector search".to_string(),
                            },
                        )
                        .collect()
                } else {
                    // If not including memories, fall back to universal search
                    self.manager
                        .universal_search(query, Some(options.limit), Some(universal_options))
                        .await?
                }
            }
            crate::core::SearchStrategy::Keyword => {
                // Force text search mode
                if options.include_types.memories {
                    let filter = SemanticSearchFilter {
                        similarity_threshold: options.min_score,
                        memory_filter: None,
                    };
                    let search_results = self
                        .manager
                        .search(query, Some(options.limit), Some(filter), SearchMode::Text)
                        .await?;
                    search_results
                        .into_iter()
                        .map(
                            |sr| crate::memory::search_extensions::UniversalSearchResult::Memory {
                                memory: sr.memory,
                                score: sr.score,
                                match_reason: "keyword search".to_string(),
                            },
                        )
                        .collect()
                } else {
                    // If not including memories, fall back to universal search
                    self.manager
                        .universal_search(query, Some(options.limit), Some(universal_options))
                        .await?
                }
            }
            crate::core::SearchStrategy::Graph => {
                // Force graph-centric search
                let mut graph_options = universal_options.clone();
                graph_options.include_graphs = true;
                graph_options.graph_depth = options.graph_depth.max(2); // Ensure meaningful graph depth
                self.manager
                    .universal_search(query, Some(options.limit), Some(graph_options))
                    .await?
            }
            crate::core::SearchStrategy::Hybrid => {
                // Use all search methods and merge results
                let mut hybrid_options = universal_options.clone();
                hybrid_options.include_memories = true;
                hybrid_options.include_entities = true;
                hybrid_options.include_graphs = options.include_types.graphs;
                self.manager
                    .universal_search(query, Some(options.limit), Some(hybrid_options))
                    .await?
            }
        };

        // Convert UniversalSearchResult to SearchResult
        Ok(results
            .into_iter()
            .map(crate::core::SearchResult::from_universal)
            .collect())
    }

    /// Search only memories (legacy compatibility)
    ///
    /// This method is deprecated. Use `search()` for universal search or
    /// `search_with_options()` with `SearchTypeFilter::memories_only()` for memory-only search.
    ///
    /// # Arguments
    /// * `query` - What to search for
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let results = locai.search_memories("what did I learn about physics?").await?;
    ///     for result in results {
    ///         println!("Found: {}", result.content);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[deprecated(
        note = "Use search() for universal search or search_with_options() with SearchTypeFilter::memories_only()"
    )]
    pub async fn search_memories(&self, query: &str) -> Result<Vec<Memory>> {
        // Use enhanced search that doesn't restrict to fact-only memories
        self.manager.enhanced_search(query, Some(10)).await
    }

    /// Universal search across all data types (legacy compatibility)
    ///
    /// This method is deprecated. Use `search()` which now provides universal search by default.
    ///
    /// # Arguments
    /// * `query` - What to search for
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let results = locai.search("john").await?; // Use this instead
    ///     Ok(())
    /// }
    /// ```
    #[deprecated(note = "Use search() which now provides universal search by default")]
    pub async fn universal_search(&self, query: &str) -> Result<Vec<crate::core::SearchResult>> {
        self.search(query).await
    }

    /// Start building an advanced search
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::{Locai, MemoryType};
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let results = locai.search_for("physics")
    ///         .limit(5)
    ///         .of_type(MemoryType::Fact)
    ///         .with_tags(&["science"])
    ///         .execute().await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn search_for(&self, query: impl Into<String>) -> SearchBuilder<'_> {
        SearchBuilder::new(&self.manager, query.into())
    }

    /// Get recent memories
    ///
    /// # Arguments
    /// * `limit` - Maximum number of memories to return (default: 10)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let recent = locai.recent_memories(Some(5)).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn recent_memories(&self, limit: Option<usize>) -> Result<Vec<Memory>> {
        self.manager.get_recent_memories(limit.unwrap_or(10)).await
    }

    /// Check if vector search is available (BYOE approach)
    ///
    /// In the BYOE approach, vector search is available when memories have embeddings.
    /// Users need to provide their own embeddings via the Memory.with_embedding() method.
    pub fn has_semantic_search(&self) -> bool {
        // For BYOE approach, vector search depends on memories having embeddings,
        // not on having an ML service. Return false to encourage users to use
        // the new SearchMode::Vector explicitly.
        false
    }

    /// Get the underlying MemoryManager for advanced operations
    ///
    /// This provides access to the full MemoryManager API for advanced use cases
    /// that aren't covered by the simplified interface.
    pub fn manager(&self) -> &MemoryManager {
        &self.manager
    }

    /// Clear all stored data
    ///
    /// ⚠️ **Warning**: This permanently deletes all memories!
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::for_testing().await?;
    ///     // ... do some testing ...
    ///     locai.clear_all().await?; // Clean up after test
    ///     Ok(())
    /// }
    /// ```
    pub async fn clear_all(&self) -> Result<()> {
        self.manager.clear_storage().await
    }

    /// Create a new version of an existing memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory to version
    /// * `content` - The new content for this version
    /// * `metadata` - Optional metadata for this version
    ///
    /// # Returns
    /// The version ID of the created version
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let memory_id = locai.remember("Initial content").await?;
    ///     let version_id = locai.remember_version(&memory_id, "Updated content", None).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn remember_version(
        &self,
        memory_id: &str,
        content: &str,
        metadata: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<String> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        // Try to downcast to SharedStorage
        // The storage is wrapped in Arc<dyn GraphStore>, so we need to downcast the Arc's content
        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::create_memory_version(shared_storage, memory_id, content, metadata)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::create_memory_version(
                    shared_storage,
                    memory_id,
                    content,
                    metadata,
                )
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Get a specific version of a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    /// * `version_id` - The ID of the version to retrieve
    ///
    /// # Returns
    /// The memory at the specified version, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let memory_id = locai.remember("Content").await?;
    ///     let version_id = locai.remember_version(&memory_id, "Updated", None).await?;
    ///     let old_version = locai.get_memory_version(&memory_id, &version_id).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_memory_version(
        &self,
        memory_id: &str,
        version_id: &str,
    ) -> Result<Option<Memory>> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::get_memory_version(shared_storage, memory_id, version_id)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::get_memory_version(
                    shared_storage,
                    memory_id,
                    version_id,
                )
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// List all versions of a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    ///
    /// # Returns
    /// A list of version information, ordered by creation time
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let memory_id = locai.remember("Content").await?;
    ///     locai.remember_version(&memory_id, "Version 2", None).await?;
    ///     let versions = locai.list_memory_versions(&memory_id).await?;
    ///     println!("Found {} versions", versions.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn list_memory_versions(
        &self,
        memory_id: &str,
    ) -> Result<Vec<crate::storage::models::MemoryVersionInfo>> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::list_memory_versions(shared_storage, memory_id)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::list_memory_versions(shared_storage, memory_id)
                    .await
                    .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Get the current (latest) version of a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    ///
    /// # Returns
    /// The current version of the memory, or None if not found
    pub async fn get_memory_current_version(&self, memory_id: &str) -> Result<Option<Memory>> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::get_memory_current_version(shared_storage, memory_id)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::get_memory_current_version(shared_storage, memory_id)
                    .await
                    .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Compute diff between two versions
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    /// * `old_version_id` - The ID of the old version
    /// * `new_version_id` - The ID of the new version
    ///
    /// # Returns
    /// A diff structure showing the changes
    pub async fn diff_memory_versions(
        &self,
        memory_id: &str,
        old_version_id: &str,
        new_version_id: &str,
    ) -> Result<crate::storage::models::MemoryDiff> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::diff_memory_versions(
                shared_storage,
                memory_id,
                old_version_id,
                new_version_id,
            )
            .await
            .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::diff_memory_versions(
                    shared_storage,
                    memory_id,
                    old_version_id,
                    new_version_id,
                )
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Get memory as it existed at a specific time
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    /// * `at_time` - The timestamp to query
    ///
    /// # Returns
    /// The memory as it existed at that time, or None if not found
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    /// use chrono::Utc;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let memory_id = locai.remember("Content").await?;
    ///     let yesterday = Utc::now() - chrono::Duration::days(1);
    ///     let old_memory = locai.get_memory_at_time(&memory_id, yesterday).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_memory_at_time(
        &self,
        memory_id: &str,
        at_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<Memory>> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::get_memory_at_time(shared_storage, memory_id, at_time)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::get_memory_at_time(shared_storage, memory_id, at_time)
                    .await
                    .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Create a snapshot of memory state
    ///
    /// # Arguments
    /// * `memory_ids` - Optional list of memory IDs to include (None = all memories)
    /// * `metadata` - Optional metadata for the snapshot
    ///
    /// # Returns
    /// The created snapshot
    pub async fn create_snapshot(
        &self,
        memory_ids: Option<&[String]>,
        metadata: Option<&std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<crate::storage::models::MemorySnapshot> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::create_snapshot(shared_storage, memory_ids, metadata)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::create_snapshot(shared_storage, memory_ids, metadata)
                    .await
                    .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Restore from snapshot
    ///
    /// # Arguments
    /// * `snapshot` - The snapshot to restore
    /// * `restore_mode` - How to handle existing memories
    pub async fn restore_snapshot(
        &self,
        snapshot: &crate::storage::models::MemorySnapshot,
        restore_mode: crate::storage::models::RestoreMode,
    ) -> Result<()> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::restore_snapshot(shared_storage, snapshot, restore_mode)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::restore_snapshot(
                    shared_storage,
                    snapshot,
                    restore_mode,
                )
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Search memories in a snapshot state
    ///
    /// # Arguments
    /// * `snapshot` - The snapshot to search
    /// * `query` - The search query
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// A list of memories matching the query as they existed in the snapshot
    pub async fn search_snapshot(
        &self,
        snapshot: &crate::storage::models::MemorySnapshot,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::search_snapshot(shared_storage, snapshot, query, limit)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::search_snapshot(shared_storage, snapshot, query, limit)
                    .await
                    .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Get versioning statistics
    ///
    /// # Arguments
    /// * `memory_id` - Optional memory ID to get stats for (None = all memories)
    ///
    /// # Returns
    /// Versioning statistics including total versions, delta versions, storage info, etc.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     let memory_id = locai.remember("Content").await?;
    ///     let stats = locai.get_versioning_stats(Some(&memory_id)).await?;
    ///     println!("Total versions: {}", stats.total_versions);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_versioning_stats(
        &self,
        memory_id: Option<&str>,
    ) -> Result<crate::storage::models::VersioningStats> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::get_versioning_stats(shared_storage, memory_id)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::get_versioning_stats(shared_storage, memory_id)
                    .await
                    .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Compact versions by removing old versions
    ///
    /// # Arguments
    /// * `memory_id` - Optional memory ID to compact (None = all memories)
    /// * `keep_count` - Number of most recent versions to keep
    /// * `older_than_days` - Remove versions older than this many days
    ///
    /// # Returns
    /// Number of versions removed
    pub async fn compact_versions(
        &self,
        memory_id: Option<&str>,
        keep_count: Option<usize>,
        older_than_days: Option<u64>,
    ) -> Result<usize> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::compact_versions(
                shared_storage,
                memory_id,
                keep_count,
                older_than_days,
            )
            .await
            .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::compact_versions(
                    shared_storage,
                    memory_id,
                    keep_count,
                    older_than_days,
                )
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Promote a delta version to full copy
    ///
    /// # Arguments
    /// * `memory_id` - The memory ID
    /// * `version_id` - The version ID to promote
    pub async fn promote_version_to_full_copy(
        &self,
        memory_id: &str,
        version_id: &str,
    ) -> Result<()> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::promote_version_to_full_copy(shared_storage, memory_id, version_id)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::promote_version_to_full_copy(
                    shared_storage,
                    memory_id,
                    version_id,
                )
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Validate version integrity
    ///
    /// # Arguments
    /// * `memory_id` - Optional memory ID to validate (None = all memories)
    ///
    /// # Returns
    /// List of integrity issues found
    pub async fn validate_versions(
        &self,
        memory_id: Option<&str>,
    ) -> Result<Vec<crate::storage::models::VersionIntegrityIssue>> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::validate_versions(shared_storage, memory_id)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::validate_versions(shared_storage, memory_id)
                    .await
                    .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }

    /// Repair corrupted versions
    ///
    /// # Arguments
    /// * `memory_id` - Optional memory ID to repair (None = all memories)
    ///
    /// # Returns
    /// Repair report with details of repairs performed
    pub async fn repair_versions(
        &self,
        memory_id: Option<&str>,
    ) -> Result<crate::storage::models::RepairReport> {
        use crate::storage::shared_storage::SharedStorage;
        use crate::storage::traits::MemoryVersionStore;

        let storage = self.manager.storage();
        let storage_any = storage.as_any();

        if let Some(shared_storage) =
            storage_any.downcast_ref::<SharedStorage<surrealdb::engine::local::Db>>()
        {
            MemoryVersionStore::repair_versions(shared_storage, memory_id)
                .await
                .map_err(|e| crate::LocaiError::Storage(e.to_string()))
        } else {
            #[cfg(feature = "surrealdb-remote")]
            if let Some(shared_storage) =
                storage_any.downcast_ref::<SharedStorage<surrealdb::engine::remote::ws::Client>>()
            {
                return MemoryVersionStore::repair_versions(shared_storage, memory_id)
                    .await
                    .map_err(|e| crate::LocaiError::Storage(e.to_string()));
            }
            Err(crate::LocaiError::Storage(
                "Memory versioning is only supported with SharedStorage".to_string(),
            ))
        }
    }
}

/// Builder for advanced Locai configuration
pub struct LocaiBuilder {
    config_builder: ConfigBuilder,
}

impl LocaiBuilder {
    fn new() -> Self {
        Self {
            config_builder: ConfigBuilder::new(),
        }
    }

    /// Set the data directory
    pub fn with_data_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.config_builder = self.config_builder.with_data_dir(path);
        self
    }

    /// Set the embedding model
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.with_embedding_model(model);
        self
    }

    /// Use in-memory storage (good for testing)
    pub fn with_memory_storage(mut self) -> Self {
        self.config_builder = self.config_builder.with_memory_storage();
        self
    }

    /// Use default production settings
    pub fn with_defaults(mut self) -> Self {
        self.config_builder = self
            .config_builder
            .with_default_storage()
            .with_default_ml()
            .with_default_logging();
        self
    }

    /// Build the Locai instance
    pub async fn build(self) -> Result<Locai> {
        let config = self
            .config_builder
            .with_default_storage()
            .with_default_ml()
            .build()?;
        let manager = crate::init(config).await?;
        Ok(Locai { manager })
    }
}

/// Builder for advanced memory creation
pub struct RememberBuilder<'a> {
    manager: &'a MemoryManager,
    content: String,
    memory_type: Option<MemoryType>,
    priority: Option<MemoryPriority>,
    tags: Vec<String>,
}

impl<'a> RememberBuilder<'a> {
    fn new(manager: &'a MemoryManager, content: String) -> Self {
        Self {
            manager,
            content,
            memory_type: None,
            priority: None,
            tags: Vec::new(),
        }
    }

    /// Set the memory type to Fact
    pub fn as_fact(mut self) -> Self {
        self.memory_type = Some(MemoryType::Fact);
        self
    }

    /// Set the memory type to Episodic
    pub fn as_episodic(mut self) -> Self {
        self.memory_type = Some(MemoryType::Episodic);
        self
    }

    /// Set the memory type to Conversation
    pub fn as_conversation(mut self) -> Self {
        self.memory_type = Some(MemoryType::Conversation);
        self
    }

    /// Set the memory type to World knowledge
    pub fn as_world(mut self) -> Self {
        self.memory_type = Some(MemoryType::World);
        self
    }

    /// Set the priority level
    pub fn with_priority(mut self, priority: MemoryPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Add tags to the memory
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags.extend(tags.iter().map(|s| s.to_string()));
        self
    }

    /// Add a single tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Save the memory
    pub async fn save(&self) -> Result<String> {
        if !self.tags.is_empty() {
            // Use add_memory_with_options for complex cases
            self.manager
                .add_memory_with_options(&self.content, |builder| {
                    let mut builder = builder
                        .memory_type(self.memory_type.clone().unwrap_or(MemoryType::Episodic))
                        .priority(self.priority.unwrap_or(MemoryPriority::Normal));

                    let tag_refs: Vec<&str> = self.tags.iter().map(|s| s.as_str()).collect();
                    builder = builder.tags(tag_refs);
                    builder
                })
                .await
        } else {
            // Use simple add_memory for basic cases
            self.manager
                .add_memory(
                    &self.content,
                    self.memory_type.clone().unwrap_or(MemoryType::Episodic),
                )
                .await
        }
    }
}

/// Builder for advanced search operations
pub struct SearchBuilder<'a> {
    manager: &'a MemoryManager,
    query: String,
    limit: Option<usize>,
    memory_type: Option<MemoryType>,
    tags: Option<Vec<String>>,
    since: Option<chrono::DateTime<chrono::Utc>>,
    mode: SearchMode,
    query_embedding: Option<Vec<f32>>,
}

impl<'a> SearchBuilder<'a> {
    fn new(manager: &'a MemoryManager, query: String) -> Self {
        Self {
            manager,
            query,
            limit: None,
            memory_type: None,
            tags: None,
            since: None,
            mode: SearchMode::Text,
            query_embedding: None,
        }
    }

    /// Limit the number of results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Filter by memory type
    pub fn of_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    /// Filter by tags
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags = Some(tags.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Filter by creation date (memories since this date)
    pub fn since(mut self, date: chrono::DateTime<chrono::Utc>) -> Self {
        self.since = Some(date);
        self
    }

    /// Set the search mode
    pub fn mode(mut self, mode: SearchMode) -> Self {
        self.mode = mode;
        self
    }

    /// Provide a query embedding for vector or hybrid search (BYOE approach)
    ///
    /// When using SearchMode::Vector or SearchMode::Hybrid, you must provide
    /// a query embedding generated by your embedding provider (OpenAI, Cohere, etc.).
    ///
    /// # Arguments
    /// * `embedding` - The query embedding vector from your provider
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::prelude::Locai;
    /// use locai::memory::SearchMode;
    ///
    /// async fn example() -> locai::Result<()> {
    ///     let locai = Locai::new().await?;
    ///     
    ///     // With embeddings from your provider
    ///     // This example shows the concept - you would use your actual embedding provider
    ///     let query_embedding = vec![0.1, 0.2, 0.3]; // Mock embedding from OpenAI client
    ///     let results = locai.search_for("search query")
    ///         .mode(SearchMode::Vector)
    ///         .with_query_embedding(query_embedding)
    ///         .execute().await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn with_query_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.query_embedding = Some(embedding);
        self
    }

    /// Execute the search
    pub async fn execute(&self) -> Result<Vec<Memory>> {
        let query = self.query.clone();
        let limit = self.limit.unwrap_or(10);

        // Create filter - use memory_type if set, otherwise no type filter
        let mut filter = if let Some(memory_type) = &self.memory_type {
            helpers::memory_by_type(&memory_type.to_string())
        } else {
            crate::storage::filters::MemoryFilter::default()
        };
        if let Some(tags) = &self.tags {
            filter.tags = Some(tags.clone());
        }
        if let Some(since) = &self.since {
            filter.created_after = Some(*since);
        }

        // For vector and hybrid search, pass the query embedding if provided
        let results = match self.mode {
            SearchMode::Vector | SearchMode::Hybrid => {
                self.manager
                    .search_with_embedding(
                        &query,
                        self.query_embedding.as_deref(),
                        Some(limit),
                        Some(SemanticSearchFilter {
                            memory_filter: Some(filter),
                            similarity_threshold: None,
                        }),
                        self.mode,
                    )
                    .await?
            }
            SearchMode::Text => {
                self.manager
                    .search(
                        &query,
                        Some(limit),
                        Some(SemanticSearchFilter {
                            memory_filter: Some(filter),
                            similarity_threshold: None,
                        }),
                        self.mode,
                    )
                    .await?
            }
        };

        Ok(results.into_iter().map(|r| r.memory).collect())
    }
}
