# Ticket: VECSTORE-1004: Repository and Worktree Query Methods

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - db tests: 129 passed, SQLite tests: 103 passed
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add repository and worktree lookup methods to the `VectorStore` trait: `get_repo_by_name()`, `get_worktree_by_name()`, `list_repos()`, and `list_worktrees()`. These enable querying indexed repositories without direct database access.

## Background
The CLI and MCP server need to look up repositories and worktrees by name, but currently must use direct database queries. These lookup methods should be exposed through the `VectorStore` trait.

**Current State**:
- PostgreSQL: **NO** dedicated lookup functions in `queries.rs` - must be written
- SQLite: No standardized lookup functions
- Trait: Has `get_or_create_*` methods, but no read-only lookup methods

**Reference**: Plan Phase 3 - Repository Query Methods (VECSTORE-1004)

## Acceptance Criteria
- [x] `RepoInfo` and `WorktreeInfo` types defined in `db/mod.rs`
- [x] `get_repo_by_name()` method added to trait and implemented
- [x] `get_worktree_by_name()` method added to trait and implemented
- [x] `list_repos()` method added to trait and implemented
- [x] `list_worktrees()` method added to trait and implemented
- [x] PostgreSQL query functions written in `queries.rs`
- [x] Both `PostgresStore` and `SqliteStore` implementations work
- [x] `list_*` methods return empty vec for empty database (not error)
- [ ] Contract tests pass for both backends (deferred to VECSTORE-1007)

## Technical Requirements

### Domain Types
Add to `crates/maproom/src/db/mod.rs`:

```rust
/// Repository metadata
pub struct RepoInfo {
    pub id: i64,
    pub name: String,
    pub root_path: String,
}

/// Worktree metadata
pub struct WorktreeInfo {
    pub id: i64,
    pub repo_id: i64,
    pub name: String,
    pub abs_path: String,
}
```

### Trait Method Signatures
Add to `VectorStore` trait:

```rust
/// Get repository by name
async fn get_repo_by_name(&self, name: &str) -> anyhow::Result<Option<RepoInfo>>;

/// Get worktree by name within a repository
async fn get_worktree_by_name(&self, repo_id: i64, name: &str) -> anyhow::Result<Option<WorktreeInfo>>;

/// List all repositories
async fn list_repos(&self) -> anyhow::Result<Vec<RepoInfo>>;

/// List all worktrees for a repository
async fn list_worktrees(&self, repo_id: i64) -> anyhow::Result<Vec<WorktreeInfo>>;
```

### PostgreSQL Queries (NEW - must be written)

**File: `crates/maproom/src/db/queries.rs`**

```rust
pub async fn get_repo_by_name(
    client: &impl GenericClient,
    name: &str,
) -> anyhow::Result<Option<RepoInfo>> {
    // SELECT id, name, root_path
    // FROM repositories
    // WHERE name = $1
}

pub async fn list_repos(
    client: &impl GenericClient,
) -> anyhow::Result<Vec<RepoInfo>> {
    // SELECT id, name, root_path
    // FROM repositories
    // ORDER BY name
}

pub async fn get_worktree_by_name(
    client: &impl GenericClient,
    repo_id: i64,
    name: &str,
) -> anyhow::Result<Option<WorktreeInfo>> {
    // SELECT id, repo_id, name, abs_path
    // FROM worktrees
    // WHERE repo_id = $1 AND name = $2
}

pub async fn list_worktrees(
    client: &impl GenericClient,
    repo_id: i64,
) -> anyhow::Result<Vec<WorktreeInfo>> {
    // SELECT id, repo_id, name, abs_path
    // FROM worktrees
    // WHERE repo_id = $1
    // ORDER BY name
}
```

### SQLite Implementation

Add to `crates/maproom/src/db/sqlite/` (new file or existing):

```rust
// crates/maproom/src/db/sqlite/repos.rs (new file)

pub fn get_repo_by_name(conn: &Connection, name: &str) -> anyhow::Result<Option<RepoInfo>> {
    // Same query logic
}

pub fn list_repos(conn: &Connection) -> anyhow::Result<Vec<RepoInfo>> {
    // Same query logic
}

pub fn get_worktree_by_name(
    conn: &Connection,
    repo_id: i64,
    name: &str,
) -> anyhow::Result<Option<WorktreeInfo>> {
    // Same query logic
}

pub fn list_worktrees(conn: &Connection, repo_id: i64) -> anyhow::Result<Vec<WorktreeInfo>> {
    // Same query logic
}
```

### Store Implementations

**PostgresStore** (`crates/maproom/src/db/postgres/mod.rs`):
```rust
async fn get_repo_by_name(&self, name: &str) -> anyhow::Result<Option<RepoInfo>> {
    let client = self.pool.get().await?;
    super::queries::get_repo_by_name(&client, name).await
}

async fn list_repos(&self) -> anyhow::Result<Vec<RepoInfo>> {
    let client = self.pool.get().await?;
    super::queries::list_repos(&client).await
}

// Similar for worktree methods
```

**SqliteStore** (`crates/maproom/src/db/sqlite/mod.rs`):
```rust
async fn get_repo_by_name(&self, name: &str) -> anyhow::Result<Option<RepoInfo>> {
    let name = name.to_string();
    self.run(move |conn| repos::get_repo_by_name(conn, &name)).await
}

async fn list_repos(&self) -> anyhow::Result<Vec<RepoInfo>> {
    self.run(move |conn| repos::list_repos(conn)).await
}

// Similar for worktree methods
```

## Implementation Notes

### Empty Results vs Errors
- `get_*` methods return `None` for not found (not an error)
- `list_*` methods return empty `Vec` for no results (not an error)
- Only return errors for actual database failures

### Schema Reference
Verify table/column names against existing schema:
- PostgreSQL: Check `migrations/*.sql` for `repositories` and `worktrees` tables
- SQLite: Check `sqlite/schema.rs` for equivalent tables

### Ordering
- `list_repos()` should order by name (alphabetical)
- `list_worktrees()` should order by name within repo

## Dependencies
- None - Repository queries are independent of other VECSTORE tickets

## Risk Assessment
- **Risk**: Table/column name differences between backends
  - **Mitigation**: Review both schemas before implementation
- **Risk**: Performance with many repositories
  - **Mitigation**: Indexed queries, reasonable LIMIT if needed

## Files/Packages Affected
- `crates/maproom/src/db/mod.rs` (types + trait)
- `crates/maproom/src/db/queries.rs` (PostgreSQL queries)
- `crates/maproom/src/db/postgres/mod.rs` (PostgresStore impl)
- `crates/maproom/src/db/sqlite/mod.rs` (SqliteStore impl)
- `crates/maproom/src/db/sqlite/repos.rs` (NEW - SQLite queries)
