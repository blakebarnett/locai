//! Command enum definitions
//!
//! This module contains all CLI command enums that define the command structure.

use crate::args::*;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Display version information
    Version,

    /// Run diagnostic checks
    Diagnose,

    /// Memory management commands
    #[command(subcommand)]
    Memory(MemoryCommands),

    /// Entity management commands
    #[command(subcommand)]
    Entity(EntityCommands),

    /// Relationship management commands
    #[command(subcommand)]
    Relationship(RelationshipCommands),

    /// Graph traversal and analysis commands
    #[command(subcommand)]
    Graph(GraphCommands),

    /// Batch operations
    #[command(subcommand)]
    Batch(BatchCommands),

    /// Relationship type management
    #[command(subcommand)]
    RelationshipType(RelationshipTypeCommands),

    /// Interactive tutorial mode
    #[command(alias = "interactive", alias = "learn")]
    Tutorial(TutorialArgs),

    /// Quick start guide - create sample data
    Quickstart(QuickstartArgs),

    /// Generate shell completion scripts
    Completions(CompletionsArgs),

    /// Clear all data from storage
    Clear,
}

#[derive(Subcommand)]
pub enum MemoryCommands {
    /// Add a new memory
    #[command(
        alias = "remember",
        long_about = r#"
Store a new memory in Locai. A memory is a piece of information that can be 
searched and retrieved later. Think of it like a note that an AI agent can remember.

WHAT IS A MEMORY?
A memory is a piece of information stored in Locai that can be searched and 
retrieved later. When you create a memory, Locai:
  1. Stores the content
  2. Indexes it for search (using BM25 text search)
  3. Optionally creates embeddings for semantic search
  4. Extracts entities and relationships automatically

MEMORY TYPES:
  • fact - Factual information (e.g., "Water boils at 100°C")
  • conversation - Dialogues or exchanges
  • episodic - Specific events or experiences
  • procedural - How-to information
  • identity - Information about people/entities
  • world - Information about the environment

PRIORITY LEVELS:
  • low - Nice to know, but not critical
  • normal - Standard importance (default)
  • high - Important information
  • critical - Must remember, very important

EXAMPLES:
  # Simple memory
  locai-cli memory add "The user likes coffee"
  
  # Fact with high priority
  locai-cli memory add "API key is secret" --priority high --type fact
  
  # Memory with tags for organization
  locai-cli memory add "Meeting notes" --tags work,meeting
  
  # Friendly alias
  locai-cli remember "Important information"

RELATED COMMANDS:
  • locai-cli memory search "query" - Search for memories
  • locai-cli memory list - List all memories
  • locai-cli memory get <id> - Show a specific memory
  • locai-cli --explain memory - Learn more about memories
"#
    )]
    Add(AddMemoryArgs),

    /// Get a memory by ID
    #[command(
        alias = "show",
        long_about = r#"
Display a specific memory by its ID. Shows all memory details including content, 
metadata, relationships, and associated entities.

EXAMPLES:
  # Get a memory by ID
  locai-cli memory get memory:abc123
  
  # Using friendly alias
  locai-cli memory show memory:abc123

RELATED COMMANDS:
  • locai-cli memory list - List all memories
  • locai-cli memory search "query" - Search for memories
  • locai-cli memory relationships <id> - View memory relationships
"#
    )]
    Get(GetMemoryArgs),

    /// Search for memories
    #[command(
        alias = "recall",
        long_about = r#"
Search for memories using text search, semantic search, or hybrid search.

SEARCH MODES:
  • hybrid (default) - Automatically combines text and semantic search when available
    - Results are tagged with [text], [semantic], or [text+semantic]
    - Falls back to text-only if embeddings unavailable
  • text - BM25 keyword search only - fast, works without embeddings
  • semantic - Vector similarity search only - finds related concepts, requires embeddings

HOW IT WORKS:
By default, hybrid search runs both text and semantic searches automatically:
  • Text search finds exact keyword matches (always works)
  • Semantic search finds related concepts (requires embeddings)
  • Results show which method found them: [text], [semantic], or [text+semantic]

SEMANTIC SEARCH:
Semantic search finds memories based on meaning, not just keywords. For example,
searching for "battle" will find memories about "war" and "warrior" even if they
don't contain the word "battle".

To enable semantic search:
  1. Set OLLAMA_URL and OLLAMA_MODEL environment variables
  2. Memories with embeddings (quickstart creates some)

