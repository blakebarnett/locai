# Memory Versioning

**Status**: Implemented (Phase 1, 2 & 3 Complete)  
**Last Updated**: December 2024

---

## Overview

Memory versioning enables tracking the evolution of memories over time. Each memory can have multiple versions, allowing you to:

- Track changes to memory content
- Retrieve historical states of memories
- Compare different versions
- Create snapshots of memory state
- Efficiently store versions using delta compression

---

## Core Concepts

### Versions

Each version represents a point-in-time snapshot of a memory's content. Versions are:

- **Immutable**: Once created, a version cannot be modified
- **Linked**: Versions form a chain, with each version referencing its parent
- **Timestamped**: Each version has a `created_at` timestamp
- **Identified**: Each version has a unique `version_id` (UUID)

### Storage Strategies

The system uses a hybrid storage approach:

1. **Full Copies**: Recent versions (default: 10 most recent) are stored as complete content
2. **Delta Storage**: Older versions are stored as diffs (changes) relative to a base version
3. **Compression**: Very old versions (default: 30+ days) are automatically compressed

This balances storage efficiency with read performance.

### Delta Reconstruction

When a delta-stored version is accessed, the system:

1. Finds the nearest full-copy version (base)
2. Loads all deltas from base to target version
3. Applies deltas sequentially to reconstruct the full content
4. Caches the reconstructed version for faster subsequent access

### Snapshots

Snapshots capture the state of multiple memories at a specific point in time. They store:

- A map of memory IDs to their version IDs at snapshot time
- Metadata about the snapshot
- Timestamp of creation

Snapshots enable:
- Point-in-time restoration
- Historical queries across multiple memories
- Backup and recovery workflows

---

## API Reference

### Version Management

#### Create a Version

```rust
let version_id = locai.remember_version(
    &memory_id,
    "New content for this version",
    Some(&metadata), // Optional metadata
).await?;
```

Creates a new version of an existing memory. Returns the `version_id`.

#### Get a Specific Version

```rust
let version = locai.get_memory_version(&memory_id, &version_id).await?;
```

Retrieves a specific version by its ID. Returns a `Memory` object with the version's content.

#### Get Current Version

```rust
let current = locai.get_memory_current_version(&memory_id).await?;
```

Returns the latest version of a memory.

#### List All Versions

```rust
let versions = locai.list_memory_versions(&memory_id).await?;
```

Returns a list of `MemoryVersionInfo` objects, ordered by creation time. Each includes:
- `version_id`: Unique identifier
- `created_at`: Timestamp
- `content_preview`: Preview of content (or "[Delta from ...]" for delta versions)
- `size_bytes`: Storage size
- `is_delta`: Whether stored as delta
- `parent_version_id`: Parent version reference

#### Delete Versions

```rust
// Delete a specific version
locai.delete_memory_version(&memory_id, Some(&version_id)).await?;

// Delete all versions
locai.delete_memory_version(&memory_id, None).await?;
```

### Time-Based Queries

#### Get Memory at Time

```rust
use chrono::{DateTime, Utc};

let at_time = DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")?;
let memory = locai.get_memory_at_time(&memory_id, at_time.with_timezone(&Utc)).await?;
```

Returns the memory as it existed at the specified time. The system:
- Finds the version that was current at that time
- Handles edge cases (before first version, after last version)
- Uses tolerance for versions created shortly after query time

### Diff Operations

#### Compute Diff Between Versions

```rust
let diff = locai.diff_memory_versions(
    &memory_id,
    &old_version_id,
    &new_version_id,
).await?;
```

Returns a `MemoryDiff` containing:
- `changes`: List of changes (content changes, metadata changes)
- `diff_hunks`: Structured diff hunks with line-by-line changes
- `diff_type`: Whether it's a full diff or delta

The diff uses the Myers algorithm (via `similar` crate) for efficient computation.

### Snapshot Operations

#### Create Snapshot

```rust
use std::collections::HashMap;
use serde_json::Value;

let metadata = HashMap::from([
    ("description".to_string(), Value::String("Backup before major changes".to_string())),
]);

// Snapshot all memories
let snapshot = locai.create_snapshot(None, Some(&metadata)).await?;

// Snapshot specific memories
let memory_ids = vec!["memory_1".to_string(), "memory_2".to_string()];
let snapshot = locai.create_snapshot(Some(&memory_ids), None).await?;
```

Creates a snapshot of the current state. Returns a `MemorySnapshot` with:
- `snapshot_id`: Unique identifier
- `created_at`: Timestamp
- `memory_count`: Number of memories in snapshot
- `memory_ids`: List of memory IDs
- `version_map`: Mapping of memory_id → version_id
- `metadata`: Custom metadata

#### Restore from Snapshot

