# Search Architecture

## Overview

Locai provides a sophisticated full-text search implementation using SurrealDB's BM25 algorithm. The system offers professional-grade search capabilities comparable to dedicated search engines, with support for relevance ranking, custom analyzers, highlighting, and fuzzy matching.

## Architecture

### Search Components

1. **BM25 Full-Text Search**
   - Industry-standard relevance ranking algorithm
   - Term frequency and inverse document frequency scoring
   - Document length normalization
   - Field-specific scoring weights

2. **Custom Analyzers**
   - `memory_analyzer`: Full stemming and normalization for memory content
   - `entity_analyzer`: Optimized for entity names with lighter processing
   - `fuzzy_analyzer`: Basic processing for typo-tolerant matching

3. **Search Strategies**
   - Exact match with BM25 scoring
   - Fuzzy search for typo tolerance
   - Temporal search with time-based filtering
   - Tag-based search on structured metadata
   - Autocomplete with prefix matching

### Unified Search Result

All search operations return results in a unified format:

```rust
pub struct SearchResult {
    pub id: String,
    pub result_type: SearchResultType,
    pub content: SearchContent,
    pub score: f32,
    pub match_info: MatchInfo,
    pub context: SearchContext,
    pub metadata: SearchMetadata,
}
```

## API Reference

### Basic Search

```rust
// Universal search across all content types
let results = locai.search("quantum computing").await?;
```

### Advanced Search

```rust
let options = SearchOptions {
    strategy: SearchStrategy::Hybrid,
    include_types: SearchTypeFilter::all(),
    min_score: Some(0.7),
    graph_depth: 3,
    ..Default::default()
};

let results = locai.search_with_options("machine learning", options).await?;
```

### Search Options

- `limit`: Maximum number of results (default: 20)
- `strategy`: Search strategy (Auto, Semantic, Keyword, Graph, Hybrid)
- `include_types`: Filter by result type (memories, entities, graphs, relationships)
- `time_range`: Temporal filtering
- `min_score`: Minimum relevance threshold
- `include_context`: Include related entities and memories
- `graph_depth`: Traversal depth for graph searches

## Implementation Details

### Query Processing

1. **Query Analysis**: Detect entities, temporal expressions, and query intent
2. **Strategy Selection**: Choose optimal search approach based on query characteristics
3. **Index Selection**: Route to appropriate indexes (content, metadata, tags, properties)
4. **Result Ranking**: Apply BM25 scoring with field-specific weights

### Performance Optimization

- Parallel execution of search strategies
- Lazy loading of full content
- Smart caching of frequently accessed data
- Early termination for sufficient results
- Optimized indexes for each search type

### Storage Integration

The search system leverages SurrealDB's native capabilities:

```sql
-- Example BM25 search with highlighting
SELECT *, 
       search::score(0) AS bm25_score,
       search::highlight('<mark>', '</mark>', 0) AS highlighted_content
FROM memory 
WHERE content @0@ $query
ORDER BY bm25_score DESC
LIMIT $limit
```

## Use Cases

### Information Retrieval
BM25 excels at finding specific facts, phrases, and technical content with exact or near-exact matches.

### Temporal Queries
Native support for time-based searches combined with text filtering enables queries like "meetings last week" or "notes from January".

### Metadata Search
Structured search on tags, properties, and custom metadata fields provides precise filtering capabilities.

### Typo Tolerance
Fuzzy matching handles common misspellings and variations without requiring exact matches.

## Future Enhancements

While the current BM25 implementation provides comprehensive search functionality, potential future enhancements include:

1. **Vector Search Integration**: Optional semantic similarity for conceptual matches
2. **Graph Traversal**: Enhanced relationship-based discovery
3. **Query Expansion**: Synonym support and query reformulation
4. **Multi-lingual Support**: Language-specific analyzers and stemming

## Performance Characteristics

- **Indexing**: Instant, no embedding computation required
- **Query Time**: Sub-millisecond for most queries
- **Storage**: Minimal overhead compared to vector embeddings
- **Scalability**: Linear with document count

## Conclusion

Locai's search implementation provides production-ready, professional-grade search capabilities. The BM25-based approach offers excellent performance, explainable results, and comprehensive functionality for text-based information retrieval. 