# Locai External Test Organization

This directory contains external integration tests for the Locai project, organized to address LLM context window limitations and improve code maintainability.

## Organization Structure

```
locai/tests/
├── README.md                       # This file
├── version_store_tests.rs          # VersionStore implementation tests
├── surrealdb_live_query_tests.rs   # SurrealDB live query tests
└── graph_traversal_tests.rs        # GraphTraversal implementation tests
```

## Why External Tests?

### Problem: Large Files with Embedded Tests

Traditional Rust practice embeds unit tests within implementation files using `#[cfg(test)]` modules. However, this creates challenges when working with LLMs:

1. **Context Window Limitations**: Large files (500+ lines) with extensive tests become difficult for LLMs to process effectively
2. **Focus Issues**: Implementation logic gets buried among test code, making it harder to understand core functionality
3. **Navigation Complexity**: Finding specific functionality becomes difficult in large files

### Solution: External Test Organization

We've moved comprehensive test suites to external files while keeping implementation files focused and concise:

- **Implementation files** contain only the core logic, staying under 500 lines when possible
- **External test files** contain comprehensive test suites with full documentation
- **Modular structure** allows testing specific components in isolation

## File Size Guidelines

- **Implementation files**: Aim for under 500 lines for optimal LLM processing
- **Test files**: Can be larger since they're focused on specific functionality
- **Refactoring trigger**: If a file approaches 500 lines due to tests, consider external test organization

## Benefits

### For LLM Workflows
- Cleaner context for understanding implementation logic
- Focused test review when working on test-specific changes
- Better code navigation and comprehension

### For Development
- Easier to find and modify specific functionality
- Clear separation between implementation and testing concerns
- Maintained comprehensive test coverage

### For Maintenance
- Simpler refactoring when implementation changes
- Independent test development and review
- Better git history tracking

## Current Test Suites

### Version Store Tests ✅ COMPLETED
- **Implementation**: `locai/src/storage/surrealdb/version.rs` (~350 lines)
- **Tests**: `locai/tests/version_store_tests.rs` (~200 lines)
- **Coverage**: Full CRUD, conversation tracking, knowledge evolution, AI assistant context management
- **Status**: All tests passing

### Search Intelligence Tests ✅ COMPLETED
- **Implementation**: `locai/src/storage/shared_storage/intelligence.rs` (~750 lines)
- **Tests**: `locai/tests/search_intelligence_tests.rs` (~650 lines)
- **Coverage**: Query analysis, BM25 search, fuzzy matching, intelligent search, suggestions, context-aware search, multi-strategy fusion
- **Status**: Comprehensive test coverage for Phase 3 search intelligence features

### GraphTraversal Tests ✅ IN PROGRESS
- **Implementation**: `locai/src/storage/surrealdb/graph.rs` (~400 lines after cleanup)
- **Tests**: `locai/tests/graph_traversal_tests.rs` (~350 lines)
- **Coverage**: Memory subgraph extraction, path finding, connected memory discovery, AI assistant use cases
- **Status**: Implementing comprehensive external tests

### SurrealDB Live Query Tests ✅ COMPLETED
- **Tests**: `locai/tests/surrealdb_live_query_tests.rs` (~125 lines)
- **Coverage**: Live query setup, event handling, real-time updates
- **Status**: All tests passing

### Entity Extraction Tests ✅ COMPLETED
- **Implementation**: `locai/src/entity_extraction/` (~multiple files under 500 lines each)
- **Tests**: `locai/tests/entity_extraction_tests.rs` (~770 lines)
- **Coverage**: 
  - Basic entity extraction (emails, URLs, phones, dates, money)
  - Entity resolution and merging strategies
  - Automatic relationship creation
  - **Hybrid entity extraction system** with NER capability
  - Extractor factory patterns for different use cases
  - Confidence filtering and deduplication
  - Integration scenarios and fallback mechanisms
- **Status**: All 34 tests passing - comprehensive coverage of structured data + NER pipeline

## Running Tests

External tests run normally with cargo from the workspace root:

```bash
# Run all tests
cargo test

# Run specific external test file
cargo test --test version_store_tests
cargo test --test graph_traversal_tests
cargo test --test surrealdb_live_query_tests
cargo test --test entity_extraction_tests

# Run specific test function
cargo test test_memory_subgraph_single_memory

# Run with specific package
cargo test --package locai
```

## AI Assistant Use Cases

The external tests focus on real-world AI assistant scenarios:

### Memory & Knowledge Management
- **Contextual Memory Retrieval**: Finding related memories for conversation context
- **Knowledge Evolution**: Tracking how understanding develops over time
- **Cross-domain Connections**: Discovering relationships between different topics

### Search Intelligence Features
- **Query Understanding**: Intent detection and strategy selection for natural language queries
- **Typo Tolerance**: Fuzzy matching for real-world user input with spelling errors
- **Context-Aware Search**: Session-based conversational search with history
- **Multi-Strategy Fusion**: Combining BM25, vector, and graph search results intelligently
- **Search Suggestions**: Auto-completion and query refinement based on knowledge base

### Graph-based Intelligence
- **Memory Subgraphs**: Building relevant context networks around specific memories
- **Path Discovery**: Finding connections between concepts for reasoning
- **Relationship Navigation**: Following semantic links for knowledge synthesis

### Real-time Adaptation
- **Live Query Integration**: Responding to knowledge updates in real-time
- **Dynamic Context Building**: Adjusting memory relevance based on conversation flow
- **Proactive Insights**: Surfacing related information before explicitly requested

## Best Practices

1. **Keep Implementation Focused**: Move extensive test suites to external files when implementation files exceed ~400-500 lines
2. **Maintain Coverage**: Ensure external tests provide the same comprehensive coverage as embedded tests
3. **Clear Naming**: Use descriptive names for test functions that reflect AI assistant use cases
4. **Documentation**: Document test purpose and expected AI assistant behavior
5. **Helper Functions**: Create reusable test utilities for common AI assistant scenarios
6. **Real-world Scenarios**: Test actual use cases an AI assistant would encounter

## When to Use External Tests

### Good Candidates ✅
- Files with 15+ test functions
- Implementation files approaching 500+ lines
- Complex integration test scenarios
- AI assistant behavior testing
- Graph traversal and relationship testing

### Keep Embedded
- Simple unit tests (1-3 functions)
- Quick validation tests
- Tests that are tightly coupled to implementation details

## Success Metrics

The reorganization has achieved its goals:

1. **File Size Reduction**: Implementation files reduced to LLM-friendly sizes
2. **LLM Compatibility**: Implementation files fit comfortably in LLM context windows
3. **Test Maintainability**: External tests are easier to navigate and modify
4. **Coverage Preservation**: All original test functionality maintained
5. **Build Integration**: Tests run seamlessly with existing cargo test workflow
6. **AI Assistant Focus**: Tests emphasize real-world AI assistant use cases

## Future Considerations

As the project grows, consider:
- Test utilities and shared helpers in `locai/tests/common/`
- Property-based testing for complex AI assistant scenarios
- Performance benchmarks for graph operations
- Integration test categories for different AI assistant use cases
- Automated test organization for new large files 