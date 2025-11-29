# Ticket: TESTENV-1001: Design test corpus with known query results

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (design/documentation ticket)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: N/A - This is a design/documentation ticket. Validation occurs when fixtures are generated in TESTENV-1003.

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary
Create test corpus source files and document the expected query→result mappings that will be used for deterministic search testing.

## Background
Currently 5 integration tests fail because they require indexed data that doesn't exist. This ticket establishes the foundation for SQL test fixtures by:
1. Creating source files that will be indexed into the test database
2. Documenting exactly what search results each query should return
3. Ensuring diverse language coverage (TypeScript, Python, Rust, Markdown)

Reference: [plan.md](../planning/plan.md) - Phase 1, Deliverable 1: "Test Corpus Design Document"

## Acceptance Criteria
- [ ] Test corpus directory created at `packages/maproom-mcp/tests/corpus/`
- [ ] 7 source files created across 4 languages (TypeScript, Python, Rust, Markdown)
- [ ] Each file contains well-defined symbols matching the query→result matrix
- [ ] `README.md` in corpus directory documents all 12 query→result pairs
- [ ] Source files are syntactically valid (can be parsed by tree-sitter)

## Technical Requirements

### Corpus Directory Structure
```
packages/maproom-mcp/tests/corpus/
├── typescript/
│   ├── auth-service.ts      # AuthService class with authenticate(), validateToken()
│   └── database-client.ts   # DatabaseClient class with connect(), query()
├── python/
│   ├── validate_token.py    # validate_token() function, TokenValidator class
│   └── user_service.py      # UserService class with get_user(), create_user()
├── rust/
│   ├── database.rs          # DatabaseConnection struct, impl Connection
│   └── config.rs            # Config struct, load_config() function
├── markdown/
│   └── api-docs.md          # API documentation with headings and code blocks
└── README.md                # Query→result documentation
```

### Query→Result Matrix (Minimum 12 Pairs)
| # | Query | Expected Top Result | Match Type |
|---|-------|-------------------|------------|
| 1 | `authenticate` | `AuthService.authenticate()` | Exact |
| 2 | `user authentication` | `AuthService` class | Conceptual |
| 3 | `validate_token` | `validate_token()` Python | Exact (snake) |
| 4 | `validateToken` | `validateToken()` TS | Exact (camel) |
| 5 | `DatabaseConnection` | `DatabaseConnection` struct | Exact |
| 6 | `connect to database` | `DatabaseClient.connect()` | Conceptual |
| 7 | `query data` | `DatabaseClient.query()` | Conceptual |
| 8 | `user CRUD` | `UserService` class | Conceptual |
| 9 | `configuration loading` | `load_config()` | Conceptual |
| 10 | `API documentation` | `api-docs.md` heading | Document |
| 11 | `get user by id` | `get_user()` | Conceptual |
| 12 | `impl Connection` | `impl Connection for DatabaseConnection` | Exact |

### Source File Requirements
- **TypeScript files**: Valid ES module syntax, exported classes/functions
- **Python files**: Valid Python 3 syntax, module-level functions and classes
- **Rust files**: Valid Rust syntax with structs, impls, functions
- **Markdown file**: Multiple headings (h2, h3), code blocks, documentation content

## Implementation Notes

1. **Keep files minimal** - Each file should be ~20-50 lines. Focus on symbol definitions, not implementation complexity.

2. **Use descriptive content** - Function bodies should contain relevant keywords that help with conceptual matching (e.g., `authenticate()` might mention "user", "password", "token").

3. **Match existing patterns** - Review `crates/maproom/tests/fixtures/` for examples of what tree-sitter parses well.

4. **Ensure parseability** - All files must be parseable by tree-sitter. Test with:
   ```bash
   tree-sitter parse packages/maproom-mcp/tests/corpus/typescript/auth-service.ts
   ```

## Dependencies
- None (first ticket in Phase 1)

## Risk Assessment
- **Risk**: Corpus files may not produce expected rankings
  - **Mitigation**: Document ranking expectations clearly; adjust files in TESTENV-1003 if needed
- **Risk**: Tree-sitter parsing differences between languages
  - **Mitigation**: Keep syntax simple and standard; avoid edge cases

## Files/Packages Affected
- `packages/maproom-mcp/tests/corpus/` (NEW directory)
- `packages/maproom-mcp/tests/corpus/typescript/auth-service.ts` (NEW)
- `packages/maproom-mcp/tests/corpus/typescript/database-client.ts` (NEW)
- `packages/maproom-mcp/tests/corpus/python/validate_token.py` (NEW)
- `packages/maproom-mcp/tests/corpus/python/user_service.py` (NEW)
- `packages/maproom-mcp/tests/corpus/rust/database.rs` (NEW)
- `packages/maproom-mcp/tests/corpus/rust/config.rs` (NEW)
- `packages/maproom-mcp/tests/corpus/markdown/api-docs.md` (NEW)
- `packages/maproom-mcp/tests/corpus/README.md` (NEW)
