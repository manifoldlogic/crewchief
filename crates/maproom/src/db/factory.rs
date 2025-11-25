use crate::db::postgres::PostgresStore;
use crate::db::sqlite::SqliteStore;
use crate::db::VectorStore;
use std::sync::Arc;

pub async fn get_store() -> anyhow::Result<Arc<dyn VectorStore>> {
    let url = crate::db::connection::get_database_url().unwrap_or_else(|_| "sqlite://maproom.db".to_string());

    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        let store = PostgresStore::connect().await?;
        Ok(Arc::new(store))
    } else if url.starts_with("sqlite://") || url.starts_with("file:") || !url.contains("://") {
        let store = SqliteStore::connect(&url).await?;
        Ok(Arc::new(store))
    } else {
        anyhow::bail!("Unsupported database URL scheme: {}", url);
    }
}

