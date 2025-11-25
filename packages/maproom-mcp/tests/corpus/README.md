# Test Corpus

This directory contains source files used to generate test fixtures for the maproom-mcp package. These files are indexed by the maproom daemon to create deterministic, searchable test data.

## Directory Structure

```
corpus/
├── typescript/
│   ├── auth-service.ts      # AuthService class with authentication methods
│   └── database-client.ts   # DatabaseClient class with query methods
├── python/
│   ├── validate_token.py    # Token validation functions and TokenValidator class
│   └── user_service.py      # UserService class with CRUD operations
├── rust/
│   ├── database.rs          # DatabaseConnection struct and Connection trait
│   └── config.rs            # Config struct and load_config function
├── markdown/
│   └── api-docs.md          # API documentation with headings and code blocks
└── README.md                # This file
```

## Query→Result Matrix

The following table documents the expected search results for each test query. These expectations are used to validate that fixtures are correctly generated.

| # | Query | Expected Top Result | Match Type | File |
|---|-------|-------------------|------------|------|
| 1 | `authenticate` | `AuthService.authenticate()` | Exact | typescript/auth-service.ts |
| 2 | `user authentication` | `AuthService` class | Conceptual | typescript/auth-service.ts |
| 3 | `validate_token` | `validate_token()` function | Exact (snake) | python/validate_token.py |
| 4 | `validateToken` | `AuthService.validateToken()` | Exact (camel) | typescript/auth-service.ts |
| 5 | `DatabaseConnection` | `DatabaseConnection` struct | Exact | rust/database.rs |
| 6 | `connect to database` | `DatabaseClient.connect()` | Conceptual | typescript/database-client.ts |
| 7 | `query data` | `DatabaseClient.query()` | Conceptual | typescript/database-client.ts |
| 8 | `user CRUD` | `UserService` class | Conceptual | python/user_service.py |
| 9 | `configuration loading` | `load_config()` function | Conceptual | rust/config.rs |
| 10 | `API documentation` | `api-docs.md` heading | Document | markdown/api-docs.md |
| 11 | `get user by id` | `UserService.get_user()` | Conceptual | python/user_service.py |
| 12 | `impl Connection` | `impl Connection for DatabaseConnection` | Exact | rust/database.rs |

## File Statistics

| Language | Files | Expected Chunks |
|----------|-------|-----------------|
| TypeScript | 2 | ~30-40 |
| Python | 2 | ~25-35 |
| Rust | 2 | ~20-30 |
| Markdown | 1 | ~10-15 |
| **Total** | **7** | **~85-120** |

## Regenerating Fixtures

When corpus files are modified, fixtures must be regenerated:

```bash
cd packages/maproom-mcp

# Ensure daemon is running
docker compose -p crewchief-dev-env --profile e2e up -d

# Generate new fixtures
./scripts/create-test-fixtures.sh

# Verify fixtures load correctly
pnpm test
```

## Design Principles

1. **Minimal Files**: Each file is 20-50 lines to keep fixtures small and fast to load.

2. **Clear Symbols**: Function and class names are descriptive and match expected query terms.

3. **Keyword-Rich Content**: Function bodies and comments contain relevant keywords for conceptual matching.

4. **Valid Syntax**: All files are syntactically valid and parseable by tree-sitter.

5. **Diverse Coverage**: Multiple languages ensure the indexer handles different grammars correctly.

## Maintenance

When adding new test cases:

1. Add the source file to the appropriate language directory
2. Update this README with the new query→result mapping
3. Regenerate fixtures using the script above
4. Verify the new query produces expected results
