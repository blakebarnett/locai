//! Schema initialization and management for SharedStorage

use crate::storage::errors::StorageError;
use surrealdb::{Connection, Surreal};

/// Initialize the SharedStorage schema with tables and relationships for Locai
pub async fn initialize_schema<C>(client: &Surreal<C>) -> Result<(), StorageError>
where
    C: Connection,
{
    // Define custom search analyzers for different content types
    let analyzers_query = r#"
        -- General content analyzer for memories and entities
        DEFINE ANALYZER memory_analyzer 
            TOKENIZERS class, blank, punct 
            FILTERS lowercase, ascii, snowball(english)
            COMMENT "Analyzer for memory content with stemming and normalization";
        
        -- Entity-focused analyzer with less aggressive stemming
        DEFINE ANALYZER entity_analyzer
            TOKENIZERS class, blank
            FILTERS lowercase, ascii
            COMMENT "Analyzer for entity names and properties";
        
        -- Fuzzy search analyzer for typo tolerance
        DEFINE ANALYZER fuzzy_analyzer
            TOKENIZERS class, blank, punct
            FILTERS lowercase, ascii
            COMMENT "Basic analyzer for fuzzy matching operations";
    "#;

    // Create the user table for authentication
    let user_table_query = r#"
        DEFINE TABLE user SCHEMAFULL
        COMMENT "Stores user accounts for authentication";
        
        DEFINE FIELD id ON user TYPE record<user>;
        DEFINE FIELD username ON user TYPE string ASSERT $value != NONE;
        DEFINE FIELD password_hash ON user TYPE string ASSERT $value != NONE;
        DEFINE FIELD email ON user TYPE option<string>;
        DEFINE FIELD role ON user TYPE string DEFAULT "viewer";
        DEFINE FIELD created_at ON user TYPE datetime DEFAULT time::now();
        DEFINE FIELD updated_at ON user TYPE datetime DEFAULT time::now() VALUE time::now();
        
        DEFINE INDEX user_username_idx ON user FIELDS username UNIQUE;
        DEFINE INDEX user_email_idx ON user FIELDS email;
        DEFINE INDEX user_role_idx ON user FIELDS role;
    "#;

    // Create the memory table with owner field and full-text search capabilities
    let memory_table_query = r#"
        DEFINE TABLE memory SCHEMALESS
        COMMENT "Stores memory records for AI agents";
        
        DEFINE FIELD id ON memory TYPE record<memory>;
        DEFINE FIELD content ON memory TYPE string;
        DEFINE FIELD metadata ON memory TYPE object DEFAULT {};
        DEFINE FIELD embedding ON memory TYPE option<array<float>>;
        DEFINE FIELD importance ON memory TYPE option<float>;
        DEFINE FIELD owner ON memory TYPE record<user>;
        DEFINE FIELD shared_with ON memory TYPE option<set<record<user>>> DEFAULT NONE;
        DEFINE FIELD created_at ON memory TYPE datetime DEFAULT time::now();
        DEFINE FIELD updated_at ON memory TYPE datetime VALUE time::now();
        
        DEFINE INDEX memory_created_at_idx ON memory FIELDS created_at;
        DEFINE INDEX memory_importance_idx ON memory FIELDS importance;
        DEFINE INDEX memory_owner_idx ON memory FIELDS owner;
        DEFINE INDEX memory_shared_idx ON memory FIELDS shared_with;
        DEFINE INDEX memory_type_idx ON memory FIELDS metadata.memory_type;
        DEFINE INDEX memory_priority_idx ON memory FIELDS metadata.priority;
        
        -- Full-text search indexes for memory content with BM25 scoring and highlighting
        DEFINE INDEX memory_content_ft ON memory 
            FIELDS content 
            SEARCH ANALYZER memory_analyzer BM25 HIGHLIGHTS
            COMMENT "Full-text search on memory content with BM25 scoring";
        
        -- Full-text search for memory metadata fields
        DEFINE INDEX memory_metadata_ft ON memory 
            FIELDS metadata.tags, metadata.source, metadata.summary
            SEARCH ANALYZER memory_analyzer
            COMMENT "Full-text search on memory metadata fields";
    "#;

    // Create the vector table for embeddings storage
    let vector_table_query = r#"
        DEFINE TABLE vector SCHEMALESS
        COMMENT "Stores vector embeddings for semantic search";
        
        DEFINE FIELD id ON vector TYPE record<vector>;
        DEFINE FIELD vector ON vector TYPE array<float>;
        DEFINE FIELD dimension ON vector TYPE int;
        DEFINE FIELD metadata ON vector TYPE object DEFAULT {};
        DEFINE FIELD source_id ON vector TYPE option<string>;
        DEFINE FIELD created_at ON vector TYPE datetime DEFAULT time::now();
        
        DEFINE INDEX vector_source_idx ON vector FIELDS source_id;
        DEFINE INDEX vector_created_at_idx ON vector FIELDS created_at;
        DEFINE INDEX vector_dimension_idx ON vector FIELDS dimension;
        
        -- Full-text search on vector metadata for hybrid search
        DEFINE INDEX vector_metadata_ft ON vector
            FIELDS metadata
            SEARCH ANALYZER memory_analyzer
            COMMENT "Full-text search on vector metadata";
    "#;

    // Create vector indexes for efficient similarity search
    // We'll create only a 1024-dimensional index for BGE-M3 model compatibility
    let vector_index_queries = [
        // HNSW index for 1024-dimensional vectors (BGE-M3 model)
        r#"DEFINE INDEX vector_hnsw_1024_idx ON vector FIELDS vector HNSW DIMENSION 1024 DIST COSINE EFC 150 M 12;"#,
    ];

    // Create the entity table with owner field and full-text search
    let entity_table_query = r#"
        DEFINE TABLE entity SCHEMALESS
        COMMENT "Stores entities extracted from memories";
        
        DEFINE FIELD id ON entity TYPE record<entity>;
        DEFINE FIELD entity_type ON entity TYPE string;
        DEFINE FIELD properties ON entity TYPE object DEFAULT {};
        DEFINE FIELD owner ON entity TYPE record<user>;
        DEFINE FIELD shared_with ON entity TYPE option<set<record<user>>> DEFAULT NONE;
        DEFINE FIELD created_at ON entity TYPE datetime DEFAULT time::now();
        DEFINE FIELD updated_at ON entity TYPE datetime VALUE time::now();
        
        DEFINE INDEX entity_type_idx ON entity FIELDS entity_type;
        DEFINE INDEX entity_created_at_idx ON entity FIELDS created_at;
        DEFINE INDEX entity_owner_idx ON entity FIELDS owner;
        DEFINE INDEX entity_shared_idx ON entity FIELDS shared_with;
        
        -- Full-text search indexes for entity properties with BM25 scoring
        DEFINE INDEX entity_properties_ft ON entity
            FIELDS properties.name, properties.description, properties.text, properties.value
            SEARCH ANALYZER entity_analyzer BM25 HIGHLIGHTS
            COMMENT "Full-text search on entity properties with BM25 scoring";
        
        -- Full-text search on entity type for categorization
        DEFINE INDEX entity_type_ft ON entity
            FIELDS entity_type
            SEARCH ANALYZER entity_analyzer
            COMMENT "Full-text search on entity types";
    "#;

    // Create the relationship table with owner field and full-text search
    let relationship_table_query = r#"
        DEFINE TABLE relationship SCHEMALESS
        COMMENT "Stores relationships between entities";
        
        DEFINE FIELD id ON relationship TYPE record<relationship>;
        DEFINE FIELD relationship_type ON relationship TYPE string;
        DEFINE FIELD source_id ON relationship TYPE string;
        DEFINE FIELD target_id ON relationship TYPE string;
        DEFINE FIELD properties ON relationship TYPE object DEFAULT {};
        DEFINE FIELD owner ON relationship TYPE record<user>;
        DEFINE FIELD shared_with ON relationship TYPE option<set<record<user>>> DEFAULT NONE;
        DEFINE FIELD created_at ON relationship TYPE datetime DEFAULT time::now();
        DEFINE FIELD updated_at ON relationship TYPE datetime VALUE time::now();
        
        DEFINE INDEX relationship_type_idx ON relationship FIELDS relationship_type;
        DEFINE INDEX relationship_source_idx ON relationship FIELDS source_id;
        DEFINE INDEX relationship_target_idx ON relationship FIELDS target_id;
        DEFINE INDEX relationship_source_target_idx ON relationship FIELDS source_id, target_id;
        DEFINE INDEX relationship_owner_idx ON relationship FIELDS owner;
        DEFINE INDEX relationship_shared_idx ON relationship FIELDS shared_with;
        
        -- Full-text search on relationship properties
        DEFINE INDEX relationship_properties_ft ON relationship
            FIELDS properties.description, properties.context, properties.notes
            SEARCH ANALYZER memory_analyzer
            COMMENT "Full-text search on relationship properties";
        
        -- Full-text search on relationship types
        DEFINE INDEX relationship_type_ft ON relationship
            FIELDS relationship_type
            SEARCH ANALYZER entity_analyzer
            COMMENT "Full-text search on relationship types";
    "#;

    // Create the version table with owner field
    let version_table_query = r#"
        DEFINE TABLE version SCHEMALESS
        COMMENT "Stores version snapshots of the knowledge graph";
        
        DEFINE FIELD id ON version TYPE record<version>;
        DEFINE FIELD description ON version TYPE string;
        DEFINE FIELD metadata ON version TYPE object DEFAULT {};
        DEFINE FIELD created_at ON version TYPE datetime DEFAULT time::now();
        DEFINE FIELD snapshot_type ON version TYPE string DEFAULT "full";
        DEFINE FIELD snapshot_data ON version TYPE object DEFAULT {};
        
        DEFINE INDEX version_created_at_idx ON version FIELDS created_at;
        DEFINE INDEX version_description_idx ON version FIELDS description;
        
        -- Full-text search on version descriptions
        DEFINE INDEX version_description_ft ON version
            FIELDS description
            SEARCH ANALYZER memory_analyzer
            COMMENT "Full-text search on version descriptions";
    "#;

    // Create edge tables for graph relationships
    let memory_entity_edge_query = r#"
        DEFINE TABLE contains SCHEMAFULL TYPE RELATION
        COMMENT "Relationship between memories and entities they contain";
        
        DEFINE FIELD in ON contains TYPE record<memory>;
        DEFINE FIELD out ON contains TYPE record<entity>;
        DEFINE FIELD confidence ON contains TYPE option<float>;
        DEFINE FIELD created_at ON contains TYPE datetime VALUE time::now();
    "#;

    let entity_relationship_edge_query = r#"
        DEFINE TABLE relates SCHEMALESS TYPE RELATION
        COMMENT "Directed relationships between entities";
        
        DEFINE FIELD in ON relates TYPE record<entity>;
        DEFINE FIELD out ON relates TYPE record<entity>;
        DEFINE FIELD relationship_type ON relates TYPE string;
        DEFINE FIELD properties ON relates TYPE object;
        DEFINE FIELD confidence ON relates TYPE option<float>;
        DEFINE FIELD created_at ON relates TYPE datetime VALUE time::now();
        
        DEFINE INDEX relates_type_idx ON relates FIELDS relationship_type;
        
        -- Full-text search on relationship properties in edges
        DEFINE INDEX relates_properties_ft ON relates
            FIELDS properties.description, properties.context
            SEARCH ANALYZER memory_analyzer
            COMMENT "Full-text search on relationship edge properties";
    "#;

    let memory_relationship_edge_query = r#"
        DEFINE TABLE references SCHEMAFULL TYPE RELATION
        COMMENT "References from memories to relationships";
        
        DEFINE FIELD in ON references TYPE record<memory>;
        DEFINE FIELD out ON references TYPE record<relationship>;
        DEFINE FIELD context ON references TYPE option<string>;
        DEFINE FIELD created_at ON references TYPE datetime VALUE time::now();
        
        -- Full-text search on reference context
        DEFINE INDEX references_context_ft ON references
            FIELDS context
            SEARCH ANALYZER memory_analyzer
            COMMENT "Full-text search on reference context";
    "#;

    // Execute schema creation queries
    execute_schema_query(client, analyzers_query, "search analyzers").await?;
    execute_schema_query(client, user_table_query, "user table").await?;
    execute_schema_query(client, memory_table_query, "memory table").await?;
    execute_schema_query(client, vector_table_query, "vector table").await?;

    // Create vector indexes (ignore errors for now since dimension might not match)
    for (i, index_query) in vector_index_queries.iter().enumerate() {
        match execute_schema_query(client, index_query, &format!("vector index {}", i)).await {
            Ok(_) => tracing::debug!("Created vector index {} successfully", i),
            Err(e) => tracing::warn!("Failed to create vector index {}: {:?}", i, e),
        }
    }

    execute_schema_query(client, entity_table_query, "entity table").await?;
    execute_schema_query(client, relationship_table_query, "relationship table").await?;
    execute_schema_query(client, version_table_query, "version table").await?;
    execute_schema_query(client, memory_entity_edge_query, "memory-entity edge").await?;
    execute_schema_query(client, entity_relationship_edge_query, "entity-entity edge").await?;
    execute_schema_query(
        client,
        memory_relationship_edge_query,
        "memory-relationship edge",
    )
    .await?;

    tracing::info!(
        "SharedStorage schema with full-text search capabilities initialized successfully"
    );
    Ok(())
}

