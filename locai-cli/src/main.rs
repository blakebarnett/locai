use clap::{Args, Parser, Subcommand};
use colored::*;
use locai::LocaiError;
use locai::config::ConfigBuilder;
use locai::memory::search_extensions::SearchMode;
use locai::models::{MemoryPriority, MemoryType};
use locai::prelude::*;
use locai::storage::filters::{
    EntityFilter, MemoryFilter, RelationshipFilter, SemanticSearchFilter,
};
use locai::storage::models::{Entity, MemoryGraph, MemoryPath, Relationship};
use serde_json::{Value, json};
use tracing::{Level, error, info};

// Define a shared structure for commands that need a MemoryManager
struct LocaiCliContext {
    memory_manager: MemoryManager,
}

impl LocaiCliContext {
    async fn new(data_dir: Option<String>) -> locai::Result<Self> {
        // Initialize Locai with custom data directory if provided
        let mm = if let Some(dir) = data_dir {
            let config = ConfigBuilder::new()
                .with_data_dir(dir)
                .with_default_storage()
                .with_default_ml()
                .with_default_logging()
                .build()?;
            locai::init(config).await?
        } else {
            locai::init_with_defaults().await?
        };
        Ok(Self { memory_manager: mm })
    }
}

// Helper function to output errors in the appropriate format
fn output_error(error_msg: &str, output_format: &str) {
    if output_format == "json" {
        let error_response = json!({
            "error": true,
            "message": error_msg,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&error_response).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        error!("{}", error_msg);
    }
}

#[derive(Parser)]
#[command(name = "locai-cli")]
#[command(about = "Locai memory service CLI", long_about = None)]
#[command(version = locai::VERSION)]
struct Cli {
    /// Custom data directory for storage
    #[arg(long, short, global = true)]
    data_dir: Option<String>,

    /// Output format (table, json) - use json for tool integration
    #[arg(long, short, default_value = "table", global = true)]
    output: String,

    /// Use machine-readable output (alias for --output json)
    #[arg(long, global = true)]
    machine: bool,

    /// Verbose output (debug level logging)
    #[arg(long, short, global = true)]
    verbose: bool,

    /// Quiet mode (suppress all logging output)
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

    /// Clear all data from storage
    Clear,
}

#[derive(Subcommand)]
enum MemoryCommands {
    /// Add a new memory
    #[command(alias = "remember")]
    Add(AddMemoryArgs),

    /// Get a memory by ID
    Get(GetMemoryArgs),

    /// Search for memories
    #[command(alias = "recall")]
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
}

#[derive(Subcommand)]
enum EntityCommands {
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
}

#[derive(Subcommand)]
enum RelationshipCommands {
    /// Create a relationship between two memories
    Create(CreateRelationshipArgs),

    /// Get a relationship by ID
    Get(GetRelationshipArgs),

    /// List relationships
    List(ListRelationshipsArgs),

    /// Delete a relationship
    Delete(DeleteRelationshipArgs),

    /// Find related memories
    Related(RelatedMemoriesArgs),
}

#[derive(Subcommand)]
enum GraphCommands {
    /// Get memory graph around a specific memory
    Subgraph(SubgraphArgs),

    /// Find paths between two memories
    Paths(PathsArgs),

    /// Find connected memories
    Connected(ConnectedArgs),
}

// Memory command arguments
#[derive(Args)]
struct AddMemoryArgs {
    /// Content of the memory
    content: String,

    /// Memory type (fact, conversation, procedural, episodic, identity, world, action, event)
    #[arg(long, short, default_value = "fact")]
    memory_type: String,

    /// Priority (low, normal, high, critical)
    #[arg(long, short, default_value = "normal")]
    priority: String,

    /// Tags to associate with the memory
    #[arg(long = "tag", short = 't')]
    tags: Vec<String>,
}

#[derive(Args)]
struct GetMemoryArgs {
    /// Memory ID
    id: String,
}

#[derive(Args)]
struct SearchArgs {
    /// Search query
    query: String,

    /// Maximum number of results
    #[arg(short, long, default_value_t = 10)]
    limit: usize,

    /// Search mode (text, vector, hybrid, keyword, bm25)
    #[arg(long, short, default_value = "text")]
    mode: String,

    /// Similarity threshold (0.0 to 1.0)
    #[arg(long)]
    threshold: Option<f32>,

    /// Filter by memory type
    #[arg(long)]
    memory_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    tag: Option<String>,
}

#[derive(Args)]
struct DeleteMemoryArgs {
    /// Memory ID
    id: String,
}

#[derive(Args)]
struct ListMemoriesArgs {
    /// Maximum number of results
    #[arg(short, long, default_value_t = 20)]
    limit: usize,

    /// Filter by memory type
    #[arg(long)]
    memory_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    tag: Option<String>,

    /// Filter by priority
    #[arg(long)]
    priority: Option<String>,
}

#[derive(Args)]
struct TagMemoryArgs {
    /// Memory ID
    id: String,

    /// Tag to add
    tag: String,
}

#[derive(Args)]
struct CountMemoriesArgs {
    /// Filter by memory type
    #[arg(long)]
    memory_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    tag: Option<String>,
}

#[derive(Args)]
struct PriorityArgs {
    /// Priority level (low, normal, high, critical)
    priority: String,

    /// Maximum number of results
    #[arg(short, long, default_value_t = 10)]
    limit: usize,
}

#[derive(Args)]
struct RecentArgs {
    /// Maximum number of results
    #[arg(short, long, default_value_t = 10)]
    limit: usize,
}

// Entity command arguments
#[derive(Args)]
struct CreateEntityArgs {
    /// Entity ID
    id: String,

    /// Entity type
    entity_type: String,

    /// Entity properties (JSON format)
    #[arg(long)]
    properties: Option<String>,
}

#[derive(Args)]
struct GetEntityArgs {
    /// Entity ID
    id: String,
}

#[derive(Args)]
struct ListEntitiesArgs {
    /// Maximum number of results
    #[arg(short, long, default_value_t = 20)]
    limit: usize,

    /// Filter by entity type
    #[arg(long)]
    entity_type: Option<String>,
}

#[derive(Args)]
struct DeleteEntityArgs {
    /// Entity ID
    id: String,
}

// Relationship command arguments
#[derive(Args)]
struct CreateRelationshipArgs {
    /// Source memory/entity ID
    from: String,

    /// Target memory/entity ID
    to: String,

    /// Relationship type
    relationship_type: String,

    /// Create bidirectional relationship
    #[arg(long)]
    bidirectional: bool,

    /// Properties (JSON format)
    #[arg(long)]
    properties: Option<String>,
}

#[derive(Args)]
struct GetRelationshipArgs {
    /// Relationship ID
    id: String,
}

#[derive(Args)]
struct ListRelationshipsArgs {
    /// Maximum number of results
    #[arg(short, long, default_value_t = 20)]
    limit: usize,

    /// Filter by relationship type
    #[arg(long)]
    relationship_type: Option<String>,
}

#[derive(Args)]
struct DeleteRelationshipArgs {
    /// Relationship ID
    id: String,
}

#[derive(Args)]
struct RelatedMemoriesArgs {
    /// Memory ID to find relationships for
    id: String,

    /// Relationship type filter
    #[arg(long)]
    relationship_type: Option<String>,

    /// Direction (outgoing, incoming, both)
    #[arg(long, default_value = "both")]
    direction: String,
}

// Graph command arguments
#[derive(Args)]
struct SubgraphArgs {
    /// Memory ID to center the subgraph on
    id: String,

    /// Depth of traversal
    #[arg(short, long, default_value_t = 2)]
    depth: u8,
}

#[derive(Args)]
struct PathsArgs {
    /// Source memory ID
    from: String,

    /// Target memory ID
    to: String,

    /// Maximum depth to search
    #[arg(short, long, default_value_t = 5)]
    depth: u8,
}

#[derive(Args)]
struct ConnectedArgs {
    /// Memory ID to start from
    id: String,

    /// Relationship type to follow
    relationship_type: String,

    /// Maximum depth to traverse
    #[arg(short, long, default_value_t = 3)]
    depth: u8,
}

#[tokio::main]
async fn main() -> locai::Result<()> {
    let cli_args = Cli::parse();

    // Determine output format - priority: machine flag > env var > cli arg > default
    let output_format = if cli_args.machine {
        "json".to_string()
    } else if let Ok(env_output) = std::env::var("LOCAI_OUTPUT") {
        env_output
    } else {
        cli_args.output.clone()
    };

    // Override quiet flag with environment variable if set
    let is_quiet = cli_args.quiet
        || std::env::var("LOCAI_QUIET")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

    // Initialize logging based on verbosity and quiet mode
    // Machine mode automatically enables quiet mode for clean JSON output
    let log_level = if is_quiet || cli_args.machine {
        Level::ERROR // Only show critical errors
    } else if cli_args.verbose {
        Level::DEBUG // Show everything when verbose
    } else {
        Level::WARN // Default to warnings only
    };

    tracing_subscriber::fmt().with_max_level(log_level).init();

    // Initialize context for commands that need it
    let mut context: Option<LocaiCliContext> = None;
    if !matches!(cli_args.command, Commands::Version) {
        context = Some(LocaiCliContext::new(cli_args.data_dir).await?);
    }

    match cli_args.command {
        Commands::Version => {
            println!(
                "{} {} {}",
                "Locai CLI".color(CliColors::accent()).bold(),
                "v".color(CliColors::muted()),
                locai::VERSION.color(CliColors::success()).bold()
            );
        }

        Commands::Diagnose => {
            if let Some(ctx) = &context {
                info!("Running diagnostic checks...");

                // Check storage health
                match ctx.memory_manager.storage().health_check().await {
                    Ok(true) => println!("{}", format_success("Storage: Healthy")),
                    Ok(false) => println!("{}", format_error("Storage: Unhealthy")),
                    Err(e) => println!("{}", format_error(&format!("Storage: Error - {}", e))),
                }

                // Check if ML service is available
                if ctx.memory_manager.config().ml.embedding.service_type
                    == locai::config::EmbeddingServiceType::Local
                {
                    println!("{}", format_success("ML Service: Enabled (Local)"));
                } else {
                    println!("{}", format_success("ML Service: Enabled (Remote)"));
                }

                // Get storage metadata
                match ctx.memory_manager.storage().get_metadata().await {
                    Ok(metadata) => {
                        if output_format == "json" {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&metadata)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );
                        } else {
                            println!("Storage metadata: {}", metadata);
                        }
                    }
                    Err(e) => error!("Failed to get storage metadata: {}", e),
                }
            }
        }

        Commands::Memory(memory_cmd) => {
            if let Some(ctx) = context {
                handle_memory_command(memory_cmd, &ctx, &output_format).await?;
            }
        }

        Commands::Entity(entity_cmd) => {
            if let Some(ctx) = context {
                handle_entity_command(entity_cmd, &ctx, &output_format).await?;
            }
        }

        Commands::Relationship(rel_cmd) => {
            if let Some(ctx) = context {
                handle_relationship_command(rel_cmd, &ctx, &output_format).await?;
            }
        }

        Commands::Graph(graph_cmd) => {
            if let Some(ctx) = context {
                handle_graph_command(graph_cmd, &ctx, &output_format).await?;
            }
        }

        Commands::Clear => {
            if let Some(ctx) = context {
                println!("Are you sure you want to clear all data? This cannot be undone.");
                println!("Type 'yes' to confirm:");
                let mut input = String::new();
                if let Err(e) = std::io::stdin().read_line(&mut input) {
                    error!("Failed to read input: {}", e);
                    return Ok(());
                }
                if input.trim() == "yes" {
                    ctx.memory_manager.clear_storage().await?;
                    println!("{}", format_success("Storage cleared successfully."));
                } else {
                    println!("{}", format_info("Operation cancelled."));
                }
            }
        }
    }

    Ok(())
}

