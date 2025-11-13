# Locai CLI Design Document

**Version**: 1.0  
**Date**: 2025-01-27  
**Status**: Current Design Specification

## Executive Summary

The Locai CLI is a first-class interface to the Locai memory management system, designed to serve multiple audiences: developers exploring Locai, users without language bindings, automation workflows, and MCP (Model Context Protocol) integrations. The CLI emphasizes standalone operation, progressive disclosure, and automation-friendliness.

## Design Philosophy

### Core Principles

1. **Standalone First**: Core features work without a server - CLI is self-contained
2. **Progressive Disclosure**: Simple by default, powerful when needed
3. **Automation-Friendly**: Excellent JSON output, stable interface, scriptable
4. **Beginner-Friendly**: Approachable for newcomers while maintaining power for advanced users
5. **Consistency**: Predictable command structure and naming conventions

### Key Context

The CLI serves as a critical entry point for Locai, especially for users who:
- Want to "test the waters" before committing to full integration
- Need a way to use Locai without language bindings (Python, JavaScript, etc.)
- Prefer a standalone tool without requiring the server API
- Want to integrate via MCP without full server deployment

This makes the CLI a **first-class interface** to Locai, not just a developer tool.

## Architecture Overview

### Module Structure

```
locai-cli/
├── src/
│   ├── main.rs              # Entry point, command routing
│   ├── args.rs              # Argument structs for all commands
│   ├── commands.rs          # Command enum definitions
│   ├── context.rs           # CLI context (MemoryManager initialization)
│   ├── handlers/            # Command handlers
│   │   ├── memory.rs
│   │   ├── entity.rs
│   │   ├── relationship.rs
│   │   ├── graph.rs
│   │   ├── batch.rs
│   │   ├── relationship_type.rs
│   │   ├── tutorial.rs
│   │   └── quickstart.rs
│   ├── output.rs            # Output formatting (table, JSON, colors)
│   ├── help/                # Help system
│   │   └── explanations.rs  # Concept explanations for --explain
│   └── utils.rs             # Utility functions
```

### Command Flow

1. **Parse**: `Cli::parse()` parses command-line arguments using `clap`
2. **Detect Output Format**: Auto-detect JSON for non-TTY output
3. **Initialize Context**: Create `LocaiCliContext` with `MemoryManager` (skipped for `version`, `completions`)
4. **Route**: Match command enum and delegate to appropriate handler
5. **Execute**: Handler performs operation using `MemoryManager`
6. **Format Output**: Format results as table or JSON based on output format
7. **Handle Errors**: Structured JSON errors for automation, human-readable for TTY

### Library vs Server API

**Current Approach**: CLI uses library directly (`MemoryManager`)

- **Core features**: Use `MemoryManager` directly (standalone, no server needed)
  - Memory CRUD operations
  - Entity operations
  - Relationship operations
  - Graph operations
  - Batch operations
  - Relationship type management
  - Search and retrieval

- **Server-only features**: Not implemented (webhooks require server)
  - Webhooks (server-only feature)
  - Remote API access (future consideration)

**Rationale**:
- **Standalone First**: Most users want CLI without server - use library directly
- **Performance**: Direct library access is faster than HTTP
- **Simplicity**: No server setup required for core functionality
- **MCP-Friendly**: Standalone operation enables MCP wrapper without server

## Command Structure

### Command Hierarchy

```
locai-cli
├── version                    # Version information
├── diagnose                   # Diagnostic checks
├── memory                     # Memory operations
│   ├── add (alias: remember)
│   ├── get (alias: show)
│   ├── search (alias: recall)
│   ├── update
│   ├── delete (alias: forget)
│   ├── list
│   ├── tag
│   ├── count
│   ├── priority
│   ├── recent
│   └── relationships
├── entity                     # Entity operations
│   ├── create
│   ├── get
│   ├── list
│   ├── update
│   ├── delete
│   ├── count
│   ├── relationships
│   ├── memories
│   └── central
├── relationship               # Relationship operations
│   ├── create (aliases: connect, link)
│   ├── get
│   ├── list
│   ├── update
│   ├── delete
│   └── related
├── graph                      # Graph operations
│   ├── subgraph
│   ├── paths
│   ├── connected
│   ├── metrics
│   ├── query
│   ├── similar
│   └── entity
├── batch                      # Batch operations
│   └── execute
├── relationship-type          # Relationship type management
│   ├── list
│   ├── get
│   ├── register
│   ├── update
│   ├── delete
│   ├── metrics
│   └── seed
├── tutorial (aliases: interactive, learn)
├── quickstart
├── completions               # Shell completion generation
└── clear                      # Clear all storage
```

