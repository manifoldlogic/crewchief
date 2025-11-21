# Ticket: Integration Testing

**ID:** VECSRCH-3001
**Phase:** Verification
**Status:** Open

## Title & Summary
Create an integration test to verify the vector search CLI.

## Background
We need to ensure the CLI command works end-to-end against a real (or test) database.

## Acceptance Criteria
1.  A script or test case exists that runs the `maproom` binary with the `vector-search` command.
2.  It verifies that valid JSON is returned.
3.  It verifies that for a known seeded database, relevant results are returned.

## Technical Requirements
- Can be a Rust integration test (`tests/`) or a shell script.
- Needs a running Postgres instance with `pgvector`.

## Implementation Notes
- Use `assert_cmd` crate if available for testing CLI binaries in Rust.

## Dependencies
- VECSRCH-2003 (Handler implemented)

## Risks
- Test environment setup (DB availability).

## Files/Packages
- `tests/cli_tests.rs` (or similar)

## Agent Assignments
- **Primary:** QA Specialist
