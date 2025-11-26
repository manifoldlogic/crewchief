use anyhow::Result;
use rusqlite::{Connection, params};
use super::embeddings::vec_to_blob;

/// Result from vector similarity search
#[derive(Debug, Clone)]
pub struct VectorResult {
    pub chunk_id: i64,
    pub distance: f64,
    pub similarity: f64,  // Normalized 0-1
}

/// Convert L2 distance to similarity score (0-1, higher = better)
pub fn distance_to_similarity(distance: f64) -> f64 {
    // L2 distance: 0 = identical, larger = more different
    // Convert to 0-1 where 1 = identical
    1.0 / (1.0 + distance)
}

/// Search for similar chunks by embedding
///
/// Uses sqlite-vec's MATCH operator to find nearest neighbors by L2 distance.
/// Results are joined from vec_code → code_embeddings → chunks via blob_sha.
/// Optional worktree filtering via chunk_worktrees junction table.
///
/// Returns empty Vec (not error) when:
/// - No results found
/// - Query embedding dimension mismatch (after validation)
/// - Extension not loaded (caller should check has_vec_extension first)
pub fn search_vector(
    conn: &Connection,
    repo: &str,
    worktree: Option<&str>,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<VectorResult>> {
    // Validate embedding dimension
    if query_embedding.len() != 1536 {
        anyhow::bail!(
            "Embedding dimension mismatch: expected 1536, got {}",
            query_embedding.len()
        );
    }

    let query_blob = vec_to_blob(query_embedding);

    // SQL with JOIN path: vec_code.rowid → code_embeddings.id → chunks.blob_sha
    // The MATCH operator expects: WHERE embedding MATCH ?1 AND k = ?N
    // The query_blob contains both the query vector and the limit parameter
    let sql = if worktree.is_some() {
        r#"
            SELECT c.id, v.distance
            FROM vec_code v
            JOIN code_embeddings e ON e.id = v.rowid
            JOIN chunks c ON c.blob_sha = e.blob_sha
            JOIN files f ON f.id = c.file_id
            JOIN repos r ON r.id = f.repo_id
            JOIN chunk_worktrees cw ON cw.chunk_id = c.id
            JOIN worktrees w ON w.id = cw.worktree_id
            WHERE v.embedding MATCH ?1
              AND k = ?4
              AND r.name = ?2
              AND w.name = ?3
            ORDER BY v.distance ASC
        "#
    } else {
        r#"
            SELECT DISTINCT c.id, v.distance
            FROM vec_code v
            JOIN code_embeddings e ON e.id = v.rowid
            JOIN chunks c ON c.blob_sha = e.blob_sha
            JOIN files f ON f.id = c.file_id
            JOIN repos r ON r.id = f.repo_id
            WHERE v.embedding MATCH ?1
              AND k = ?3
              AND r.name = ?2
            ORDER BY v.distance ASC
        "#
    };

    let mut stmt = conn.prepare(sql)?;

    let mut vec_results = Vec::new();

    if let Some(wt) = worktree {
        let rows = stmt.query_map(params![query_blob, repo, wt, limit as i64], |row| {
            let chunk_id: i64 = row.get(0)?;
            let distance: f64 = row.get(1)?;
            Ok(VectorResult {
                chunk_id,
                distance,
                similarity: distance_to_similarity(distance),
            })
        })?;

        for result in rows {
            vec_results.push(result?);
        }
    } else {
        let rows = stmt.query_map(params![query_blob, repo, limit as i64], |row| {
            let chunk_id: i64 = row.get(0)?;
            let distance: f64 = row.get(1)?;
            Ok(VectorResult {
                chunk_id,
                distance,
                similarity: distance_to_similarity(distance),
            })
        })?;

        for result in rows {
            vec_results.push(result?);
        }
    }

    Ok(vec_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_to_similarity_identical() {
        // Distance 0 = identical vectors
        let sim = distance_to_similarity(0.0);
        assert!((sim - 1.0).abs() < 1e-6, "Identical vectors should have similarity 1.0");
    }

    #[test]
    fn test_distance_to_similarity_different() {
        // Distance 1.0
        let sim = distance_to_similarity(1.0);
        assert!((sim - 0.5).abs() < 1e-6, "Distance 1.0 should have similarity 0.5");
    }

    #[test]
    fn test_distance_to_similarity_far() {
        // Large distance
        let sim = distance_to_similarity(10.0);
        assert!(sim < 0.1, "Large distance should have low similarity");
        assert!(sim > 0.0, "Similarity should be positive");
    }

    #[test]
    fn test_distance_to_similarity_monotonic() {
        // Similarity should decrease as distance increases
        let sim1 = distance_to_similarity(0.5);
        let sim2 = distance_to_similarity(1.0);
        let sim3 = distance_to_similarity(2.0);

        assert!(sim1 > sim2, "Smaller distance should have higher similarity");
        assert!(sim2 > sim3, "Similarity should decrease monotonically");
    }

    #[test]
    fn test_distance_to_similarity_range() {
        // All similarities should be in range (0, 1]
        for dist in [0.0, 0.1, 1.0, 5.0, 100.0] {
            let sim = distance_to_similarity(dist);
            assert!(sim > 0.0, "Similarity should be positive for distance {}", dist);
            assert!(sim <= 1.0, "Similarity should be <= 1.0 for distance {}", dist);
        }
    }
}
