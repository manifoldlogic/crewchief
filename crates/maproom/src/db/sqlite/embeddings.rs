use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

/// Convert f32 slice to little-endian bytes for SQLite BLOB storage
pub fn vec_to_blob(vec: &[f32]) -> Vec<u8> {
    vec.iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
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

/// Record for batch embedding operations
#[derive(Clone)]
pub struct EmbeddingRecord {
    pub blob_sha: String,
    pub embedding: Vec<f32>,
    pub model_version: String,
}

/// Store or update embedding by content hash
pub fn upsert_embedding(
    conn: &Connection,
    blob_sha: &str,
    embedding: &[f32],
    model_version: &str,
) -> Result<i64> {
    // Validate embedding dimension is 1536
    if embedding.len() != 1536 {
        bail!(
            "Unsupported embedding dimension: {}. Only 1536-dimensional embeddings are currently supported.",
            embedding.len()
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
    let rowid: i64 = conn.query_row(
        "SELECT id FROM code_embeddings WHERE blob_sha = ?1",
        params![blob_sha],
        |row| row.get(0),
    )
    .context("Failed to retrieve embedding id")?;

    // Also insert/update in vec_code virtual table for similarity search
    // First check if exists in vec_code
    let exists_in_vec_code: bool = conn
        .query_row(
            "SELECT 1 FROM vec_code WHERE rowid = ?1",
            params![rowid],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if exists_in_vec_code {
        // Update existing vector
        conn.execute(
            "UPDATE vec_code SET embedding = ?1 WHERE rowid = ?2",
            params![blob, rowid],
        )
        .context("Failed to update vec_code")?;
    } else {
        // Insert new vector with explicit rowid
        conn.execute(
            "INSERT INTO vec_code(rowid, embedding) VALUES (?1, ?2)",
            params![rowid, blob],
        )
        .context("Failed to insert into vec_code")?;
    }

    Ok(rowid)
}

/// Batch upsert with deduplication
pub fn upsert_embeddings_batch(
    conn: &mut Connection,
    embeddings: &[EmbeddingRecord],
) -> Result<()> {
    // Validate all embeddings are 1536 dimensions
    for (idx, record) in embeddings.iter().enumerate() {
        if record.embedding.len() != 1536 {
            bail!(
                "Embedding at index {} has unsupported dimension: {}. Only 1536-dimensional embeddings are currently supported.",
                idx,
                record.embedding.len()
            );
        }
    }

    // Use a transaction for batch operation
    let tx = conn
        .transaction()
        .context("Failed to begin transaction for batch embedding upsert")?;

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

        let mut get_id_stmt = tx.prepare(
            "SELECT id FROM code_embeddings WHERE blob_sha = ?1"
        )?;

        let mut check_vec_stmt = tx.prepare(
            "SELECT 1 FROM vec_code WHERE rowid = ?1"
        )?;

        let mut update_vec_stmt = tx.prepare(
            "UPDATE vec_code SET embedding = ?1 WHERE rowid = ?2"
        )?;

        let mut insert_vec_stmt = tx.prepare(
            "INSERT INTO vec_code(rowid, embedding) VALUES (?1, ?2)"
        )?;

        for record in embeddings {
            let blob = vec_to_blob(&record.embedding);

            // Upsert into code_embeddings
            upsert_stmt.execute(params![
                record.blob_sha,
                blob,
                record.embedding.len() as i32,
                record.model_version,
            ])?;

            // Get the rowid
            let rowid: i64 = get_id_stmt.query_row(params![record.blob_sha], |row| row.get(0))?;

            // Check if exists in vec_code
            let exists_in_vec_code = check_vec_stmt
                .exists(params![rowid])
                .unwrap_or(false);

            if exists_in_vec_code {
                update_vec_stmt.execute(params![blob, rowid])?;
            } else {
                insert_vec_stmt.execute(params![rowid, blob])?;
            }
        }
    }

    tx.commit()
        .context("Failed to commit batch embedding upsert transaction")?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
