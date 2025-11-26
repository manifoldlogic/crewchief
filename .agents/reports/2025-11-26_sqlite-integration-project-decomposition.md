# SQLite Integration Project Decomposition

**Date**: 2025-11-26
**Source**: `2025-11-26_sqlite-project-integration-impact-analysis.md`
**Framework**: `.agents/reference/project-boundry-evaluation.md`
**Purpose**: Break down the proposed SQLITE-INTEGRATION work into well-bounded projects suitable for agent-based development

---

## Executive Summary

The original integration impact analysis proposed a single **SQLITE-INTEGRATION** project spanning 18-24 days across 6 components. Applying the Project Boundary Framework reveals this violates **Context Coherence** (spans Rust/TypeScript/config across 4+ architectural layers) and has **Interface Stability** concerns (VectorStore trait must be stabilized first).

**Recommendation**: Split into **5 discrete projects** with clear dependency ordering:

| Project | Priority | Estimate | Dependencies |
|---------|----------|----------|--------------|
| VECSTORE | Critical | 5-7 days | None (foundation) |
| MAPROOMCLI | High | 4-5 days | VECSTORE |
| MCPDB | Medium | 2-3 days | MAPROOMCLI |
| VSCODEDB | Medium | 3-4 days | MAPROOMCLI |
| SQLITEINFRA | Low | 2-3 days | All above |

**Total**: 16-22 days (similar effort, better agent effectiveness)

---

## Analysis of Original Proposal

### Original SQLITE-INTEGRATION Project

The impact analysis proposed:
- 4 phases, 12 tickets
- Touches: Rust daemon, Rust CLI, PostgreSQL queries, VSCode extension (TypeScript), MCP server (TypeScript), Docker configs, CI/CD, documentation
- Estimate: 18-24 days

### Framework Evaluation

```yaml
## Core Requirements Assessment

interface_stability:
  ☐ All external APIs documented       # VectorStore trait incomplete
  ☐ Data formats finalized             # ✅ SQLite schema is done
  ☐ Integration points stable          # ❌ Trait needs new methods
  ☐ No expected interface changes      # ❌ Trait will grow during project

context_coherence:
  ☐ Project explainable in <500 words  # ❌ Too many domains
  ☐ Less than 20 domain concepts       # ❌ ~30+ (Rust+TS+Docker+CI)
  ☐ Tightly clustered codebase         # ❌ 4+ separate packages
  ☐ Single area of architecture        # ❌ Spans all layers

testable_completion:
  ☐ Measurable success criteria        # ✅ "SQLite works E2E"
  ☐ Automated test suite possible      # ✅ Can test SQLite flow
  ☐ Binary pass/fail determination     # ✅
  ☐ No subjective requirements         # ✅
```

**Verdict**: Fails 2 of 3 core criteria. Must be split.

### Anti-Patterns Detected

1. **The Kitchen Sink** - "Update ~50+ files across 6 components" is too broad
2. **The Scatter Shot** - Changes spread across Rust, TypeScript, Docker, CI/CD with no unified context

---

## Proposed Project Decomposition

### Project 1: VECSTORE - VectorStore Trait Completion

**Pattern**: Capability Layer
**Priority**: CRITICAL (Foundation - blocks all other work)

#### Summary
Complete the VectorStore trait abstraction by moving all PostgreSQL-specific queries into trait methods. This stabilizes the interface that all other projects depend on.

#### Boundary Evaluation

```yaml
interface_stability:
  ✅ All external APIs documented       # Trait is the API
  ✅ Data formats finalized             # SQLite schema locked
  ✅ Integration points stable          # Defining the stable interface
  ✅ No expected interface changes      # This project defines them

context_coherence:
  ✅ Project explainable in <500 words  # "Complete VectorStore trait"
  ✅ Less than 20 domain concepts       # ~12 (Store, query types)
  ✅ Tightly clustered codebase         # crates/maproom/src/db/ only
  ✅ Single area of architecture        # Database layer

testable_completion:
  ✅ Measurable success criteria        # All queries go through trait
  ✅ Automated test suite possible      # Test both backends
  ✅ Binary pass/fail determination     # Trait has all methods
  ✅ No subjective requirements         # Yes
```

