use clap::{Parser, Subcommand, CommandFactory};
use colored::*;
use tracing::{Level, error, info};

mod args;
mod commands;
mod context;
mod handlers;
mod help;
mod output;
mod utils;

use context::LocaiCliContext;
use handlers::*;
use output::{CliColors, format_success, format_error, format_info};

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

    /// Debug level logging (same as --verbose)
    #[arg(long, global = true)]
    debug: bool,

    /// Quiet mode (suppress all logging output)
    #[arg(long, short, global = true)]
    quiet: bool,

    /// Set log level explicitly (off, error, warn, info, debug, trace)
    #[arg(long, global = true)]
    log_level: Option<String>,

    /// Explain a concept (memory, entity, relationship, graph, search, batch)
    #[arg(long, global = true)]
    explain: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display version information
    Version,

    /// Run diagnostic checks
    Diagnose,

    /// Memory operations
    #[command(subcommand)]
    Memory(commands::MemoryCommands),

    /// Entity operations
    #[command(subcommand)]
    Entity(commands::EntityCommands),

    /// Relationship operations
    #[command(subcommand)]
    Relationship(commands::RelationshipCommands),

    /// Graph operations
    #[command(subcommand)]
    Graph(commands::GraphCommands),

    /// Batch operations
    #[command(subcommand)]
    Batch(commands::BatchCommands),

    /// Relationship type operations
    #[command(subcommand)]
    RelationshipType(commands::RelationshipTypeCommands),

    /// Interactive tutorial mode
    #[command(alias = "interactive", alias = "learn")]
    Tutorial(args::TutorialArgs),

    /// Quick start guide - create sample data
    Quickstart(args::QuickstartArgs),

    /// Generate shell completion scripts
    Completions(args::CompletionsArgs),

    /// Clear all storage (use with caution!)
    Clear,
}

#[tokio::main]
async fn main() {
    let cli_args = Cli::parse();
    
    let output_format_str = if cli_args.machine {
        "json".to_string()
    } else if atty::isnt(atty::Stream::Stdout) {
        // Auto-detect: if stdout is not a TTY (piped/redirected), default to JSON
        "json".to_string()
    } else {
        cli_args.output.clone()
    };
    
    let result = run(cli_args, &output_format_str).await;
    
    if let Err(e) = result {
        crate::output::output_error_json(&e, &output_format_str);
        std::process::exit(1);
    }
}