async fn handle_memory_command(
    cmd: MemoryCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        MemoryCommands::Add(args) => {
            let memory_type = parse_memory_type(&args.memory_type)?;
            let priority = parse_priority(&args.priority)?;

            let memory_id = ctx
                .memory_manager
                .add_memory_with_options(args.content, |builder| {
                    let mut b = builder.memory_type(memory_type).priority(priority);
                    for tag in args.tags {
                        b = b.tag(tag);
                    }
                    b
                })
                .await?;

            if output_format == "json" {
                let result = serde_json::json!({ "memory_id": memory_id });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!(
                    "{}",
                    format_success(&format!(
                        "Memory created with ID: {}",
                        memory_id.color(CliColors::accent()).bold()
                    ))
                );
            }
        }

        MemoryCommands::Get(args) => match ctx.memory_manager.get_memory(&args.id).await? {
            Some(memory) => {
                if output_format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&memory).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    print_memory(&memory);
                }
            }
            None => {
                println!(
                    "{}",
                    format_warning(&format!(
                        "Memory with ID '{}' not found.",
                        args.id.color(CliColors::accent())
                    ))
                );
            }
        },

        MemoryCommands::Search(args) => {
            let search_mode = match args.mode.as_str() {
                "vector" => SearchMode::Vector,
                "hybrid" => SearchMode::Hybrid,
                "semantic" => SearchMode::Vector, // Requires embeddings
                "text" | "keyword" | "bm25" => SearchMode::Text,
                _ => SearchMode::Text, // Default to BM25 text search
            };

            // Create search filter
            let filter =
                if args.threshold.is_some() || args.memory_type.is_some() || args.tag.is_some() {
                    Some(SemanticSearchFilter {
                        similarity_threshold: args.threshold,
                        memory_filter: {
                            let mut mem_filter = MemoryFilter::default();
                            if let Some(mem_type) = args.memory_type {
                                mem_filter.memory_type = Some(mem_type);
                            }
                            if let Some(tag) = args.tag {
                                mem_filter.tags = Some(vec![tag]);
                            }
                            Some(mem_filter)
                        },
                    })
                } else {
                    None
                };

            match ctx
                .memory_manager
                .search(&args.query, Some(args.limit), filter, search_mode)
                .await
            {
                Ok(results) => {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&results)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else if results.is_empty() {
                        println!(
                            "{}",
                            format_info(&format!(
                                "No memories found matching '{}'",
                                args.query.color(CliColors::accent())
                            ))
                        );
                    } else {
                        println!(
                            "{} (query: {})",
                            format_info(&format!("Found {} memories:", results.len())),
                            args.query.color(CliColors::accent()).italic()
                        );
                        for (i, result) in results.iter().enumerate() {
                            let score = result.score.unwrap_or(0.0);
                            let (score_label, score_color) = match score {
                                s if s > 1.4 => ("Excellent", CliColors::success()),
                                s if s > 1.2 => ("Good", CliColors::info()),
                                s if s > 1.0 => ("Fair", CliColors::warning()),
                                _ => ("Weak", CliColors::muted()),
                            };
                            println!(
                                "{}. {} {}",
                                format!("{}", i + 1).color(CliColors::muted()),
                                format!("[{} match: {:.2}]", score_label, score).color(score_color),
                                result.memory.content
                            );
                        }
                    }
                }
                Err(e) => {
                    output_error(&format!("Search failed: {}", e), output_format);
                }
            }
        }

        MemoryCommands::Delete(args) => match ctx.memory_manager.delete_memory(&args.id).await? {
            true => println!(
                "{}",
                format_success(&format!(
                    "Memory '{}' deleted successfully.",
                    args.id.color(CliColors::accent())
                ))
            ),
            false => println!(
                "{}",
                format_warning(&format!(
                    "Memory '{}' not found or could not be deleted.",
                    args.id.color(CliColors::accent())
                ))
            ),
        },

        MemoryCommands::List(args) => {
            let mut filter = MemoryFilter::default();

            if let Some(mem_type) = args.memory_type {
                filter.memory_type = Some(mem_type);
            }

            if let Some(tag) = args.tag {
                filter.tags = Some(vec![tag]);
            }

            let memories = ctx
                .memory_manager
                .filter_memories(filter, None, None, Some(args.limit))
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }

        MemoryCommands::Tag(args) => {
            match ctx.memory_manager.tag_memory(&args.id, &args.tag).await? {
                true => println!("Tag '{}' added to memory '{}'.", args.tag, args.id),
                false => println!("Failed to add tag or memory not found."),
            }
        }

        MemoryCommands::Count(args) => {
            let mut filter = MemoryFilter::default();

            if let Some(mem_type) = args.memory_type {
                filter.memory_type = Some(mem_type);
            }

            if let Some(tag) = args.tag {
                filter.tags = Some(vec![tag]);
            }

            let count = ctx.memory_manager.count_memories(Some(filter)).await?;

            if output_format == "json" {
                let result = serde_json::json!({ "count": count });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!("Total memories: {}", count);
            }
        }

        MemoryCommands::Priority(args) => {
            let priority = parse_priority(&args.priority)?;
            let memories = ctx
                .memory_manager
                .get_memories_by_priority(priority, Some(args.limit))
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }

        MemoryCommands::Recent(args) => {
            let memories = ctx.memory_manager.get_recent_memories(args.limit).await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }
    }

    Ok(())
}