### Design Patterns

#### 1. Resource-Based Commands
Commands follow REST-like patterns: `resource operation [args]`

- `memory add` - Create memory
- `memory get <id>` - Read memory
- `memory update <id>` - Update memory
- `memory delete <id>` - Delete memory

**Rationale**: Familiar pattern, easy to discover, consistent across resources

#### 2. Conversational Aliases
Friendly aliases for common operations:

- `remember` → `memory add`
- `recall` → `memory search`
- `forget` → `memory delete`
- `show` → `memory get`
- `connect` / `link` → `relationship create`

**Rationale**: Lower barrier to entry, more intuitive for beginners, while maintaining technical commands for power users

#### 3. Subcommands for Related Operations
Related operations grouped under resource:

- `memory relationships <id>` - View memory relationships
- `entity relationships <id>` - View entity relationships
- `entity memories <id>` - Get memories for entity

**Rationale**: Logical grouping, discoverable via `--help`, reduces command namespace pollution

## Output Formatting

### Output Modes

#### 1. Table Format (Default)
Human-readable table format for interactive use:

```
Found 5 memories:

ID                                    Type      Priority  Content
─────────────────────────────────────────────────────────────────────
memory:abc123                         Fact      Normal    Example memory
memory:def456                         Episodic  High      Another memory
```

**Features**:
- Color-coded output (success, error, info, accent colors)
- Formatted tables with proper alignment
- Truncated content with ellipsis for long text
- Visual hierarchy with separators and headers

#### 2. JSON Format
Machine-readable JSON for automation:

```json
[
  {
    "id": "memory:abc123",
    "content": "Example memory",
    "memory_type": "Fact",
    "priority": "Normal",
    "created_at": "2025-01-27T12:00:00Z"
  }
]
```

**Features**:
- Valid JSON output (pretty-printed)
- Consistent structure across commands
- Includes all metadata
- Suitable for scripting and MCP integration

### Auto-Detection

**TTY Detection**: Automatically switches to JSON when stdout is not a TTY (piped/redirected)

```rust
let output_format_str = if cli_args.machine {
    "json".to_string()
} else if atty::isnt(atty::Stream::Stdout) {
    "json".to_string()  // Auto-detect piped output
} else {
    cli_args.output.clone()
};
```

