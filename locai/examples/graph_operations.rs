//! Graph operations example for Locai
//!
//! This example demonstrates how to use the graph-centric operations in Locai
//! to create, connect, and traverse memories.

use locai::models::MemoryPriority;
use locai::models::MemoryType;
use locai::prelude::*;
use locai::storage::filters::{MemoryFilter, SortOrder};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize Locai with defaults (simplest approach)
    let memory_manager = init_with_defaults().await?;

    info!("Locai initialized successfully");

    // Add some memories with the simplified API
    let sky_id = memory_manager
        .add_fact("The sky is blue because of Rayleigh scattering.")
        .await?;

    info!("Added fact about the sky, ID: {}", sky_id);

    // Add related memories in one step
    let scattering_id = memory_manager
        .add_related_memory(
            &sky_id,
            "Rayleigh scattering affects blue light more than red light.",
            "explains",
            None,
        )
        .await?;

    info!(
        "Added related fact about Rayleigh scattering, ID: {}",
        scattering_id
    );

    let sun_id = memory_manager
        .add_related_memory(
            &sky_id,
            "The sun appears yellow because blue light is scattered away.",
            "related_to",
            None,
        )
        .await?;

    info!("Added related fact about the sun, ID: {}", sun_id);

    // Create a bidirectional relationship
    memory_manager
        .create_bidirectional_relationship(&scattering_id, &sun_id, "connected_to")
        .await?;

    info!("Created bidirectional relationship between scattering and sun");

    // Get the memory graph centered on the sky memory
    let graph = memory_manager.get_memory_graph(&sky_id, 2).await?;

    info!(
        "Memory graph contains {} memories and {} relationships",
        graph.memories.len(),
        graph.relationships.len()
    );

    // List the memories in the graph
    for (id, memory) in &graph.memories {
        info!("Memory {}: {}", id, memory.content);
    }

    // Find paths between memories
    let paths = memory_manager.find_paths(&sky_id, &sun_id, 3).await?;

    info!("Found {} paths between sky and sun", paths.len());

    for (i, path) in paths.iter().enumerate() {
        info!("Path {} (length {})", i + 1, path.length());

        for (j, memory) in path.memories.iter().enumerate() {
            info!("  Node {}: {}", j + 1, memory.content);
        }

        for (j, rel) in path.relationships.iter().enumerate() {
            info!(
                "  Edge {}: {} ({}) -> {}",
                j + 1,
                rel.source_id,
                rel.relationship_type,
                rel.target_id
            );
        }
    }

    // Find all memories connected to the sky memory via "explains" relationship
    let explained_by = memory_manager
        .find_connected_memories(&sky_id, "explains", 2)
        .await?;

    info!(
        "Found {} memories that explain the sky memory",
        explained_by.len()
    );

    for memory in explained_by {
        info!("Explanation: {}", memory.content);
    }

    // You can still use the more verbose API for advanced cases
    // Create a memory with custom options
    let weather_id = memory_manager
        .add_memory_with_options(
            "Weather patterns are affected by the atmosphere's composition.",
            |builder| {
                builder
                    .memory_type(MemoryType::Fact)
                    .priority(MemoryPriority::High)
                    .tag("weather")
                    .tag("atmosphere")
            },
        )
        .await?;

    info!("Added memory about weather with ID: {}", weather_id);

    // Connect to the sky memory
    memory_manager
        .create_relationship(&sky_id, &weather_id, "related_to")
        .await?;

    // Use filter operations to find all high-priority memories
    // Note: MemoryFilter doesn't have priority field, so we'll filter by type instead
    let mut filter = MemoryFilter::default();
    filter.memory_type = Some("fact".to_string());

    let high_priority = memory_manager
        .filter_memories(
            filter,
            Some("created_at"),
            Some(SortOrder::Descending),
            Some(10),
        )
        .await?;

    info!("Found {} fact memories", high_priority.len());

    Ok(())
}
