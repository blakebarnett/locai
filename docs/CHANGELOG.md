# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2025-11-01

### Added

#### Temporal Search Features
- **Temporal filtering in search API**
  - Added `created_after` parameter to filter memories created after a specific timestamp
  - Added `created_before` parameter to filter memories created before a specific timestamp
  - ISO 8601 timestamp format support with timezone awareness
  - Combines with existing filters (memory_type, tags, priority, content search)
  - Comprehensive error handling for invalid timestamp formats
- **Temporal span analysis in graph API**
  - Added `include_temporal_span` parameter to memory and entity graph endpoints
  - Returns temporal span metadata: start time, end time, duration (days/seconds), memory count
  - Optional feature (opt-in via parameter) for backward compatibility
  - Useful for understanding memory community age and activity density
- 9 comprehensive integration tests for temporal features
- Complete Swagger/OpenAPI documentation for new parameters and responses
- User documentation with Python integration examples

### Fixed
- Temporal filters now properly exposed via HTTP API (were previously only available at library level)

### Documentation
- Added `LOCAI_TEMPORAL_FEATURES_DOCUMENTATION.md` - Complete user guide for temporal features
- Added `RESPONSE_TO_ZERA_ENHANCEMENT_REQUESTS.md` - Analysis and response to enhancement requests
- Added `TEMPORAL_FEATURES_TESTS_AND_DOCS_SUMMARY.md` - Testing and documentation summary
- Updated OpenAPI schemas to include `TemporalSpanDto` and enhanced `GraphMetadata`

## [0.2.0] - 2025-01-26

### Added

#### Memory Lifecycle Tracking (RFC 001 - Work Stream 1)
- **Automatic lifecycle tracking** for memory access patterns
  - Tracks access count, last accessed timestamp
  - Configurable update modes: batched (non-blocking), async, or blocking
  - Batched updates with time-based (60s) and threshold-based (100 operations) flushing
- **LifecycleUpdateQueue** for efficient batched updates with update merging
- **Comprehensive configuration system** for lifecycle tracking behavior
  - Enable/disable tracking globally
  - Control which operations trigger tracking (get, search, list)
  - Configure batching behavior and flush intervals
- Full test coverage with 13 unit tests for lifecycle tracking

#### Relationship Type Registry (RFC 001 - Work Stream 2)
- **Dynamic relationship type registration** at runtime
  - Thread-safe registry with RwLock-based access
  - Support for custom relationship types beyond built-in types
  - Configurable properties: directionality, constraints, validation rules
- **JSON Schema validation** for relationship metadata
- **Constraint enforcement** for symmetric and transitive relationships
- **Relationship metrics tracking** for usage analysis
- **REST API endpoints** for relationship type management:
  - `GET /api/v1/relationship-types` - List all registered types
  - `GET /api/v1/relationship-types/{name}` - Get specific type
  - `POST /api/v1/relationship-types` - Register new type
  - `PUT /api/v1/relationship-types/{name}` - Update type
  - `DELETE /api/v1/relationship-types/{name}` - Delete type
  - `GET /api/v1/relationship-types/metrics` - Export metrics
  - `POST /api/v1/relationship-types/seed` - Seed common types
- 55 comprehensive unit tests for registry functionality

#### Hook System (RFC 001 - Work Stream 3)
- **Extensible hook system** for memory lifecycle events
  - `MemoryHook` async trait for custom event handlers
  - Support for 4 lifecycle events: created, accessed, updated, deleted
  - Priority-based hook execution (0-100)
  - Timeout enforcement (5000ms default)
  - Veto support for deletion prevention
- **HookRegistry** for thread-safe hook management
- **WebhookHook** implementation for HTTP event notifications
  - POST/PUT method support
  - Custom headers and authentication
  - Retry policy with exponential backoff
  - Configurable timeouts
- Integrated with storage layer across all CRUD operations
- 154 passing tests covering all hook functionality

#### Batch Operations (RFC 001 - Work Stream 4)
- **Batch API** for efficient multi-operation execution
  - Support for 7 operation types: CreateMemory, UpdateMemory, DeleteMemory, CreateRelationship, UpdateRelationship, DeleteRelationship, UpdateMetadata
  - Transaction mode support (all-or-nothing semantics)
  - Configurable batch size limits (default: 100 operations)
  - Configurable timeout per batch (default: 30s)