async fn handle_entity_command(
    cmd: EntityCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        EntityCommands::Create(args) => {
            let properties = if let Some(props) = args.properties {
                match serde_json::from_str(&props) {
                    Ok(props) => props,
                    Err(e) => {
                        error!("Failed to parse properties JSON: {}", e);
                        return Ok(());
                    }
                }
            } else {
                Value::Null
            };

            let entity = Entity {
                id: args.id.clone(),
                entity_type: args.entity_type,
                properties,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            let created = ctx.memory_manager.create_entity(entity).await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&created).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!("Entity created with ID: {}", created.id);
            }
        }

        EntityCommands::Get(args) => match ctx.memory_manager.get_entity(&args.id).await? {
            Some(entity) => {
                if output_format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&entity).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    print_entity(&entity);
                }
            }
            None => {
                println!("Entity with ID '{}' not found.", args.id);
            }
        },

        EntityCommands::List(args) => {
            let filter = args.entity_type.map(|entity_type| EntityFilter {
                entity_type: Some(entity_type),
                ..Default::default()
            });

            let entities = ctx
                .memory_manager
                .list_entities(filter, Some(args.limit), None)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&entities).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_entity_list(&entities);
            }
        }

        EntityCommands::Delete(args) => match ctx.memory_manager.delete_entity(&args.id).await? {
            true => println!("Entity '{}' deleted successfully.", args.id),
            false => println!("Entity '{}' not found or could not be deleted.", args.id),
        },

        EntityCommands::Count => {
            let count = ctx.memory_manager.count_entities(None).await?;

            if output_format == "json" {
                let result = serde_json::json!({ "count": count });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                println!("Total entities: {}", count);
            }
        }
    }

    Ok(())
}