```rust
use locai::storage::models::RestoreMode;

// Overwrite existing memories
locai.restore_snapshot(&snapshot, RestoreMode::Overwrite).await?;

// Skip memories that already exist
locai.restore_snapshot(&snapshot, RestoreMode::SkipExisting).await?;

// Create new versions instead of overwriting
locai.restore_snapshot(&snapshot, RestoreMode::CreateVersions).await?;
```

Restores memories from a snapshot. Three restore modes:
- **Overwrite**: Replace current memories with snapshot state
- **SkipExisting**: Only restore memories that don't exist
- **CreateVersions**: Create new versions instead of overwriting

#### Search Snapshot

```rust
let results = locai.search_snapshot(&snapshot, "query text", Some(10)).await?;
```

Searches memories as they existed in the snapshot state.

#### Get Memory from Snapshot

```rust
let memory = locai.get_memory_from_snapshot(&snapshot, &memory_id).await?;
```

Retrieves a specific memory from a snapshot.

### Management APIs

#### Get Versioning Statistics

```rust
// Statistics for all memories
let stats = locai.get_versioning_stats(None).await?;

// Statistics for a specific memory
let stats = locai.get_versioning_stats(Some(&memory_id)).await?;
```

Returns `VersioningStats` with:
- `total_versions`: Total number of versions
- `total_delta_versions`: Number of delta-stored versions
- `total_full_versions`: Number of full-copy versions
- `storage_size_bytes`: Total storage used
- `storage_savings_bytes`: Estimated savings from delta storage
- `compressed_versions`: Number of compressed versions
- `average_versions_per_memory`: Average version count

#### Compact Versions

```rust
// Keep only the 10 most recent versions
locai.compact_versions(Some(&memory_id), Some(10), None).await?;

// Remove versions older than 90 days
locai.compact_versions(None, None, Some(90)).await?;

// Remove old versions, keeping 5 most recent
locai.compact_versions(Some(&memory_id), Some(5), Some(30)).await?;
```

Removes old versions based on retention policies:
- `memory_id`: Optional memory ID (None = all memories)
- `keep_count`: Optional number of recent versions to keep
- `older_than_days`: Optional age threshold

#### Validate Versions

```rust
// Validate all versions
let issues = locai.validate_versions(None).await?;

// Validate versions for a specific memory
let issues = locai.validate_versions(Some(&memory_id)).await?;
```

Checks version integrity and returns a list of issues:
- Missing parent versions
- Broken delta chains
- Self-referencing versions (cycles)

#### Repair Versions

```rust
// Repair all versions
let report = locai.repair_versions(None).await?;

// Repair versions for a specific memory
let report = locai.repair_versions(Some(&memory_id)).await?;
```

Attempts to repair corrupted versions. Returns a `RepairReport` with:
- `versions_repaired`: Number successfully repaired
- `versions_failed`: Number that couldn't be repaired
- `repair_details`: List of repair actions taken

Common repairs:
- Promoting delta versions with missing parents to full copies
- Fixing broken delta chains

#### Promote Version to Full Copy

```rust
locai.promote_version_to_full_copy(&memory_id, &version_id).await?;
```

Converts a delta-stored version to a full copy. Useful for:
- Frequently accessed versions (improves read performance)
- Repairing broken delta chains
- Manual optimization

---

## Configuration

Versioning behavior is controlled by `VersioningConfig`:

```rust
pub struct VersioningConfig {
    // Delta storage
    pub delta_threshold: usize,              // Keep N most recent as full copies (default: 10)
    pub max_delta_chain_length: usize,      // Max deltas before promotion (default: 100)
    
    // Promotion
    pub enable_auto_promotion: bool,         // Auto-promote based on access patterns
    pub promotion_access_threshold: u32,    // Promote after N accesses (default: 5)
    pub promotion_time_window_hours: u64,    // Within time window (default: 24)
    pub promotion_cost_threshold_ms: u64,    // Promote if slower than this (default: 50)
    
    // Caching
    pub enable_reconstruction_cache: bool,   // Cache reconstructed versions
    pub cache_size: usize,                  // Max cached versions (default: 1000)
    pub cache_ttl_seconds: u64,             // Cache expiration (default: 3600)
    pub cache_strategy: CacheStrategy,      // Auto, Server, or Embedded
    
    // Compression
    pub enable_compression: bool,           // Enable compression (default: true)
    pub compression_threshold_days: u64,    // Compress versions older than N days (default: 30)
}
```

---

## Usage Examples

### Basic Versioning

