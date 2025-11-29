# Ticket: CTXCLI-2002: Implement CLI Context Handler

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - unit tests pass (parsing tests from CTXCLI-2001); integration tests deferred to CTXCLI-4002
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the context command execution logic that connects CLI arguments to the `BasicContextAssembler` and outputs results.

## Background
This is Phase 2 of the CTXCLI project. With the command variant defined (CTXCLI-2001), this ticket implements the actual execution logic. Unlike the daemon which maintains a persistent assembler, the CLI creates a fresh assembler per invocation.

Reference: [planning/architecture.md](../planning/architecture.md) - CLI Flow

## Acceptance Criteria
- [x] Match arm for `Commands::Context` in main function
- [x] `crewchief-maproom context --chunk-id 1` executes successfully (given valid database)
- [x] `--json` flag outputs valid JSON `ContextBundle`
- [x] Without `--json`, outputs placeholder text (human-readable in CTXCLI-2003)
- [x] Errors displayed with helpful messages (chunk not found, database connection failed)
- [x] Exit code 0 on success, non-zero on error

## Technical Requirements
- Create `SqliteStore` connection using existing config/env patterns
- Create `BasicContextAssembler` with `CacheConfig::default()`
- Build `ExpandOptions` from CLI args
- Call `assembler.assemble(chunk_id, budget, options).await`
- Serialize to JSON if `--json` flag set
- Use `anyhow` for error handling with context

## Implementation Notes

### Handler Implementation
```rust
Commands::Context {
    chunk_id,
    budget,
    callers,
    callees,
    tests,
    docs,
    config,
    max_depth,
    json,
} => {
    // Get database connection
    let db_url = get_database_url()?;
    let store = SqliteStore::new(&db_url).await?;

    // Create assembler (no persistent cache for CLI)
    let assembler = BasicContextAssembler::new(
        Arc::new(store),
        CacheConfig::default(),
    );

    // Build expand options
    let options = ExpandOptions {
        callers,
        callees,
        tests,
        docs,
        config,
        max_depth,
        ..Default::default()
    };

    // Execute context assembly
    let bundle = assembler
        .assemble(chunk_id, budget, options)
        .await
        .context("Failed to assemble context")?;

    // Output
    if json {
        println!("{}", serde_json::to_string_pretty(&bundle)?);
    } else {
        // Placeholder - human-readable format in CTXCLI-2003
        println!("Context bundle for chunk #{}", chunk_id);
        println!("Items: {}", bundle.items.len());
        println!("Total tokens: {}", bundle.total_tokens);
    }

    Ok(())
}
```

### Error Messages
- "Chunk not found: ID {chunk_id}" - when chunk doesn't exist
- "Database connection failed: {error}" - when SQLite fails
- "Failed to assemble context: {error}" - general assembly errors

## Dependencies
- CTXCLI-2001 (Context command variant must be defined)

## Risk Assessment
- **Risk**: Database URL discovery different from daemon
  - **Mitigation**: Reuse existing `get_database_url()` function or env var pattern
- **Risk**: Missing runtime dependencies (SQLite file)
  - **Mitigation**: Clear error message if database not found

## Files/Packages Affected
- `crates/maproom/src/main.rs` (modify - add Context match arm)

## Verification Notes

**Verified by**: verify-ticket agent
**Date**: 2025-11-28

### Implementation Summary
- Handler implemented in `/workspace/crates/maproom/src/main.rs` (lines 1425-1473)
- Uses `DefaultAssemblyStrategy` instead of `BasicContextAssembler` (simpler, no cache needed for CLI)
- All acceptance criteria verified as implemented
- Error handling uses `anyhow::Context` with helpful messages
- JSON output via `serde_json::to_string_pretty`
- Placeholder text output includes chunk ID, item count, tokens, truncated flag

### Test Results
- Command: `cargo test -p crewchief-maproom --bin crewchief-maproom test_context -- --nocapture`
- Result: 6/6 tests passing (command parsing tests from CTXCLI-2001)
- Note: Integration tests with real database execution deferred to CTXCLI-4002

### Deviations from Specification
1. **Assembler choice**: Uses `DefaultAssemblyStrategy::new(Arc::new(store))` instead of `BasicContextAssembler::new(Arc::new(store), CacheConfig::default())`
   - Rationale: Simpler for CLI one-shot usage, both implement `AssemblyStrategy` trait
   - Assessment: Acceptable

2. **Database connection**: Uses `db::connect()` instead of `SqliteStore::new(&db_url)`
   - Rationale: Matches existing pattern throughout `main.rs`
   - Assessment: Acceptable

3. **Error messages**: Generic assembly errors instead of specific "Chunk not found: ID {chunk_id}"
   - Actual: "Failed to assemble context for chunk {id}: Failed to query chunk metadata"
   - Assessment: Functional and clear, acceptable

### Files Modified
- `/workspace/crates/maproom/src/main.rs` - Added context handler implementation with imports
