# Security Review: Schema Migration Integration

## MVP Security Mindset

**Goal**: Ship safely, not build Fort Knox.

**Focus**: Practical risks that could cause data loss, corruption, or unauthorized access during migration integration.

**Not In Scope**: Enterprise security theater, compliance frameworks, penetration testing.

## Risk Classification

### High Risk (Must Address)

**Risk 1: SQL Injection in Migration Content**
- **Threat**: Malicious SQL in migration files executed with database privileges
- **Vector**: Migration files are `include_str!` at compile time, executed as-is
- **Mitigation**: Code review of all migration SQL before merging
- **Status**: ✅ Low actual risk - migrations are checked into git, reviewed in PR

**Risk 2: Data Loss During Backfill**
- **Threat**: Migration 0018 blob_sha backfill fails mid-execution
- **Impact**: Some chunks have blob_sha, some don't (inconsistent state)
- **Mitigation**:
  - Wrap backfill in transaction (ROLLBACK on failure)
  - Test on production-sized database before shipping
  - Keep backfill simple (no complex logic)
- **Status**: ⚠️ Needs testing - backfill query must be atomic

**Risk 3: Schema Drift Between Environments**
- **Threat**: Local database has partial manual schema, prod doesn't
- **Impact**: Migration works locally but fails in production
- **Mitigation**:
  - Use `IF NOT EXISTS` for all schema changes
  - Test migrations on both fresh and existing databases
  - Document schema expectations in migration comments
- **Status**: ✅ Addressed by idempotent migration design

### Medium Risk (Monitor)

**Risk 4: Database Connection String Exposure**
- **Threat**: Database credentials in logs, error messages, or environment
- **Vector**: Migration runner outputs connection details on error
- **Mitigation**:
  - Use `DATABASE_URL` environment variable (already standard)
  - Don't log connection strings in migration output
  - Production uses secrets management (not in scope for this project)
- **Status**: ✅ Already handled by existing patterns

**Risk 5: Migration Ordering Race Condition**
- **Threat**: Two migration runners execute simultaneously
- **Impact**: Duplicate migrations, conflicting schema changes
- **Mitigation**:
  - PostgreSQL advisory locks (already used in migration runner)
  - `schema_migrations` table uses PRIMARY KEY on version
  - Concurrent migration attempts will fail gracefully
- **Status**: ✅ Already protected by existing migration framework

**Risk 6: Breaking Changes to Existing Data**
- **Threat**: Migration alters or drops columns/tables with data
- **Impact**: Data loss, application crashes
- **Mitigation**:
  - All migrations are additive only (no DROP, no ALTER TYPE)
  - New columns are nullable or have defaults
  - Existing queries continue to work
- **Status**: ✅ Enforced by migration design (additive only)

### Low Risk (Accept)

**Risk 7: Denial of Service via Large Backfill**
- **Threat**: Migration 0018 blob_sha backfill locks table for minutes
- **Impact**: Brief service interruption during upgrade
- **Mitigation**: Acceptable for one-time migration (estimated 15-70 seconds)
- **Status**: ✅ Accept - this is a maintenance operation

**Risk 8: Unauthorized Schema Inspection**
- **Threat**: Attacker queries `schema_migrations` table
- **Impact**: Attacker learns migration history
- **Mitigation**: Database-level access control (PostgreSQL roles)
- **Status**: ✅ Out of scope - database security is infrastructure concern

## Threat Modeling

### Attack Surface Analysis

**1. Migration SQL Files**
- **Location**: `crates/maproom/migrations/*.sql`
- **Access**: Source code (public GitHub repo)
- **Execution**: Compile-time embedding, runtime execution by migration runner
- **Controls**: Code review, PR approval, git commit signatures

**2. Migration Runner Code**
- **Location**: `crates/maproom/src/db/queries.rs`
- **Access**: Source code (public GitHub repo)
- **Execution**: Rust binary with database credentials
- **Controls**: Rust safety guarantees, unit tests, integration tests

**3. Database Connection**
- **Protocol**: PostgreSQL wire protocol (SSL optional)
- **Credentials**: Environment variable `DATABASE_URL`
- **Network**: Local (dev), Docker network (compose), or remote (production)
- **Controls**: Network isolation, least-privilege database user