#### Scope

**In Scope:**
- Audit all direct PostgreSQL queries in codebase
- Add missing methods to `VectorStore` trait
- Implement methods in both `PostgresStore` and `SqliteStore`
- Move query implementations to `db/postgres/queries.rs`
- Verify `db/factory.rs:get_store()` returns functional store

**Out of Scope:**
- CLI command updates (MAPROOMCLI)
- Daemon restructuring (MAPROOMCLI)
- TypeScript changes (other projects)

#### Tickets (Estimated: 5-7 days)

| Ticket | Description |
|--------|-------------|
| VECSTORE-1001 | Audit and catalog all direct PostgreSQL queries |
| VECSTORE-1002 | Define complete VectorStore trait interface |
| VECSTORE-1003 | Implement search methods (FTS, vector, hybrid) in trait |
| VECSTORE-1004 | Implement context methods in trait |
| VECSTORE-1005 | Implement incremental indexing methods in trait |
| VECSTORE-1006 | Integration tests: both stores pass same test suite |

#### Success Criteria
- [ ] `cargo test --features sqlite` passes all trait tests
- [ ] `cargo test` (PostgreSQL) passes all trait tests
- [ ] No raw SQL queries outside `db/postgres/` or `db/sqlite/`
- [ ] `get_store()` returns working store for both backends

---

### Project 2: MAPROOMCLI - Maproom CLI Abstraction

**Pattern**: Service Module
**Priority**: HIGH (Enables CLI and daemon for SQLite)

#### Summary
Update the `crewchief-maproom` binary and daemon to use the `VectorStore` trait instead of direct PostgreSQL connections. All Rust code, single binary.

#### Boundary Evaluation

```yaml
interface_stability:
  ✅ All external APIs documented       # VectorStore trait (from VECSTORE)
  ✅ Data formats finalized             # Trait methods define contract
  ✅ Integration points stable          # VECSTORE must complete first
  ✅ No expected interface changes      # Trait locked after VECSTORE

context_coherence:
  ✅ Project explainable in <500 words  # "Use VectorStore in CLI/daemon"
  ✅ Less than 20 domain concepts       # ~10 (CLI commands, daemon)
  ✅ Tightly clustered codebase         # main.rs, daemon/mod.rs
  ✅ Single area of architecture        # CLI layer

testable_completion:
  ✅ Measurable success criteria        # All commands work with SQLite
  ✅ Automated test suite possible      # E2E tests with SQLite
  ✅ Binary pass/fail determination     # Commands succeed/fail
  ✅ No subjective requirements         # Yes
```

#### Scope

**In Scope:**
- Update `main.rs` to use `get_store()` instead of `db::connect()`
- Update daemon to accept `Arc<dyn VectorStore>` instead of `PgPool`
- Update `DaemonState` to be database-agnostic
- Add `--sqlite` CLI flag or auto-detect based on config
- Update all command handlers to use trait methods

**Out of Scope:**
- VectorStore trait definition (VECSTORE)
- TypeScript MCP server (MCPDB)
- VSCode extension (VSCODEDB)

#### Tickets (Estimated: 4-5 days)

| Ticket | Description |
|--------|-------------|
| MAPROOMCLI-1001 | Update main.rs to use get_store() factory |
| MAPROOMCLI-1002 | Refactor daemon to use VectorStore trait |
| MAPROOMCLI-1003 | Add SQLite backend detection/configuration |
| MAPROOMCLI-1004 | Update all CLI commands for trait-based access |
| MAPROOMCLI-1005 | E2E integration tests with SQLite backend |

#### Success Criteria
- [ ] `crewchief-maproom scan --sqlite /path/to/repo` works
- [ ] `crewchief-maproom search --sqlite "query"` returns results
- [ ] Daemon serves JSON-RPC requests using SQLite
- [ ] All existing PostgreSQL functionality unchanged

---

### Project 3: MCPDB - MCP Server SQLite Support

**Pattern**: Service Module
**Priority**: MEDIUM (Enables MCP clients to use SQLite)

#### Summary
Update the TypeScript MCP server (`packages/maproom-mcp/`) to support SQLite database URLs and file paths. Small, focused TypeScript project.

