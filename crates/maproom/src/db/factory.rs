use crate::db::postgres::PostgresStore;
#[cfg(feature = "sqlite")]
use crate::db::sqlite::SqliteStore;
use crate::db::VectorStore;
use std::sync::Arc;

pub async fn get_store() -> anyhow::Result<Arc<dyn VectorStore>> {
    let url = crate::db::connection::get_database_url().unwrap_or_else(|_| {
        #[cfg(feature = "sqlite")]
        return "sqlite://maproom.db".to_string();
        #[cfg(not(feature = "sqlite"))]
        return "postgresql://maproom:maproom@localhost:5432/maproom".to_string();
    });

    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        let store = PostgresStore::connect().await?;
        Ok(Arc::new(store))
    } else {
        #[cfg(feature = "sqlite")]
        if url.starts_with("sqlite://") || url.starts_with("file:") || !url.contains("://") {
            let store = SqliteStore::connect(&url).await?;
            return Ok(Arc::new(store));
        }
        anyhow::bail!("Unsupported database URL scheme: {}. Enable 'sqlite' feature for SQLite support.", url)
    }
}