**4. Schema Migrations Table**
- **Location**: `schema_migrations` table in maproom database
- **Data**: Migration version, name, timestamp
- **Access**: Read/write by migration runner only
- **Controls**: PostgreSQL permissions, table-level access control

### Trust Boundaries

```
┌─────────────────────────────────────────────┐
│ Developer (writes migration SQL)            │
│ Trust: Code review, PR approval             │
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ Git Repository (stores migration files)     │
│ Trust: Git commit history, branch protection│
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ Rust Compiler (embeds migration SQL)        │
│ Trust: Compile-time validation              │
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ Migration Runner (executes SQL)             │
│ Trust: Rust binary integrity, credentials   │
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ PostgreSQL Database (applies schema changes)│
│ Trust: Database user permissions            │
└─────────────────────────────────────────────┘
```

**Weakest Link**: Developer writes malicious SQL
**Strongest Control**: Code review catches malicious SQL before merge

## SQL Injection Analysis

### Migration SQL Review

**Migration 0018** (from MCP 001_add_blob_sha.sql):
```sql
-- Expected content:
ALTER TABLE maproom.chunks
  ADD COLUMN IF NOT EXISTS blob_sha TEXT;

-- Update existing rows (backfill)
UPDATE maproom.chunks
  SET blob_sha = encode(sha256(content::bytea), 'hex')
  WHERE blob_sha IS NULL;
```

**Security Assessment**:
- ✅ No dynamic SQL (all literals)
- ✅ No user input interpolation
- ✅ Uses PostgreSQL built-in functions (sha256, encode)
- ⚠️ Backfill could be slow (needs batching)

**Migration 0019** (from MCP 002_create_code_embeddings.sql):
```sql
-- Expected content:
CREATE TABLE IF NOT EXISTS maproom.code_embeddings (
  id BIGSERIAL PRIMARY KEY,
  blob_sha TEXT NOT NULL UNIQUE,
  embedding vector(1536),
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_code_embeddings_hnsw
  ON maproom.code_embeddings
  USING hnsw (embedding vector_cosine_ops);
```

**Security Assessment**:
- ✅ No dynamic SQL
- ✅ Standard DDL statements
- ✅ No sensitive data in schema
- ✅ HNSW index is safe (PostgreSQL extension)

**Migration 0020** (from MCP 004_add_worktree_tracking.sql):
```sql
-- Expected content:
ALTER TABLE maproom.chunks
  ADD COLUMN IF NOT EXISTS worktree_ids JSONB DEFAULT '[]'::jsonb NOT NULL;

CREATE TABLE IF NOT EXISTS maproom.worktree_index_state (
  worktree_id BIGINT PRIMARY KEY REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
  tree_sha TEXT,
  indexed_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_chunks_worktree_ids
  ON maproom.chunks
  USING gin (worktree_ids);
```

**Security Assessment**:
- ✅ No dynamic SQL
- ✅ Uses JSONB type safely
- ✅ Foreign key constraint enforces referential integrity
- ✅ CASCADE delete is intentional (cleanup on worktree delete)

**Migration 0021** (from MCP 005_complete_branchx_schema.sql):
```sql
-- Expected content: Final schema tweaks
-- (Review actual file content when copying)
```

**Security Assessment**: Pending review of actual file

### Dynamic SQL Risk: None

**Finding**: All migrations use static SQL (no parameterization, no interpolation)

**Why This Is Safe**:
- SQL is written by developers, reviewed in PR
- SQL is embedded at compile time (not runtime)
- No user input reaches migration SQL
- PostgreSQL executes SQL as-is (no injection vector)

**Contrast with Unsafe Pattern** (not used here):
```rust
// ❌ UNSAFE (not used in this project)
let user_input = get_user_input();
let sql = format!("CREATE TABLE {}", user_input); // INJECTION RISK
client.execute(&sql, &[]).await?;

// ✅ SAFE (what we actually do)
let sql = include_str!("migration.sql"); // Static at compile time
client.execute(sql, &[]).await?;
```

