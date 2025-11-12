# Enhanced Search Scoring

## Overview

Locai's enhanced search scoring system allows you to customize how search results are ranked by combining multiple relevance factors:

- **BM25 Keyword Matching**: Proven probabilistic relevance ranking
- **Vector Similarity**: Semantic search using embeddings (BYOE approach)
- **Recency**: Boost recent memories with configurable time decay
- **Access Frequency**: Memories accessed more often typically indicate relevance
- **Priority**: Explicit importance levels from Low to Critical

This flexible system enables different ranking strategies optimized for specific use cases.

## Quick Start

### Default Scoring

The simplest approach - uses balanced default weights:

```rust
use locai::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let locai = Locai::new().await?;
    let storage = locai.manager().get_storage();
    
    // Search with default scoring
    let results = storage
        .search_memories_with_scoring("wizard magic", None, Some(10))
        .await?;
    
    for (memory, score) in results {
        println!("Score: {:.4} - {}", score, memory.content);
    }
    
    Ok(())
}
```

### Custom Scoring Configuration

Tailor the ranking to your specific use case:

```rust
use locai::prelude::*;

let config = ScoringConfig {
    bm25_weight: 1.0,
    vector_weight: 1.0,
    recency_boost: 0.5,
    access_boost: 0.3,
    priority_boost: 0.2,
    decay_function: DecayFunction::Exponential,
    decay_rate: 0.1,
};

let results = storage
    .search_memories_with_scoring("wizard magic", Some(config), Some(10))
    .await?;
```

## Scoring Configuration Options

### ScoringConfig Structure

```rust
pub struct ScoringConfig {
    /// Weight for BM25 keyword matching (0.0+)
    pub bm25_weight: f32,
    
    /// Weight for vector embedding similarity (0.0+)
    pub vector_weight: f32,
    
    /// Boost factor for recent memories (0.0+)
    pub recency_boost: f32,
    
    /// Boost factor for frequently accessed memories (0.0+)
    pub access_boost: f32,
    
    /// Boost factor for high-priority memories (0.0+)
    pub priority_boost: f32,
    
    /// Time-based decay function
    pub decay_function: DecayFunction,
    
    /// Decay rate parameter (> 0.0)
    pub decay_rate: f32,
}
```

### Decay Functions

The decay function models how recency boost diminishes over time:

#### **DecayFunction::None**
No time decay - all memories have equal recency weight regardless of age.

```
Boost = recency_boost (constant)
```

Use when: Recency is not important to your ranking.

#### **DecayFunction::Linear**
Importance decreases linearly with age.

```
Boost = recency_boost * max(0, 1 - age_hours * decay_rate)
```

- Linear from full boost to zero
- `decay_rate` determines hours until full decay (e.g., 0.1 = 10 hours to zero)
- Reaches zero and stays zero after cutoff

Use when: You want a hard cutoff after a certain age.

#### **DecayFunction::Exponential** (default)
Importance decreases exponentially with age, modeling the Ebbinghaus forgetting curve.

```
Boost = recency_boost * exp(-decay_rate * age_hours)
```

- Never reaches zero (asymptotic)
- `decay_rate` controls steepness (higher = faster decay)
- More natural memory fade than linear
- Typical values: 0.05-0.2

Use when: You want a natural forgetting curve like human memory.

#### **DecayFunction::Logarithmic**
Importance decreases logarithmically (slower than exponential).

```
Boost = recency_boost / (1 + ln(1 + age_hours * decay_rate))
```

- Decays slowly for a long time
- Useful for long-term memory preservation
- `decay_rate` controls steepness (typical: 0.05)

Use when: You want to preserve long-term relevance more than exponential.

## Pre-configured Scoring Profiles

### 1. Default Scoring (Balanced)

```rust
let config = ScoringConfig::default();
```

**Configuration:**
- BM25 weight: 1.0
- Vector weight: 1.0
- Recency boost: 0.5
- Access boost: 0.3
- Priority boost: 0.2
- Decay function: Exponential
- Decay rate: 0.1

**Use cases:**
- General-purpose search
- When you're unsure what to optimize for
- Balanced trade-off between all factors

### 2. Recency-Focused (Active Games)

```rust
let config = ScoringConfig::recency_focused();
```

**Configuration:**
- BM25 weight: 0.5
- Vector weight: 0.5
- Recency boost: 2.0
- Access boost: 0.2
- Priority boost: 0.1
- Decay function: Exponential
- Decay rate: 0.2 (faster decay)

**Use cases:**
- Game worlds with active events
- Real-time systems where "now" matters most
- News aggregation or trending content
- Chat history (recent context is most relevant)

**Example:**
```rust
// Fresh memory gets max boost, 6 hours old = ~50% of max boost
let config = ScoringConfig::recency_focused();
```

### 3. Semantic-Focused (Vector Search)

```rust
let config = ScoringConfig::semantic_focused();
```

**Configuration:**
- BM25 weight: 0.3
- Vector weight: 1.5
- Recency boost: 0.3
- Access boost: 0.2
- Priority boost: 0.2
- Decay function: Exponential
- Decay rate: 0.1

