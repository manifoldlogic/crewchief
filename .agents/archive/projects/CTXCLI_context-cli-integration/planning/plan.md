# Implementation Plan: Context CLI Integration

## Overview

This plan outlines the implementation phases for exposing the Rust context assembler via CLI command and JSON-RPC daemon, enabling the MCP server to use the unified SQLite-based context assembly.

## Phase 1: Daemon Context Method (Foundation)

### CTXCLI-1001: Add Context Params Types

**Description:** Add `ContextParams` and `ExpandConfig` types to daemon types module.

**Files:**
- `crates/maproom/src/daemon/types.rs`

**Tasks:**
1. Add `ContextParams` struct with chunk_id, budget_tokens, expand
2. Add `ExpandConfig` struct with all expand options
3. Add default value functions
4. Add unit tests for deserialization

**Acceptance Criteria:**
- [ ] `ContextParams` deserializes from JSON correctly
- [ ] Default values applied when fields missing
- [ ] All expand options supported (including React-specific)

---

### CTXCLI-1002: Implement Daemon Context Handler with State Support

**Description:** Add `context` method handler to JSON-RPC daemon with `BasicContextAssembler` in `DaemonState` to enable caching across requests.

> **Note:** This ticket combines the original CTXCLI-1002 (handler) and CTXCLI-1003 (state support) to ensure proper initialization order. The assembler must be in DaemonState *before* the handler is called to enable caching.

**Files:**
- `crates/maproom/src/daemon/mod.rs`

**Tasks:**
1. Add `BasicContextAssembler` to `DaemonState` struct
2. Initialize assembler with `CacheConfig::default()` in `DaemonState::new()`
3. Share `SqliteStore` between search and context assembler
4. Add `context` case to `handle_request()` match
5. Implement `execute_context()` function using `state.assembler`
6. Convert `ContextParams` to `ExpandOptions`
7. Call `assembler.assemble()` on the state's assembler (not a new instance)
8. Serialize result to JSON-RPC response
9. Handle errors with appropriate codes

**Acceptance Criteria:**
- [ ] `BasicContextAssembler` is in `DaemonState` struct
- [ ] Assembler reuses database connection from `SqliteStore`
- [ ] Context cache persists across requests (verified by test)
- [ ] Daemon responds to `context` method
- [ ] Returns valid `ContextBundle` JSON
- [ ] Returns -32000 error for missing chunk
- [ ] Returns -32602 error for invalid params
- [ ] No performance regression for search

**Dependencies:** CTXCLI-1001

---

## Phase 2: CLI Context Command

### CTXCLI-2001: Add Context Command Variant

**Description:** Add `Context` command to CLI enum with all arguments.

**Files:**
- `crates/maproom/src/main.rs`

**Tasks:**
1. Add `Context` variant to `Commands` enum
2. Add all CLI arguments (chunk_id, budget, callers, callees, etc.)
3. Add `--json` flag for machine-readable output

**Acceptance Criteria:**
- [ ] `crewchief-maproom context --help` shows all options
- [ ] Arguments parse correctly
- [ ] Default values applied

---

### CTXCLI-2002: Implement Context Command Handler

**Description:** Implement the context command execution logic.

**Files:**
- `crates/maproom/src/main.rs`

**Tasks:**
1. Add match arm for `Commands::Context`
2. Create `SqliteStore` connection
3. Create `BasicContextAssembler`
4. Build `ExpandOptions` from CLI args
5. Call `assembler.assemble()`
6. Format and print output (text or JSON)

**Acceptance Criteria:**
- [ ] `crewchief-maproom context --chunk-id 1` works
- [ ] `--json` outputs valid JSON
- [ ] Errors displayed with helpful messages

**Dependencies:** CTXCLI-2001

---

### CTXCLI-2003: Add Human-Readable Output Format

**Description:** Implement pretty-printed output for CLI context command.

**Files:**
- `crates/maproom/src/main.rs`

**Tasks:**
1. Create `format_context_bundle()` function
2. Print primary chunk with syntax highlighting
3. Print related items grouped by role
4. Show token summary