/// Execute a schema query and handle errors
async fn execute_schema_query<C>(
    client: &Surreal<C>,
    query: &str,
    description: &str,
) -> Result<(), StorageError>
where
    C: Connection,
{
    client
        .query(query)
        .await
        .map_err(|e| StorageError::Query(format!("Failed to create {}: {}", description, e)))?;

    tracing::debug!("Created {} successfully", description);
    Ok(())
}

/// Drop all Locai tables (useful for testing)
pub async fn drop_schema<C>(client: &Surreal<C>) -> Result<(), StorageError>
where
    C: Connection,
{
    let drop_queries = vec![
        "REMOVE TABLE IF EXISTS references;",
        "REMOVE TABLE IF EXISTS relates;",
        "REMOVE TABLE IF EXISTS contains;",
        "REMOVE TABLE IF EXISTS version;",
        "REMOVE TABLE IF EXISTS relationship;",
        "REMOVE TABLE IF EXISTS entity;",
        "REMOVE TABLE IF EXISTS vector;",
        "REMOVE TABLE IF EXISTS memory;",
    ];

    for query in drop_queries {
        client
            .query(query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to drop tables: {}", e)))?;
    }

    tracing::info!("SharedStorage schema dropped successfully");
    Ok(())
}

/// Check if schema is initialized
pub async fn is_schema_initialized<C>(client: &Surreal<C>) -> Result<bool, StorageError>
where
    C: Connection,
{
    // Check if required tables exist by trying to query them
    let check_query = "SELECT VALUE count() FROM memory LIMIT 1;";
    let check_result = client.query(check_query).await;
    Ok(check_result.is_ok())
}
