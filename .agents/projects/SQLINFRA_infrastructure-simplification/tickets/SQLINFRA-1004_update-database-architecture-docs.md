# Ticket: SQLINFRA-1004: Update Database Architecture Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation only)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- This is a documentation ticket; "Tests pass - N/A" applies
- Verification is manual review of documentation accuracy and completeness

## Agents
- general-purpose
- unit-test-runner (N/A - documentation only)
- verify-ticket
- commit-ticket

## Summary
Add a comprehensive SQLite section to `docs/architecture/DATABASE_ARCHITECTURE.md`, including backend comparison and SQLite-specific troubleshooting.

## Background
The DATABASE_ARCHITECTURE.md document is a detailed 467-line reference focused entirely on PostgreSQL. With SQLite now a fully-supported backend, users need equivalent documentation for SQLite architecture, capabilities, and troubleshooting.

This ticket adds SQLite documentation while preserving all existing PostgreSQL content. The goal is a unified database architecture reference that covers both backends.

Reference: [SQLINFRA Plan - Phase 2](../planning/plan.md#phase-2-core-documentation)

## Acceptance Criteria
- [ ] SQLite section appears prominently in the document (after Overview)
- [ ] Comparison table helps users choose between SQLite and PostgreSQL
- [ ] SQLite architecture details documented (file location, schema, limitations)
- [ ] SQLite troubleshooting section available
- [ ] All existing PostgreSQL documentation remains unchanged
- [ ] Internal links work correctly

## Technical Requirements
- **New "Database Backend Options" Section** (after Overview):
  - Comparison table: SQLite vs PostgreSQL
  - When to use each backend
  - Migration considerations

- **SQLite Backend Section**:
  - Zero configuration defaults
  - File location (`~/.maproom/maproom.db`)
  - Schema overview (tables: chunks, chunk_edges, worktrees, repos)
  - Vector search via sqlite-vec (768/1536 dimensions)
  - Limitations: single-writer, no concurrent indexing, no parallel queries

- **SQLite Troubleshooting Section**:
  - Database locked errors
  - Corrupt database recovery
  - Re-indexing procedure
  - Disk space considerations

- **Preserve PostgreSQL Content**:
  - Do not modify existing PostgreSQL sections
  - Update any "PostgreSQL-only" language to "both backends" where applicable
  - Add cross-references between SQLite and PostgreSQL sections

## Implementation Notes
- File location: `docs/architecture/DATABASE_ARCHITECTURE.md`
- Current structure:
  ```
  ## Overview
  ## Database Schema
  ## Schema Details
  ## Connection Management
  ## Troubleshooting
  ```
- Target structure:
  ```
  ## Overview
  ## Database Backend Options (NEW)
  ### Comparison Table (NEW)
  ### SQLite Backend (NEW)
  ### PostgreSQL Backend (pointer to existing sections)
  ## Database Schema (existing - applies to both)
  ## Schema Details (existing)
  ## Connection Management (existing)
  ## SQLite Troubleshooting (NEW)
  ## PostgreSQL Troubleshooting (existing)
  ```
- Comparison table columns: Backend, Use Case, Setup, Performance, Limitations

## Dependencies
- None - can start immediately
- Can run in parallel with SQLINFRA-1003

## Risk Assessment
- **Risk**: Documentation becomes inconsistent between backends
  - **Mitigation**: Use unified schema documentation; clearly mark backend-specific sections
- **Risk**: Document becomes too long
  - **Mitigation**: Use expandable sections or link to separate SQLite-specific docs if needed
- **Risk**: Outdated information
  - **Mitigation**: Reference code/schema rather than duplicating implementation details

## Files/Packages Affected
- `docs/architecture/DATABASE_ARCHITECTURE.md` - Add SQLite sections
