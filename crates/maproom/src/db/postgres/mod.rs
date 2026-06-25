//! PostgreSQL + pgvector storage backend for the `maproom` crate (spec §6).
//!
//! Implements the 9 sub-trait `Store` contract (`src/db/traits.rs`) against a
//! native-async `sqlx::PgPool`. Selected at runtime by a `postgres://` database
//! URL and gated behind the `postgres` Cargo feature, so the default build is
//! unchanged (R-DEP-4). The `Store` supertrait is satisfied via its blanket
//! impl — there is intentionally NO `impl Store for PostgresStore`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

mod chunks;
mod cleanup;
mod core;
mod embeddings;
mod encoding;
mod graph;
mod index_state;
mod migration;
pub mod migrations;
mod search;

#[cfg(test)]
mod tests;

/// A PostgreSQL-backed [`Store`](crate::db::traits::Store).
///
/// Cloneable and `Send + Sync` (the pool is internally reference-counted), so it
/// can be held as `Arc<dyn Store + Send + Sync>` across `.await` points by async
/// consumers (R-WIRE-2/7).
#[derive(Clone)]
pub struct PostgresStore {
    pub(crate) pool: PgPool,
    /// Cached result of the one-time `pg_extension` probe for pgvector. When
    /// false, vector/hybrid search degrades to FTS-only (R-TRAIT-3 / R-SEARCH-5).
    pub(crate) vec_available: Arc<AtomicBool>,
}

impl PostgresStore {
    /// Connect to a `postgres://`/`postgresql://` DSN, auto-run migrations
    /// (R-MIG-2), and probe pgvector availability. The pool is tuned from
    /// `DatabaseConfig` (R-WIRE-6); the DSN itself stays in the URL.
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let cfg = crate::config::DatabaseConfig::default();
        let pool = Self::tuned_pool(url, &cfg).await?;
        Self::from_pool(pool).await
    }

    /// Build a `PgPool` tuned from `DatabaseConfig` (R-WIRE-6): pool size +
    /// connection/lifetime timeouts at the pool level, and per-session GUCs
    /// (`statement_timeout`/`lock_timeout`/`idle_in_transaction_session_timeout`/
    /// `work_mem`) applied on every new connection via `after_connect`.
    async fn tuned_pool(url: &str, cfg: &crate::config::DatabaseConfig) -> anyhow::Result<PgPool> {
        use std::time::Duration;
        // SET ... values come from trusted config defaults (Postgres `SET` does
        // not accept bind params, so they are formatted in).
        let session_sql = format!(
            "SET statement_timeout = {}; SET lock_timeout = {}; \
             SET idle_in_transaction_session_timeout = {}; SET work_mem = '{}'",
            cfg.statement_timeout_ms,
            cfg.lock_timeout_ms,
            cfg.idle_in_transaction_timeout_ms,
            cfg.work_mem,
        );
        let pool = PgPoolOptions::new()
            .max_connections(cfg.pool_size as u32)
            .acquire_timeout(Duration::from_millis(cfg.connection_timeout_ms))
            .max_lifetime(Duration::from_secs(cfg.max_connection_lifetime_secs))
            .idle_timeout(Duration::from_secs(cfg.idle_connection_timeout_secs))
            .after_connect(move |conn, _meta| {
                let session_sql = session_sql.clone();
                Box::pin(async move {
                    use sqlx::Executor;
                    conn.execute(session_sql.as_str()).await?;
                    Ok(())
                })
            })
            .connect(url)
            .await?;
        Ok(pool)
    }

    /// Build a store from an existing pool (used by `connect` and tests).
    pub async fn from_pool(pool: PgPool) -> anyhow::Result<Self> {
        migrations::run(&pool).await?;
        let vec_available = probe_vector_extension(&pool).await?;
        Ok(Self {
            pool,
            vec_available: Arc::new(AtomicBool::new(vec_available)),
        })
    }

    /// Test-only constructor that forces the cached pgvector flag, so the
    /// FTS-only degraded branch can be exercised on `PostgresStore` itself
    /// without a separate mock (R-SEARCH-5 option (b), wired up in Phase 2).
    #[cfg(test)]
    #[allow(dead_code)] // used by the Phase-2 degraded-search test
    pub(crate) fn with_vec_available(pool: PgPool, vec_available: bool) -> Self {
        Self {
            pool,
            vec_available: Arc::new(AtomicBool::new(vec_available)),
        }
    }

    /// Cached pgvector availability (the single synchronous `Store` method).
    pub fn has_vector_extension(&self) -> bool {
        self.vec_available.load(Ordering::Relaxed)
    }
}

/// One-time `SELECT 1 FROM pg_extension WHERE extname = 'vector'` probe.
async fn probe_vector_extension(pool: &PgPool) -> anyhow::Result<bool> {
    let found: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM pg_extension WHERE extname = 'vector'")
            .fetch_optional(pool)
            .await?;
    Ok(found.is_some())
}

// ── Compile-time gates (R-WIRE-7 / object-safety, spec §2.2) ────────────────
// PostgresStore satisfies `Store` solely via the blanket impl over the 9
// sub-traits; these dead-but-type-checked fns prove object-safety and that the
// value is usable as `Arc<dyn Store + Send + Sync>` across `.await` points.
#[allow(dead_code)]
fn _assert_postgres_object_safe(s: &PostgresStore) {
    let _: &dyn crate::db::traits::StoreCore = s;
    let _: &dyn crate::db::traits::StoreChunks = s;
    let _: &dyn crate::db::traits::StoreSearch = s;
    let _: &dyn crate::db::traits::StoreGraph = s;
    let _: &dyn crate::db::traits::StoreEmbeddings = s;
    let _: &dyn crate::db::traits::StoreMigration = s;
    let _: &dyn crate::db::traits::StoreCleanup = s;
    let _: &dyn crate::db::traits::StoreIndexState = s;
    let _: &dyn crate::db::traits::StoreEncoding = s;
    let _: &(dyn crate::db::traits::Store + Send + Sync) = s;
}

#[allow(dead_code)]
fn _assert_arc_store_send_sync(s: std::sync::Arc<dyn crate::db::traits::Store + Send + Sync>) {
    fn req<T: Send + Sync + 'static>(_: &T) {}
    req(&s);
}

#[allow(dead_code)]
fn _assert_postgres_is_arc_store(s: PostgresStore) {
    _assert_arc_store_send_sync(std::sync::Arc::new(s));
}
