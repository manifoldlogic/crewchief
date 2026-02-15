use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

/// Convert f32 slice to little-endian bytes for SQLite BLOB storage
pub fn vec_to_blob(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Convert bytes back to f32 slice
pub fn blob_to_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}

/// Format for sqlite-vec query parameter (same as vec_to_blob)
pub fn vec_to_sqlite_param(vec: &[f32]) -> Vec<u8> {
    vec_to_blob(vec) // sqlite-vec accepts raw bytes
}

pub use crate::db::types::EmbeddingRecord;

/// Supported embedding dimensions
const SUPPORTED_DIMENSIONS: &[usize] = &[768, 1024, 1536];

/// Get the appropriate vec table name for a given dimension
fn get_vec_table_name(dimension: usize) -> Result<&'static str> {
    match dimension {
        768 => Ok("vec_code_768"),
        1024 => Ok("vec_code_1024"),
        1536 => Ok("vec_code"),
        _ => bail!(
            "Unsupported embedding dimension: {}. Supported dimensions: {:?}",
            dimension,
            SUPPORTED_DIMENSIONS
        ),
    }
}

/// Store or update embedding by content hash
pub fn upsert_embedding(
    conn: &Connection,
    blob_sha: &str,
    embedding: &[f32],
    model_version: &str,
) -> Result<i64> {
    // Validate embedding dimension
    let dimension = embedding.len();
    if !SUPPORTED_DIMENSIONS.contains(&dimension) {
        bail!(
            "Unsupported embedding dimension: {}. Supported dimensions: {:?}",
            dimension,
            SUPPORTED_DIMENSIONS
        );
    }

    let blob = vec_to_blob(embedding);

    // Insert or update in code_embeddings table
    conn.execute(
        "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(blob_sha) DO UPDATE SET
           embedding = excluded.embedding,
           model_version = excluded.model_version,
           embedding_dim = excluded.embedding_dim",
        params![blob_sha, blob, embedding.len() as i32, model_version],
    )
    .context("Failed to upsert embedding")?;

    // Get the rowid for the inserted/updated embedding
    let rowid: i64 = conn
        .query_row(
            "SELECT id FROM code_embeddings WHERE blob_sha = ?1",
            params![blob_sha],
            |row| row.get(0),
        )
        .context("Failed to retrieve embedding id")?;

    Ok(rowid)
}

/// Batch upsert with deduplication
///
/// Returns a vector of (embedding_id, embedding) pairs for subsequent syncing to vec_code
pub fn upsert_embeddings_batch(
    conn: &mut Connection,
    embeddings: &[EmbeddingRecord],
) -> Result<Vec<(i64, Vec<f32>)>> {
    // Validate all embeddings have supported dimensions
    for (idx, record) in embeddings.iter().enumerate() {
        let dimension = record.embedding.len();
        if !SUPPORTED_DIMENSIONS.contains(&dimension) {
            bail!(
                "Embedding at index {} has unsupported dimension: {}. Supported dimensions: {:?}",
                idx,
                dimension,
                SUPPORTED_DIMENSIONS
            );
        }
    }

    // Use a transaction for batch operation
    let tx = conn
        .transaction()
        .context("Failed to begin transaction for batch embedding upsert")?;

    let mut result = Vec::new();

    {
        // Prepare statements for reuse
        let mut upsert_stmt = tx.prepare(
            "INSERT INTO code_embeddings (blob_sha, embedding, embedding_dim, model_version)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(blob_sha) DO UPDATE SET
               embedding = excluded.embedding,
               model_version = excluded.model_version,
               embedding_dim = excluded.embedding_dim",
        )?;

        let mut get_id_stmt = tx.prepare("SELECT id FROM code_embeddings WHERE blob_sha = ?1")?;

        for record in embeddings {
            let blob = vec_to_blob(&record.embedding);

            // Upsert into code_embeddings
            upsert_stmt.execute(params![
                record.blob_sha,
                blob,
                record.embedding.len() as i32,
                record.model_version,
            ])?;

            // Get the rowid for syncing
            let embedding_id: i64 =
                get_id_stmt.query_row(params![record.blob_sha], |row| row.get(0))?;

            result.push((embedding_id, record.embedding.clone()));
        }
    }

    tx.commit()
        .context("Failed to commit batch embedding upsert transaction")?;

    Ok(result)
}