**Example Output:**
```
📦 Context Bundle for chunk #12345
   Budget: 6000 tokens | Used: 2450 tokens | Truncated: No

📄 PRIMARY: src/auth.ts:10-30 (authenticate)
   ─────────────────────────────────────────
   async function authenticate(user: User) {
     const token = await generateToken(user);
     return { token, user };
   }
   ─────────────────────────────────────────
   Tokens: 150

🔗 CALLER: src/login.ts:40-60 (login)
   Reason: Calls authenticate function
   Tokens: 120

🧪 TEST: src/__tests__/auth.test.ts:5-25 (authenticate tests)
   Reason: Test file for primary function
   Tokens: 200
```

**Acceptance Criteria:**
- [ ] Output is readable and well-formatted
- [ ] Token counts displayed
- [ ] Roles clearly indicated

**Dependencies:** CTXCLI-2002

---

## Phase 3: MCP Integration

### CTXCLI-3001: Update MCP Context Schema

**Description:** Extend MCP context schema with React-specific options, ensuring full parity with Rust `ExpandOptions`.

**Files:**
- `packages/maproom-mcp/src/tools/context_schema.ts`

**Tasks:**
1. Add `hooks`, `jsx_parents`, `jsx_children` to expand schema
2. Update Zod validation
3. Update TypeScript types
4. Add comment referencing Rust `ExpandOptions` location for future sync

**Acceptance Criteria:**
- [ ] Schema accepts React-specific options
- [ ] Validation rejects invalid values
- [ ] Types match Rust `ExpandOptions` exactly (callers, callees, tests, docs, config, max_depth, hooks, jsx_parents, jsx_children)
- [ ] Cross-reference comment added: `// Sync with: crates/maproom/src/context/types.rs ExpandOptions`

---

### CTXCLI-3002: Replace PostgreSQL with Daemon Client

**Description:** Update MCP context tool to use daemon client instead of PostgreSQL, including adding the `DaemonClient.context()` method and mapping the Rust response to MCP format.

> **Note:** Follow the pattern established in `search.ts` for daemon client integration, including error handling for `DaemonStartError`, `DaemonTimeoutError`, and `RpcError`.

**Files:**
- `packages/daemon-client/src/index.ts` (add `context()` method)
- `packages/maproom-mcp/src/tools/context.ts`

**Tasks:**
1. Add `context()` method to `DaemonClient` class in daemon-client package
2. Define TypeScript types for `ContextParams` and `ContextBundle`
3. Import `DaemonClient` from daemon-client package in context.ts
4. Remove `pg` client usage
5. Call `daemonClient.context(params)`
6. Map Rust `ContextBundle` response to MCP format:
   - Pass through: `items`, `total_tokens`, `truncated`
   - Compute: `budget_tokens` (from request params)
   - Compute: `budget_remaining` = `budget_tokens - total_tokens`
   - Add: `metadata` object (worktree info from first item)
7. Update error handling following `search.ts` pattern

**Acceptance Criteria:**
- [ ] `DaemonClient.context()` method exists in daemon-client package
- [ ] MCP context tool uses daemon (not PostgreSQL)
- [ ] Response format matches existing MCP ContextBundle interface:
  - `items`, `total_tokens`, `budget_tokens`, `budget_remaining`, `truncated`, `metadata`
- [ ] Error handling follows `search.ts` pattern
- [ ] Error messages match existing format

**Dependencies:** CTXCLI-3001, CTXCLI-1002

---

### CTXCLI-3003: Add Daemon Client to MCP Server

**Description:** Ensure daemon client is initialized and passed to context handler.

**Files:**
- `packages/maproom-mcp/src/index.ts`

**Tasks:**
1. Import daemon client from existing setup
2. Pass to `handleContextTool()` function
3. Handle daemon connection errors

**Acceptance Criteria:**
- [ ] Daemon client available in context handler
- [ ] Graceful error if daemon not running

**Dependencies:** CTXCLI-3002

---

## Phase 4: Testing & Polish

### CTXCLI-4001: Add Daemon Context Integration Tests

**Description:** Add integration tests for daemon context method, including creating the test database fixture.

**Files:**
- `crates/maproom/tests/context_daemon_test.rs`
- `crates/maproom/tests/fixtures/context_test.sql` (create)

