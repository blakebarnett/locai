//! Concept explanations for the --explain flag

use crate::output::*;
use colored::Colorize;

pub fn show_explanation(concept: &str) -> locai::Result<()> {
    match concept.to_lowercase().as_str() {
        "memory" | "memories" => show_memory_explanation(),
        "entity" | "entities" => show_entity_explanation(),
        "relationship" | "relationships" => show_relationship_explanation(),
        "graph" => show_graph_explanation(),
        "search" => show_search_explanation(),
        "batch" => show_batch_explanation(),
        _ => Err(locai::LocaiError::Other(format!(
            "Unknown concept: {}. Available concepts: memory, entity, relationship, graph, search, batch",
            concept
        ))),
    }
}

fn show_memory_explanation() -> locai::Result<()> {
    println!("{}", "━━━ Memory Concept ━━━".color(CliColors::accent()).bold());
    println!();
    println!("{}", "What is a Memory?".white().bold());
    println!();
    println!("A memory is a piece of information stored in Locai. It can represent:");
    println!("  • Facts - Objective information (e.g., 'Paris is the capital of France')");
    println!("  • Episodes - Specific events or experiences");
    println!("  • Conversations - Dialogues or exchanges");
    println!("  • Procedural knowledge - How to do something");
    println!();
    println!("{}", "Key Features:".bold());
    println!("  • Each memory has a unique ID");
    println!("  • Can be tagged for organization");
    println!("  • Has a priority level (Critical, High, Normal, Low)");
    println!("  • Can be linked to other memories via relationships");
    println!("  • Supports semantic search");
    println!();
    println!("{}", "Common Commands:".bold());
    println!("  locai-cli memory add \"Content\"     # Create a new memory");
    println!("  locai-cli memory search \"query\"    # Search memories semantically");
    println!("  locai-cli memory list               # List all memories");
    println!("  locai-cli memory get <id>           # Get a specific memory");
    println!();
    Ok(())
}

fn show_entity_explanation() -> locai::Result<()> {
    println!("{}", "━━━ Entity Concept ━━━".color(CliColors::entity()).bold());
    println!();
    println!("{}", "What is an Entity?".bold());
    println!();
    println!("An entity represents a real-world object, person, place, or concept that");
    println!("can be extracted from memories. Examples:");
    println!("  • People: 'Alice', 'Bob'");
    println!("  • Places: 'Paris', 'New York'");
    println!("  • Organizations: 'Acme Corp', 'MIT'");
    println!("  • Concepts: 'Machine Learning', 'Quantum Physics'");
    println!();
    println!("{}", "Key Features:".bold());
    println!("  • Automatically extracted from memory content");
    println!("  • Can have custom properties");
    println!("  • Can be linked to other entities via relationships");
    println!("  • Helps connect related memories");
    println!();
    println!("{}", "Common Commands:".bold());
    println!("  locai-cli entity list               # List all entities");
    println!("  locai-cli entity get <id>           # Get a specific entity");
    println!("  locai-cli entity search \"query\"     # Search entities");
    println!();
    Ok(())
}

fn show_relationship_explanation() -> locai::Result<()> {
    println!("{}", "━━━ Relationship Concept ━━━".color(CliColors::info()).bold());
    println!();
    println!("{}", "What is a Relationship?".bold());
    println!();
    println!("A relationship connects two memories or entities, creating a graph structure.");
    println!("Relationships can be:");
    println!("  • Memory-to-Memory: Direct connections between memories");
    println!("  • Entity-to-Entity: Connections between entities");
    println!("  • Memory-to-Entity: Memories containing entities");
    println!();
    println!("{}", "Common Relationship Types:".bold());
    println!("  • related_to - General connection");
    println!("  • temporal_sequence - Time-based ordering");
    println!("  • causes - Causal relationship");
    println!("  • contains - Containment relationship");
    println!("  • Custom types - You can define your own");
    println!();
    println!("{}", "Key Features:".bold());
    println!("  • Relationships enable graph traversal");
    println!("  • Can have metadata/properties");
    println!("  • Support bidirectional and symmetric relationships");
    println!("  • Enable finding paths between memories");
    println!();
    println!("{}", "Common Commands:".bold());
    println!("  locai-cli relationship create <source> <target> <type>");
    println!("  locai-cli relationship list         # List all relationships");
    println!("  locai-cli relationship delete <id>  # Delete a relationship");
    println!();
    Ok(())
}

