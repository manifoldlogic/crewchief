//! PostgreSQL migration mechanism (spec §5).
//!
//! Schema is applied from the numeric-prefixed SQL files in `migrations_pg/`,
//! embedded at compile time with `include_str!`. Each file's leading integer is
//! its version, tracked in a `schema_migrations(version)` table so the
//! `get_applied_migrations() -> HashSet<i32>` adapter (§5.2) is exact and needs
//! no reverse-engineering of any migrator's version scheme.
//!
//! DEVIATION from §5.1 (documented): this is a hand-rolled runner rather than
//! `sqlx::migrate!`. The sqlx `migrate` feature transitively enables
//! `sqlx-sqlite` → `libsqlite3-sys 0.28`, which conflicts with the crate's
//! `rusqlite 0.29` (`libsqlite3-sys 0.26`) under Cargo's `links` uniqueness
//! rule. Bumping rusqlite is a non-goal (§3.2). This runner preserves the
//! spec's intent: idempotent, transactional, auto-run at connect, integer
//! versioning, tolerant of a missing tracking table.

use std::collections::HashSet;

use sqlx::PgPool;

/// `(version, name, sql)` for each migration, embedded at build time. The
/// version is the file's numeric prefix (`0001_init.sql` → 1), recovered here
/// explicitly so it round-trips through `schema_migrations.version`.
const MIGRATIONS: &[(i32, &str, &str)] = &[
    (
        1,
        "init",
        include_str!("../../../migrations_pg/0001_init.sql"),
    ),
    (
        2,
        "code_embeddings",
        include_str!("../../../migrations_pg/0002_code_embeddings.sql"),
    ),
];

/// The migration tracking table (analogous to SQLite's `schema_migrations`).
const TRACKING_DDL: &str = "CREATE TABLE IF NOT EXISTS schema_migrations (\
     version INTEGER PRIMARY KEY, \
     name TEXT NOT NULL, \
     applied_at TIMESTAMPTZ NOT NULL DEFAULT now())";

/// Apply all pending migrations. Idempotent (R-MIG-2): already-applied versions
/// are skipped, so re-running at connect time adds no duplicate work. Each
/// migration runs in its own transaction together with its tracking-row insert,
/// so a failure leaves no partially-recorded version.
pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(TRACKING_DDL).execute(pool).await?;
    let applied = get_applied(pool).await?;
    for (version, name, sql) in MIGRATIONS {
        if applied.contains(version) {
            continue;
        }
        // raw_sql uses the simple-query protocol, so a file with multiple
        // statements (CREATE EXTENSION; CREATE TABLE; CREATE INDEX; …) runs as a
        // single implicit transaction. We append the tracking-row insert to the
        // same batch so DDL + version record commit atomically — and execute on
        // the pool directly (no held `Transaction`, which would trip the
        // async_trait Send/Executor higher-ranked-lifetime check at the caller).
        // `version` is an i32 literal and `name` is a compile-time constant, so
        // inlining them carries no injection risk.
        let batch = format!(
            "{sql}\nINSERT INTO schema_migrations (version, name) VALUES ({version}, '{name}');"
        );
        sqlx::raw_sql(&batch).execute(pool).await?;
    }
    Ok(())
}

/// Applied migration versions. Tolerates a missing tracking table by returning
/// an empty set (parity with SQLite tolerating a missing `schema_migrations`) —
/// R-MIG-4 / §5.2.
pub async fn get_applied(pool: &PgPool) -> anyhow::Result<HashSet<i32>> {
    let table_exists: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = 'schema_migrations'",
    )
    .fetch_optional(pool)
    .await?;
    if table_exists.is_none() {
        return Ok(HashSet::new());
    }
    let versions: Vec<i32> = sqlx::query_scalar("SELECT version FROM schema_migrations")
        .fetch_all(pool)
        .await?;
    Ok(versions.into_iter().collect())
}
