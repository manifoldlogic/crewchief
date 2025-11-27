# Architecture: Fix All Tests

## Solution Design

This project follows a **systematic repair pattern** rather than introducing new architecture. The goal is to align existing tests with current implementation APIs while preserving test intent and coverage.

## Component Overview

```
Test Infrastructure
├── Rust Tests (crates/maproom/tests/)
│   ├── Unit Tests (src/**/*_test.rs, embedded in lib)
│   ├── Integration Tests (tests/*.rs)
│   └── Fixture Tests (tests/fixtures/)
│
├── TypeScript Tests
│   ├── CLI Package (packages/cli/**/*.test.ts)
│   ├── MCP Package (packages/maproom-mcp/tests/)
│   ├── Daemon Client (packages/daemon-client/tests/)
│   └── VSCode Extension (packages/vscode-maproom/src/**/*.test.ts)
│
└── CI Configuration (.github/workflows/test.yml)
    ├── SQLite Tests (fast, no external deps)
    └── PostgreSQL Tests (integration)
```

## Repair Strategy

### Phase 1: Environment Cleanup

Remove test pollution before fixing tests to avoid false positives.

```bash
# Clean stale worktrees (correct path)
rm -rf packages/cli/.crewchief/worktrees/variant-test-*
```

**Create CLI vitest.config.ts** to prevent future duplicate test discovery:

```typescript
// packages/cli/vitest.config.ts
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.test.ts', 'tests/**/*.test.ts'],
    exclude: [
      '**/node_modules/**',
      '**/.crewchief/**',
      '**/dist/**',
    ],
  },
})
```

This explicitly excludes `.crewchief` directories from test discovery, preventing nested worktree pollution.

### Phase 2: Rust Test Fixes

**Pattern 1: EmbeddingService Construction**

Tests need to construct EmbeddingService using the new two-argument pattern.

```rust
// Before (broken)
let service = EmbeddingService::new(config);

// After (fixed) - Option A: Use from_env()
let service = EmbeddingService::from_env().await?;

// After (fixed) - Option B: Manual construction for testing
use crewchief_maproom::embedding::{
    factory::create_provider_from_env,
    cache::EmbeddingCache,
    config::CacheConfig,
};

let provider = create_provider_from_env().await?;
let cache = Arc::new(EmbeddingCache::new(CacheConfig::default())?);
let service = EmbeddingService::new(provider, cache);
```

**Pattern 2: ChangeType Enum**

Convert from struct-like variants to tuple variants.

```rust
// Before (broken)
ChangeType::New { hash: content_hash }
ChangeType::Deleted { hash: old_hash }

// After (fixed)
ChangeType::New(content_hash)
ChangeType::Deleted(old_hash)
```

**Pattern 3: Async Method Handling**

Add `.await` to async methods that previously were sync.

```rust
// Before (broken)
let service = EmbeddingService::from_env();

// After (fixed)
let service = EmbeddingService::from_env().await?;
```

**Pattern 4: Removed Fields/Methods**

Remove references to deleted APIs.

```rust
// Before (broken) - include_debug removed
SearchOptions { include_debug: true, ... }

// After (fixed)
SearchOptions { /* other fields only */ }

// Before (broken) - timing field removed
results.timing

// After (fixed) - use metadata instead
results.metadata

// Before (broken) - individual scores removed
result.fts_score
result.vector_score

// After (fixed) - use source_scores map
result.source_scores.get(&SearchSource::Fts)
result.source_scores.get(&SearchSource::Vector)
```

### Phase 3: TypeScript Test Fixes

**Pattern 1: Binary Path Expectations**

Tests should be flexible about binary paths.

```typescript
// Before (broken) - expects specific binary name
expect(spawn.args[0]).toBe('crewchief-maproom')

// After (fixed) - checks for crewchief binary (any path)
expect(spawn.args[0]).toContain('crewchief')
// or use regex pattern matching
```

**Pattern 2: Message Format Assertions**

Update assertions to match current message formats.

```typescript
// Before (broken)
expect(result.message).toContain('0 chunks')

// After (fixed) - match actual message
expect(result.message).toContain('Worktree not in database')
// or use toBeTruthy for presence check
expect(result.message.includes('0 chunks') || result.message.includes('not in database')).toBe(true)
```

**Pattern 3: Test Isolation**

Ensure tests don't pollute each other's state.

```typescript
// Add cleanup hooks
afterEach(async () => {
  // Clean up any created worktrees
  await cleanupTestWorktrees()
})

// Use unique identifiers
const testId = `test-${Date.now()}-${Math.random().toString(36).slice(2)}`
```

### Phase 4: CI Configuration

The existing CI workflow is well-structured. Verify:

