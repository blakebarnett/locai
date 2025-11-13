//! Tutorial command handlers

use crate::context::LocaiCliContext;
use crate::args::TutorialArgs;
use crate::output::*;
use colored::Colorize;
use locai::LocaiError;
use std::io::{self, Write};

pub async fn handle_tutorial_command(
    args: TutorialArgs,
    ctx: &LocaiCliContext,
    _output_format: &str,
) -> locai::Result<()> {
    if !args.examples_only {
        println!("{}", "━━━ Welcome to Locai Tutorial ━━━".color(CliColors::accent()).bold());
        println!();
        println!("This interactive tutorial will help you learn Locai step by step.");
        println!("You'll create sample data and see how concepts work together.");
        println!();
        print!("Press Enter to continue, or Ctrl+C to exit...");
        io::stdout().flush().map_err(|e| LocaiError::Other(format!("Failed to flush stdout: {}", e)))?;
        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| LocaiError::Other(format!("Failed to read input: {}", e)))?;
        println!();
    }

    let topic = args.topic.to_lowercase();
    match topic.as_str() {
        "memory" | "memories" => run_memory_tutorial(ctx, args.examples_only).await?,
        "entity" | "entities" => run_entity_tutorial(ctx, args.examples_only).await?,
        "relationship" | "relationships" => run_relationship_tutorial(ctx, args.examples_only).await?,
        "graph" => run_graph_tutorial(ctx, args.examples_only).await?,
        "all" => {
            run_memory_tutorial(ctx, args.examples_only).await?;
            if !args.examples_only {
                wait_for_continue()?;
            }
            run_entity_tutorial(ctx, args.examples_only).await?;
            if !args.examples_only {
                wait_for_continue()?;
            }
            run_relationship_tutorial(ctx, args.examples_only).await?;
            if !args.examples_only {
                wait_for_continue()?;
            }
            run_graph_tutorial(ctx, args.examples_only).await?;
        }
        _ => {
            println!("{}", format_error(&format!("Unknown topic: {}. Use: memory, entity, relationship, graph, or all", args.topic)));
            return Ok(());
        }
    }

    if !args.examples_only {
        println!();
        println!("{}", "━━━ Tutorial Complete! ━━━".color(CliColors::accent()).bold());
        println!();
        println!("Next steps:");
        println!("  • Try: locai-cli memory add \"your content\"");
        println!("  • Try: locai-cli memory search \"your query\"");
        println!("  • Explore: locai-cli --help");
    }

    Ok(())
}

fn wait_for_continue() -> locai::Result<()> {
    println!();
    print!("Press Enter to continue...");
    io::stdout().flush().map_err(|e| LocaiError::Other(format!("Failed to flush stdout: {}", e)))?;
    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| LocaiError::Other(format!("Failed to read input: {}", e)))?;
    println!();
    Ok(())
}

async fn run_memory_tutorial(ctx: &LocaiCliContext, examples_only: bool) -> locai::Result<()> {
    println!("{}", "━━━ Lesson 1: What is a Memory? ━━━".color(CliColors::accent()).bold());
    println!();
    println!("A memory is a piece of information stored in Locai. Let's create one:");
    println!();

    let memory_id = ctx
        .memory_manager
        .add_memory_with_options(
            "Locai is a memory management system for AI agents".to_string(),
            |builder| builder.memory_type(locai::models::MemoryType::Fact).priority(locai::models::MemoryPriority::Normal),
        )
        .await?;

    println!("{}", format_success(&format!("✓ Created: {}", memory_id.color(CliColors::accent()))));
    println!("  Content: \"Locai is a memory management system for AI agents\"");
    println!();

    if !examples_only {
        wait_for_continue()?;
    }

    println!("Now let's search for it:");
    println!();

    let results = ctx
        .memory_manager
        .search("memory", Some(5), None, locai::memory::search_extensions::SearchMode::Text)
        .await?;

    if !results.is_empty() {
        println!("{}", format_success(&format!("✓ Found {} memory:", results.len())));
        for (i, result) in results.iter().take(3).enumerate() {
            println!("  {}. {}", i + 1, result.memory.content);
        }
    }

    println!();
    println!("{}", "━━━ Lesson 2: Memory Types ━━━".color(CliColors::accent()).bold());
    println!();
    println!("Memories can have different types. Let's create a few examples:");
    println!();

    let fact_id = ctx
        .memory_manager
        .add_memory_with_options(
            "Water boils at 100°C".to_string(),
            |builder| builder.memory_type(locai::models::MemoryType::Fact),
        )
        .await?;
    println!("{}", format_success(&format!("✓ Created fact memory: {}", fact_id[..8].color(CliColors::accent()))));

    let episodic_id = ctx
        .memory_manager
        .add_memory_with_options(
            "I learned about Locai today".to_string(),
            |builder| builder.memory_type(locai::models::MemoryType::Episodic),
        )
        .await?;
    println!("{}", format_success(&format!("✓ Created episodic memory: {}", episodic_id[..8].color(CliColors::accent()))));

    Ok(())
}

async fn run_entity_tutorial(ctx: &LocaiCliContext, _examples_only: bool) -> locai::Result<()> {
    println!("{}", "━━━ Lesson 3: Entities ━━━".color(CliColors::accent()).bold());
    println!();
    println!("Locai automatically extracts entities from memories. Let's create an entity:");
    println!();

    let entity = locai::storage::models::Entity {
        id: "entity:tutorial:locai".to_string(),
        entity_type: "Organization".to_string(),
        properties: serde_json::json!({"name": "Locai"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created = ctx.memory_manager.create_entity(entity).await?;
    println!("{}", format_success(&format!("✓ Created entity: {}", created.id.color(CliColors::accent()))));
    println!("  Type: Organization");
    println!("  Name: Locai");

    Ok(())
}

async fn run_relationship_tutorial(ctx: &LocaiCliContext, _examples_only: bool) -> locai::Result<()> {
    println!("{}", "━━━ Lesson 4: Relationships ━━━".color(CliColors::accent()).bold());
    println!();
    println!("Relationships connect memories and entities. Let's create one:");
    println!();

    let memories = ctx
        .memory_manager
        .filter_memories(locai::storage::filters::MemoryFilter::default(), None, None, Some(2))
        .await?;

    if memories.len() >= 2 {
        let source_id = &memories[0].id;
        let target_id = &memories[1].id;

        ctx.memory_manager
            .create_relationship(source_id, target_id, "related_to")
            .await?;

        println!("{}", format_success(&format!("✓ Created relationship: {} → related_to → {}", 
            source_id[..8].color(CliColors::accent()),
            target_id[..8].color(CliColors::accent())
        )));
    }

    Ok(())
}

async fn run_graph_tutorial(ctx: &LocaiCliContext, _examples_only: bool) -> locai::Result<()> {
    println!("{}", "━━━ Lesson 5: Graph Visualization ━━━".color(CliColors::accent()).bold());
    println!();
    println!("Let's see how memories connect:");
    println!();

    let memories = ctx
        .memory_manager
        .filter_memories(locai::storage::filters::MemoryFilter::default(), None, None, Some(1))
        .await?;

    if let Some(memory) = memories.first() {
        let graph = ctx.memory_manager.get_memory_graph(&memory.id, 1).await?;
        println!("{}", format_success(&format!("✓ Graph created for memory: {}", memory.id[..8].color(CliColors::accent()))));
        println!("  - {} memories", graph.memories.len());
        println!("  - {} relationships", graph.relationships.len());
    }

    Ok(())
}