**Use cases:**
- When embeddings are available and semantic similarity matters
- Dense information retrieval
- Cross-language or paraphrase matching
- Conceptual matching over keyword matching

**Example:**
```rust
// Search for semantically related ideas, not just keyword matches
let results = storage
    .search_memories_with_scoring(
        "magical creature that flies",
        Some(ScoringConfig::semantic_focused()),
        Some(10)
    )
    .await?;
// Matches concepts like "dragon", "phoenix" even if keywords don't align
```

### 4. Importance-Focused (Knowledge Systems)

```rust
let config = ScoringConfig::importance_focused();
```

**Configuration:**
- BM25 weight: 0.7
- Vector weight: 0.7
- Recency boost: 0.2
- Access boost: 1.0
- Priority boost: 0.8
- Decay function: Logarithmic
- Decay rate: 0.05 (slow decay)

**Use cases:**
- Knowledge management systems
- Documentation/wiki systems
- Reference materials that don't change often
- Historical or archival search

**Example:**
```rust
// Frequently accessed and high-priority memories rank highest
let results = storage
    .search_memories_with_scoring(
        "philosophy",
        Some(ScoringConfig::importance_focused()),
        Some(10)
    )
    .await?;
// High-access memories and Critical priority items appear first
```

## Creating Custom Configurations

Build a configuration tailored to your specific use case:

```rust
use locai::prelude::*;

let custom_config = ScoringConfig {
    bm25_weight: 2.0,           // Emphasize keyword matching
    vector_weight: 0.5,          // Downplay semantic similarity
    recency_boost: 0.0,          // Ignore age completely
    access_boost: 2.0,           // Strongly favor accessed memories
    priority_boost: 0.5,         // Light emphasis on priority
    decay_function: DecayFunction::Linear,
    decay_rate: 0.05,            // 20 hours until zero recency (not used here)
};

// Validate before use (catches invalid configurations)
custom_config.validate()?;

let results = storage
    .search_memories_with_scoring(
        "wizard",
        Some(custom_config),
        Some(10)
    )
    .await?;
```

## Weight Normalization

The system automatically normalizes BM25 and vector weights to ensure they don't dominate boosts:

```rust
let mut config = ScoringConfig::default();

// Weights can be any values initially
config.bm25_weight = 10.0;
config.vector_weight = 5.0;

// After normalize_weights():
// bm25_weight = 10.0 / 15.0 ≈ 0.667
// vector_weight = 5.0 / 15.0 ≈ 0.333
config.normalize_weights();

// The ratio is preserved: 2:1 relationship remains
```

Normalization ensures:
- Primary scores (BM25/vector) don't overwhelm boosts
- Relative weight ratios are preserved
- Boosts can meaningfully impact ranking

## Scoring Calculation Formula

The final score combines all factors:

```
final_score = (bm25_score × bm25_weight)
            + (vector_score × vector_weight)
            + recency_boost(decay_function, memory.age)
            + ln(1 + access_count) × access_boost
            + priority_value × priority_boost

where:
  recency_boost = depends on decay_function
  priority_value = 0 (Low), 1 (Normal), 2 (High), 3 (Critical)
```

### Example Calculation

**Memory characteristics:**
- BM25 score: 8.0
- Vector score: 0.6
- Created: 5 hours ago
- Access count: 10
- Priority: High (value = 2)

**Configuration:**
- bm25_weight: 0.5
- vector_weight: 0.5
- recency_boost: 0.5
- access_boost: 0.1
- priority_boost: 0.2
- decay_function: Exponential
- decay_rate: 0.1

**Calculation:**
```
bm25 component: 8.0 × 0.5 = 4.0
vector component: 0.6 × 0.5 = 0.3
recency component: 0.5 × exp(-0.1 × 5) ≈ 0.303
access component: ln(1 + 10) × 0.1 ≈ 0.232
priority component: 2 × 0.2 = 0.4

final_score ≈ 4.0 + 0.3 + 0.303 + 0.232 + 0.4 ≈ 5.235
```

## Performance Considerations

### Overhead

The enhanced scoring system adds minimal overhead:
- BM25 search is the primary cost (already performed)
- Score calculation is O(n) where n = number of results
- Memory metadata lookups are constant time

### Typical Performance

- Query with default scoring: ~5-10ms for 100 memories
- Score calculation overhead: <1ms for typical result sets
- Vector search (if enabled): depends on embedding dimension

### Optimization Tips

1. **Limit result set size** before scoring:
   ```rust
   // Fetch 2x limit, score top results
   let results = storage.bm25_search_memories(query, Some(limit * 2)).await?;
   ```

2. **Disable unused factors**:
   ```rust
   let config = ScoringConfig {
       bm25_weight: 1.0,
       vector_weight: 0.0,  // Disable vector search
       recency_boost: 0.0,  // Disable time-based ranking
       access_boost: 0.0,   // Disable access frequency
       priority_boost: 0.0, // Disable priority
       ..Default::default()
   };
   ```

