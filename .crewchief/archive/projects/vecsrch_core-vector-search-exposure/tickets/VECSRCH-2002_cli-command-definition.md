# Ticket: Implement CLI Command Definition

**ID:** VECSRCH-2002
**Phase:** Implementation
**Status:** ✅ Completed
**Completed:** 2025-11-21

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

---

## Completion Notes

**Implementation Summary:**

Added `VectorSearch` command to the CLI (src/main.rs):

**Command Definition (lines 175-204):**
```rust
VectorSearch {
    repo: String,                // Required: repository name
    worktree: Option<String>,    // Optional: worktree filter
    query: String,               // Required: search query text
    k: usize,                    // Optional: default 10
    threshold: Option<f32>,      // Optional: similarity threshold (0.0-1.0)
}
```

**Handler Stub (lines 980-996):**
- Added placeholder handler to satisfy exhaustive match
- Returns error message pointing to VECSRCH-2003
- Displays all parameters for debugging

**Acceptance Criteria Met:**

✅ 1. CLI accepts new `vector-search` command
   - Command enum variant added: `Commands::VectorSearch`
   - Includes comprehensive doc comments with examples

✅ 2. Command accepts query string argument
   - `query: String` parameter (required)

✅ 3. Command accepts optional arguments for k and threshold
   - `k: usize` with default value of 10
   - `threshold: Option<f32>` for similarity filtering

**Additional Parameters (from ticket review):**
✅ Added `repo: String` (required for filtering)
✅ Added `worktree: Option<String>` (optional for filtering)

**Files Modified:**
- `crates/maproom/src/main.rs`:
  - Lines 175-204: Command definition
  - Lines 980-996: Placeholder handler

**Usage Examples:**
```bash
maproom vector-search --repo myproject --query "authentication logic"
maproom vector-search --repo myproject --worktree main --query "error handling" --k 20
maproom vector-search --repo myproject --query "database migration" --threshold 0.7
```

**Next Steps:**
- VECSRCH-2003 will implement the actual handler logic
- Handler will generate embeddings and execute vector search
- JSON output will follow schema defined in VECSRCH-2003