async fn handle_relationship_command(
    cmd: RelationshipCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        RelationshipCommands::Create(args) => {
            if args.bidirectional {
                ctx.memory_manager
                    .create_bidirectional_relationship(
                        &args.from,
                        &args.to,
                        &args.relationship_type,
                    )
                    .await?;
                println!(
                    "Bidirectional relationship created between '{}' and '{}'",
                    args.from, args.to
                );
            } else {
                ctx.memory_manager
                    .create_relationship(&args.from, &args.to, &args.relationship_type)
                    .await?;
                println!("Relationship created from '{}' to '{}'", args.from, args.to);
            }
        }

        RelationshipCommands::Get(args) => {
            match ctx.memory_manager.get_relationship(&args.id).await? {
                Some(relationship) => {
                    if output_format == "json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&relationship)
                                .unwrap_or_else(|_| "{}".to_string())
                        );
                    } else {
                        print_relationship(&relationship);
                    }
                }
                None => {
                    println!("Relationship with ID '{}' not found.", args.id);
                }
            }
        }

        RelationshipCommands::List(args) => {
            let filter = args.relationship_type.map(|rel_type| RelationshipFilter {
                relationship_type: Some(rel_type),
                ..Default::default()
            });

            let relationships = ctx
                .memory_manager
                .list_relationships(filter, Some(args.limit), None)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&relationships)
                        .unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_relationship_list(&relationships);
            }
        }

        RelationshipCommands::Delete(args) => {
            match ctx.memory_manager.delete_relationship(&args.id).await? {
                true => println!("Relationship '{}' deleted successfully.", args.id),
                false => println!(
                    "Relationship '{}' not found or could not be deleted.",
                    args.id
                ),
            }
        }

        RelationshipCommands::Related(args) => {
            let memories = ctx
                .memory_manager
                .get_related_memories(&args.id, args.relationship_type.as_deref(), &args.direction)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }
    }

    Ok(())
}