EXAMPLES:
  # Hybrid search (default) - automatically combines both methods
  locai-cli memory search "warrior"
  # Results tagged: [text], [semantic], or [text+semantic]
  
  # Text search only
  locai-cli memory search "warrior" --mode text
  
  # Semantic search only
  locai-cli memory search "battle" --mode semantic
  
  # Filter by type
  locai-cli memory search "meeting" --memory-type episodic
  
  # Filter by tag
  locai-cli memory search "important" --tag urgent
  
  # Using friendly alias
  locai-cli recall "query"

RELATED COMMANDS:
  • locai-cli memory list - List all memories
  • locai-cli memory get <id> - Get a specific memory
  • locai-cli --explain search - Learn more about search modes
"#
    )]
    Search(SearchArgs),

    /// Delete a memory by ID
    #[command(alias = "forget")]
    Delete(DeleteMemoryArgs),

    /// List memories with optional filters
    List(ListMemoriesArgs),

    /// Add a tag to a memory
    Tag(TagMemoryArgs),

    /// Count memories
    Count(CountMemoriesArgs),

    /// Get memories by priority
    Priority(PriorityArgs),

    /// Get recent memories
    Recent(RecentArgs),

    /// Update a memory
    Update(UpdateMemoryArgs),

    /// Manage memory relationships
    Relationships(MemoryRelationshipsArgs),
}

#[derive(Subcommand)]
pub enum EntityCommands {
    /// Create a new entity
    Create(CreateEntityArgs),

    /// Get an entity by ID
    Get(GetEntityArgs),

    /// List entities
    List(ListEntitiesArgs),

    /// Delete an entity
    Delete(DeleteEntityArgs),

    /// Count entities
    Count,

    /// Update an entity
    Update(UpdateEntityArgs),

    /// Manage entity relationships
    Relationships(EntityRelationshipsArgs),

    /// Get memories for an entity
    Memories(EntityMemoriesArgs),

    /// Get central entities
    Central(CentralEntitiesArgs),
}

#[derive(Subcommand)]
pub enum RelationshipCommands {
    /// Create a relationship between two memories
    #[command(
        alias = "connect",
        alias = "link",
        long_about = r#"
Create a relationship between two memories or entities, connecting them in the graph.

RELATIONSHIP TYPES:
Common relationship types include:
  • related_to - General connection
  • references - One memory references another
  • follows - Sequential relationship
  • mentions - One memory mentions an entity
  • has_character - Memory involves a character/entity
  • takes_place_in - Memory occurs in a location/entity

EXAMPLES:
  # Connect two memories
  locai-cli relationship create memory:abc123 memory:def456 related_to
  
  # Connect memory to entity
  locai-cli relationship create memory:abc123 entity:person:john has_character
  
  # With properties
  locai-cli relationship create memory:abc123 memory:def456 references \
    --properties '{"context": "follow-up"}'
  
  # Using friendly aliases
  locai-cli relationship connect memory:abc123 memory:def456 related_to
  locai-cli relationship link memory:abc123 entity:person:john mentions

RELATED COMMANDS:
  • locai-cli relationship list - List all relationships
  • locai-cli relationship-type list - See available relationship types
  • locai-cli graph subgraph <id> - View connected memories
  • locai-cli --explain relationship - Learn more about relationships
"#
    )]
    Create(CreateRelationshipArgs),

    /// Get a relationship by ID
    Get(GetRelationshipArgs),

    /// List relationships
    List(ListRelationshipsArgs),

    /// Delete a relationship
    Delete(DeleteRelationshipArgs),

    /// Find related memories
    Related(RelatedMemoriesArgs),

    /// Update a relationship
    Update(UpdateRelationshipArgs),
}

#[derive(Subcommand)]
pub enum GraphCommands {
    /// Get memory graph around a specific memory
    Subgraph(SubgraphArgs),

    /// Find paths between two memories
    Paths(PathsArgs),

    /// Find connected memories
    Connected(ConnectedArgs),

    /// Get graph metrics
    Metrics,

    /// Query graph with pattern
    Query(GraphQueryArgs),

    /// Find similar graph structures
    Similar(GraphSimilarArgs),

    /// Get entity graph
    Entity(GraphEntityArgs),
}

#[derive(Subcommand)]
pub enum BatchCommands {
    /// Execute batch operations from a file
    Execute(ExecuteBatchArgs),
}

#[derive(Subcommand)]
pub enum RelationshipTypeCommands {
    /// List all relationship types
    List,

    /// Get a relationship type by name
    Get(GetRelationshipTypeArgs),

    /// Register a new relationship type
    Register(RegisterRelationshipTypeArgs),

    /// Update an existing relationship type
    Update(UpdateRelationshipTypeArgs),

    /// Delete a relationship type
    Delete(DeleteRelationshipTypeArgs),

    /// Get relationship type metrics
    Metrics,

    /// Seed common relationship types
    Seed,
}
