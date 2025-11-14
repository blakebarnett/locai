//! Comprehensive Example: Memory Versioning API
//!
//! This example demonstrates all memory versioning features across all phases:
//!
//! **Phase 1: Core Versioning**
//! - Creating and retrieving versions
//! - Listing versions
//! - Getting current version
//!
//! **Phase 2: Advanced Features**
//! - Computing diffs between versions
//! - Time-based queries (get memory at specific time)
//! - Snapshots (create, restore, search)
//!
//! **Phase 3: Optimization**
//! - Delta storage for efficient version management
//! - Compression for old versions
//! - Versioning statistics and management
//! - Version promotion and compaction

use locai::prelude::*;
use locai::storage::models::RestoreMode;

#[tokio::main]
async fn main() -> Result<()> {
    // Suppress debug logging - only show errors
    // This must be called before Locai::for_testing() to prevent it from initializing logging
    // Filter out surrealdb debug logs and only show errors
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error"))
                .add_directive("surrealdb=error".parse().unwrap())
                .add_directive("surrealdb_core=error".parse().unwrap()),
        )
        .try_init();

    println!("=== Memory Versioning Comprehensive Example ===\n");

    // Initialize Locai
    let locai = Locai::for_testing().await?;

    // ========================================================================
    // PHASE 1: Core Versioning
    // ========================================================================
    println!("## Phase 1: Core Versioning\n");

    // Create a memory
    let memory_id = locai.remember("Initial memory content").await?;
    println!("✓ Created memory: {}", memory_id);

    // Create multiple versions
    println!("\n--- Creating Versions ---");
    let mut version_ids = Vec::new();
    for i in 1..=5 {
        let content = format!("Updated content version {}", i);
        let version_id = locai.remember_version(&memory_id, &content, None).await?;
        version_ids.push(version_id.clone());
        println!("  Created version {}: {}", i, &version_id[..8]);
    }

    // List all versions
    println!("\n--- Listing All Versions ---");
    let versions = locai.list_memory_versions(&memory_id).await?;
    println!("Total versions: {}", versions.len());
    for (i, version) in versions.iter().enumerate() {
        println!(
            "  {}: {} (created: {}, size: {} bytes)",
            i + 1,
            &version.version_id[..8],
            version.created_at.format("%H:%M:%S"),
            version.size_bytes
        );
    }

    // Get a specific version
    println!("\n--- Getting Specific Version ---");
    if let Some(version_id) = version_ids.first() {
        let version = locai
            .get_memory_version(&memory_id, version_id)
            .await?
            .expect("Version should exist");
        println!("Retrieved version: {}", &version_id[..8]);
        println!("Content: {}", version.content);
    }

    // Get current version
    println!("\n--- Getting Current Version ---");
    let current = locai
        .get_memory_current_version(&memory_id)
        .await?
        .expect("Current version should exist");
    println!("Current version content: {}", current.content);

    // ========================================================================
    // PHASE 2: Advanced Features
    // ========================================================================
    println!("\n## Phase 2: Advanced Features\n");

    // Compute diff between versions
    println!("--- Computing Diff Between Versions ---");
    if version_ids.len() >= 2 {
        let old_version_id = &version_ids[0];
        let new_version_id = &version_ids[1];

        let diff = locai
            .diff_memory_versions(&memory_id, old_version_id, new_version_id)
            .await?;

        println!(
            "Diff from {} to {}:",
            &old_version_id[..8],
            &new_version_id[..8]
        );
        println!("  Changes: {}", diff.changes.len());
        for change in &diff.changes {
            match change {
                locai::storage::models::Change::ContentChanged {
                    old_content,
                    new_content,
                    diff_hunks,
                } => {
                    println!("    Content changed:");
                    println!(
                        "      Old: {}...",
                        &old_content[..30.min(old_content.len())]
                    );
                    println!(
                        "      New: {}...",
                        &new_content[..30.min(new_content.len())]
                    );
                    println!("      Diff hunks: {}", diff_hunks.len());
                }
                _ => println!("    Other change"),
            }
        }
    }

    // Time-based query
    println!("\n--- Time-Based Query ---");
    use chrono::Utc;

    // Create a version at a known time
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let time_marker = Utc::now();
    let _time_version_id = locai
        .remember_version(&memory_id, "Content at specific time", None)
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Query memory at that time
    let memory_at_time = locai.get_memory_at_time(&memory_id, time_marker).await?;

    if let Some(mem) = memory_at_time {
        println!(
            "Memory at {}: {}",
            time_marker.format("%H:%M:%S"),
            mem.content
        );
    } else {
        println!("No memory found at that time");
    }

    // Snapshots
    println!("\n--- Snapshots ---");

    // Create another memory for snapshot demo
    let memory2_id = locai.remember("Second memory content").await?;
    locai
        .remember_version(&memory2_id, "Second memory version 2", None)
        .await?;

    // Create snapshot
    let memory_ids = vec![memory_id.clone(), memory2_id.clone()];
    let snapshot = locai.create_snapshot(Some(&memory_ids), None).await?;

    println!("Created snapshot: {}", snapshot.snapshot_id);
    println!("  Memories in snapshot: {}", snapshot.memory_count);
    println!("  Created at: {}", snapshot.created_at.format("%H:%M:%S"));

    // Modify memories after snapshot
    locai
        .remember_version(&memory_id, "Modified after snapshot", None)
        .await?;

    // Search snapshot
    println!("\n--- Searching Snapshot ---");
    let snapshot_results = locai
        .search_snapshot(&snapshot, "specific", Some(10))
        .await?;
    println!("Found {} results in snapshot", snapshot_results.len());

    // Restore snapshot
    println!("\n--- Restoring Snapshot ---");
    let before_restore = locai.get_memory_current_version(&memory_id).await?.unwrap();
    println!("Before restore: {}", before_restore.content);

    locai
        .restore_snapshot(&snapshot, RestoreMode::Overwrite)
        .await?;

    let after_restore = locai.get_memory_current_version(&memory_id).await?.unwrap();
    println!("After restore: {}", after_restore.content);
    println!("✓ Snapshot restored successfully");

    // ========================================================================
    // PHASE 3: Optimization Features
    // ========================================================================
    println!("\n## Phase 3: Optimization Features\n");

    // Create many versions to trigger delta storage (threshold is 10 by default)
    println!("--- Creating Many Versions (Delta Storage) ---");
    for i in 6..=20 {
        let content = format!("Content version {}", i);
        locai.remember_version(&memory_id, &content, None).await?;
    }
    println!("Created 15 more versions (total: 20+)");

    // List versions and show delta storage
    println!("\n--- Version List (showing delta storage) ---");
    let all_versions = locai.list_memory_versions(&memory_id).await?;
    println!("Total versions: {}", all_versions.len());

    let full_copies: Vec<_> = all_versions.iter().filter(|v| !v.is_delta).collect();
    let deltas: Vec<_> = all_versions.iter().filter(|v| v.is_delta).collect();

    println!("  Full copies: {}", full_copies.len());
    println!("  Delta versions: {}", deltas.len());

    if !deltas.is_empty() {
        println!("\n  Example delta version:");
        let delta = &deltas[0];
        println!("    Version ID: {}", &delta.version_id[..8]);
        println!("    Created: {}", delta.created_at.format("%H:%M:%S"));
        println!("    Size: {} bytes", delta.size_bytes);
        println!(
            "    Preview: {}...",
            &delta.content_preview[..50.min(delta.content_preview.len())]
        );
    }

    // Get versioning statistics
    println!("\n--- Versioning Statistics ---");
    let stats = locai.get_versioning_stats(Some(&memory_id)).await?;

    println!("Total versions: {}", stats.total_versions);
    println!("Delta versions: {}", stats.total_delta_versions);
    println!("Full copy versions: {}", stats.total_full_versions);
    println!("Storage size: {} bytes", stats.storage_size_bytes);
    println!("Storage savings: {} bytes", stats.storage_savings_bytes);
    println!("Compressed versions: {}", stats.compressed_versions);
    println!(
        "Average versions per memory: {:.2}",
        stats.average_versions_per_memory
    );

    // Promote a delta version to full copy
    if let Some(delta_version) = deltas.first() {
        println!("\n--- Promoting Delta to Full Copy ---");
        println!("Promoting version: {}", &delta_version.version_id[..8]);

        locai
            .promote_version_to_full_copy(&memory_id, &delta_version.version_id)
            .await?;

        println!("✓ Version promoted successfully");

        // Verify promotion
        let versions_after = locai.list_memory_versions(&memory_id).await?;
        let promoted = versions_after
            .iter()
            .find(|v| v.version_id == delta_version.version_id)
            .unwrap();

        println!("  Is delta: {}", promoted.is_delta);
        assert!(!promoted.is_delta, "Version should no longer be a delta");
    }

    // Validate versions
    println!("\n--- Validating Versions ---");
    let issues = locai.validate_versions(Some(&memory_id)).await?;

    if issues.is_empty() {
        println!("✓ No integrity issues found");
    } else {
        println!("Found {} issues:", issues.len());
        for issue in &issues {
            println!("  - {:?}: {}", issue.issue_type, issue.description);
        }
    }

    // Compact versions (keep only 10 most recent)
    println!("\n--- Compacting Versions ---");
    let versions_before = locai.list_memory_versions(&memory_id).await?;
    println!("Versions before compaction: {}", versions_before.len());

    locai
        .compact_versions(Some(&memory_id), Some(10), None)
        .await?;

    let versions_after = locai.list_memory_versions(&memory_id).await?;
    println!("Versions after compaction: {}", versions_after.len());
    println!("✓ Compaction completed");

    // Repair versions (demonstrates repair API)
    println!("\n--- Repairing Versions ---");
    let repair_report = locai.repair_versions(Some(&memory_id)).await?;
    println!("Versions repaired: {}", repair_report.versions_repaired);
    println!("Versions failed: {}", repair_report.versions_failed);
    if !repair_report.repair_details.is_empty() {
        println!("Repair details:");
        for detail in &repair_report.repair_details {
            println!("  - {}", detail);
        }
    }

    // Global statistics
    println!("\n--- Global Versioning Statistics ---");
    let global_stats = locai.get_versioning_stats(None).await?;
    println!(
        "Total versions across all memories: {}",
        global_stats.total_versions
    );
    println!(
        "Total delta versions: {}",
        global_stats.total_delta_versions
    );
    println!(
        "Total full copy versions: {}",
        global_stats.total_full_versions
    );
    println!(
        "Average versions per memory: {:.2}",
        global_stats.average_versions_per_memory
    );

    // Demonstrate transparent decompression
    println!("\n--- Transparent Decompression ---");
    if let Some(version) = versions_after.first() {
        let retrieved = locai
            .get_memory_version(&memory_id, &version.version_id)
            .await?;

        if let Some(memory) = retrieved {
            println!("Retrieved version: {}", &version.version_id[..8]);
            println!(
                "Content: {}...",
                &memory.content[..50.min(memory.content.len())]
            );
            println!(
                "✓ Version retrieved successfully (compression/decompression handled automatically)"
            );
        }
    }

    println!("\n=== Example Complete ===");
    println!("\nAll memory versioning features demonstrated:");
    println!("  ✓ Phase 1: Core versioning (create, get, list)");
    println!("  ✓ Phase 2: Diffs, time-based queries, snapshots");
    println!("  ✓ Phase 3: Delta storage, compression, statistics, management");

    Ok(())
}