async fn handle_graph_command(
    cmd: GraphCommands,
    ctx: &LocaiCliContext,
    output_format: &str,
) -> locai::Result<()> {
    match cmd {
        GraphCommands::Subgraph(args) => {
            let graph = ctx
                .memory_manager
                .get_memory_graph(&args.id, args.depth)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_graph(&graph);
            }
        }

        GraphCommands::Paths(args) => {
            let paths = ctx
                .memory_manager
                .find_paths(&args.from, &args.to, args.depth)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&paths).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_paths(&paths);
            }
        }

        GraphCommands::Connected(args) => {
            let memories = ctx
                .memory_manager
                .find_connected_memories(&args.id, &args.relationship_type, args.depth)
                .await?;

            if output_format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&memories).unwrap_or_else(|_| "{}".to_string())
                );
            } else {
                print_memory_list(&memories);
            }
        }
    }

    Ok(())
}

// Color constants matching web app theme
struct CliColors;

impl CliColors {
    // Status colors (same as web app)
    fn success() -> Color {
        Color::TrueColor {
            r: 34,
            g: 197,
            b: 94,
        }
    } // #22c55e
    fn error() -> Color {
        Color::TrueColor {
            r: 239,
            g: 68,
            b: 68,
        }
    } // #ef4444
    fn warning() -> Color {
        Color::TrueColor {
            r: 245,
            g: 158,
            b: 11,
        }
    } // #f59e0b
    fn info() -> Color {
        Color::TrueColor {
            r: 59,
            g: 130,
            b: 246,
        }
    } // #3b82f6