- **REST API endpoint**: `POST /api/v1/batch`
- Comprehensive response tracking with success/failure counts
- 17 integration tests for batch operations

#### Enhanced Search Scoring (RFC 001 - Work Stream 5)
- **Sophisticated search scoring system** combining multiple signals:
  - BM25 keyword matching (proven relevance algorithm)
  - Vector similarity (semantic search via embeddings)
  - Recency boost with configurable decay functions
  - Access frequency boost (logarithmically weighted)
  - Priority boost for explicit importance levels
- **Four decay functions**: None, Linear, Exponential, Logarithmic
- **Pre-configured scoring profiles**:
  - `default()` - Balanced scoring
  - `recency_focused()` - For active games/real-time applications
  - `semantic_focused()` - For vector search emphasis
  - `importance_focused()` - For knowledge systems
- **ScoringConfig** with full validation and serialization support
- **REST API support**: JSON-encoded scoring parameter in `GET /api/memories/search`
- New trait method: `search_memories_with_scoring()`
- New API methods: `MemoryManager::search_with_scoring()`, `SearchExtensions::search_with_scoring()`
- 23 unit tests for scoring functionality
- Comprehensive documentation in `docs/ENHANCED_SEARCH.md` and `docs/API_ENHANCED_SEARCH_EXAMPLES.md`

#### Integration & Testing (RFC 001 - Work Stream 6)
- **15 comprehensive integration tests** covering end-to-end workflows
- **6 performance benchmark suites** measuring overhead and scalability
- Deep code analysis identifying improvement areas

#### Build Optimization
- **sccache configuration** for consistent build caching
  - 95.68% cache hit rate (up from 0.54%)
  - Build time reduced from ~60s to ~15s for unchanged code
  - C/C++ dependencies (RocksDB) now 100% cached
- Automatic sccache usage via `.cargo/config.toml`

#### Documentation
- `docs/LIFECYCLE_TRACKING.md` - Comprehensive lifecycle tracking guide
- `docs/ENHANCED_SEARCH.md` - Search scoring documentation with examples
- `docs/HOOKS.md` - Hook system usage guide
- `docs/BATCH_OPERATIONS.md` - Batch API reference
- Working examples for all new features

### Changed
- **Relationship validation** now supports flexible node types
  - Memory-to-memory relationships with custom types now work
  - Memory-to-entity relationships with custom types now work
  - Entity-to-memory relationships with custom types now work
  - Removed hard-coded relationship type matching
- **API route consolidation** to prevent route conflicts
  - Combined GET/POST methods on single routes where appropriate
  - Consistent parameter naming across endpoints
- **Memory access patterns** now automatically tracked (when enabled)
- **Search API** enhanced with configurable scoring (backward compatible)

### Fixed
- **Critical: Memory-to-memory relationships** with custom types now work correctly
  - Previously failed with "Source entity not found" error
  - Removed hard-coded relationship type assumptions
  - Now supports all node type combinations (memory↔memory, memory↔entity, entity↔memory, entity↔entity)
- **Critical: API route conflicts** causing server panic on startup
  - Fixed duplicate route registration with different parameter names
  - Server now starts reliably without route conflicts
- **Critical: sccache inconsistent usage** causing slow builds
  - Added explicit sccache configuration to `.cargo/config.toml`
  - Cache hit rate improved from 0.54% to 95.68%

### Performance
- Lifecycle tracking overhead: < 5ms per operation (batched mode)
- Hook execution: < 10ms for simple hooks
- Batch operations: 50-100 operations/second (implementation-dependent)
- Build cache efficiency: 95%+ hit rate with sccache

### Security
- Input validation for all batch operations
- Size limits on batch requests (100 operations default)
- Timeout enforcement for hooks to prevent blocking
- Schema validation for relationship metadata

## [0.1.0-alpha.1] - 2024-12-19

### Added
- Initial alpha release of Locai memory management system
- Core memory storage and retrieval functionality  
- Semantic search capabilities with embedding models
- Graph-based entity relationships and automatic linking
- REST API server with WebSocket support for real-time updates
- Command-line interface for memory management
- ModernBERT integration for advanced entity extraction
- SurrealDB integration for unified graph and vector storage
- Comprehensive documentation and examples
- Feature flags for optional components
- Live query support for real-time change notifications

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.1.0-alpha.0] - 2024-12-18

### Added
- Initial alpha release of the Locai project 