3. **Use appropriate decay function**:
   - `None`: Fastest (no calculation)
   - `Linear`: Fast
   - `Exponential`: Medium (default)
   - `Logarithmic`: Slightly slower

## Real-World Examples

### Example 1: RPG Game World

```rust
// Recent events matter most, but important lore stays relevant
let game_config = ScoringConfig {
    bm25_weight: 1.0,
    vector_weight: 0.5,
    recency_boost: 2.0,
    access_boost: 0.5,
    priority_boost: 0.5,
    decay_function: DecayFunction::Exponential,
    decay_rate: 0.15,  // Fast decay - 6-7 days to half value
};
```

### Example 2: Documentation System

```rust
// Important docs stay relevant long-term
let docs_config = ScoringConfig {
    bm25_weight: 2.0,
    vector_weight: 0.5,
    recency_boost: 0.0,
    access_boost: 1.5,
    priority_boost: 1.0,
    decay_function: DecayFunction::None,
    decay_rate: 0.01,  // Ignored, but must be > 0
};
```

### Example 3: Chat Assistant

```rust
// Recent context is critical, but important facts matter
let chat_config = ScoringConfig {
    bm25_weight: 0.5,
    vector_weight: 1.0,
    recency_boost: 3.0,
    access_boost: 0.2,
    priority_boost: 0.3,
    decay_function: DecayFunction::Exponential,
    decay_rate: 0.3,  // Very fast decay - ~3 days to half
};
```

### Example 4: Knowledge Graph

```rust
// Semantic relationships matter most
let kg_config = ScoringConfig {
    bm25_weight: 0.5,
    vector_weight: 2.0,
    recency_boost: 0.0,
    access_boost: 0.5,
    priority_boost: 0.5,
    decay_function: DecayFunction::Logarithmic,
    decay_rate: 0.05,  // Slow decay
};
```

## API Reference

### ScoringConfig Methods

```rust
// Create with defaults
let config = ScoringConfig::default();
let config = ScoringConfig::new();

// Use pre-configured profiles
let config = ScoringConfig::recency_focused();
let config = ScoringConfig::semantic_focused();
let config = ScoringConfig::importance_focused();

// Normalize weights
let mut config = ScoringConfig::default();
config.normalize_weights();

// Validate configuration
config.validate()?;

// Check if any boosts enabled
if config.has_any_boosts() {
    println!("At least one boost is enabled");
}
```

### Search API

```rust
use locai::prelude::*;

let storage = locai.manager().get_storage();

// Search with custom scoring
let results = storage
    .search_memories_with_scoring(
        "search query",
        Some(ScoringConfig::default()),  // Optional config
        Some(10)                          // Optional limit
    )
    .await?;

// Results are (Memory, score) tuples, sorted by score descending
for (memory, score) in results {
    println!("Score: {:.4} - {}", score, memory.content);
}
```

## Troubleshooting

### Unexpected Ranking Order

1. **Check weight normalization**:
   ```rust
   let mut config = ScoringConfig::default();
   config.normalize_weights();
   ```

2. **Verify configuration values**:
   ```rust
   let config = ScoringConfig::default();
   config.validate()?;  // Will error if invalid
   ```

3. **Disable factors to isolate issues**:
   ```rust
   // Test with only BM25
   let simple_config = ScoringConfig {
       bm25_weight: 1.0,
       vector_weight: 0.0,
       recency_boost: 0.0,
       access_boost: 0.0,
       priority_boost: 0.0,
       ..Default::default()
   };
   ```

### Vector Search Not Working

- Ensure embeddings are present in memories
- Use `semantic_focused()` profile to emphasize vector scoring
- Check that vector_weight > 0.0

### Recency Boost Not Applied

- Verify `recency_boost > 0.0`
- Check `decay_function` is not `None`
- Verify `decay_rate > 0.0`

## Best Practices

1. **Start with pre-configured profiles**, then customize:
   ```rust
   let mut config = ScoringConfig::recency_focused();
   config.access_boost = 0.8;  // Tweak one factor
   ```

2. **Always validate before use**:
   ```rust
   config.validate()?;
   ```

3. **Test with real data** - scoring effectiveness depends on your data characteristics

4. **Monitor performance** - enable profiling if search becomes slow

5. **Document your choice**:
   ```rust
   // Game uses recency-focused for active story events
   let config = ScoringConfig::recency_focused();
   ```

## Migration from Basic Search

If you're currently using basic BM25 search:

```rust
// Before: Only keyword matching
let results = storage.bm25_search_memories(query, Some(10)).await?;

// After: Enhanced multi-factor ranking
let results = storage
    .search_memories_with_scoring(query, None, Some(10))  // Uses default config
    .await?;

// Or customize:
let config = ScoringConfig::recency_focused();
let results = storage
    .search_memories_with_scoring(query, Some(config), Some(10))
    .await?;
```

## See Also

- [Search Guide](/docs/SEARCH.md) - General search functionality
- [Memory Model](/docs/DESIGN.md) - Memory structure and fields
- [API Documentation](/docs/API.md) - Complete API reference