**Tasks:**
1. Create test database fixture (`tests/fixtures/context_test.sql`):
   - Insert test repository and worktree
   - Insert test file and chunks (primary, callers, callees, tests)
   - Insert chunk_edges for relationship testing
2. Create test helper to load fixture into in-memory SQLite
3. Test successful context retrieval
4. Test expand options (callers, callees, tests, docs, config)
5. Test cache persistence across requests (call twice, verify faster second call)
6. Test error cases (missing chunk, invalid params)

**Acceptance Criteria:**
- [ ] Test fixture file `context_test.sql` created and committed
- [ ] Tests pass in CI
- [ ] Coverage > 75% for handler
- [ ] Cache persistence verified by test

**Dependencies:** CTXCLI-1002

---

### CTXCLI-4002: Add CLI Context Integration Tests

**Description:** Add integration tests for CLI context command.

**Files:**
- `crates/maproom/tests/context_cli_test.rs`

**Tasks:**
1. Test CLI argument parsing
2. Test command execution
3. Test JSON output format
4. Test error messages

**Acceptance Criteria:**
- [ ] Tests pass in CI
- [ ] All CLI options tested

**Dependencies:** CTXCLI-2003

---

### CTXCLI-4003: Add MCP Context E2E Tests

**Description:** Add E2E tests for MCP context tool via daemon.

**Files:**
- `packages/maproom-mcp/tests/context.e2e.test.ts`

**Tasks:**
1. Start daemon and MCP server in test
2. Test context retrieval
3. Test React-specific options
4. Test error handling

**Acceptance Criteria:**
- [ ] Tests pass in CI
- [ ] Round-trip < 200ms

**Dependencies:** CTXCLI-3003

---

### CTXCLI-4004: Documentation and CLAUDE.md Updates

**Description:** Update documentation for new context command.

**Files:**
- `crates/maproom/CLAUDE.md`
- `packages/maproom-mcp/CLAUDE.md`

**Tasks:**
1. Document CLI context command usage
2. Document MCP context tool changes
3. Add troubleshooting for common errors

**Acceptance Criteria:**
- [ ] CLAUDE.md updated with context command
- [ ] MCP tool documentation current

**Dependencies:** All previous tickets

---

## Ticket Summary

| Phase | Ticket | Description | Effort |
|-------|--------|-------------|--------|
| 1 | CTXCLI-1001 | Context params types | S |
| 1 | CTXCLI-1002 | Daemon context handler + state support | M |
| 2 | CTXCLI-2001 | CLI context command variant | S |
| 2 | CTXCLI-2002 | CLI context handler | M |
| 2 | CTXCLI-2003 | Human-readable output | S |
| 3 | CTXCLI-3001 | MCP context schema update | S |
| 3 | CTXCLI-3002 | Replace PostgreSQL with daemon + DaemonClient.context() | M |
| 3 | CTXCLI-3003 | Daemon client in MCP server | S |
| 4 | CTXCLI-4001 | Daemon integration tests + test fixture | M |
| 4 | CTXCLI-4002 | CLI integration tests | M |
| 4 | CTXCLI-4003 | MCP E2E tests | M |
| 4 | CTXCLI-4004 | Documentation updates | S |

**Total: 12 tickets** (5 Small, 7 Medium)

> **Note:** Original CTXCLI-1002 and CTXCLI-1003 merged per review findings to fix initialization order.

## Execution Order

1. **Foundation**: CTXCLI-1001 → CTXCLI-1002
2. **CLI**: CTXCLI-2001 → CTXCLI-2002 → CTXCLI-2003
3. **MCP**: CTXCLI-3001 → CTXCLI-3002 → CTXCLI-3003
4. **Testing**: CTXCLI-4001, CTXCLI-4002, CTXCLI-4003 (parallel)
5. **Documentation**: CTXCLI-4004

## Success Metrics

- [ ] `crewchief-maproom context --chunk-id <id>` works
- [ ] MCP `context` tool uses daemon (no PostgreSQL)
- [ ] Context assembly < 100ms (cached)
- [ ] All tests pass in CI
- [ ] Documentation updated