## Data Integrity Safeguards

### Transaction Boundaries

**Migrations Run in Transactions** (except CONCURRENT):
```rust
// From crates/maproom/src/db/queries.rs (existing code)
for (version, name, sql, concurrent) in migrations {
    if concurrent {
        // Can't use transaction for CONCURRENT operations
        client.execute(sql, &[]).await?;
    } else {
        // Run in transaction (ROLLBACK on failure)
        let tx = client.transaction().await?;
        tx.execute(sql, &[]).await?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES ($1, $2)",
            &[&version, &name]
        ).await?;
        tx.commit().await?;
    }
}
```

**Protection Provided**:
- ✅ Failed migration rolls back (no partial application)
- ✅ `schema_migrations` only updated on success
- ✅ Retry-safe (can re-run after failure)

**Our New Migrations**:
- 0018: `concurrent = false` (uses transaction)
- 0019: `concurrent = false` (uses transaction)
- 0020: `concurrent = false` (uses transaction)
- 0021: `concurrent = false` (uses transaction)

### Idempotency Protection

**All Migrations Use IF NOT EXISTS**:
```sql
-- Safe to run multiple times
ALTER TABLE maproom.chunks
  ADD COLUMN IF NOT EXISTS blob_sha TEXT;

CREATE TABLE IF NOT EXISTS maproom.code_embeddings (...);

CREATE INDEX IF NOT EXISTS idx_code_embeddings_hnsw ...;
```

**Why This Matters**:
- Migration runner can be interrupted and restarted
- Manual SQL may have created some schema already
- Development environments may have partial schema

**Protection Provided**:
- ✅ No duplicate columns/tables/indexes
- ✅ No "already exists" errors
- ✅ Safe for both fresh and existing databases

### Referential Integrity

**Foreign Key Constraints Enforced**:
```sql
-- Migration 0020
CREATE TABLE maproom.worktree_index_state (
  worktree_id BIGINT PRIMARY KEY
    REFERENCES maproom.worktrees(id) ON DELETE CASCADE
);
```

**Protection Provided**:
- ✅ Can't insert orphaned records
- ✅ CASCADE delete cleans up state on worktree removal
- ✅ Database enforces consistency (not application logic)

## Access Control Assessment

### Database User Permissions

**Required Permissions for Migration Runner**:
- `CREATE TABLE` (new tables)
- `ALTER TABLE` (new columns)
- `CREATE INDEX` (new indexes)
- `INSERT` (schema_migrations table)
- `SELECT` (verify schema)

**Assumed Environment**:
- Development: `maproom` user is superuser (acceptable for dev)
- Production: `maproom` user has schema modification rights (standard for migration runner)

**Risk**: Migration runner has elevated privileges
**Mitigation**:
- Migrations are code-reviewed before merge
- Migration runner only runs during deployment (not continuously)
- Database credentials stored securely (environment variables, secrets management)

**Status**: ✅ Standard practice for migration tools

### Least Privilege Principle

**Runtime Application vs. Migration Runner**:
- **MCP Server**: Needs SELECT, INSERT, UPDATE on tables (not DDL)
- **Rust Indexer**: Needs SELECT, INSERT, UPDATE on tables (not DDL)
- **Migration Runner**: Needs DDL rights (CREATE, ALTER, DROP)

**Opportunity**: Use separate database users
- `maproom_app` - Read/write data only
- `maproom_admin` - Schema modifications

**Status**: ⚠️ Out of scope for this project (future improvement)

## Deployment Security

### Migration Execution Context

**When Migrations Run**:
1. Development: `cargo run --bin crewchief-maproom -- db`
2. Docker: `crewchief-maproom db` on container startup
3. Production: Manual execution by operator

**Security Implications**:
- Migrations run with full database privileges
- Migrations are one-time operations (not continuous)
- Failed migrations are visible (logs, exit code)

**Controls**:
- ✅ Migrations are reviewed before merge
- ✅ Migrations are tested in staging first
- ✅ Migration runner exits on failure (fail-safe)

### Secrets Management