fn show_graph_explanation() -> locai::Result<()> {
    println!("{}", "━━━ Graph Concept ━━━".color(CliColors::accent()).bold());
    println!();
    println!("{}", "What is the Graph?".bold());
    println!();
    println!("The graph is the network of memories and entities connected by relationships.");
    println!("It enables powerful operations like:");
    println!("  • Finding connected memories");
    println!("  • Discovering paths between memories");
    println!("  • Identifying central/important memories");
    println!("  • Analyzing graph structure and metrics");
    println!();
    println!("{}", "Graph Operations:".bold());
    println!("  • Subgraph - Get memories connected to a specific memory");
    println!("  • Paths - Find paths between two memories");
    println!("  • Metrics - Analyze graph statistics");
    println!("  • Query - Search for patterns (connected, isolated, etc.)");
    println!("  • Central - Find most important/central memories");
    println!();
    println!("{}", "Common Commands:".bold());
    println!("  locai-cli graph subgraph <id>       # Get connected memories");
    println!("  locai-cli graph paths <id1> <id2>   # Find paths between memories");
    println!("  locai-cli graph metrics              # View graph statistics");
    println!("  locai-cli graph query \"connected\"   # Query graph patterns");
    println!();
    Ok(())
}

fn show_search_explanation() -> locai::Result<()> {
    println!("{}", "━━━ Search Concept ━━━".color(CliColors::info()).bold());
    println!();
    println!("{}", "How Does Search Work?".bold());
    println!();
    println!("Locai supports semantic search, which understands the meaning of your");
    println!("query, not just exact keyword matches.");
    println!();
    println!("{}", "Search Types:".bold());
    println!("  • Semantic - Finds memories by meaning (default)");
    println!("  • Keyword - Exact text matching");
    println!("  • Hybrid - Combines semantic and keyword search");
    println!();
    println!("{}", "Search Features:".bold());
    println!("  • Understands synonyms and related concepts");
    println!("  • Ranks results by relevance");
    println!("  • Can filter by type, priority, tags");
    println!("  • Supports time-based filtering");
    println!();
    println!("{}", "Common Commands:".bold());
    println!("  locai-cli memory search \"query\"     # Semantic search");
    println!("  locai-cli memory search \"query\" --type fact");
    println!("  locai-cli memory search \"query\" --tags tag1,tag2");
    println!();
    Ok(())
}

fn show_batch_explanation() -> locai::Result<()> {
    println!("{}", "━━━ Batch Operations Concept ━━━".color(CliColors::accent()).bold());
    println!();
    println!("{}", "What are Batch Operations?".bold());
    println!();
    println!("Batch operations allow you to perform multiple Locai operations in a");
    println!("single request, optionally as a transaction.");
    println!();
    println!("{}", "Key Features:".bold());
    println!("  • Execute multiple operations atomically");
    println!("  • All-or-nothing transaction support");
    println!("  • Efficient for bulk imports/exports");
    println!("  • Progress tracking for large batches");
    println!();
    println!("{}", "Operation Types:".bold());
    println!("  • create_memory - Add new memories");
    println!("  • update_memory - Update existing memories");
    println!("  • delete_memory - Remove memories");
    println!("  • create_relationship - Add relationships");
    println!("  • create_entity - Add entities");
    println!();
    println!("{}", "Common Commands:".bold());
    println!("  locai-cli batch execute batch.json   # Execute batch file");
    println!("  locai-cli batch execute batch.json --transaction");
    println!();
    println!("{}", "Example Batch File:".bold());
    println!("  {{");
    println!("    \"operations\": [");
    println!("      {{ \"operation\": \"create_memory\", \"content\": \"Memory 1\" }},");
    println!("      {{ \"operation\": \"create_memory\", \"content\": \"Memory 2\" }}");
    println!("    ]");
    println!("  }}");
    println!();
    Ok(())
}

