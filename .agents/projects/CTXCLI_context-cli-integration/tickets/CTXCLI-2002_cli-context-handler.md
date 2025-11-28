# Ticket: CTXCLI-2002: Implement CLI Context Handler

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

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
- [ ] Match arm for `Commands::Context` in main function
- [ ] `crewchief-maproom context --chunk-id 1` executes successfully (given valid database)
- [ ] `--json` flag outputs valid JSON `ContextBundle`
- [ ] Without `--json`, outputs placeholder text (human-readable in CTXCLI-2003)
- [ ] Errors displayed with helpful messages (chunk not found, database connection failed)
- [ ] Exit code 0 on success, non-zero on error

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