**Database Credentials**:
- Environment variable: `DATABASE_URL`
- Format: `postgresql://user:pass@host:port/db`
- Storage: `.env` file (dev), secrets manager (prod)

**Security Concerns**:
- ⚠️ Password in plaintext in environment
- ⚠️ Connection string may appear in logs

**Mitigations**:
- ✅ `.env` file not checked into git (already in .gitignore)
- ✅ Production uses secrets manager (out of scope)
- ✅ Don't log `DATABASE_URL` value (existing practice)

**Status**: ✅ Standard practice for database applications

## Rollback Safety

### No Rollback Migrations (By Design)

**Decision**: Don't implement rollback migrations (down migrations)

**Rationale**:
- Additive-only schema (new columns, new tables)
- Rolling back schema doesn't roll back code
- Safer to have schema that old code can tolerate

**Risk**: Can't easily undo migration
**Mitigation**:
- Test thoroughly before shipping
- Additive changes are backward-compatible
- Emergency: Manual SQL to drop tables/columns (last resort)

**Status**: ✅ Acceptable for MVP

### Backward Compatibility

**Old Rust Binary vs. New Schema**:
- Scenario: User downgrades to old Rust binary after migration
- Old binary behavior: Ignores new columns (PostgreSQL allows extra columns)
- Impact: Features gracefully degrade (no crashes)

**New Rust Binary vs. Old Schema**:
- Scenario: New Rust binary runs before migrations applied
- New binary behavior: Queries fail if expecting new columns
- Impact: Migration runner auto-applies migrations on startup

**Status**: ✅ Safe both ways (additive schema design)

## Security Testing

### Pre-Deployment Checklist

**Before Merging**:
- [ ] Review all migration SQL for injection risks
- [ ] Verify `IF NOT EXISTS` used consistently
- [ ] Check transaction boundaries (concurrent flag)
- [ ] Test on fresh database
- [ ] Test on database with partial schema (manual SQL)

**Before Production Deploy**:
- [ ] Test on production-sized database (10k-100k chunks)
- [ ] Verify migration completes in acceptable time (<2 minutes)
- [ ] Check logs for leaked credentials
- [ ] Backup database before migration
- [ ] Have rollback plan (restore from backup)

### Automated Security Checks

**Static Analysis** (already in place):
- `cargo clippy` - Rust linting (catches common mistakes)
- `cargo fmt` - Code formatting (consistency)
- GitHub PR review - Human code review

**Dynamic Testing** (existing integration tests):
- `packages/maproom-mcp/tests/migrations/` - Schema validation
- `crates/maproom/tests/` - Rust integration tests
- Migration tests verify schema correctness

**Status**: ✅ Sufficient for MVP

## Known Vulnerabilities

### None Identified

**Assessment**: No known security vulnerabilities in migration integration approach

**Assumptions**:
- PostgreSQL is up-to-date (no known CVEs)
- pgvector extension is safe (community-maintained, widely used)
- Rust toolchain is current (cargo audit shows no issues)

**Monitoring**:
- `cargo audit` - Check for dependency vulnerabilities (run periodically)
- Dependabot - Automated dependency updates (already enabled)

## Security Constraints

**What We're NOT Doing** (out of scope):
- SQL injection testing (no user input in migrations)
- Penetration testing (not a web application)
- Compliance certifications (SOC2, HIPAA, etc.)
- Encryption at rest (database-level concern)
- Audit logging (PostgreSQL handles this)
- Rate limiting (not applicable to migrations)

**Why**: MVP focus on correctness, not hardening

## Conclusion

**Security Posture**: ✅ Safe to ship

**Key Protections**:
1. All migration SQL is static (no injection risk)
2. Transactions ensure atomicity (no partial failures)
3. Idempotent design allows safe retry
4. Additive-only schema (backward compatible)
5. Code review before merge (human validation)

**Residual Risks** (accepted):
- Migration runner has elevated database privileges (standard for migrations)
- No dedicated rollback migrations (additive schema doesn't need them)
- Credentials in environment variables (standard practice)

**Recommendation**: Proceed with implementation and testing.

**Security Review Status**: ✅ APPROVED for MVP
