//! Command argument structures
//!
//! This module contains all CLI argument structs organized by command category.

use clap::Args;

// Memory command arguments
#[derive(Args)]
pub struct AddMemoryArgs {
    /// Content of the memory
    pub content: String,

    /// Memory type (fact, conversation, procedural, episodic, identity, world, action, event)
    #[arg(long, short, default_value = "fact")]
    pub memory_type: String,

    /// Priority (low, normal, high, critical)
    #[arg(long, short, default_value = "normal")]
    pub priority: String,

    /// Tags to associate with the memory
    #[arg(long = "tag", short = 't')]
    pub tags: Vec<String>,
}

#[derive(Args)]
pub struct GetMemoryArgs {
    /// Memory ID
    pub id: String,
}

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Maximum number of results
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,

    /// Search mode (text, vector, hybrid, keyword, bm25)
    /// Default: hybrid (automatically combines text and semantic search when available)
    #[arg(long, short, default_value = "hybrid")]
    pub mode: String,

    /// Similarity threshold (0.0 to 1.0)
    #[arg(long)]
    pub threshold: Option<f32>,

    /// Filter by memory type
    #[arg(long)]
    pub memory_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Filter by creation time (ISO 8601)
    #[arg(long)]
    pub created_after: Option<String>,

    /// Filter by creation time (ISO 8601)
    #[arg(long)]
    pub created_before: Option<String>,
}

#[derive(Args)]
pub struct DeleteMemoryArgs {
    /// Memory ID
    pub id: String,
}

#[derive(Args)]
pub struct ListMemoriesArgs {
    /// Maximum number of results
    #[arg(short, long, default_value_t = 20)]
    pub limit: usize,

    /// Filter by memory type
    #[arg(long)]
    pub memory_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Filter by priority
    #[arg(long)]
    pub priority: Option<String>,
}

#[derive(Args)]
pub struct TagMemoryArgs {
    /// Memory ID
    pub id: String,

    /// Tag to add
    pub tag: String,
}

#[derive(Args)]
pub struct CountMemoriesArgs {
    /// Filter by memory type
    #[arg(long)]
    pub memory_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,
}

#[derive(Args)]
pub struct PriorityArgs {
    /// Priority level (low, normal, high, critical)
    pub priority: String,

    /// Maximum number of results
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,
}

#[derive(Args)]
pub struct RecentArgs {
    /// Maximum number of results
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,
}

#[derive(Args)]
pub struct UpdateMemoryArgs {
    /// Memory ID
    pub id: String,

    /// New content (optional)
    #[arg(long)]
    pub content: Option<String>,

    /// New memory type (optional)
    #[arg(long)]
    pub memory_type: Option<String>,

    /// New priority (optional)
    #[arg(long)]
    pub priority: Option<String>,

    /// New tags (replaces existing, optional)
    #[arg(long = "tag", short = 't')]
    pub tags: Option<Vec<String>>,

    /// Properties to update (JSON, optional)
    #[arg(long)]
    pub properties: Option<String>,
}

// Entity command arguments
#[derive(Args)]
pub struct CreateEntityArgs {
    /// Entity ID
    pub id: String,

    /// Entity type
    pub entity_type: String,

    /// Entity properties (JSON format)
    #[arg(long)]
    pub properties: Option<String>,
}

#[derive(Args)]
pub struct GetEntityArgs {
    /// Entity ID
    pub id: String,
}

#[derive(Args)]
pub struct ListEntitiesArgs {
    /// Maximum number of results
    #[arg(short, long, default_value_t = 20)]
    pub limit: usize,

    /// Filter by entity type
    #[arg(long)]
    pub entity_type: Option<String>,
}

#[derive(Args)]
pub struct DeleteEntityArgs {
    /// Entity ID
    pub id: String,
}

#[derive(Args)]
pub struct UpdateEntityArgs {
    /// Entity ID
    pub id: String,

    /// New entity type (optional)
    #[arg(long)]
    pub entity_type: Option<String>,

    /// Properties to update (JSON, optional)
    #[arg(long)]
    pub properties: Option<String>,
}

// Relationship command arguments
#[derive(Args)]
pub struct CreateRelationshipArgs {
    /// Source memory/entity ID
    pub from: String,

    /// Target memory/entity ID
    pub to: String,

    /// Relationship type
    pub relationship_type: String,

    /// Create bidirectional relationship
    #[arg(long)]
    pub bidirectional: bool,

    /// Properties (JSON format)
    #[arg(long)]
    pub properties: Option<String>,
}

#[derive(Args)]
pub struct GetRelationshipArgs {
    /// Relationship ID
    pub id: String,
}

#[derive(Args)]
pub struct ListRelationshipsArgs {
    /// Maximum number of results
    #[arg(short, long, default_value_t = 20)]
    pub limit: usize,

    /// Filter by relationship type
    #[arg(long)]
    pub relationship_type: Option<String>,
}

#[derive(Args)]
pub struct DeleteRelationshipArgs {
    /// Relationship ID
    pub id: String,
}

#[derive(Args)]
pub struct RelatedMemoriesArgs {
    /// Memory ID to find relationships for
    pub id: String,

    /// Relationship type filter
    #[arg(long)]
    pub relationship_type: Option<String>,

    /// Direction (outgoing, incoming, both)
    #[arg(long, default_value = "both")]
    pub direction: String,
}

#[derive(Args)]
pub struct UpdateRelationshipArgs {
    /// Relationship ID
    pub id: String,

    /// New relationship type (optional)
    #[arg(long)]
    pub relationship_type: Option<String>,

    /// Properties to update (JSON, optional)
    #[arg(long)]
    pub properties: Option<String>,
}

// Graph command arguments
#[derive(Args)]
pub struct SubgraphArgs {
    /// Memory ID to center the subgraph on
    pub id: String,

