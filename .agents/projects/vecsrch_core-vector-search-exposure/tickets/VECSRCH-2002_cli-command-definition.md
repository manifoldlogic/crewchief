# Ticket: Implement CLI Command Definition

**ID:** VECSRCH-2002
**Phase:** Implementation
**Status:** Open

## Title & Summary
Implement CLI command definition for `vector-search`.

## Background
We need to add a new subcommand to the existing `clap` CLI to trigger the vector search.

## Acceptance Criteria
1.  The CLI accepts a new command (e.g., `vector-search` or `search --vector`).
2.  The command accepts a query string argument.
3.  The command accepts optional arguments for `k` (limit) and `threshold`.

## Technical Requirements
- Update the `Commands` enum in `crates/maproom/src/main.rs` (or wherever `clap` structs are defined).
- Define a new struct/variant for the vector search arguments.

## Implementation Notes
- Look at existing `Search` command in `src/main.rs` for inspiration.
- Maybe extend the existing `Search` command with a `--vector` flag instead of a new command, if that fits the UX better. (Architecture doc suggested `vector-search` or `search` with flag). Let's go with a distinct subcommand or flag that clearly distinguishes it from FTS.
- *Decision:* Let's add a `VectorSearch` subcommand to be explicit for now, or check if `Search` can be easily adapted. The plan says "include a vector-search (or simply search with a flag)". Let's add `VectorSearch` to avoid breaking existing `Search` behavior for now.

## Dependencies
- VECSRCH-2001 (for types, though not strictly needed just for the CLI struct definition).

## Risks
- Breaking changes to existing CLI structure (unlikely if adding a new variant).

## Files/Packages
- `crates/maproom/src/main.rs`

## Agent Assignments
- **Primary:** Rust Developer
