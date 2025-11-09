# E2E Test Plan for Branch-Aware Indexing (BRANCHX-1013)

## Overview

This document outlines the end-to-end test suite for validating the complete branch-aware indexing workflow. These tests verify that the entire system works correctly from user perspective: indexing branches, switching between them, and searching with worktree filtering.

## Test Environment Requirements

### Prerequisites
- PostgreSQL with pgvector extension (via Docker Compose)
- Git installed and available in PATH
- Node.js environment for TypeScript tests
- Rust toolchain for CLI tests
- Sufficient disk space for test repositories

### Docker Setup
Tests require the following containers running:
```yaml
# From packages/maproom-mcp/config/docker-compose.yml
services:
  maproom-postgres:
    image: pgvector/pgvector:pg16
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: maproom
      POSTGRES_USER: maproom
      POSTGRES_PASSWORD: maproom
```

Start with: `docker-compose -f config/docker-compose.yml up -d`

## Test Suite 1: Branch Switch Workflow

**File**: `packages/maproom-mcp/tests/e2e/branch-workflow.test.ts`

### Test 1.1: Initial Branch Indexing

**Purpose**: Verify that indexing a branch from scratch works correctly

**Setup**:
```typescript
// Create test repository with main branch
const repo = await createTestRepo({
  name: 'test-repo-main',
  files: [
    { path: 'src/auth.ts', content: 'export function authenticate() {...}' },
    { path: 'src/user.ts', content: 'export class User {...}' },
    { path: 'src/utils.ts', content: 'export function helper() {...}' },
    // ... 20 total files for realistic scenario
  ]
});
```

**Test Steps**:
1. Run `maproom scan --repo test-repo --worktree main`
2. Capture stats: files_processed, chunks_processed, duration
3. Query database to verify chunks created
4. Verify all files indexed (SELECT COUNT from chunks WHERE worktree_ids ? '1')

**Success Criteria**:
- All 20 files indexed
- Chunks created with worktree_ids containing main worktree ID
- Stats printed to console match database counts
- No errors logged

### Test 1.2: Incremental Update on Branch Switch

**Purpose**: Verify that switching to a similar branch uses incremental updates

**Setup**:
```typescript
// Create feature branch with 80% overlap (modify 4 files, add 1 file)
await git.checkoutBranch(repo, 'feature');
await modifyFile(repo, 'src/auth.ts', 'export function authenticateV2() {...}');
await modifyFile(repo, 'src/user.ts', 'export class UserV2 {...}');
await addFile(repo, 'src/feature.ts', 'export function newFeature() {...}');
await git.commit(repo, 'Feature changes');
```

**Test Steps**:
1. Run `maproom scan --repo test-repo --worktree feature`
2. Capture stats: files_processed, chunks_processed, duration
3. Compare against initial scan stats
4. Verify tree SHA optimization used (check logs)

**Success Criteria**:
- files_processed ≈ 5 (only changed files)
- Duration < 30% of initial scan time (5-10x speedup)
- Chunks with same content share blob_sha but have different worktree_ids
- Log messages indicate incremental mode

### Test 1.3: No Changes Detection

**Purpose**: Verify that re-scanning unchanged branch skips all work

**Setup**: Re-scan feature branch without any changes

**Test Steps**:
1. Run `maproom scan --repo test-repo --worktree feature` again
2. Capture stats and duration

**Success Criteria**:
- files_processed = 0
- Duration < 100ms (tree SHA check only)
- Log message: "No changes detected (tree SHA match)"
- No database writes occurred

### Test 1.4: Force Flag Bypasses Optimization

**Purpose**: Verify --force flag works correctly

**Test Steps**:
1. Run `maproom scan --repo test-repo --worktree feature --force`
2. Capture stats

**Success Criteria**:
- All files re-scanned (files_processed = 20)
- Log message indicates force mode
- Duration similar to initial scan (no tree SHA optimization)

## Test Suite 2: Worktree Filtering in Search

**File**: `packages/maproom-mcp/tests/e2e/worktree-search.test.ts`

### Test 2.1: Search Returns Branch-Specific Results

**Purpose**: Verify MCP search filters by worktree correctly

**Setup**:
```typescript
// Index two branches with different content
await indexBranch(repo, 'main', [
  { path: 'src/main-only.ts', content: 'export function mainFunction() {...}' }
]);
await indexBranch(repo, 'feature', [
  { path: 'src/feature-only.ts', content: 'export function featureFunction() {...}' }
]);
```

**Test Steps**:
1. Search with `{ query: 'function', worktree: 'main' }`
2. Verify results only include main-only.ts
3. Search with `{ query: 'function', worktree: 'feature' }`
4. Verify results only include feature-only.ts
5. Search without worktree parameter
6. Verify results include both files

**Success Criteria**:
- Main search: 0 results from feature branch
- Feature search: 0 results from main branch
- Unfiltered search: results from both branches
- Each result includes correct worktree_ids array

### Test 2.2: Shared Content Across Branches

**Purpose**: Verify chunks shared across branches are deduplicated

**Setup**: Both branches have identical file `src/shared.ts`

**Test Steps**:
1. Query chunks table for shared.ts content
2. Verify only ONE chunk exists (by blob_sha)
3. Verify chunk has worktree_ids = [main_id, feature_id]
4. Search for shared content with each worktree filter
5. Verify same chunk returned in both cases

**Success Criteria**:
- Single chunk with multiple worktree_ids
- Search with worktree=main returns the chunk
- Search with worktree=feature returns the same chunk
- No duplicate chunks in database

## Test Suite 3: File Deletion Handling

**File**: `packages/maproom-mcp/tests/e2e/deletion-workflow.test.ts`

