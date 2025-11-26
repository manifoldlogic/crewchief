use anyhow::Context;
use async_trait::async_trait;
use std::collections::HashSet;

use crate::db::{ChunkRecord, FileRecord, PgPool, SearchHit, VectorStore};

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn connect() -> anyhow::Result<Self> {
        let pool = crate::db::pool::create_pool().await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl VectorStore for PostgresStore {
    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::get_or_create_repo(&client, name, root_path).await
    }

    async fn get_or_create_worktree(
        &self,
        repo_id: i64,
        name: &str,
        abs_path: &str,
    ) -> anyhow::Result<i64> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::get_or_create_worktree(&client, repo_id, name, abs_path).await
    }

    async fn get_or_create_commit(
        &self,
        repo_id: i64,
        sha: &str,
        committed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<i64> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::get_or_create_commit(&client, repo_id, sha, committed_at).await
    }

    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::upsert_file(
            &client,
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
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::insert_chunk(
            &client,
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
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
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

        super::queries::insert_chunks_batch(&client, &tuples).await
    }

    async fn insert_chunk_edge(
        &self,
        src_chunk_id: i64,
        dst_chunk_id: i64,
        edge_type: &str,
    ) -> anyhow::Result<()> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::insert_chunk_edge(&client, src_chunk_id, dst_chunk_id, edge_type).await
    }

    async fn upsert_embeddings(
        &self,
        chunk_id: i64,
        code_embedding: Option<&[f32]>,
        text_embedding: Option<&[f32]>,
        dimension: usize,
    ) -> anyhow::Result<()> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::upsert_embeddings(
            &client,
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
        // Get a connection from the pool - we have exclusive ownership of this connection
        // so we can get a mutable reference for transaction support
        let mut client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::batch_upsert_embeddings(&mut client, embeddings, dimension).await
    }

    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::search_chunks_fts(&client, repo, worktree, query, k, debug).await
    }

    async fn search_chunks_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        k: i64,
        debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::search_chunks_vector(&client, repo, worktree, embedding, k, debug).await
    }

    async fn search_chunks_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        embedding: &[f32],
        k: i64,
        debug: bool,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::search_chunks_hybrid(&client, repo, worktree, query, embedding, k, debug).await
    }

    async fn find_chunk_by_symbol(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        symbol_name: &str,
        relpath: Option<&str>,
    ) -> anyhow::Result<Option<i64>> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::find_chunk_by_symbol(
            &client,
            repo_id,
            worktree_id,
            symbol_name,
            relpath,
        )
        .await
    }

    async fn get_chunk_by_id(&self, chunk_id: i64) -> anyhow::Result<Option<crate::db::ChunkFull>> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::get_chunk_by_id(&client, chunk_id).await
    }

    async fn get_file_chunks(&self, file_id: i64) -> anyhow::Result<Vec<crate::db::ChunkSummary>> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::get_file_chunks(&client, file_id).await
    }

    async fn get_chunk_context(&self, chunk_id: i64, surrounding: usize) -> anyhow::Result<Option<crate::db::ChunkContext>> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::get_chunk_context(&client, chunk_id, surrounding).await
    }

    async fn migrate(&self) -> anyhow::Result<()> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::migrate(&client).await
    }

    async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>> {
        let client = self.pool.get().await.context("Failed to get connection from pool")?;
        super::queries::get_applied_migrations(&client).await
    }
}