#### Boundary Evaluation

```yaml
interface_stability:
  ✅ All external APIs documented       # Daemon JSON-RPC (from MAPROOMCLI)
  ✅ Data formats finalized             # Database URL formats
  ✅ Integration points stable          # MAPROOMCLI daemon works
  ✅ No expected interface changes      # URL parsing is additive

context_coherence:
  ✅ Project explainable in <500 words  # "Support sqlite:// URLs in MCP"
  ✅ Less than 20 domain concepts       # ~6 (URL parsing, config)
  ✅ Tightly clustered codebase         # packages/maproom-mcp/src/
  ✅ Single area of architecture        # MCP layer only

testable_completion:
  ✅ Measurable success criteria        # MCP tools work with SQLite
  ✅ Automated test suite possible      # Test URL parsing
  ✅ Binary pass/fail determination     # MCP calls succeed
  ✅ No subjective requirements         # Yes
```

#### Scope

**In Scope:**
- Update `resolve-database.ts` to support `sqlite://` scheme
- Update `daemon.ts` to handle SQLite file paths
- Add SQLite-based test helpers (no `pg` dependency in SQLite tests)
- Update environment variable handling

**Out of Scope:**
- Daemon implementation (MAPROOMCLI)
- VectorStore trait (VECSTORE)
- VSCode extension (VSCODEDB)

#### Tickets (Estimated: 2-3 days)

| Ticket | Description |
|--------|-------------|
| MCPDB-1001 | Update URL parsing for sqlite:// scheme |
| MCPDB-1002 | Add SQLite file path detection in daemon.ts |
| MCPDB-1003 | Create SQLite test helpers |
| MCPDB-1004 | Integration tests with SQLite backend |

#### Success Criteria
- [ ] `MAPROOM_DATABASE_URL=sqlite:///path/to/db.sqlite` works
- [ ] Auto-detection of `~/.maproom/maproom.db` works
- [ ] MCP tools (`search`, `status`, `open`) work with SQLite
- [ ] Tests pass without PostgreSQL service

---

### Project 4: VSCODEDB - VSCode Extension Database Modernization

**Pattern**: Service Module
**Priority**: MEDIUM (Simplifies extension setup)

#### Summary
Update the VSCode extension to support SQLite as the default database, removing the PostgreSQL requirement for basic usage. TypeScript-only project.

#### Boundary Evaluation

```yaml
interface_stability:
  ✅ All external APIs documented       # VSCode Extension API stable
  ✅ Data formats finalized             # SQLite file path config
  ✅ Integration points stable          # Daemon (MAPROOMCLI) works
  ✅ No expected interface changes      # Additive config changes

context_coherence:
  ✅ Project explainable in <500 words  # "SQLite support in VSCode ext"
  ✅ Less than 20 domain concepts       # ~8 (checker, config, settings)
  ✅ Tightly clustered codebase         # packages/vscode-maproom/src/
  ✅ Single area of architecture        # VSCode extension

testable_completion:
  ✅ Measurable success criteria        # Extension works without Docker
  ✅ Automated test suite possible      # Test activation, commands
  ✅ Binary pass/fail determination     # Commands succeed
  ✅ No subjective requirements         # Yes
```

#### Scope

**In Scope:**
- Replace `postgres-checker.ts` with `database-checker.ts`
- Update extension settings for SQLite file path option
- Make Docker compose optional (not started by default)
- Update activation to check for SQLite file OR PostgreSQL

**Out of Scope:**
- Daemon changes (MAPROOMCLI)
- MCP server changes (MCPDB)
- CI/CD changes (SQLITEINFRA)

#### Tickets (Estimated: 3-4 days)

| Ticket | Description |
|--------|-------------|
| VSCODEDB-1001 | Create database-checker.ts with SQLite support |
| VSCODEDB-1002 | Update extension settings schema |
| VSCODEDB-1003 | Make Docker containers optional |
| VSCODEDB-1004 | Update activation flow for SQLite-first |
| VSCODEDB-1005 | Update extension documentation |

#### Success Criteria
- [ ] Extension activates without Docker running
- [ ] SQLite file at `~/.maproom/maproom.db` auto-detected
- [ ] Search commands work with SQLite backend
- [ ] PostgreSQL mode still works when configured

