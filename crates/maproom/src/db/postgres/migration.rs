//! `StoreMigration` impl — applies `migrations_pg/` and exposes applied integer
//! versions (§5.2 / §6.3). Backed by the hand-rolled runner in [`super::migrations`].

use std::collections::HashSet;

use async_trait::async_trait;

use super::PostgresStore;
use crate::db::traits::StoreMigration;

#[async_trait]
impl StoreMigration for PostgresStore {
    async fn migrate(&self) -> anyhow::Result<()> {
        super::migrations::run(self.pool.clone()).await
    }

    async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>> {
        super::migrations::get_applied(&self.pool).await
    }

    async fn verify_schema(&self) -> anyhow::Result<Vec<String>> {
        // R03 / R-VER-3: Postgres checks ONLY the 11 core tables —
        // `context_cache` is SQLite-only by design and must not be flagged
        // here (a single shared 12-table list would mark every healthy PG
        // database as damaged).
        let mut missing = Vec::new();
        for name in crate::db::traits::REQUIRED_TABLES_CORE {
            let exists: Option<bool> = sqlx::query_scalar(
                "SELECT to_regclass('public.' || $1) IS NOT NULL",
            )
            .bind(name)
            .fetch_one(&self.pool)
            .await?;
            if !exists.unwrap_or(false) {
                missing.push(name.to_string());
            }
        }
        Ok(missing)
    }
}