    // Memory type colors (same as web app nodes)
    fn memory_fact() -> Color {
        Color::TrueColor {
            r: 59,
            g: 130,
            b: 246,
        }
    } // #3b82f6
    fn memory_episodic() -> Color {
        Color::TrueColor {
            r: 34,
            g: 197,
            b: 94,
        }
    } // #22c55e
    fn memory_semantic() -> Color {
        Color::TrueColor {
            r: 168,
            g: 85,
            b: 247,
        }
    } // #a855f7
    fn entity() -> Color {
        Color::TrueColor {
            r: 245,
            g: 158,
            b: 11,
        }
    } // #f59e0b

    // Text colors
    fn muted() -> Color {
        Color::TrueColor {
            r: 148,
            g: 163,
            b: 184,
        }
    } // #94a3b8
    fn primary() -> Color {
        Color::White
    }
    fn accent() -> Color {
        Color::TrueColor {
            r: 59,
            g: 130,
            b: 246,
        }
    } // #3b82f6
}

// Utility functions for styled output
fn format_success(msg: &str) -> String {
    format!(
        "{} {}",
        "✓".color(CliColors::success()).bold(),
        msg.color(CliColors::success())
    )
}

fn format_error(msg: &str) -> String {
    format!(
        "{} {}",
        "✗".color(CliColors::error()).bold(),
        msg.color(CliColors::error())
    )
}

fn format_warning(msg: &str) -> String {
    format!(
        "{} {}",
        "⚠".color(CliColors::warning()).bold(),
        msg.color(CliColors::warning())
    )
}

fn format_info(msg: &str) -> String {
    format!(
        "{} {}",
        "ℹ".color(CliColors::info()).bold(),
        msg.color(CliColors::info())
    )
}

fn format_memory_type(memory_type: &MemoryType) -> ColoredString {
    match memory_type {
        MemoryType::Fact => "Fact".color(CliColors::memory_fact()),
        MemoryType::Episodic => "Episodic".color(CliColors::memory_episodic()),
        MemoryType::Procedural
        | MemoryType::World
        | MemoryType::Action
        | MemoryType::Event
        | MemoryType::Wisdom => format!("{:?}", memory_type).color(CliColors::memory_semantic()),
        MemoryType::Conversation | MemoryType::Identity => {
            format!("{:?}", memory_type).color(CliColors::memory_episodic())
        }
        MemoryType::Custom(name) => name.color(CliColors::memory_semantic()),
    }
}

fn format_priority(priority: &MemoryPriority) -> ColoredString {
    match priority {
        MemoryPriority::Critical => "Critical".color(CliColors::error()).bold(),
        MemoryPriority::High => "High".color(CliColors::warning()),
        MemoryPriority::Normal => "Normal".color(CliColors::muted()),
        MemoryPriority::Low => "Low".color(CliColors::muted()).dimmed(),
    }
}

// Helper functions for parsing and printing

fn parse_memory_type(type_str: &str) -> locai::Result<MemoryType> {
    match type_str {
        "fact" => Ok(MemoryType::Fact),
        "conversation" => Ok(MemoryType::Conversation),
        "procedural" => Ok(MemoryType::Procedural),
        "episodic" => Ok(MemoryType::Episodic),
        "identity" => Ok(MemoryType::Identity),
        "world" => Ok(MemoryType::World),
        "action" => Ok(MemoryType::Action),
        "event" => Ok(MemoryType::Event),
        _ => Err(LocaiError::Other(format!(
            "Invalid memory type: {}",
            type_str
        ))),
    }
}

fn parse_priority(priority_str: &str) -> locai::Result<MemoryPriority> {
    match priority_str {
        "low" => Ok(MemoryPriority::Low),
        "normal" => Ok(MemoryPriority::Normal),
        "high" => Ok(MemoryPriority::High),
        "critical" => Ok(MemoryPriority::Critical),
        _ => Err(LocaiError::Other(format!(
            "Invalid priority: {}",
            priority_str
        ))),
    }
}

