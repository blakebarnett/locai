use colored::*;
use locai::models::{MemoryPriority, MemoryType};
use locai::prelude::Memory;
use locai::storage::models::{Entity, MemoryGraph, MemoryPath, Relationship};
use serde_json::json;

pub struct CliColors;

impl CliColors {
    pub fn success() -> Color {
        Color::TrueColor {
            r: 34,
            g: 197,
            b: 94,
        }
    }

    pub fn error() -> Color {
        Color::TrueColor {
            r: 239,
            g: 68,
            b: 68,
        }
    }

    pub fn warning() -> Color {
        Color::TrueColor {
            r: 245,
            g: 158,
            b: 11,
        }
    }

    pub fn info() -> Color {
        Color::TrueColor {
            r: 59,
            g: 130,
            b: 246,
        }
    }

    pub fn memory_fact() -> Color {
        Color::TrueColor {
            r: 59,
            g: 130,
            b: 246,
        }
    }

    pub fn memory_episodic() -> Color {
        Color::TrueColor {
            r: 34,
            g: 197,
            b: 94,
        }
    }

    pub fn memory_semantic() -> Color {
        Color::TrueColor {
            r: 168,
            g: 85,
            b: 247,
        }
    }

    pub fn entity() -> Color {
        Color::TrueColor {
            r: 245,
            g: 158,
            b: 11,
        }
    }

    pub fn muted() -> Color {
        Color::TrueColor {
            r: 148,
            g: 163,
            b: 184,
        }
    }

    pub fn primary() -> Color {
        Color::White
    }

    pub fn accent() -> Color {
        Color::TrueColor {
            r: 59,
            g: 130,
            b: 246,
        }
    }
}

pub fn output_error(error_msg: &str, output_format: &str) {
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
        tracing::error!("{}", error_msg);
    }
}

