# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) documenting major architectural decisions made in the Locai project.

## What are ADRs?

ADRs are documents that capture important architectural decisions along with their context and consequences. They serve as a historical record of why certain design choices were made.

## ADR Format

Each ADR follows a standard format:
- **Status**: Draft, Proposed, Accepted, Deprecated, or Superseded
- **Context**: The situation that led to the decision
- **Decision**: The architectural choice made
- **Consequences**: The positive and negative impacts

## Current ADRs

- [**ADR-001: Memory Lifecycle & Extensibility Enhancements**](ADR-001-memory-lifecycle-extensibility.md) - RFC 001 implementation decisions for memory lifecycle tracking, relationship registry, hooks, batch operations, and enhanced search scoring

## Guidelines

- **Never update ADRs** - They are historical records
- If an ADR is superseded, create a new ADR and link them
- Keep ADRs focused on architectural decisions, not implementation details
- Update this README when adding new ADRs

