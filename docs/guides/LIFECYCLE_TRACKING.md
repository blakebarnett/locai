# Memory Lifecycle Tracking

> **Status**: Implemented in Locai 0.2.0+  
> **Stability**: Stable  
> **Configuration**: `lifecycle_tracking` in `locai.yaml`

## Overview

Memory Lifecycle Tracking automatically updates memory metadata (`access_count`, `last_accessed`) when memories are retrieved or accessed. This enables:

- **Forgetting curves**: Implement time-based memory decay algorithms
- **Importance tracking**: Calculate memory importance based on frequency and recency
- **Analytics**: Understand which memories are accessed and when
- **Optimization**: Identify frequently-accessed vs. stale memories

## Key Features

### Automatic Tracking
Every time a memory is retrieved via `get_memory()`, its `access_count` is incremented and `last_accessed` is updated to the current time.

```rust
// Automatically updates access_count and last_accessed
let memory = storage.get_memory("memory_123").await?;
println!("Access count: {}", memory.access_count);
println!("Last accessed: {}", memory.last_accessed);
```

### Flexible Configuration
Fine-grained control over what operations trigger tracking:

```yaml
memory:
  lifecycle_tracking:
    enabled: true                 # Master switch
    update_on_get: true          # Track memory retrieval
    update_on_search: false      # Don't count searches as accesses
    update_on_list: false        # Don't count browsing as accesses
    blocking: false              # Don't block on updates
    batched: true                # Batch updates for performance
    flush_interval_secs: 60      # Flush batched updates every 60s
    flush_threshold_count: 100   # Or when 100 updates are pending
```

### Three Update Modes

#### 1. **Batched (Default, Best Performance)**
```yaml
lifecycle_tracking:
  batched: true
  flush_interval_secs: 60
  flush_threshold_count: 100
```

Updates are aggregated in memory and flushed in batches:
- ✅ Minimal write load on database
- ✅ Excellent performance (no blocking)
- ✅ Slight delay in persistence (up to 60 seconds)
- ✅ Updates for same memory are merged

**Use case**: Most applications, default configuration

#### 2. **Non-Blocking Async**
```yaml
lifecycle_tracking:
  batched: false
  blocking: false
```

Updates are fired-and-forgotten asynchronously:
- ✅ Immediate return from `get_memory()`
- ✅ Updates applied in background
- ⚠️ Slightly higher I/O load than batching
- ⚠️ No guarantee of persistence

**Use case**: Real-time requirements, high-performance scenarios

#### 3. **Blocking (Strict Consistency)**
```yaml
lifecycle_tracking:
  batched: false
  blocking: true
```

Updates are applied immediately and synchronously:
- ✅ Strong consistency guarantee
- ✅ Predictable behavior
- ❌ Blocks `get_memory()` call for each update
- ❌ Highest I/O load

**Use case**: Critical systems requiring strict tracking, testing

## Configuration Examples

### 1. Default Configuration (Recommended)
Batched updates with reasonable defaults:

```yaml
memory:
  lifecycle_tracking:
    enabled: true
    update_on_get: true
    update_on_search: false
    update_on_list: false
    blocking: false
    batched: true
    flush_interval_secs: 60
    flush_threshold_count: 100
```

### 2. High-Performance (Minimal Overhead)
Async non-blocking updates for performance-critical applications:

```yaml
memory:
  lifecycle_tracking:
    enabled: true
    update_on_get: true
    update_on_search: false
    update_on_list: false
    blocking: false
    batched: false
```

### 3. Track Everything
Include search and list operations in tracking:

```yaml
memory:
  lifecycle_tracking:
    enabled: true
    update_on_get: true
    update_on_search: true    # Count searches as accesses
    update_on_list: true      # Count browsing as accesses
    blocking: false
    batched: true
    flush_interval_secs: 30   # More frequent flushes
    flush_threshold_count: 50 # Lower threshold
```

### 4. Disable Tracking
Turn off lifecycle tracking completely:

```yaml
memory:
  lifecycle_tracking:
    enabled: false
```

## API Usage

### Accessing Lifecycle Data

```rust
use locai::storage::traits::MemoryStore;

let memory = storage.get_memory("memory_123").await?;

// Access lifecycle information
println!("Created: {}", memory.created_at);
println!("Last accessed: {:?}", memory.last_accessed);
println!("Access count: {}", memory.access_count);
```

### Calculating Memory Importance

```rust
use chrono::Utc;

fn calculate_importance(memory: &Memory) -> f32 {
    let access_weight = (memory.access_count as f32) / 100.0; // 0.0-1.0
    
    let recency = if let Some(last_accessed) = memory.last_accessed {
        let age_days = (Utc::now() - last_accessed).num_days() as f32;
        1.0 / (1.0 + age_days) // Decay over days
    } else {
        0.5
    };
    
    let priority_weight = match memory.priority {
        MemoryPriority::Low => 0.5,
        MemoryPriority::Normal => 1.0,
        MemoryPriority::High => 2.0,
        MemoryPriority::Critical => 3.0,
    };
    
    (access_weight * 0.3 + recency * 0.4 + priority_weight * 0.3) * 100.0
}

let memory = storage.get_memory("memory_123").await?;
let importance = calculate_importance(&memory);
println!("Memory importance score: {:.2}", importance);
```