/// Output a LocaiError in structured JSON format
pub fn output_error_json(error: &locai::LocaiError, output_format: &str) {
    if output_format == "json" {
        let (code, message, details) = match error {
            locai::LocaiError::Storage(msg) => ("STORAGE_ERROR", msg.clone(), None),
            locai::LocaiError::ML(msg) => ("ML_ERROR", msg.clone(), None),
            locai::LocaiError::Configuration(msg) => ("CONFIGURATION_ERROR", msg.clone(), None),
            locai::LocaiError::Logging(e) => ("LOGGING_ERROR", e.to_string(), None),
            locai::LocaiError::Memory(msg) => ("MEMORY_ERROR", msg.clone(), None),
            locai::LocaiError::Entity(msg) => ("ENTITY_ERROR", msg.clone(), None),
            locai::LocaiError::Relationship(msg) => ("RELATIONSHIP_ERROR", msg.clone(), None),
            locai::LocaiError::Version(msg) => ("VERSION_ERROR", msg.clone(), None),
            locai::LocaiError::MLNotConfigured => (
                "ML_NOT_CONFIGURED",
                error.to_string(),
                Some(json!({
                    "hint": "Initialize with Locai::builder().with_defaults().build().await or use ConfigBuilder::new().with_default_ml()"
                })),
            ),
            locai::LocaiError::StorageNotAccessible { path } => (
                "STORAGE_NOT_ACCESSIBLE",
                error.to_string(),
                Some(json!({
                    "path": path
                })),
            ),
            locai::LocaiError::InvalidEmbeddingModel { model } => (
                "INVALID_EMBEDDING_MODEL",
                error.to_string(),
                Some(json!({
                    "model": model,
                    "hint": "Try using a supported model like 'BAAI/bge-m3'"
                })),
            ),
            locai::LocaiError::Connection(msg) => ("CONNECTION_ERROR", msg.clone(), None),
            locai::LocaiError::Authentication(msg) => ("AUTHENTICATION_ERROR", msg.clone(), None),
            locai::LocaiError::Protocol(msg) => ("PROTOCOL_ERROR", msg.clone(), None),
            locai::LocaiError::Timeout(msg) => ("TIMEOUT_ERROR", msg.clone(), None),
            locai::LocaiError::EmptySearchQuery => ("EMPTY_SEARCH_QUERY", error.to_string(), None),
            locai::LocaiError::NoMemoriesFound => ("NO_MEMORIES_FOUND", error.to_string(), None),
            locai::LocaiError::FeatureNotEnabled { feature } => (
                "FEATURE_NOT_ENABLED",
                error.to_string(),
                Some(json!({
                    "feature": feature
                })),
            ),
            locai::LocaiError::Other(msg) => ("OTHER_ERROR", msg.clone(), None),
        };

        let mut error_response = json!({
            "error": true,
            "code": code,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        if let Some(details) = details {
            error_response["details"] = details;
        }

        eprintln!(
            "{}",
            serde_json::to_string_pretty(&error_response).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        eprintln!("{}", format_error(&error.to_string()));
    }
}

pub fn format_success(msg: &str) -> String {
    format!(
        "{} {}",
        "✓".color(CliColors::success()).bold(),
        msg.color(CliColors::success())
    )
}

pub fn format_error(msg: &str) -> String {
    format!(
        "{} {}",
        "✗".color(CliColors::error()).bold(),
        msg.color(CliColors::error())
    )
}

pub fn format_warning(msg: &str) -> String {
    format!(
        "{} {}",
        "⚠".color(CliColors::warning()).bold(),
        msg.color(CliColors::warning())
    )
}

pub fn format_info(msg: &str) -> String {
    format!(
        "{} {}",
        "ℹ".color(CliColors::info()).bold(),
        msg.color(CliColors::info())
    )
}

pub fn format_memory_type(memory_type: &MemoryType) -> ColoredString {
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

pub fn format_priority(priority: &MemoryPriority) -> ColoredString {
    match priority {
        MemoryPriority::Critical => "Critical".color(CliColors::error()).bold(),
        MemoryPriority::High => "High".color(CliColors::warning()),
        MemoryPriority::Normal => "Normal".color(CliColors::muted()),
        MemoryPriority::Low => "Low".color(CliColors::muted()).dimmed(),
    }
}

pub fn print_memory(memory: &Memory) {
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

pub fn print_memory_list(memories: &[Memory]) {
    if memories.is_empty() {
        println!("{}", format_info("No memories found."));
        return;
    }

    println!(
        "{}",
        format_info(&format!("Found {} memories:", memories.len()))
    );
    println!();

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

pub async fn print_connected_memories_tree(
    source_id: &str,
    graph: &MemoryGraph,
    _exclude_temporal: bool,
) -> locai::Result<()> {
    use locai::LocaiError;

    if graph.memories.is_empty() {
        println!("{}", format_info("No memories found."));
        return Ok(());
    }

    let source_memory = graph.memories.get(source_id).ok_or_else(|| {
        LocaiError::Other(format!("Source memory {} not found in graph", source_id))
    })?;

    let connected_count = graph.memories.len().saturating_sub(1);
    if connected_count > 0 {
        println!(
            "{}",
            format_info(&format!("Found {} connected memories:", connected_count))
        );
        println!();
    } else {
        println!("{}", format_info("No connected memories found."));
        println!();
    }

    let mut adjacency: std::collections::HashMap<String, Vec<(String, String, bool)>> =
        std::collections::HashMap::new();

    for rel in &graph.relationships {
        let source_is_memory = graph.memories.contains_key(&rel.source_id);
        let target_is_memory = graph.memories.contains_key(&rel.target_id);

        if source_is_memory && target_is_memory {
            adjacency.entry(rel.source_id.clone()).or_default().push((
                rel.target_id.clone(),
                rel.relationship_type.clone(),
                true,
            ));
        }
    }

    let mut visited = std::collections::HashSet::new();
    visited.insert(source_id.to_string());

    print_tree_node(
        source_id,
        source_memory,
        &adjacency,
        &graph.memories,
        &mut visited,
        "",
        true,
        true,
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn print_tree_node(
    memory_id: &str,
    memory: &Memory,
    adjacency: &std::collections::HashMap<String, Vec<(String, String, bool)>>,
    all_memories: &std::collections::HashMap<String, Memory>,
    visited: &mut std::collections::HashSet<String>,
    prefix: &str,
    is_last: bool,
    is_root: bool,
) {
    let content_preview = if memory.content.len() > 50 {
        format!("{}...", &memory.content[..47])
    } else {
        memory.content.clone()
    };

    if is_root {
        println!(
            "{} {} [{}] {}",
            "●".color(CliColors::accent()).bold(),
            memory_id[..8].color(CliColors::accent()).bold(),
            format_memory_type(&memory.memory_type),
            content_preview.color(CliColors::primary())
        );
    } else {
        let connector = if is_last { "└──" } else { "├──" };
        println!(
            "{}{} {} [{}] {}",
            prefix.color(CliColors::muted()),
            connector.color(CliColors::muted()),
            memory_id[..8].color(CliColors::accent()),
            format_memory_type(&memory.memory_type),
            content_preview.color(CliColors::primary())
        );
    }

    if let Some(children) = adjacency.get(memory_id) {
        let child_count = children.len();
        for (idx, (child_id, rel_type, is_direct)) in children.iter().enumerate() {
            if !all_memories.contains_key(child_id) {
                continue;
            }

            let child_memory = &all_memories[child_id];
            let is_last_child = idx == child_count - 1;

            let new_prefix = if is_root {
                "".to_string()
            } else if is_last {
                format!("{}   ", prefix)
            } else {
                format!("{}│  ", prefix)
            };

            let rel_display = if *is_direct {
                if rel_type == "temporal_sequence" {
                    let source_mem = all_memories.get(memory_id);
                    let target_mem = all_memories.get(child_id);

                    if let (Some(source), Some(target)) = (source_mem, target_mem) {
                        let is_later = target.created_at > source.created_at;
                        let direction_str = if is_later { "later" } else { "earlier" };
                        let colored_direction = if is_later {
                            direction_str.color(CliColors::info())
                        } else {
                            direction_str.color(CliColors::muted())
                        };
                        format!("[temporal: {}]", colored_direction)
                    } else {
                        "[temporal]".color(CliColors::info()).to_string()
                    }
                } else {
                    format!("[{}]", rel_type)
                        .color(CliColors::info())
                        .to_string()
                }
            } else {
                "[via entities]"
                    .color(CliColors::muted())
                    .dimmed()
                    .to_string()
            };

            println!(
                "{}{} {}",
                new_prefix.color(CliColors::muted()),
                if is_last_child { "└─" } else { "├─" },
                rel_display
            );

            if !visited.contains(child_id) {
                visited.insert(child_id.clone());
                print_tree_node(
                    child_id,
                    child_memory,
                    adjacency,
                    all_memories,
                    visited,
                    &new_prefix,
                    is_last_child,
                    false,
                );
                visited.remove(child_id);
            } else {
                let connector = if is_last_child { "└─" } else { "├─" };
                println!(
                    "{}{} {} {}",
                    new_prefix.color(CliColors::muted()),
                    connector.color(CliColors::muted()),
                    "↻".color(CliColors::warning()),
                    format!("(cycle: {})", &child_id[..8])
                        .color(CliColors::muted())
                        .dimmed()
                );
            }
        }
    }
}

pub fn print_entity(entity: &Entity) {
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
    if entity.properties != serde_json::Value::Null {
        println!(
            "{}: {}",
            "Properties".color(CliColors::muted()),
            serde_json::to_string_pretty(&entity.properties)
                .unwrap_or_default()
                .color(CliColors::primary())
        );
    }
}

pub fn print_entity_list(entities: &[Entity]) {
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

pub fn print_relationship(relationship: &Relationship) {
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
    if relationship.properties != serde_json::Value::Null {
        println!(
            "{}: {}",
            "Properties".color(CliColors::muted()),
            serde_json::to_string_pretty(&relationship.properties)
                .unwrap_or_default()
                .color(CliColors::primary())
        );
    }
}

pub fn print_relationship_list(relationships: &[Relationship]) {
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

pub fn print_memory_graph(graph: &MemoryGraph) {
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

pub fn print_paths(paths: &[MemoryPath]) {
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