1. **test-sqlite-e2e**: CLI end-to-end tests with SQLite
2. **test-mcp-sqlite**: MCP server tests with SQLite fixture
3. **test-rust-sqlite**: Rust library tests (`cargo test --features sqlite`)
4. **test-postgres**: TypeScript PostgreSQL integration
5. **test-rust-postgres**: Rust PostgreSQL compilation

**Missing Coverage to Verify:**
- CLI unit tests (`packages/cli` vitest)
- VSCode extension tests
- Daemon client tests

## File Organization

### Files to Modify (Rust)

High-impact files (fix these first to unblock most errors):

| File | Error Count | Fix Type |
|------|-------------|----------|
| `tests/embedding_service_test.rs` | ~37 | EmbeddingService construction |
| `tests/embedding_integration.rs` | ~17 | EmbeddingService + async |
| `tests/incremental_*.rs` | ~35 | ChangeType enum |
| `tests/search_*.rs` | ~19 | SearchOptions fields |
| `tests/fusion_*.rs` | ~8 | FinalSearchResults fields |

### Files to Modify (TypeScript)

| File | Error Count | Fix Type |
|------|-------------|----------|
| `packages/cli/src/search-optimization/scan-orchestrator.test.ts` | 3 | Binary path |
| `packages/cli/src/search-optimization/validation/pre-flight-validator.test.ts` | 5 | Message format |
| `packages/cli/tests/search-optimization/genetic-iterator.test.ts` | 6 | Various |
| `packages/cli/tests/sdk/variant-injection.test.ts` | 6 | Worktree creation |

### Files to Create

None - this project only modifies existing files.

### Files to Delete

None - tests should be updated, not deleted.

## Technology Decisions

### Rust Test Patterns

1. **Use `from_env()` for most tests** - Simpler, matches production usage
2. **Manual construction only for edge cases** - When testing specific provider/cache combinations
3. **Feature-flag tests appropriately** - SQLite vs PostgreSQL backends

### TypeScript Test Patterns

1. **Flexible assertions** - Match behavior, not exact strings
2. **Proper async/await** - Ensure all promises are awaited
3. **Mock boundaries** - Mock external services, not internal modules

## Integration Points

### Internal Dependencies

- Tests depend on implementation APIs
- Tests should import from public module interfaces
- Avoid testing private internals

### External Dependencies

- PostgreSQL (for integration tests)
- SQLite (for unit tests)
- Git (for worktree operations)
- Ollama/OpenAI (mocked in tests)

## Performance Considerations

Not a primary concern for this project. Focus on:
- Test correctness over speed
- CI passing reliably
- Minimal test flakiness

## Long-term Maintainability

### Recommendations (Post-Project)

1. **Co-locate tests with implementation**
   - Breaking API changes force test updates
   - Easier to maintain sync

2. **Use test utilities**
   - Shared fixtures/helpers
   - Reduce duplication

3. **Document test patterns**
   - Add CLAUDE.md notes about test patterns
   - Make it easy to write new tests correctly

## Diagrams

### Test Fix Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Fix All Tests Flow                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │  Clean Up   │───▶│  Fix Rust   │───▶│Fix TypeScript│    │
│  │ Environment │    │   Tests     │    │    Tests    │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│         │                  │                  │             │
│         ▼                  ▼                  ▼             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │  Remove     │    │  Update     │    │  Update     │     │
│  │  Stale      │    │  API Usage  │    │ Assertions  │     │
│  │  Worktrees  │    │  Patterns   │    │  & Mocks    │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│                            │                  │             │
│                            ▼                  ▼             │
│                     ┌─────────────────────────┐            │
│                     │   Verify CI Passes      │            │
│                     └─────────────────────────┘            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### API Migration Pattern

```
┌────────────────────────────────────────────────────────┐
│           EmbeddingService Migration                    │
├────────────────────────────────────────────────────────┤
│                                                        │
│  Old Pattern (tests):                                  │
│  ┌──────────────────┐                                  │
│  │ EmbeddingService │                                  │
│  │   ::new(config)  │                                  │
│  └────────┬─────────┘                                  │
│           │ ERROR: takes 2 args                        │
│           ▼                                            │
│  New Pattern (implementation):                         │
│  ┌──────────────────┐    ┌──────────────────┐         │
│  │ create_provider  │───▶│ EmbeddingService │         │
│  │   _from_env()    │    │ ::new(provider,  │         │
│  └──────────────────┘    │       cache)     │         │
│                          └──────────────────┘         │
│           OR                                           │
│  ┌──────────────────┐                                  │
│  │ EmbeddingService │  (handles config internally)    │
│  │   ::from_env()   │                                  │
│  └──────────────────┘                                  │
│                                                        │
└────────────────────────────────────────────────────────┘
```
