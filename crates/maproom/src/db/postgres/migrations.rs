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
    (
        3,
        "indexes",
        include_str!("../../../migrations_pg/0003_indexes.sql"),
    ),
];

/// Session-level advisory-lock key serializing concurrent migration runs across
/// processes/connections (ascii "maproom"). The sqlx native migrator takes an
/// equivalent lock; the hand-rolled runner must too, or two processes racing on
/// a fresh DB both run the non-idempotent DDL and the loser aborts with
/// "relation already exists".
const MIGRATION_LOCK_KEY: i64 = 0x006D_6170_726F_6F6D;

/// The migration tracking table (analogous to SQLite's `schema_migrations`).
const TRACKING_DDL: &str = "CREATE TABLE IF NOT EXISTS schema_migrations (\
     version INTEGER PRIMARY KEY, \
     name TEXT NOT NULL, \
     applied_at TIMESTAMPTZ NOT NULL DEFAULT now())";

/// Apply all pending migrations. Idempotent (R-MIG-2): already-applied versions
/// are skipped, so re-running at connect time adds no duplicate work. The entire
/// sequence runs in ONE advisory-lock-guarded transaction (all-or-nothing), so a
/// failure leaves no partially-recorded version and concurrent callers serialize.
pub async fn run(pool: PgPool) -> anyhow::Result<()> {
    // NOTE: takes `PgPool` by value (it is `Arc`-backed, so cloning is cheap). A
    // `&PgPool` here would give this future a higher-ranked borrowed lifetime that
    // the async_trait `migrate()` caller cannot prove `Send`-general-enough once a
    // transaction is held across awaits.
    //
    // Run the entire migration sequence inside ONE transaction guarded by a
    // transaction-scoped advisory lock, so concurrent first-connect callers can't
    // both execute the non-idempotent DDL (the loser would abort with "relation
    // already exists"). The lock auto-releases on commit/rollback, and the whole
    // batch is atomic. `pool.begin()` yields an owned `Transaction<'static>`,
    // which (unlike a held `&mut PoolConnection`) keeps this free fn's future
    // Send-general-enough to be awaited from the async_trait `migrate()` method.
    // `Executor::execute(&str)` (method form) for the multi-statement DDL batches
    // below: the `sqlx::raw_sql(..).execute(&mut *tx)` form trips async_trait's
    // higher-ranked `Executor`/`Send` check at the `migrate()` caller.
    use sqlx::Executor;
    let mut tx = pool.begin().await?;
    // Lift ALL per-session timeouts (set in tuned_pool) for this transaction, via
    // one simple-protocol batch (SET LOCAL reverts at tx end):
    //   * statement_timeout — large DDL (e.g. index builds) can exceed the 5s cap;
    //   * lock_timeout — a loser blocked on pg_advisory_xact_lock under contention
    //     would otherwise abort after the 1s cap, defeating the serialization;
    //   * idle_in_transaction_session_timeout — a loser idle-waiting on the lock
    //     would otherwise be terminated after 30s.
    (&mut *tx)
        .execute(
            "SET LOCAL statement_timeout = 0; \
             SET LOCAL lock_timeout = 0; \
             SET LOCAL idle_in_transaction_session_timeout = 0",
        )
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(MIGRATION_LOCK_KEY)
        .execute(&mut *tx)
        .await?;

    sqlx::query(TRACKING_DDL).execute(&mut *tx).await?;
    // The tracking table now exists (CREATE TABLE IF NOT EXISTS above), so query
    // applied versions directly.
    let versions: Vec<i32> = sqlx::query_scalar("SELECT version FROM schema_migrations")
        .fetch_all(&mut *tx)
        .await?;
    let applied: HashSet<i32> = versions.into_iter().collect();
    for (version, name, sql) in MIGRATIONS {
        if applied.contains(version) {
            continue;
        }
        // raw_sql uses the simple-query protocol, so a file with multiple
        // statements (CREATE EXTENSION; CREATE TABLE; CREATE INDEX; …) runs as one
        // unit. We append the tracking-row insert to the same batch. `version` is
        // an i32 literal and `name` is a compile-time constant, so inlining them
        // carries no injection risk.
        let batch = format!(
            "{sql}\nINSERT INTO schema_migrations (version, name) VALUES ({version}, '{name}');"
        );
        (&mut *tx).execute(batch.as_str()).await?;
    }
    tx.commit().await?;
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
