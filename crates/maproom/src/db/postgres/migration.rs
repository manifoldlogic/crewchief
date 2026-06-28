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
}