/// Check if embedding exists for blob_sha
pub fn has_embedding(conn: &Connection, blob_sha: &str) -> Result<bool> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM code_embeddings WHERE blob_sha = ?1",
            params![blob_sha],
            |_| Ok(true),
        )
        .optional()
        .context("Failed to check if embedding exists")?
        .unwrap_or(false);

    Ok(exists)
}

/// Get embedding by blob_sha
pub fn get_embedding(conn: &Connection, blob_sha: &str) -> Result<Option<Vec<f32>>> {
    let result: Option<Vec<u8>> = conn
        .query_row(
            "SELECT embedding FROM code_embeddings WHERE blob_sha = ?1",
            params![blob_sha],
            |row| row.get(0),
        )
        .optional()
        .context("Failed to get embedding")?;

    Ok(result.map(|blob| blob_to_vec(&blob)))
}

/// Sync single embedding to vector index (vec_code or vec_code_768 table)
///
/// This function syncs an embedding from code_embeddings to the appropriate vec_code virtual table
/// for vector similarity search. The rowid in vec_code matches the id in code_embeddings
/// to enable joining search results back to chunks.
pub fn sync_embedding_to_vec(
    conn: &Connection,
    embedding_id: i64,
    embedding: &[f32],
) -> Result<()> {
    // Determine which vec table to use based on dimension
    let dimension = embedding.len();
    let vec_table = get_vec_table_name(dimension)?;

    // Delete existing if any (for updates)
    // This is needed because vec_code doesn't support UPDATE
    let delete_sql = format!("DELETE FROM {} WHERE rowid = ?1", vec_table);
    conn.execute(&delete_sql, params![embedding_id])
        .with_context(|| format!("Failed to delete from {}", vec_table))?;

    // Convert embedding to blob
    let blob = vec_to_blob(embedding);

    // Insert with explicit rowid to match code_embeddings.id
    let insert_sql = format!(
        "INSERT INTO {}(rowid, embedding) VALUES (?1, ?2)",
        vec_table
    );
    conn.execute(&insert_sql, params![embedding_id, blob])
        .with_context(|| format!("Failed to insert into {}", vec_table))?;

    Ok(())
}

