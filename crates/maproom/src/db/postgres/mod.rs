use anyhow::Context;
use async_trait::async_trait;
use std::collections::HashSet;
use tokio_postgres::{Client, NoTls};

use crate::db::{ChunkRecord, FileRecord, SearchHit, VectorStore};

pub struct PostgresStore {
    pub client: Client,
}

impl PostgresStore {
    pub async fn connect() -> anyhow::Result<Self> {
        let database_url = crate::db::connection::get_database_url()
            .context("Failed to determine database connection URL")?;
        let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
        
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("postgres connection error: {e}");
            }
        });

        client.batch_execute("SET ivfflat.probes = 10").await?;

        Ok(Self { client })
    }
}

#[async_trait]
impl VectorStore for PostgresStore {
    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64> {
        super::queries::get_or_create_repo(&self.client, name, root_path).await
    }

    async fn get_or_create_worktree(
        &self,
        repo_id: i64,
        name: &str,
        abs_path: &str,
    ) -> anyhow::Result<i64> {
        super::queries::get_or_create_worktree(&self.client, repo_id, name, abs_path).await
    }

    async fn get_or_create_commit(
        &self,
        repo_id: i64,
        sha: &str,
        committed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<i64> {
        super::queries::get_or_create_commit(&self.client, repo_id, sha, committed_at).await
    }

    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64> {
        super::queries::upsert_file(
            &self.client,
            file.repo_id,
            file.worktree_id,
            file.commit_id,
            &file.relpath,
            file.language.as_deref(),
            &file.content_hash,
            file.size_bytes,
            file.last_modified,
        )
        .await
    }

    async fn insert_chunk(&self, chunk: &ChunkRecord) -> anyhow::Result<i64> {
        super::queries::insert_chunk(
            &self.client,
            chunk.file_id,
            &chunk.blob_sha,
            chunk.symbol_name.as_deref(),
            &chunk.kind,
            chunk.signature.as_deref(),
            chunk.docstring.as_deref(),
            chunk.start_line,
            chunk.end_line,
            &chunk.preview,
            &chunk.ts_doc_text,
            chunk.recency_score,
            chunk.churn_score,
            chunk.metadata.as_ref(),
            chunk.worktree_id,
        )
        .await
    }

    async fn insert_chunks_batch(&self, chunks: &[ChunkRecord]) -> anyhow::Result<Vec<i64>> {
        // Transform ChunkRecord into the tuple format expected by existing insert_chunks_batch
        // This is temporary until we refactor queries.rs to use ChunkRecord directly
        let tuples: Vec<_> = chunks.iter().map(|c| (
            c.file_id,
            c.blob_sha.clone(),
            c.symbol_name.clone(),
            c.kind.clone(),
            c.signature.clone(),
            c.docstring.clone(),
            c.start_line,
            c.end_line,
            c.preview.clone(),
            c.ts_doc_text.clone(),
            c.recency_score,
            c.churn_score,
            c.metadata.clone(),
            c.worktree_id,
        )).collect();
        
        super::queries::insert_chunks_batch(&self.client, &tuples).await
    }

    async fn insert_chunk_edge(
        &self,
        src_chunk_id: i64,
        dst_chunk_id: i64,
        edge_type: &str,
    ) -> anyhow::Result<()> {
        super::queries::insert_chunk_edge(&self.client, src_chunk_id, dst_chunk_id, edge_type).await
    }

    async fn upsert_embeddings(
        &self,
        chunk_id: i64,
        code_embedding: Option<&[f32]>,
        text_embedding: Option<&[f32]>,
        dimension: usize,
    ) -> anyhow::Result<()> {
        super::queries::upsert_embeddings(
            &self.client,
            chunk_id,
            code_embedding,
            text_embedding,
            dimension,
        )
        .await
    }

    async fn batch_upsert_embeddings(
        &self,
        embeddings: &[(i64, Option<Vec<f32>>, Option<Vec<f32>>)],
        dimension: usize,
    ) -> anyhow::Result<()> {
        // Client::transaction is async and requires &mut self, but we only have &self.
        // However, our `client` field is internal and `Client` methods take &self for queries.
        // The issue is `batch_upsert_embeddings` in queries.rs takes `&mut Client`.
        // We need to fix that signature or handle mutability.
        // `tokio_postgres::Client` handles interior mutability for basic queries, but transaction requires `&mut`.
        // Actually, we can't easily get &mut Client from &self in a Sync trait.
        // We might need to clone the client (it's cheap, just a handle) or change the signature in queries.rs
        // to take &Client and use `build_transaction` if possible? No, transaction requires mut.
        // 
        // Workaround: In `queries.rs`, `batch_upsert_embeddings` takes `&mut Client`.
        // Since `PostgresStore` owns `Client`, we can't get mut ref from `&self`.
        // But wait, `VectorStore` trait defines `batch_upsert_embeddings` taking `&self`.
        // We might need internal mutability (Mutex) or change the implementation to not require &mut Client 
        // if possible (e.g. simple query without explicit transaction object, or just batch statements).
        // Or, we clone the client? `Client` is Clone? Yes, it is a handle.
        // Let's try cloning.
        
        let mut client_clone = self.client.clone();
        super::queries::batch_upsert_embeddings(&mut client_clone, embeddings, dimension).await
    }

    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>> {
        super::queries::search_chunks_fts(&self.client, repo, worktree, query, k, debug).await
    }

    async fn find_chunk_by_symbol(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        symbol_name: &str,
        relpath: Option<&str>,
    ) -> anyhow::Result<Option<i64>> {
        super::queries::find_chunk_by_symbol(
            &self.client,
            repo_id,
            worktree_id,
            symbol_name,
            relpath,
        )
        .await
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        super::queries::migrate(&self.client).await
    }

    async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>> {
        super::queries::get_applied_migrations(&self.client).await
    }
}