```rust
use locai::prelude::*;

let locai = Locai::for_testing().await?;

// Create a memory
let memory_id = locai.remember("Initial content").await?;

// Create versions
let v1_id = locai.remember_version(&memory_id, "Updated content v1", None).await?;
let v2_id = locai.remember_version(&memory_id, "Updated content v2", None).await?;

// List versions
let versions = locai.list_memory_versions(&memory_id).await?;
println!("Total versions: {}", versions.len());

// Get a specific version
let v1 = locai.get_memory_version(&memory_id, &v1_id).await?;
println!("Version 1 content: {}", v1.content);

// Get current version
let current = locai.get_memory_current_version(&memory_id).await?;
println!("Current content: {}", current.content);
```

### Time Travel

```rust
use chrono::{DateTime, Utc};

// Record a timestamp
let before_update = Utc::now();

// Make changes
locai.remember_version(&memory_id, "New content", None).await?;

// Get memory as it was before the update
let old_memory = locai.get_memory_at_time(&memory_id, before_update).await?;
println!("Old content: {}", old_memory.content);
```

### Comparing Versions

```rust
// Get diff between versions
let diff = locai.diff_memory_versions(&memory_id, &old_version_id, &new_version_id).await?;

for change in diff.changes {
    match change {
        Change::ContentChanged { old_content, new_content, diff_hunks } => {
            println!("Content changed:");
            for hunk in diff_hunks {
                println!("  Lines {}:{}", hunk.old_start_line, hunk.new_start_line);
                for line in hunk.lines {
                    match line {
                        DiffLine::Removed(s) => println!("  - {}", s),
                        DiffLine::Added(s) => println!("  + {}", s),
                        DiffLine::Context(s) => println!("    {}", s),
                    }
                }
            }
        }
        _ => {}
    }
}
```

### Snapshots

```rust
// Create a snapshot before making changes
let snapshot = locai.create_snapshot(None, None).await?;
println!("Created snapshot: {}", snapshot.snapshot_id);

// Make changes
locai.remember_version(&memory_id, "Changed content", None).await?;

// Restore from snapshot
locai.restore_snapshot(&snapshot, RestoreMode::Overwrite).await?;

// Search snapshot state
let results = locai.search_snapshot(&snapshot, "query", Some(10)).await?;
```

### Management

```rust
// Get statistics
let stats = locai.get_versioning_stats(None).await?;
println!("Total versions: {}", stats.total_versions);
println!("Storage used: {} bytes", stats.storage_size_bytes);
println!("Storage saved: {} bytes", stats.storage_savings_bytes);

// Validate integrity
let issues = locai.validate_versions(None).await?;
if !issues.is_empty() {
    println!("Found {} integrity issues", issues.len());
    let report = locai.repair_versions(None).await?;
    println!("Repaired: {}, Failed: {}", report.versions_repaired, report.versions_failed);
}

// Compact old versions
let deleted = locai.compact_versions(None, Some(10), Some(90)).await?;
println!("Deleted {} old versions", deleted);
```

---

## Performance Characteristics

### Version Creation

- **Full copy**: ~1-5ms (direct database write)
- **Delta**: ~2-8ms (compute diff + write)

### Version Retrieval

- **Full copy**: ~1-2ms (direct database read)
- **Delta (cached)**: ~0.1-1ms (cache hit)
- **Delta (uncached)**: ~10-50ms (reconstruction overhead)

### Storage Efficiency

- **Full copy**: Stores complete content
- **Delta**: Typically 50-90% storage reduction
- **Compression**: Additional 30-50% reduction for old versions

### Scalability

- Supports 1000+ versions per memory
- Supports 1M+ total versions
- Supports snapshots with 10K+ memories
- Handles 100+ concurrent version operations

---

## Best Practices

1. **Version Creation**: Create versions when content meaningfully changes, not on every update
2. **Delta Threshold**: Adjust `delta_threshold` based on access patterns (higher for frequently accessed memories)
3. **Compression**: Use compression for long-term archival (30+ days old)
4. **Snapshots**: Create snapshots before major changes or deployments
5. **Validation**: Regularly validate version integrity, especially after bulk operations
6. **Compaction**: Use compaction to manage storage growth while preserving recent history

---

## Limitations

1. **Concurrent Writes**: Race conditions in version creation may result in slightly more full copies than intended (doesn't affect correctness)
2. **Delta Chain Length**: Very long delta chains (>100) may have slower reconstruction
3. **Snapshot Size**: Very large snapshots (>10K memories) may take longer to create/restore
4. **Search**: Standard `search()` only queries current versions; use `search_snapshot()` for historical queries

---

## Related Documentation

- [Example: Comprehensive Memory Versioning](../locai/examples/memory_versioning.rs) - Complete example showcasing all features
- [Storage Traits](../locai/src/storage/traits.rs) - Trait definitions
- [Versioning Implementation](../locai/src/storage/shared_storage/memory_version.rs) - Implementation details

