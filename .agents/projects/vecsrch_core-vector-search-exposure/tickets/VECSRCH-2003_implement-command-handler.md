# Ticket: Implement Command Handler

**ID:** VECSRCH-2003
**Phase:** Implementation
**Status:** Open

## Title & Summary
Implement the command handler for the vector search CLI command.

## Background
Once the CLI command is defined, we need the logic to actually execute the search using `VectorExecutor` and print the results.

## Acceptance Criteria
1.  The command handler instantiates `VectorExecutor`.
2.  It executes the search using the provided query and parameters.
3.  It outputs the results in a structured JSON format to stdout.
4.  Errors are printed to stderr.

## Technical Requirements
- Use `serde_json` to serialize the output.
- Ensure the output schema is consistent and documented (for the MCP client to consume).
- Handle database connection initialization within the handler.

## Implementation Notes
- The handler should be an async function.
- Output JSON should include: `chunk_id`, `score`, `content`, `file_path`.

## Dependencies
- VECSRCH-2001 (Types exposed)
- VECSRCH-2002 (CLI definition)

## Risks
- Database connection failure.
- Slow cold start (accepted risk).

## Files/Packages
- `crates/maproom/src/main.rs`

## Agent Assignments
- **Primary:** Rust Developer