---

### Project 5: SQLITEINFRA - Infrastructure Simplification

**Pattern**: Capability Layer
**Priority**: LOW (Quality of life improvements)

#### Summary
Update CI/CD, Docker configuration, and documentation to treat SQLite as the default, zero-config option. Configuration-focused project.

#### Boundary Evaluation

```yaml
interface_stability:
  ✅ All external APIs documented       # GitHub Actions API stable
  ✅ Data formats finalized             # YAML, Markdown
  ✅ Integration points stable          # All other projects complete
  ✅ No expected interface changes      # Config changes only

context_coherence:
  ✅ Project explainable in <500 words  # "SQLite-first infrastructure"
  ✅ Less than 20 domain concepts       # ~6 (CI, Docker, docs)
  ✅ Tightly clustered codebase         # .github/, config/, docs/
  ✅ Single area of architecture        # Infrastructure layer

testable_completion:
  ✅ Measurable success criteria        # CI passes with SQLite
  ✅ Automated test suite possible      # CI itself is the test
  ✅ Binary pass/fail determination     # Workflows succeed
  ✅ No subjective requirements         # Yes
```

#### Scope

**In Scope:**
- Add SQLite-only CI test job (no service containers)
- Make PostgreSQL service container optional in CI
- Update Docker compose files with comments
- Update all documentation for SQLite-first

**Out of Scope:**
- Code changes (previous projects)
- Extension changes (VSCODEDB)
- MCP changes (MCPDB)

#### Tickets (Estimated: 2-3 days)

| Ticket | Description |
|--------|-------------|
| SQLITEINFRA-1001 | Add SQLite CI test job |
| SQLITEINFRA-1002 | Make PostgreSQL CI optional |
| SQLITEINFRA-1003 | Update Docker documentation |
| SQLITEINFRA-1004 | Update all project documentation |

#### Success Criteria
- [ ] CI passes SQLite tests without PostgreSQL service
- [ ] CI still passes PostgreSQL tests
- [ ] README shows SQLite as default getting started
- [ ] Architecture docs reflect dual-backend support

---

## Dependency Graph

```
VECSTORE (Foundation)
    │
    ▼
MAPROOMCLI (CLI/Daemon)
    │
    ├───────┬───────┐
    ▼       ▼       ▼
 MCPDB  VSCODEDB   │
    │       │       │
    └───────┴───────┘
            │
            ▼
      SQLITEINFRA
```

**Execution Order**:
1. VECSTORE (must complete first - defines interfaces)
2. MAPROOMCLI (enables all downstream work)
3. MCPDB and VSCODEDB (can run in parallel)
4. SQLITEINFRA (cleanup after all code complete)

---

## Risk Comparison

| Risk | Original (Single Project) | Decomposed (5 Projects) |
|------|---------------------------|-------------------------|
| Interface Churn | HIGH - trait changes break all work | LOW - VECSTORE locks interfaces first |
| Context Confusion | HIGH - agents lose track of Rust vs TS | LOW - each project is single-language |
| Partial Completion | HIGH - nothing works until everything works | LOW - each project delivers value |
| Testing Complexity | HIGH - must test all integrations | MEDIUM - isolated test suites |
| Agent Effectiveness | LOW - scattered context | HIGH - focused domains |

---

## Conclusion

The original SQLITE-INTEGRATION proposal should be replaced with 5 well-bounded projects:

1. **VECSTORE** - Foundation, must run first
2. **MAPROOMCLI** - Core CLI/daemon work
3. **MCPDB** - MCP TypeScript updates
4. **VSCODEDB** - VSCode extension updates
5. **SQLITEINFRA** - Infrastructure cleanup

Each project passes the Project Boundary Framework evaluation and can be executed by agents with high confidence. The total effort is similar (16-22 days vs 18-24 days), but with significantly better agent effectiveness and reduced risk of confusion or inconsistency.

**Recommended Next Steps**:
1. Create VECSTORE project using `/create-project`
2. Complete VECSTORE before starting any other project
3. Run MCPDB and VSCODEDB in parallel after MAPROOMCLI
4. Finish with SQLITEINFRA documentation pass