async fn run(cli_args: Cli, output_format: &str) -> locai::Result<()> {
    // Handle --explain flag early
    if let Some(concept) = &cli_args.explain {
        help::explanations::show_explanation(concept)?;
        return Ok(());
    }

    // Skip logging and context initialization for commands that don't need them
    let skip_init = matches!(cli_args.command, Commands::Version | Commands::Completions(_));
    
    if !skip_init {
        // Determine log level: explicit > quiet > verbose/debug > default (OFF)
        // Always initialize logging to prevent library from initializing with defaults
        let log_level = if let Some(level_str) = &cli_args.log_level {
            // Parse explicit log level
            match level_str.to_lowercase().as_str() {
                "off" => Level::ERROR, // Use ERROR as "off" to suppress everything
                "error" => Level::ERROR,
                "warn" => Level::WARN,
                "info" => Level::INFO,
                "debug" => Level::DEBUG,
                "trace" => Level::TRACE,
                _ => {
                    eprintln!("Warning: Invalid log level '{}'. Valid levels: off, error, warn, info, debug, trace", level_str);
                    Level::ERROR // Default to ERROR (suppressed) on invalid level
                }
            }
        } else if cli_args.quiet {
            Level::ERROR
        } else if cli_args.verbose || cli_args.debug {
            Level::DEBUG
        } else {
            // Default: suppress all logging by using ERROR level
            // This prevents the library from initializing logging with its defaults
            Level::ERROR
        };

        // Always initialize logging BEFORE creating context
        // This prevents the library from initializing logging with its own config
        // Use try_init() so we don't error if somehow already initialized
        let _ = tracing_subscriber::fmt()
            .with_max_level(log_level)
            .with_writer(std::io::stderr) // Log to stderr so it doesn't interfere with JSON output
            .try_init();
    }

    let mut context: Option<LocaiCliContext> = None;
    // Skip context initialization for commands that don't need storage
    if !skip_init {
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

                match ctx.memory_manager.storage().health_check().await {
                    Ok(true) => println!("{}", format_success("Storage: Healthy")),
                    Ok(false) => println!("{}", format_error("Storage: Unhealthy")),
                    Err(e) => println!("{}", format_error(&format!("Storage: Error - {}", e))),
                }

                if ctx.memory_manager.config().ml.embedding.service_type
                    == locai::config::EmbeddingServiceType::Local
                {
                    println!("{}", format_success("ML Service: Enabled (Local)"));
                } else {
                    println!("{}", format_success("ML Service: Enabled (Remote)"));
                }

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
                handle_memory_command(memory_cmd, &ctx, output_format).await?;
            }
        }

        Commands::Entity(entity_cmd) => {
            if let Some(ctx) = context {
                handle_entity_command(entity_cmd, &ctx, output_format).await?;
            }
        }

        Commands::Relationship(rel_cmd) => {
            if let Some(ctx) = context {
                handle_relationship_command(rel_cmd, &ctx, output_format).await?;
            }
        }

        Commands::Graph(graph_cmd) => {
            if let Some(ctx) = context {
                handle_graph_command(graph_cmd, &ctx, output_format).await?;
            }
        }

        Commands::Batch(batch_cmd) => {
            if let Some(ctx) = context {
                handle_batch_command(batch_cmd, &ctx, output_format).await?;
            }
        }

        Commands::RelationshipType(rel_type_cmd) => {
            if let Some(ctx) = context {
                handle_relationship_type_command(rel_type_cmd, &ctx, output_format).await?;
            }
        }

        Commands::Tutorial(tutorial_args) => {
            if let Some(ctx) = context {
                handle_tutorial_command(tutorial_args, &ctx, output_format).await?;
            }
        }

        Commands::Quickstart(quickstart_args) => {
            if let Some(ctx) = context {
                handle_quickstart_command(quickstart_args, &ctx, output_format).await?;
            }
        }

        Commands::Completions(completions_args) => {
            use clap_complete::generate;
            let mut cmd = Cli::command();
            
            // Generate completion script
            match completions_args.shell {
                args::Shell::Bash => {
                    generate(clap_complete::shells::Bash, &mut cmd, "locai-cli", &mut std::io::stdout());
                    // Show installation instructions on stderr (only if stderr is a TTY)
                    if atty::is(atty::Stream::Stderr) {
                        eprintln!("\n{}", format_info("Bash completion script generated. Installation options:"));
                        eprintln!("  1. Direct sourcing: source <(locai-cli completions bash)");
                        eprintln!("  2. Save and source: locai-cli completions bash > ~/.locai-cli.bash && echo 'source ~/.locai-cli.bash' >> ~/.bashrc");
                        eprintln!("  3. System-wide: sudo sh -c 'locai-cli completions bash > /etc/bash_completion.d/locai-cli'");
                        eprintln!("\nNote: ~/.bash_completion.d/ is NOT automatically loaded by bash.");
                        eprintln!("See docs/SHELL_COMPLETION_INSTALLATION.md for details.");
                    }
                }
                args::Shell::Zsh => {
                    generate(clap_complete::shells::Zsh, &mut cmd, "locai-cli", &mut std::io::stdout());
                    if atty::is(atty::Stream::Stderr) {
                        eprintln!("\n{}", format_info("Zsh completion script generated. To install:"));
                        eprintln!("  mkdir -p ~/.zsh/completions");
                        eprintln!("  locai-cli completions zsh > ~/.zsh/completions/_locai-cli");
                        eprintln!("  echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc");
                        eprintln!("  echo 'autoload -U compinit && compinit' >> ~/.zshrc");
                    }
                }
                args::Shell::Fish => {
                    generate(clap_complete::shells::Fish, &mut cmd, "locai-cli", &mut std::io::stdout());
                    if atty::is(atty::Stream::Stderr) {
                        eprintln!("\n{}", format_info("Fish completion script generated. To install:"));
                        eprintln!("  mkdir -p ~/.config/fish/completions");
                        eprintln!("  locai-cli completions fish > ~/.config/fish/completions/locai-cli.fish");
                        eprintln!("  (Fish automatically loads completions from this directory)");
                    }
                }
                args::Shell::Power => {
                    generate(clap_complete::shells::PowerShell, &mut cmd, "locai-cli", &mut std::io::stdout());
                    if atty::is(atty::Stream::Stderr) {
                        eprintln!("\n{}", format_info("PowerShell completion script generated. To install:"));
                        eprintln!("  locai-cli completions powershell > $PROFILE\\locai-cli.ps1");
                        eprintln!("  Add 'source $PROFILE\\locai-cli.ps1' to your PowerShell profile");
                    }
                }
                args::Shell::Elvish => {
                    generate(clap_complete::shells::Elvish, &mut cmd, "locai-cli", &mut std::io::stdout());
                    if atty::is(atty::Stream::Stderr) {
                        eprintln!("\n{}", format_info("Elvish completion script generated. To install:"));
                        eprintln!("  mkdir -p ~/.config/elvish/lib");
                        eprintln!("  locai-cli completions elvish > ~/.config/elvish/lib/locai-cli.elv");
                        eprintln!("  (Elvish automatically loads completions from this directory)");
                    }
                }
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