### Querying by Lifecycle Metadata

```rust
use locai::storage::filters::MemoryFilter;

// Find frequently accessed memories
let filter = MemoryFilter {
    // Custom filters for memories
    ..Default::default()
};

let memories = storage.list_memories(Some(filter), Some(100), None).await?;

let frequently_accessed: Vec<_> = memories
    .into_iter()
    .filter(|m| m.access_count > 10)
    .collect();

println!("Frequently accessed: {} memories", frequently_accessed.len());
```

### Finding Recently Accessed Memories

```rust
use chrono::{Utc, Duration};

let memories = storage.list_memories(None, Some(1000), None).await?;

let cutoff = Utc::now() - Duration::days(7);

let recent: Vec<_> = memories
    .into_iter()
    .filter(|m| {
        m.last_accessed
            .map(|la| la > cutoff)
            .unwrap_or(false)
    })
    .collect();

println!("Accessed in last 7 days: {} memories", recent.len());
```

## Performance Considerations

### Batching Benefits

With 1000 concurrent accesses:

| Mode | Total Updates | DB Writes | Avg Latency |
|------|---------------|-----------|-------------|
| Batched (60s flush) | 1000 | ~10 | < 1ms |
| Async Non-blocking | 1000 | 1000 | 1-5ms |
| Blocking | 1000 | 1000 | 10-50ms |

**Recommendation**: Use batched mode (default) for best performance.

### Memory Overhead

The lifecycle tracking queue maintains an in-memory map of pending updates:

- Per update: ~200 bytes (ID + counters + timestamps)
- At 100-update batch size: ~20KB per cycle
- Negligible compared to typical data sizes

### Database Load

Batched mode dramatically reduces write load:

```
1000 memory retrievals per second:
- Batched (60s cycle): ~17 writes/sec
- Async: 1000 writes/sec  
- Blocking: 1000 writes/sec (+ request blocking)

With batching, you can handle 100x more traffic for same I/O.
```

## Troubleshooting

### Access Count Not Increasing

**Problem**: `access_count` stays at 0

**Solution**: Verify `lifecycle_tracking.enabled: true` and `lifecycle_tracking.update_on_get: true`

```yaml
memory:
  lifecycle_tracking:
    enabled: true           # ← Must be true
    update_on_get: true     # ← Must be true
```

### Last Accessed Not Updating

**Problem**: `last_accessed` is always `None` or very old

**Likely Causes**:
1. Tracking is disabled
2. Using non-batched mode with `blocking: false` - updates may not be applied yet
3. Memory was created but never retrieved

**Solution**: 
- Check configuration for enabled flag
- Wait for batch flush period (default 60s)
- Retrieve memory again to trigger update

### High Database Load

**Problem**: Too many update operations

**Solution**: Increase batch settings:

```yaml
lifecycle_tracking:
  batched: true
  flush_interval_secs: 120    # Double the interval
  flush_threshold_count: 200  # Increase threshold
```

### Memory Queue Full

**Problem**: "Lifecycle update queue full" errors

**Cause**: More than 1000 unique memories accessed before batch flush

**Solution**: 
- Use shorter `flush_interval_secs`
- Lower `flush_threshold_count` to flush more frequently
- Distribute load across time

## Implementation Details

### Architecture

```
get_memory(id)
    ↓
[Retrieve from storage]
    ↓
[Increment access_count in memory]
    ↓
[Update last_accessed in memory]
    ↓
┌─ Is tracking enabled? ─ No ─→ Return memory
│
├─ Batched mode?
│  ├─ Yes → Queue update (merged with existing) → Return memory
│  └─ No → Is blocking mode?
│       ├─ Yes → Update DB synchronously → Return memory
│       └─ No → Spawn async update task → Return memory immediately
│
Return updated memory
```

### Update Queue Behavior

The queue merges multiple accesses to the same memory:

```rust
// Access memory 5 times
for _ in 0..5 {
    storage.get_memory("mem_123").await?;
}

// Queue contains 1 entry (not 5):
// {
//   memory_id: "mem_123",
//   access_count_delta: 5,
//   last_accessed: <most recent timestamp>
// }
```

When flushed, all 5 increments are applied in a single update.

## Future Enhancements

### Planned (0.2.x)
- Live query events for lifecycle updates
- Metrics export (access patterns)
- Automatic cleanup of stale memories

### Considered (0.3.0+)
- Configurable decay functions
- Time-series analysis of access patterns
- Memory consolidation based on lifecycle data
- Query-time scoring based on recency/frequency

## Related Features

- **Enhanced Search Scoring**: Use lifecycle data for better search ranking
- **Live Queries**: Get notified of lifecycle updates
- **Memory Hooks**: Trigger custom logic on access
- **Batch Operations**: Efficiently update multiple memories

## Examples

See `/locai/examples/lifecycle_tracking.rs` for a working example.

## Support

For issues or questions:

- **GitHub Issues**: [Report a bug](https://github.com/locai/locai/issues)
- **Discussions**: [Ask a question](https://github.com/locai/locai/discussions)
- **Documentation**: Check [API docs](./API.md)

---

**Last Updated**: 2025-01-24  
**Version**: 1.0  
**Maintainer**: Locai Core Team