    /// Depth of traversal
    #[arg(long, default_value_t = 2)]
    pub depth: u8,

    /// Include temporal span analysis
    #[arg(long)]
    pub include_temporal_span: bool,
}

#[derive(Args)]
pub struct PathsArgs {
    /// Source memory ID
    pub from: String,

    /// Target memory ID
    pub to: String,

    /// Maximum depth to search
    #[arg(long, default_value_t = 5)]
    pub depth: u8,
}

#[derive(Args)]
pub struct ConnectedArgs {
    /// Memory ID to start from
    pub id: String,

    /// Relationship type to follow (use "all" to traverse all relationship types)
    #[arg(default_value = "all")]
    pub relationship_type: String,

    /// Maximum depth to traverse
    #[arg(long, default_value_t = 3)]
    pub depth: u8,

    /// Exclude temporal relationships (temporal_sequence) from the tree view
    #[arg(long)]
    pub no_temporal: bool,
}

#[derive(Args)]
pub struct GraphQueryArgs {
    /// Query pattern
    pub pattern: String,

    /// Maximum results
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,
}

#[derive(Args)]
pub struct GraphSimilarArgs {
    /// Pattern memory/entity ID
    pub pattern_id: String,

    /// Maximum results
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,
}

#[derive(Args)]
pub struct GraphEntityArgs {
    /// Entity ID
    pub id: String,

    /// Traversal depth
    #[arg(long, default_value_t = 2)]
    pub depth: u8,

    /// Include temporal span analysis
    #[arg(long)]
    pub include_temporal_span: bool,
}

#[derive(Args)]
pub struct MemoryRelationshipsArgs {
    /// Memory ID
    pub id: String,

    /// Create a new relationship (optional)
    #[command(subcommand)]
    pub command: Option<MemoryRelationshipSubcommand>,
}

#[derive(clap::Subcommand)]
pub enum MemoryRelationshipSubcommand {
    /// Create a relationship
    Create(CreateMemoryRelationshipArgs),
}

#[derive(Args)]
pub struct CreateMemoryRelationshipArgs {
    /// Target memory/entity ID
    pub target: String,

    /// Relationship type
    pub relationship_type: String,

    /// Properties (JSON format, optional)
    #[arg(long)]
    pub properties: Option<String>,
}

#[derive(Args)]
pub struct EntityRelationshipsArgs {
    /// Entity ID
    pub id: String,

    /// Create a new relationship (optional)
    #[command(subcommand)]
    pub command: Option<EntityRelationshipSubcommand>,
}

#[derive(clap::Subcommand)]
pub enum EntityRelationshipSubcommand {
    /// Create a relationship
    Create(CreateEntityRelationshipArgs),
}

#[derive(Args)]
pub struct CreateEntityRelationshipArgs {
    /// Target entity ID
    pub target: String,

    /// Relationship type
    pub relationship_type: String,

    /// Properties (JSON format, optional)
    #[arg(long)]
    pub properties: Option<String>,
}

#[derive(Args)]
pub struct EntityMemoriesArgs {
    /// Entity ID
    pub id: String,

    /// Maximum number of results
    #[arg(short, long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Args)]
pub struct CentralEntitiesArgs {
    /// Maximum results
    #[arg(short, long, default_value_t = 10)]
    pub limit: usize,
}

// Batch command arguments
#[derive(Args)]
pub struct ExecuteBatchArgs {
    /// Path to batch operations file (JSON)
    pub file: String,

    /// Execute as transaction (all-or-nothing)
    #[arg(long, short)]
    pub transaction: bool,

    /// Continue on errors (don't stop at first failure)
    #[arg(long)]
    pub continue_on_error: bool,
}

// Relationship type command arguments
#[derive(Args)]
pub struct GetRelationshipTypeArgs {
    /// Relationship type name
    pub name: String,
}

#[derive(Args)]
pub struct RegisterRelationshipTypeArgs {
    /// Relationship type name
    pub name: String,

    /// Inverse type name (optional)
    #[arg(long)]
    pub inverse: Option<String>,

    /// Make symmetric (bidirectional)
    #[arg(long)]
    pub symmetric: bool,

    /// Make transitive
    #[arg(long)]
    pub transitive: bool,

    /// Metadata schema (JSON file path, optional)
    #[arg(long)]
    pub schema: Option<String>,
}

#[derive(Args)]
pub struct UpdateRelationshipTypeArgs {
    /// Relationship type name
    pub name: String,

    /// Inverse type name (optional)
    #[arg(long)]
    pub inverse: Option<String>,

    /// Make symmetric (optional)
    #[arg(long)]
    pub symmetric: Option<bool>,

    /// Make transitive (optional)
    #[arg(long)]
    pub transitive: Option<bool>,

    /// Metadata schema (JSON file path, optional)
    #[arg(long)]
    pub schema: Option<String>,
}

#[derive(Args)]
pub struct DeleteRelationshipTypeArgs {
    /// Relationship type name
    pub name: String,
}

// Tutorial and Quickstart command arguments
#[derive(Args)]
pub struct TutorialArgs {
    /// Tutorial topic (memory, entity, relationship, graph, all)
    #[arg(default_value = "all")]
    pub topic: String,

    /// Skip interactive prompts (show examples only)
    #[arg(long)]
    pub examples_only: bool,
}

#[derive(Args)]
pub struct QuickstartArgs {
    /// Remove sample data created by quickstart
    #[arg(long)]
    pub cleanup: bool,

    /// Show step-by-step guide (1-3)
    #[arg(long)]
    pub step: Option<u8>,
}

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell type
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(clap::ValueEnum, Clone)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    #[clap(name = "powershell")]
    Power,
    Elvish,
}
