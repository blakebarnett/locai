use crate::context::LocaiCliContext;
use locai::LocaiError;
use locai::models::{MemoryPriority, MemoryType};

pub fn parse_memory_type(type_str: &str) -> locai::Result<MemoryType> {
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

pub fn parse_priority(priority_str: &str) -> locai::Result<MemoryPriority> {
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

pub async fn resolve_memory_id(ctx: &LocaiCliContext, id: &str) -> locai::Result<String> {
    if ctx.memory_manager.get_memory(id).await?.is_some() {
        return Ok(id.to_string());
    }

    if id.len() >= 20 {
        return Err(LocaiError::Other(format!("Memory '{}' not found", id)));
    }

    use locai::storage::filters::MemoryFilter;
    let all_memories = ctx
        .memory_manager
        .filter_memories(MemoryFilter::default(), None, None, Some(1000))
        .await?;

    let matches: Vec<_> = all_memories
        .iter()
        .filter(|m| m.id.starts_with(id))
        .collect();

    match matches.len() {
        0 => Err(LocaiError::Other(format!(
            "No memory found with ID prefix '{}'",
            id
        ))),
        1 => Ok(matches[0].id.clone()),
        _ => {
            let suggestions: Vec<String> = matches
                .iter()
                .take(5)
                .map(|m| format!("  - {} ({})", m.id, &m.content[..50.min(m.content.len())]))
                .collect();
            Err(LocaiError::Other(format!(
                "Ambiguous ID prefix '{}': {} matches found.\nSuggestions:\n{}",
                id,
                matches.len(),
                suggestions.join("\n")
            )))
        }
    }
}