fn print_memory(memory: &Memory) {
    println!(
        "{}",
        "━━━ Memory Details ━━━".color(CliColors::accent()).bold()
    );
    println!(
        "{}: {}",
        "ID".color(CliColors::muted()),
        memory.id.color(CliColors::accent()).bold()
    );
    println!(
        "{}: {}",
        "Type".color(CliColors::muted()),
        format_memory_type(&memory.memory_type)
    );
    println!(
        "{}: {}",
        "Priority".color(CliColors::muted()),
        format_priority(&memory.priority)
    );
    println!(
        "{}: {}",
        "Content".color(CliColors::muted()),
        memory.content
    );
    println!(
        "{}: {}",
        "Created".color(CliColors::muted()),
        memory
            .created_at
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string()
            .color(CliColors::primary())
    );
    if let Some(last_accessed) = memory.last_accessed {
        println!(
            "{}: {}",
            "Last Accessed".color(CliColors::muted()),
            last_accessed
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string()
                .color(CliColors::primary())
        );
    }
    if !memory.tags.is_empty() {
        println!(
            "{}: {}",
            "Tags".color(CliColors::muted()),
            memory
                .tags
                .iter()
                .map(|tag| format!("#{}", tag))
                .collect::<Vec<_>>()
                .join(" ")
                .color(CliColors::info())
        );
    }
    if memory.embedding.is_some() {
        println!(
            "{}: {}",
            "Has Embedding".color(CliColors::muted()),
            "Yes".color(CliColors::success())
        );
    }
}

fn print_memory_list(memories: &[Memory]) {
    if memories.is_empty() {
        println!("{}", format_info("No memories found."));
        return;
    }

    println!(
        "{}",
        format_info(&format!("Found {} memories:", memories.len()))
    );
    println!();

    // Print header with colors
    println!(
        "{:<36} {:<15} {:<10} {}",
        "ID".color(CliColors::muted()).bold(),
        "Type".color(CliColors::muted()).bold(),
        "Priority".color(CliColors::muted()).bold(),
        "Content".color(CliColors::muted()).bold()
    );
    println!("{}", "─".repeat(80).color(CliColors::muted()));

    for memory in memories {
        let content = if memory.content.len() > 50 {
            format!("{}...", &memory.content[..47])
        } else {
            memory.content.clone()
        };

        println!(
            "{:<36} {:<24} {:<18} {}",
            memory.id.color(CliColors::accent()),
            format_memory_type(&memory.memory_type),
            format_priority(&memory.priority),
            content.color(CliColors::primary())
        );
    }
}

fn print_entity(entity: &Entity) {
    println!(
        "{}",
        "━━━ Entity Details ━━━".color(CliColors::entity()).bold()
    );
    println!(
        "{}: {}",
        "ID".color(CliColors::muted()),
        entity.id.color(CliColors::entity()).bold()
    );
    println!(
        "{}: {}",
        "Type".color(CliColors::muted()),
        entity.entity_type.color(CliColors::entity())
    );
    println!(
        "{}: {}",
        "Created".color(CliColors::muted()),
        entity
            .created_at
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string()
            .color(CliColors::primary())
    );
    println!(
        "{}: {}",
        "Updated".color(CliColors::muted()),
        entity
            .updated_at
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string()
            .color(CliColors::primary())
    );
    if entity.properties != Value::Null {
        println!(
            "{}: {}",
            "Properties".color(CliColors::muted()),
            serde_json::to_string_pretty(&entity.properties)
                .unwrap_or_default()
                .color(CliColors::primary())
        );
    }
}

fn print_entity_list(entities: &[Entity]) {
    if entities.is_empty() {
        println!("{}", format_info("No entities found."));
        return;
    }

    println!(
        "{}",
        format_info(&format!("Found {} entities:", entities.len()))
    );
    println!();

    println!(
        "{:<36} {}",
        "ID".color(CliColors::muted()).bold(),
        "Type".color(CliColors::muted()).bold()
    );
    println!("{}", "─".repeat(60).color(CliColors::muted()));

    for entity in entities {
        println!(
            "{:<36} {}",
            entity.id.color(CliColors::accent()),
            entity.entity_type.color(CliColors::entity())
        );
    }
}