**Rationale**: 
- Better UX for automation (no need to specify `--output json`)
- Works seamlessly with pipes: `locai-cli memory list | jq`
- MCP-friendly (wrappers don't need to specify format)

### Error Output

**Structured JSON Errors** (when `--output json`):

```json
{
  "error": true,
  "code": "NOT_FOUND",
  "message": "Memory with ID 'abc123' not found",
  "timestamp": "2025-01-27T12:00:00Z",
  "details": {
    "resource_type": "memory",
    "resource_id": "abc123"
  }
}
```

**Human-Readable Errors** (default):

```
✗ Memory with ID 'abc123' not found
```

**Rationale**: 
- Structured errors enable programmatic error handling
- Human-readable errors provide immediate feedback
- Consistent error codes for automation

## Error Handling

### Error Types

The CLI maps `LocaiError` variants to standardized error codes:

- `STORAGE_ERROR` - Storage-related errors
- `MEMORY_ERROR` - Memory operation errors
- `ENTITY_ERROR` - Entity operation errors
- `RELATIONSHIP_ERROR` - Relationship operation errors
- `ML_ERROR` - Machine learning/embedding errors
- `CONFIGURATION_ERROR` - Configuration errors
- `NOT_FOUND` - Resource not found (mapped from specific errors)
- `OTHER_ERROR` - Generic errors

### Error Output Strategy

1. **JSON Mode**: Structured JSON with error code, message, timestamp, optional details
2. **Table Mode**: Colored, human-readable error messages
3. **Exit Codes**: Non-zero exit code on error (enables shell error handling)

**Rationale**: 
- Enables automation workflows to handle errors programmatically
- Provides clear feedback for interactive use
- Standard exit codes for shell integration

## Beginner-Friendly Features

### 1. Enhanced Help Text

Commands include comprehensive `long_about` text with:
- Concept explanations (what is a memory, search modes, etc.)
- Multiple usage examples
- Related commands (cross-references)
- Links to `--explain` for more details

**Example** (`memory add --help`):
```
Store a new memory in Locai. A memory is a piece of information that can be 
searched and retrieved later.

WHAT IS A MEMORY?
A memory is a piece of information stored in Locai that can be searched and 
retrieved later. When you create a memory, Locai:
  1. Stores the content
  2. Indexes it for search (using BM25 text search)
  3. Optionally creates embeddings for semantic search
  4. Extracts entities and relationships automatically

MEMORY TYPES:
  • fact - Factual information (e.g., "Water boils at 100°C")
  • conversation - Dialogues or exchanges
  ...

EXAMPLES:
  # Simple memory
  locai-cli memory add "The user likes coffee"
  
  # Fact with high priority
  locai-cli memory add "API key is secret" --priority high --type fact

RELATED COMMANDS:
  • locai-cli memory search "query" - Search for memories
  • locai-cli --explain memory - Learn more about memories
```

**Rationale**: 
- Progressive disclosure: basic help by default, detailed help available
- Educational: helps users learn concepts as they use the tool
- Reduces need to consult external documentation

### 2. --explain Flag

Global flag to explain concepts: `locai-cli --explain <concept>`

**Supported Concepts**:
- `memory` / `memories`
- `entity` / `entities`
- `relationship` / `relationships`
- `graph`
- `search`
- `batch`

**Example** (`locai-cli --explain memory`):
```
━━━ Memory Concept ━━━

What is a Memory?

A memory is a piece of information stored in Locai. It can represent:
  • Facts - Objective information (e.g., 'Paris is the capital of France')
  • Episodes - Specific events or experiences
  • Conversations - Dialogues or exchanges
  • Procedural knowledge - How to do something

Key Features:
  • Each memory has a unique ID
  • Can be tagged for organization
  • Has a priority level (Critical, High, Normal, Low)
  • Can be linked to other memories via relationships
  • Supports semantic search

Common Commands:
  locai-cli memory add "Content"     # Create a new memory
  locai-cli memory search "query"    # Search memories semantically
  ...
```

**Rationale**: 
- On-demand learning without leaving the CLI
- Consistent with help text but more detailed
- Helps users understand concepts before using commands

### 3. Interactive Tutorial

Command: `locai-cli tutorial [topic]` (aliases: `interactive`, `learn`)

**Features**:
- Step-by-step interactive lessons
- Sample data generation
- Multiple topics (basics, entities, relationships, graph, advanced)
- Cleanup option for tutorial data

**Rationale**: 
- Hands-on learning experience
- Reduces initial learning curve
- Creates sample data for exploration

### 4. Quick Start Command

Command: `locai-cli quickstart [--cleanup]`

**Features**:
- Creates sample memories, entities, relationships
- Provides example commands to try
- Cleanup option to remove sample data

**Rationale**: 
- Immediate value: users can explore without creating data
- Demonstrates key features with real examples
- Low commitment: easy cleanup

### 5. Smart Defaults

**Progressive Disclosure**: Commands work with minimal arguments, reveal complexity when needed

- Memory type: `fact` (most common)
- Priority: `normal` (balanced)
- Search mode: `hybrid` (default) - automatically combines text and semantic search when available
- Output format: `table` (human-readable)
- Limit: Sensible defaults (10 for search, 20 for list)

**Rationale**: 
- Simple by default: `locai-cli remember "content"` just works
- Powerful when needed: add flags for advanced features
- Reduces cognitive load for beginners

## Automation Support

### JSON Output

All commands support `--output json` or `--machine` for machine-readable output.

**Features**:
- Valid JSON (pretty-printed for readability)
- Consistent structure across commands
- Auto-detected for piped output
- Suitable for scripting and MCP integration

### Shell Completions

Command: `locai-cli completions <shell>`

**Supported Shells**:
- Bash
- Zsh
- Fish
- PowerShell
- Elvish

**Features**:
- Fast generation (~0.012s, no storage initialization)
- Installation instructions shown on stderr
- Clean completion scripts (no logging output)

**Rationale**: 
- Improves developer experience
- Reduces typos and errors
- Discoverable commands via tab completion

### Progress Indicators

Batch operations show progress bars for large batches (>5 operations).

**Features**:
- Only shown when stdout is a TTY
- Skipped for JSON output
- Uses `indicatif` crate for smooth progress bars

**Rationale**: 
- Provides feedback for long-running operations
- Doesn't interfere with JSON parsing
- Only shown when appropriate (interactive use)

### Stable Interface

**Design Principles**:
- Command structure doesn't change between versions
- Arguments are stable and well-documented
- Error codes are consistent
- Output format is predictable

**Rationale**: 
- Enables automation and integration
- MCP wrappers can rely on stable interface
- Scripts won't break with updates

## Command Reference

### Memory Operations

```bash
# Create
locai-cli memory add "Content" [--type <type>] [--priority <priority>] [--tags <tags>]
locai-cli remember "Content"  # Alias

# Read
locai-cli memory get <id>
locai-cli memory show <id>    # Alias

# Search
locai-cli memory search "query" [--mode <mode>] [--memory-type <type>] [--tag <tag>]
locai-cli recall "query"      # Alias

# Update
locai-cli memory update <id> [--content <content>] [--priority <priority>] [--tags <tags>]

# Delete
locai-cli memory delete <id>
locai-cli memory forget <id>  # Alias

# List and Filter
locai-cli memory list [--limit <n>] [--memory-type <type>] [--tag <tag>]
locai-cli memory count [--memory-type <type>] [--tag <tag>]
locai-cli memory priority <priority> [--limit <n>]
locai-cli memory recent [--limit <n>]

# Relationships
locai-cli memory relationships <id>
locai-cli memory relationship create <id> <target> <type> [--properties <json>]
```

### Entity Operations

```bash
# CRUD
locai-cli entity create <id> <type> [--properties <json>]
locai-cli entity get <id>
locai-cli entity list [--limit <n>] [--entity-type <type>]
locai-cli entity update <id> [--entity-type <type>] [--properties <json>]
locai-cli entity delete <id>
locai-cli entity count

# Relationships and Analysis
locai-cli entity relationships <id>
locai-cli entity relationship create <id> <target> <type> [--properties <json>]
locai-cli entity memories <id>
locai-cli entity central [--limit <n>]
```

### Relationship Operations

```bash
# CRUD
locai-cli relationship create <from> <to> <type> [--bidirectional] [--properties <json>]
locai-cli relationship connect <from> <to> <type>  # Alias
locai-cli relationship link <from> <to> <type>     # Alias
locai-cli relationship get <id>
locai-cli relationship list [--limit <n>] [--relationship-type <type>]
locai-cli relationship update <id> [--relationship-type <type>] [--properties <json>]
locai-cli relationship delete <id>

# Query
locai-cli relationship related <id> [--relationship-type <type>] [--direction <dir>]
```

### Graph Operations

```bash
# Graph Analysis
locai-cli graph subgraph <id> [--depth <n>] [--no-temporal]
locai-cli graph paths <from> <to> [--max-depth <n>]
locai-cli graph connected <id> [--depth <n>] [--relationship-type <type>]
locai-cli graph metrics
locai-cli graph query <pattern> [--limit <n>]
locai-cli graph similar <pattern-id> [--limit <n>]
locai-cli graph entity <id> [--depth <n>] [--include-temporal-span]
```

### Batch Operations

```bash
# Execute batch file
locai-cli batch execute <file> [--transaction] [--continue-on-error]
```

**Batch File Format**:
```json
{
  "operations": [
    {
      "operation": "create_memory",
      "content": "Memory 1",
      "memory_type": "fact"
    },
    {
      "operation": "create_memory",
      "content": "Memory 2",
      "memory_type": "episodic"
    },
    {
      "operation": "create_relationship",
      "source": "memory:1",
      "target": "memory:2",
      "relationship_type": "references"
    }
  ]
}
```

### Relationship Type Management

```bash
# CRUD
locai-cli relationship-type list
locai-cli relationship-type get <name>
locai-cli relationship-type register <name> [--inverse <name>] [--symmetric] [--transitive] [--schema <file>]
locai-cli relationship-type update <name> [--inverse <name>] [--symmetric <bool>] [--transitive <bool>] [--schema <file>]
locai-cli relationship-type delete <name>

# Utilities
locai-cli relationship-type metrics
locai-cli relationship-type seed
```

### Learning and Exploration

```bash
# Interactive tutorial
locai-cli tutorial [topic]
locai-cli interactive [topic]  # Alias
locai-cli learn [topic]       # Alias

# Quick start
locai-cli quickstart [--cleanup]

# Concept explanations
locai-cli --explain memory
locai-cli --explain entity
locai-cli --explain relationship
locai-cli --explain graph
locai-cli --explain search
locai-cli --explain batch
```

### Utilities

```bash
# Version and diagnostics
locai-cli version
locai-cli diagnose

# Shell completions
locai-cli completions <shell>  # bash, zsh, fish, powershell, elvish

# Storage management
locai-cli clear  # Clear all storage (with confirmation)
```

## Global Flags

### Output Control

- `--output <format>` - Output format: `table` (default) or `json`
- `--machine` - Alias for `--output json`
- `--quiet` - Suppress all logging output
- `--verbose` / `--debug` - Enable debug-level logging
- `--log-level <level>` - Set log level: `off`, `error`, `warn`, `info`, `debug`, `trace`

### Configuration

- `--data-dir <path>` - Custom data directory for storage
- `--explain <concept>` - Explain a concept and exit

## Design Decisions

### Why Standalone?

**Decision**: CLI uses library directly, not HTTP client

**Rationale**:
- Most users want CLI without server setup
- Faster than HTTP (no network overhead)
- Simpler (no server configuration)
- Enables MCP integration without server

**Trade-offs**:
- Webhooks require server (documented limitation)
- Remote server access not supported (future consideration)

### Why Conversational Aliases?

**Decision**: Provide friendly aliases alongside technical commands

**Rationale**:
- Lower barrier to entry for beginners
- More intuitive for exploration use case
- Maintains technical commands for power users
- No breaking changes (both work identically)

**Trade-offs**:
- Slightly larger command namespace
- Need to document both forms

### Why Auto-Detect JSON?

**Decision**: Automatically use JSON when stdout is not a TTY

**Rationale**:
- Better UX for automation (no need to specify format)
- Works seamlessly with pipes
- MCP-friendly (wrappers don't need to specify format)

**Trade-offs**:
- Slight complexity in output format detection
- Users might be surprised (but this is expected for automation)

### Why Structured Errors?

**Decision**: Output structured JSON errors in JSON mode

**Rationale**:
- Enables programmatic error handling
- Consistent with JSON output format
- Better for automation and MCP integration

**Trade-offs**:
- More complex error formatting logic
- Need to maintain error code mapping

### Why Progressive Disclosure?

**Decision**: Simple defaults, reveal complexity when needed

**Rationale**:
- Reduces cognitive load for beginners
- Commands work immediately with minimal arguments
- Advanced features available when needed
- Better discoverability (help text shows options)

**Trade-offs**:
- Need to choose good defaults
- Some users might want more explicit control

## Future Considerations

### Potential Enhancements

1. **Temporal Parameter Enhancements**: Add more temporal filtering options to search and graph commands
2. **Webhook Management**: Add webhook commands if server integration is needed
3. **Remote Server Support**: Option to connect to remote Locai server
4. **Interactive Mode**: REPL-style interface for exploration
5. **Export/Import**: Bulk export/import in various formats

### Design Principles for Future Features

- **Standalone First**: Prefer library-based implementation over HTTP
- **Backward Compatibility**: Don't break existing commands
- **Progressive Disclosure**: Simple defaults, advanced options available
- **Automation-Friendly**: Always support JSON output
- **Beginner-Friendly**: Provide help text and examples

## Conclusion

The Locai CLI is designed as a first-class interface to Locai, serving multiple audiences with a focus on standalone operation, progressive disclosure, and automation-friendliness. The design emphasizes consistency, discoverability, and ease of use while maintaining the power needed for advanced workflows.

Key strengths:
- ✅ Standalone operation (no server required)
- ✅ Comprehensive feature set (CRUD + advanced operations)
- ✅ Beginner-friendly (aliases, help text, tutorial)
- ✅ Automation-friendly (JSON output, stable interface)
- ✅ Well-documented (help text, explanations, examples)

The CLI successfully bridges the gap between exploration and production use, making Locai accessible to users at all levels.