### Test 3.1: Deleted File Removes Worktree

**Purpose**: Verify file deletion updates worktree_ids correctly

**Setup**:
```typescript
// Index file in main branch
await indexBranch(repo, 'main', [
  { path: 'src/to-delete.ts', content: 'export function toDelete() {...}' }
]);

// Delete file and commit
await git.rm(repo, 'src/to-delete.ts');
await git.commit(repo, 'Remove to-delete.ts');
```

**Test Steps**:
1. Run incremental scan on main
2. Query chunks table for to-delete.ts chunks
3. Verify worktree_ids no longer contains main_id
4. If no other worktrees reference it, verify chunk deleted

**Success Criteria**:
- Chunk's worktree_ids array updated (main_id removed)
- If empty worktree_ids, chunk garbage collected
- Incremental scan detects deletion via git diff-tree

## Test Suite 4: CLI Integration

**File**: `crates/maproom/tests/cli_integration.rs`

### Test 4.1: Scan Command Output Format

**Purpose**: Verify CLI prints stats in expected format

**Test Steps**:
```rust
let output = Command::new("maproom")
    .args(["scan", "--repo", "test", "--worktree", "main"])
    .output()?;

assert!(output.status.success());
let stdout = String::from_utf8(output.stdout)?;

// Verify output format
assert!(stdout.contains("⚡ Incremental scan mode"));
assert!(stdout.contains("Files processed:"));
assert!(stdout.contains("Chunks processed:"));
assert!(stdout.contains("Cache hit rate:"));
```

**Success Criteria**:
- Exit code 0 (success)
- Output includes scan mode indicator
- Stats displayed in user-friendly format
- No errors printed to stderr

### Test 4.2: Force Flag Behavior

**Test Steps**:
```rust
let output = Command::new("maproom")
    .args(["scan", "--repo", "test", "--worktree", "main", "--force"])
    .output()?;

let stdout = String::from_utf8(output.stdout)?;
assert!(stdout.contains("🔄 Full scan mode"));
```

**Success Criteria**:
- Output indicates force mode active
- All files scanned (not incremental)

## Test Utilities

### Helper: createTestRepo()

```typescript
interface TestRepoOptions {
  name: string;
  files: Array<{ path: string; content: string }>;
  branches?: string[];
}

async function createTestRepo(options: TestRepoOptions): Promise<TestRepo> {
  const repoPath = path.join(tmpdir(), options.name);

  // Initialize git repo
  await exec(`git init ${repoPath}`);
  await exec(`git -C ${repoPath} config user.email "test@example.com"`);
  await exec(`git -C ${repoPath} config user.name "Test User"`);

  // Create files
  for (const file of options.files) {
    const filePath = path.join(repoPath, file.path);
    await fs.mkdir(path.dirname(filePath), { recursive: true });
    await fs.writeFile(filePath, file.content);
  }

  // Initial commit
  await exec(`git -C ${repoPath} add .`);
  await exec(`git -C ${repoPath} commit -m "Initial commit"`);

  return {
    path: repoPath,
    cleanup: async () => await fs.rm(repoPath, { recursive: true })
  };
}
```

### Helper: indexBranch()

```typescript
async function indexBranch(
  repo: TestRepo,
  worktree: string,
  additionalFiles?: Array<{ path: string; content: string }>
): Promise<ScanStats> {
  if (additionalFiles) {
    // Add files and commit
    for (const file of additionalFiles) {
      await writeFile(repo, file.path, file.content);
    }
    await exec(`git -C ${repo.path} add .`);
    await exec(`git -C ${repo.path} commit -m "Add files for ${worktree}"`);
  }

  // Run scan
  const result = await exec(
    `maproom scan --repo ${repo.name} --worktree ${worktree} --path ${repo.path}`
  );

  return parseStats(result.stdout);
}
```

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_DB: maproom
          POSTGRES_USER: maproom
          POSTGRES_PASSWORD: maproom
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - name: Run E2E tests
        run: |
          pnpm install
          pnpm test:e2e
        env:
          MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5432/maproom
```

## Running Tests Locally

### Prerequisites
```bash
# Start database
docker-compose -f packages/maproom-mcp/config/docker-compose.yml up -d

# Wait for database ready
until pg_isready -h localhost -p 5432; do sleep 1; done
```

### Run TypeScript E2E Tests
```bash
cd packages/maproom-mcp
pnpm test:e2e
```

### Run Rust CLI Tests
```bash
cd crates/maproom
cargo test --test cli_integration -- --ignored --nocapture
```

## Test Implementation Status

**Current Status**: Test framework documented, implementation deferred

All E2E tests outlined in this document are marked for future implementation because:
1. Require complex test repository setup with git operations
2. Need Docker containers running (PostgreSQL, potentially Ollama)
3. Timing-dependent assertions can be flaky in CI
4. Database cleanup between tests requires careful handling

**Recommended Implementation Order**:
1. Test Suite 4 (CLI Integration) - Simplest, least dependencies
2. Test Suite 1 (Branch Switch Workflow) - Core functionality
3. Test Suite 2 (Worktree Filtering) - MCP search validation
4. Test Suite 3 (File Deletion) - Edge cases

**Test Coverage Goal**: Achieve 80%+ coverage of branch-aware indexing user workflows through E2E tests.

## Notes

- E2E tests complement unit tests (BRANCHX-1010) and integration tests (BRANCHX-1008, 1009)
- Focus on user-facing workflows rather than internal implementation details
- Use realistic test data (20+ files, multiple branches, varied content)
- Generous timeouts to prevent flakiness in CI
- Comprehensive cleanup to prevent test pollution