fn print_relationship(relationship: &Relationship) {
    println!(
        "{}",
        "━━━ Relationship Details ━━━"
            .color(CliColors::info())
            .bold()
    );
    println!(
        "{}: {}",
        "ID".color(CliColors::muted()),
        relationship.id.color(CliColors::accent()).bold()
    );
    println!(
        "{}: {}",
        "Source".color(CliColors::muted()),
        relationship.source_id.color(CliColors::accent())
    );
    println!(
        "{}: {}",
        "Target".color(CliColors::muted()),
        relationship.target_id.color(CliColors::accent())
    );
    println!(
        "{}: {}",
        "Type".color(CliColors::muted()),
        relationship.relationship_type.color(CliColors::info())
    );
    println!(
        "{}: {}",
        "Created".color(CliColors::muted()),
        relationship
            .created_at
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string()
            .color(CliColors::primary())
    );
    if relationship.properties != Value::Null {
        println!(
            "{}: {}",
            "Properties".color(CliColors::muted()),
            serde_json::to_string_pretty(&relationship.properties)
                .unwrap_or_default()
                .color(CliColors::primary())
        );
    }
}

fn print_relationship_list(relationships: &[Relationship]) {
    if relationships.is_empty() {
        println!("{}", format_info("No relationships found."));
        return;
    }

    println!(
        "{}",
        format_info(&format!("Found {} relationships:", relationships.len()))
    );
    println!();

    println!(
        "{:<20} {:<36} {:<36} {}",
        "Type".color(CliColors::muted()).bold(),
        "Source".color(CliColors::muted()).bold(),
        "Target".color(CliColors::muted()).bold(),
        "ID".color(CliColors::muted()).bold()
    );
    println!("{}", "─".repeat(120).color(CliColors::muted()));

    for rel in relationships {
        println!(
            "{:<20} {:<36} {:<36} {}",
            rel.relationship_type.color(CliColors::info()),
            rel.source_id.color(CliColors::accent()),
            rel.target_id.color(CliColors::accent()),
            rel.id.color(CliColors::muted())
        );
    }
}

fn print_memory_graph(graph: &MemoryGraph) {
    println!(
        "{}",
        "━━━ Memory Graph ━━━".color(CliColors::accent()).bold()
    );
    println!(
        "{}: {}",
        "Memories".color(CliColors::muted()),
        graph.memories.len().to_string().color(CliColors::success())
    );
    println!(
        "{}: {}",
        "Relationships".color(CliColors::muted()),
        graph
            .relationships
            .len()
            .to_string()
            .color(CliColors::info())
    );

    if !graph.memories.is_empty() {
        println!();
        println!("{}", "Memories:".color(CliColors::primary()).bold());
        for memory in graph.memories.values() {
            let content = if memory.content.len() > 60 {
                format!("{}...", &memory.content[..57])
            } else {
                memory.content.clone()
            };
            println!(
                "  {} [{}] {}",
                "●".color(CliColors::accent()),
                format_memory_type(&memory.memory_type),
                content.color(CliColors::primary())
            );
        }
    }

    if !graph.relationships.is_empty() {
        println!();
        println!("{}", "Relationships:".color(CliColors::primary()).bold());
        for rel in &graph.relationships {
            println!(
                "  {} {} {} {}",
                rel.source_id.color(CliColors::accent()),
                "→".color(CliColors::info()),
                format!("[{}]", rel.relationship_type).color(CliColors::info()),
                rel.target_id.color(CliColors::accent())
            );
        }
    }
}

fn print_paths(paths: &[MemoryPath]) {
    if paths.is_empty() {
        println!("{}", format_info("No paths found."));
        return;
    }

    println!("{}", format_info(&format!("Found {} paths:", paths.len())));
    for (i, path) in paths.iter().enumerate() {
        println!();
        println!(
            "{} {} {}",
            "Path".color(CliColors::primary()).bold(),
            format!("{}", i + 1).color(CliColors::accent()).bold(),
            format!("({} steps)", path.memories.len()).color(CliColors::muted())
        );
        for (j, memory) in path.memories.iter().enumerate() {
            if j > 0 {
                println!("  {}", "↓".color(CliColors::info()));
            }
            let content = if memory.content.len() > 60 {
                format!("{}...", &memory.content[..57])
            } else {
                memory.content.clone()
            };
            println!(
                "  {} [{}] {}",
                "●".color(CliColors::accent()),
                format_memory_type(&memory.memory_type),
                content.color(CliColors::primary())
            );
        }
    }
}
