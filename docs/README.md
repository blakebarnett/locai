# Locai Documentation

## Overview

This directory contains technical documentation for the Locai project. For general information and quick start guides, see the main [README.md](../README.md).

## Documentation Structure

### Core Documentation

- [**Architecture Overview**](ARCHITECTURE.md) - System design and component architecture
- [**API Reference**](API.md) - HTTP API endpoints and usage examples
- [**Search Architecture**](SEARCH.md) - BM25 search implementation and capabilities
- [**Entity Extraction**](ENTITY_EXTRACTION.md) - Entity extraction pipeline architecture
- [**Feature Flags**](FEATURES.md) - Compilation features and configuration options
- [**Changelog**](CHANGELOG.md) - Project history and version changes

### Architecture Decision Records

- [**Architecture Decisions**](architecture/) - ADRs documenting major architectural decisions

### User Guides

- [**Docker Quick Start**](guides/DOCKER_QUICK_START.md) - Get started with Docker in under 2 minutes
- [**Docker Deployment**](guides/DOCKER.md) - Complete Docker build and deployment guide
- [**Docker CLI**](guides/DOCKER_CLI.md) - Using the Locai CLI in Docker containers
- [**Data Directory Guide**](guides/DATA_DIRECTORY_GUIDE.md) - Configuring data storage locations
- [**Temporal Features**](guides/TEMPORAL_FEATURES.md) - Using temporal search and graph analysis
- [**Lifecycle Tracking**](guides/LIFECYCLE_TRACKING.md) - Configuring and using memory lifecycle tracking
- [**Batch Operations**](guides/BATCH_OPERATIONS.md) - Using the batch API for bulk operations
- [**Enhanced Search**](guides/ENHANCED_SEARCH.md) - Customizing search scoring and ranking

### Development

- [**Design Document**](DESIGN.md) - Detailed technical design and implementation notes
- [**Release Process**](RELEASE.md) - Release automation and versioning strategy
- [**Live Queries**](LIVE_QUERIES.md) - Real-time subscription system design

## Getting Started

For developers:
1. Read the [Architecture Overview](ARCHITECTURE.md) to understand the system design
2. Review the [API Reference](API.md) for integration details
3. Check [Feature Flags](FEATURES.md) for build configuration

For contributors:
1. Study the [Design Document](DESIGN.md) for implementation details
2. Follow the [Release Process](RELEASE.md) for version management
3. Review specific component documentation as needed

## Additional Resources

- [Examples](../examples/) - Code examples and usage patterns
- [Tests](../locai/tests/) - Test cases demonstrating functionality
- [Benchmarks](../benches/) - Performance benchmarks

## Documentation Standards

When contributing documentation:
- Use clear, concise technical language
- Include code examples where applicable
- Keep formatting consistent
- Update the index when adding new documents
- Avoid unnecessary emoji or decorative elements

## API Documentation

Interactive API documentation is available when running the server:
```bash
cargo run --bin locai-server
# Visit http://localhost:3000/docs
```

## Questions?

For questions not covered in the documentation:
1. Check the [GitHub Issues](https://github.com/BoundlessStudio/locai/issues)
2. Review the source code documentation
3. Open a new issue for clarification 