/// Sync all embeddings not yet in vec_code, vec_code_768, or vec_code_1024
///
/// This function finds all embeddings in code_embeddings that don't have a corresponding
/// entry in their respective vec tables and syncs them. Returns the number of embeddings synced.
pub fn sync_all_embeddings_to_vec(conn: &Connection) -> Result<usize> {
    let mut count = 0;

    // Sync 1536-dim embeddings to vec_code
    let mut stmt_1536 = conn
        .prepare(
            "SELECT e.id, e.embedding FROM code_embeddings e
             WHERE e.embedding_dim = 1536
               AND NOT EXISTS (SELECT 1 FROM vec_code v WHERE v.rowid = e.id)",
        )
        .context("Failed to prepare query for unsynced 1536-dim embeddings")?;

    let rows_1536 = stmt_1536
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .context("Failed to query unsynced 1536-dim embeddings")?;

    for row in rows_1536 {
        let (id, blob) = row.context("Failed to read 1536-dim embedding row")?;
        conn.execute(
            "INSERT INTO vec_code(rowid, embedding) VALUES (?1, ?2)",
            params![id, blob],
        )
        .context("Failed to insert into vec_code during batch sync")?;
        count += 1;
    }

    // Sync 1024-dim embeddings to vec_code_1024
    let mut stmt_1024 = conn
        .prepare(
            "SELECT e.id, e.embedding FROM code_embeddings e
             WHERE e.embedding_dim = 1024
               AND NOT EXISTS (SELECT 1 FROM vec_code_1024 v WHERE v.rowid = e.id)",
        )
        .context("Failed to prepare query for unsynced 1024-dim embeddings")?;

    let rows_1024 = stmt_1024
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .context("Failed to query unsynced 1024-dim embeddings")?;

    for row in rows_1024 {
        let (id, blob) = row.context("Failed to read 1024-dim embedding row")?;
        conn.execute(
            "INSERT INTO vec_code_1024(rowid, embedding) VALUES (?1, ?2)",
            params![id, blob],
        )
        .context("Failed to insert into vec_code_1024 during batch sync")?;
        count += 1;
    }

    // Sync 768-dim embeddings to vec_code_768
    let mut stmt_768 = conn
        .prepare(
            "SELECT e.id, e.embedding FROM code_embeddings e
             WHERE e.embedding_dim = 768
               AND NOT EXISTS (SELECT 1 FROM vec_code_768 v WHERE v.rowid = e.id)",
        )
        .context("Failed to prepare query for unsynced 768-dim embeddings")?;

    let rows_768 = stmt_768
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .context("Failed to query unsynced 768-dim embeddings")?;

    for row in rows_768 {
        let (id, blob) = row.context("Failed to read 768-dim embedding row")?;
        conn.execute(
            "INSERT INTO vec_code_768(rowid, embedding) VALUES (?1, ?2)",
            params![id, blob],
        )
        .context("Failed to insert into vec_code_768 during batch sync")?;
        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_connection() -> Connection {
        // Register extension globally
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                crate::db::sqlite::sqlite3_vec_init as *const (),
            )));
        }

        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");

        // Enable foreign keys
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            "#,
        )
        .expect("Failed to enable foreign keys");

        // Create schema
        conn.execute_batch(
            r#"
            CREATE TABLE code_embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                blob_sha TEXT NOT NULL UNIQUE,
                embedding BLOB,
                embedding_dim INTEGER NOT NULL DEFAULT 1536,
                model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX idx_embeddings_blob ON code_embeddings(blob_sha);

            CREATE VIRTUAL TABLE vec_code USING vec0(
                embedding float[1536]
            );

            CREATE VIRTUAL TABLE vec_code_1024 USING vec0(
                embedding float[1024]
            );

            CREATE VIRTUAL TABLE vec_code_768 USING vec0(
                embedding float[768]
            );
            "#,
        )
        .expect("Failed to create schema");

        conn
    }

    #[test]
    fn test_vec_to_blob_and_back() {
        let original = vec![0.1, 0.2, 0.3, -0.5, 1.0];
        let blob = vec_to_blob(&original);
        let recovered = blob_to_vec(&blob);

        assert_eq!(original.len(), recovered.len());
        for (a, b) in original.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_vec_to_blob_size() {
        let vec = vec![1.0; 1536];
        let blob = vec_to_blob(&vec);
        // Each f32 is 4 bytes
        assert_eq!(blob.len(), 1536 * 4);
    }

    #[test]
    fn test_empty_vec() {
        let vec: Vec<f32> = vec![];
        let blob = vec_to_blob(&vec);
        assert_eq!(blob.len(), 0);
        let recovered = blob_to_vec(&blob);
        assert_eq!(recovered.len(), 0);
    }

    #[test]
    fn test_vec_to_sqlite_param() {
        let vec = vec![1.0, 2.0, 3.0];
        let param = vec_to_sqlite_param(&vec);
        let blob = vec_to_blob(&vec);
        assert_eq!(param, blob);
    }

    #[test]
    fn test_vector_table_sync() {
        let conn = setup_test_connection();

        // Create a 1536-dimensional embedding
        let embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();

        // Insert embedding into code_embeddings
        let embedding_id = upsert_embedding(&conn, "test_blob_sha", &embedding, "model-v1")
            .expect("Failed to upsert embedding");

        assert!(embedding_id > 0);

        // Sync to vec_code
        sync_embedding_to_vec(&conn, embedding_id, &embedding).expect("Failed to sync to vec_code");

        // Verify the embedding exists in vec_code with matching rowid
        let vec_code_rowid: i64 = conn
            .query_row(
                "SELECT rowid FROM vec_code WHERE rowid = ?1",
                params![embedding_id],
                |row| row.get(0),
            )
            .expect("Failed to query vec_code rowid");

        assert_eq!(
            vec_code_rowid, embedding_id,
            "Rowid in vec_code should match embedding_id"
        );

        // Verify the embedding data is correct
        let vec_code_blob: Vec<u8> = conn
            .query_row(
                "SELECT embedding FROM vec_code WHERE rowid = ?1",
                params![embedding_id],
                |row| row.get(0),
            )
            .expect("Failed to query vec_code embedding");

        let retrieved_embedding = blob_to_vec(&vec_code_blob);
        assert_eq!(retrieved_embedding.len(), 1536);
        for (a, b) in embedding.iter().zip(retrieved_embedding.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_vector_table_sync_update() {
        let conn = setup_test_connection();

        // Create two different embeddings
        let embedding1: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding2: Vec<f32> = (0..1536).map(|i| (i as f32 + 1.0) / 1536.0).collect();

        // Insert first embedding
        let embedding_id = upsert_embedding(&conn, "test_blob", &embedding1, "model-v1")
            .expect("Failed to upsert embedding");

        // Sync first embedding
        sync_embedding_to_vec(&conn, embedding_id, &embedding1)
            .expect("Failed to sync first embedding");

        // Update with second embedding (same blob_sha)
        let updated_id = upsert_embedding(&conn, "test_blob", &embedding2, "model-v2")
            .expect("Failed to update embedding");

        assert_eq!(
            embedding_id, updated_id,
            "ID should remain the same on update"
        );

        // Sync updated embedding (should replace old one)
        sync_embedding_to_vec(&conn, updated_id, &embedding2)
            .expect("Failed to sync updated embedding");

        // Verify only one entry exists in vec_code
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM vec_code WHERE rowid = ?1",
                params![embedding_id],
                |row| row.get(0),
            )
            .expect("Failed to count vec_code entries");

        assert_eq!(count, 1, "Should only have one entry in vec_code");

        // Verify the updated embedding is stored
        let vec_code_blob: Vec<u8> = conn
            .query_row(
                "SELECT embedding FROM vec_code WHERE rowid = ?1",
                params![embedding_id],
                |row| row.get(0),
            )
            .expect("Failed to query vec_code embedding");

        let retrieved = blob_to_vec(&vec_code_blob);
        // Verify it's embedding2, not embedding1
        assert!((retrieved[0] - embedding2[0]).abs() < 1e-6);
        assert!((retrieved[100] - embedding2[100]).abs() < 1e-6);
    }

    #[test]
    fn test_sync_all_embeddings_to_vec() {
        let conn = setup_test_connection();

        // Create multiple embeddings
        let embedding1: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding2: Vec<f32> = (0..1536).map(|i| (i as f32 + 1.0) / 1536.0).collect();
        let embedding3: Vec<f32> = (0..1536).map(|i| (i as f32 + 2.0) / 1536.0).collect();

        // Insert embeddings into code_embeddings only (not vec_code)
        upsert_embedding(&conn, "blob1", &embedding1, "model-v1")
            .expect("Failed to upsert embedding1");
        upsert_embedding(&conn, "blob2", &embedding2, "model-v1")
            .expect("Failed to upsert embedding2");
        upsert_embedding(&conn, "blob3", &embedding3, "model-v1")
            .expect("Failed to upsert embedding3");

        // Verify vec_code is empty
        let count_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))
            .expect("Failed to count vec_code");
        assert_eq!(count_before, 0);

        // Sync all embeddings
        let synced_count =
            sync_all_embeddings_to_vec(&conn).expect("Failed to sync all embeddings");

        assert_eq!(synced_count, 3, "Should have synced 3 embeddings");

        // Verify vec_code now has all embeddings
        let count_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))
            .expect("Failed to count vec_code");
        assert_eq!(count_after, 3);

        // Verify rowid mapping is correct
        let id1: i64 = conn
            .query_row(
                "SELECT id FROM code_embeddings WHERE blob_sha = 'blob1'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to get id1");

        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM vec_code WHERE rowid = ?1",
                params![id1],
                |_| Ok(true),
            )
            .unwrap_or(false);

        assert!(
            exists,
            "vec_code should have entry with rowid matching code_embeddings.id"
        );

        // Run sync again - should sync 0 (idempotent)
        let synced_again = sync_all_embeddings_to_vec(&conn).expect("Failed to sync again");
        assert_eq!(synced_again, 0, "Second sync should find nothing to sync");
    }

    #[test]
    fn test_vector_table_sync_graceful_degradation() {
        // Create connection without vec extension
        let conn = Connection::open_in_memory().expect("Failed to open in-memory database");

        conn.execute_batch(
            r#"
            CREATE TABLE code_embeddings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                blob_sha TEXT NOT NULL UNIQUE,
                embedding BLOB,
                embedding_dim INTEGER NOT NULL DEFAULT 1536,
                model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "#,
        )
        .expect("Failed to create schema");

        let embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();

        // Insert embedding should work
        let embedding_id = upsert_embedding(&conn, "test", &embedding, "model-v1")
            .expect("Upsert should work even without vec extension");

        // Sync should fail because vec_code doesn't exist
        let result = sync_embedding_to_vec(&conn, embedding_id, &embedding);
        assert!(
            result.is_err(),
            "Sync should fail when vec_code table doesn't exist"
        );

        // sync_all should also fail
        let result = sync_all_embeddings_to_vec(&conn);
        assert!(
            result.is_err(),
            "Sync all should fail when vec_code table doesn't exist"
        );
    }

    #[test]
    fn test_768_dim_embedding_storage() {
        let conn = setup_test_connection();

        // Create a 768-dimensional embedding
        let embedding: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();

        // Insert embedding
        let embedding_id = upsert_embedding(&conn, "test_768", &embedding, "nomic-embed-text")
            .expect("Failed to upsert 768-dim embedding");

        assert!(embedding_id > 0);

        // Verify embedding was stored with correct dimension
        let stored_dim: i32 = conn
            .query_row(
                "SELECT embedding_dim FROM code_embeddings WHERE blob_sha = 'test_768'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query embedding_dim");

        assert_eq!(stored_dim, 768, "Stored dimension should be 768");

        // Retrieve and verify embedding
        let retrieved = get_embedding(&conn, "test_768")
            .expect("Failed to get embedding")
            .expect("Embedding should exist");

        assert_eq!(retrieved.len(), 768);
        for (a, b) in embedding.iter().zip(retrieved.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_768_dim_vector_table_sync() {
        let conn = setup_test_connection();

        // Create a 768-dimensional embedding
        let embedding: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();

        // Insert embedding
        let embedding_id = upsert_embedding(&conn, "test_768_sync", &embedding, "nomic-embed-text")
            .expect("Failed to upsert 768-dim embedding");

        // Sync to vec_code_768
        sync_embedding_to_vec(&conn, embedding_id, &embedding)
            .expect("Failed to sync 768-dim embedding to vec_code_768");

        // Verify the embedding exists in vec_code_768 with matching rowid
        let vec_code_rowid: i64 = conn
            .query_row(
                "SELECT rowid FROM vec_code_768 WHERE rowid = ?1",
                params![embedding_id],
                |row| row.get(0),
            )
            .expect("Failed to query vec_code_768 rowid");

        assert_eq!(
            vec_code_rowid, embedding_id,
            "Rowid in vec_code_768 should match embedding_id"
        );

        // Verify the embedding data is correct
        let vec_code_blob: Vec<u8> = conn
            .query_row(
                "SELECT embedding FROM vec_code_768 WHERE rowid = ?1",
                params![embedding_id],
                |row| row.get(0),
            )
            .expect("Failed to query vec_code_768 embedding");

        let retrieved_embedding = blob_to_vec(&vec_code_blob);
        assert_eq!(retrieved_embedding.len(), 768);
        for (a, b) in embedding.iter().zip(retrieved_embedding.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_1024_dim_embedding_storage() {
        let conn = setup_test_connection();

        // Create a 1024-dimensional embedding
        let embedding: Vec<f32> = (0..1024).map(|i| i as f32 / 1024.0).collect();

        // Insert embedding
        let embedding_id = upsert_embedding(&conn, "test_1024", &embedding, "mxbai-embed-large")
            .expect("Failed to upsert 1024-dim embedding");

        assert!(embedding_id > 0);

        // Verify embedding was stored with correct dimension
        let stored_dim: i32 = conn
            .query_row(
                "SELECT embedding_dim FROM code_embeddings WHERE blob_sha = 'test_1024'",
                [],
                |row| row.get(0),
            )
            .expect("Failed to query embedding_dim");

        assert_eq!(stored_dim, 1024, "Stored dimension should be 1024");

        // Retrieve and verify embedding
        let retrieved = get_embedding(&conn, "test_1024")
            .expect("Failed to get embedding")
            .expect("Embedding should exist");

        assert_eq!(retrieved.len(), 1024);
        for (a, b) in embedding.iter().zip(retrieved.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_1024_dim_vector_table_sync() {
        let conn = setup_test_connection();

        // Create a 1024-dimensional embedding
        let embedding: Vec<f32> = (0..1024).map(|i| i as f32 / 1024.0).collect();

        // Insert embedding
        let embedding_id =
            upsert_embedding(&conn, "test_1024_sync", &embedding, "mxbai-embed-large")
                .expect("Failed to upsert 1024-dim embedding");

        // Sync to vec_code_1024
        sync_embedding_to_vec(&conn, embedding_id, &embedding)
            .expect("Failed to sync 1024-dim embedding to vec_code_1024");

        // Verify the embedding exists in vec_code_1024 with matching rowid
        let vec_code_rowid: i64 = conn
            .query_row(
                "SELECT rowid FROM vec_code_1024 WHERE rowid = ?1",
                params![embedding_id],
                |row| row.get(0),
            )
            .expect("Failed to query vec_code_1024 rowid");

        assert_eq!(
            vec_code_rowid, embedding_id,
            "Rowid in vec_code_1024 should match embedding_id"
        );

        // Verify the embedding data is correct
        let vec_code_blob: Vec<u8> = conn
            .query_row(
                "SELECT embedding FROM vec_code_1024 WHERE rowid = ?1",
                params![embedding_id],
                |row| row.get(0),
            )
            .expect("Failed to query vec_code_1024 embedding");

        let retrieved_embedding = blob_to_vec(&vec_code_blob);
        assert_eq!(retrieved_embedding.len(), 1024);
        for (a, b) in embedding.iter().zip(retrieved_embedding.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_mixed_dimensions_storage() {
        let conn = setup_test_connection();

        // Create 768-dim, 1024-dim, and 1536-dim embeddings
        let embedding_768: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
        let embedding_1024: Vec<f32> = (0..1024).map(|i| i as f32 / 1024.0).collect();
        let embedding_1536: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();

        // Insert all three
        let id_768 = upsert_embedding(&conn, "blob_768", &embedding_768, "nomic-embed-text")
            .expect("Failed to upsert 768-dim");
        let id_1024 = upsert_embedding(&conn, "blob_1024", &embedding_1024, "mxbai-embed-large")
            .expect("Failed to upsert 1024-dim");
        let id_1536 = upsert_embedding(
            &conn,
            "blob_1536",
            &embedding_1536,
            "text-embedding-3-small",
        )
        .expect("Failed to upsert 1536-dim");

        // Verify all exist with correct dimensions
        let dim_768: i32 = conn
            .query_row(
                "SELECT embedding_dim FROM code_embeddings WHERE id = ?1",
                params![id_768],
                |row| row.get(0),
            )
            .expect("Failed to query dim_768");

        let dim_1024: i32 = conn
            .query_row(
                "SELECT embedding_dim FROM code_embeddings WHERE id = ?1",
                params![id_1024],
                |row| row.get(0),
            )
            .expect("Failed to query dim_1024");

        let dim_1536: i32 = conn
            .query_row(
                "SELECT embedding_dim FROM code_embeddings WHERE id = ?1",
                params![id_1536],
                |row| row.get(0),
            )
            .expect("Failed to query dim_1536");

        assert_eq!(dim_768, 768);
        assert_eq!(dim_1024, 1024);
        assert_eq!(dim_1536, 1536);
    }

    #[test]
    fn test_sync_all_mixed_dimensions() {
        let conn = setup_test_connection();

        // Create embeddings of all three dimensions
        let embedding_768_a: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
        let embedding_768_b: Vec<f32> = (0..768).map(|i| (i as f32 + 1.0) / 768.0).collect();
        let embedding_1024_a: Vec<f32> = (0..1024).map(|i| i as f32 / 1024.0).collect();
        let embedding_1024_b: Vec<f32> = (0..1024).map(|i| (i as f32 + 1.0) / 1024.0).collect();
        let embedding_1536_a: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        let embedding_1536_b: Vec<f32> = (0..1536).map(|i| (i as f32 + 1.0) / 1536.0).collect();

        // Insert all embeddings
        upsert_embedding(&conn, "blob_768_a", &embedding_768_a, "nomic-embed-text")
            .expect("Failed to upsert 768-dim a");
        upsert_embedding(&conn, "blob_768_b", &embedding_768_b, "nomic-embed-text")
            .expect("Failed to upsert 768-dim b");
        upsert_embedding(&conn, "blob_1024_a", &embedding_1024_a, "mxbai-embed-large")
            .expect("Failed to upsert 1024-dim a");
        upsert_embedding(&conn, "blob_1024_b", &embedding_1024_b, "mxbai-embed-large")
            .expect("Failed to upsert 1024-dim b");
        upsert_embedding(
            &conn,
            "blob_1536_a",
            &embedding_1536_a,
            "text-embedding-3-small",
        )
        .expect("Failed to upsert 1536-dim a");
        upsert_embedding(
            &conn,
            "blob_1536_b",
            &embedding_1536_b,
            "text-embedding-3-small",
        )
        .expect("Failed to upsert 1536-dim b");

        // Verify vec tables are empty
        let count_768_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM vec_code_768", [], |row| row.get(0))
            .expect("Failed to count vec_code_768");
        let count_1024_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM vec_code_1024", [], |row| row.get(0))
            .expect("Failed to count vec_code_1024");
        let count_1536_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))
            .expect("Failed to count vec_code");

        assert_eq!(count_768_before, 0);
        assert_eq!(count_1024_before, 0);
        assert_eq!(count_1536_before, 0);

        // Sync all embeddings
        let synced_count =
            sync_all_embeddings_to_vec(&conn).expect("Failed to sync all embeddings");

        assert_eq!(
            synced_count, 6,
            "Should have synced 6 embeddings (2 of each dimension)"
        );

        // Verify correct counts in each table
        let count_768_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM vec_code_768", [], |row| row.get(0))
            .expect("Failed to count vec_code_768");
        let count_1024_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM vec_code_1024", [], |row| row.get(0))
            .expect("Failed to count vec_code_1024");
        let count_1536_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM vec_code", [], |row| row.get(0))
            .expect("Failed to count vec_code");

        assert_eq!(count_768_after, 2, "vec_code_768 should have 2 embeddings");
        assert_eq!(
            count_1024_after, 2,
            "vec_code_1024 should have 2 embeddings"
        );
        assert_eq!(count_1536_after, 2, "vec_code should have 2 embeddings");

        // Run sync again - should sync 0 (idempotent)
        let synced_again = sync_all_embeddings_to_vec(&conn).expect("Failed to sync again");
        assert_eq!(synced_again, 0, "Second sync should find nothing to sync");
    }

    #[test]
    fn test_unsupported_dimension() {
        let conn = setup_test_connection();

        // Try to insert 512-dim embedding (unsupported)
        let embedding_512: Vec<f32> = (0..512).map(|i| i as f32 / 512.0).collect();

        let result = upsert_embedding(&conn, "test_512", &embedding_512, "bad-model");
        assert!(result.is_err(), "Should reject unsupported dimension");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("512"),
            "Error should mention the dimension"
        );
        assert!(
            err_msg.contains("768") && err_msg.contains("1024") && err_msg.contains("1536"),
            "Error should list supported dimensions"
        );
    }
}
