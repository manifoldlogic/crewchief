use super::embeddings::vec_to_blob;
use anyhow::{bail, Result};
use rusqlite::Connection;

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

/// Result from vector similarity search
#[derive(Debug, Clone)]
pub struct VectorResult {
    pub chunk_id: i64,
    pub distance: f64,
    pub similarity: f64, // Normalized 0-1
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
/// Results are joined from vec_code/vec_code_768 → code_embeddings → chunks via blob_sha.
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
    kind_filter: Option<&[String]>,
    lang_filter: Option<&[String]>,
) -> Result<Vec<VectorResult>> {
    // Validate embedding dimension and get the appropriate table
    let dimension = query_embedding.len();
    if !SUPPORTED_DIMENSIONS.contains(&dimension) {
        bail!(
            "Unsupported embedding dimension: {}. Supported dimensions: {:?}",
            dimension,
            SUPPORTED_DIMENSIONS
        );
    }

    let vec_table = get_vec_table_name(dimension)?;
    let query_blob = vec_to_blob(query_embedding);

    // SQL with JOIN path: vec_code/vec_code_768.rowid → code_embeddings.id → chunks.blob_sha
    // The MATCH operator expects: WHERE embedding MATCH ?1 AND k = ?N
    // Base params: ?1 = query_blob, ?2 = repo name
    // With worktree: ?3 = worktree name, ?4 = k, then filters
    // Without worktree: ?3 = k, then filters
    //
    // Note: For sqlite-vec, k must be specified early in the WHERE clause alongside MATCH.
    // The k parameter position is fixed relative to the MATCH clause.
    // Filter conditions are appended after the base WHERE clauses.
    let k_param_idx: usize = if worktree.is_some() { 4 } else { 3 };
    let mut param_idx: usize = k_param_idx + 1;
    let mut filter_conditions = Vec::new();

    if let Some(kinds) = kind_filter {
        if !kinds.is_empty() {
            let placeholders = (0..kinds.len())
                .map(|i| format!("?{}", param_idx + i))
                .collect::<Vec<_>>()
                .join(", ");
            filter_conditions.push(format!("c.kind IN ({})", placeholders));
            param_idx += kinds.len();
        }
    }

    if let Some(langs) = lang_filter {
        if !langs.is_empty() {
            let placeholders = (0..langs.len())
                .map(|i| format!("?{}", param_idx + i))
                .collect::<Vec<_>>()
                .join(", ");
            filter_conditions.push(format!("f.language IN ({})", placeholders));
            // param_idx += langs.len(); // Not needed as no more params follow
        }
    }

    let filter_clause = if filter_conditions.is_empty() {
        String::new()
    } else {
        format!(" AND {}", filter_conditions.join(" AND "))
    };

    let sql = if worktree.is_some() {
        format!(
            r#"
            SELECT c.id, v.distance
            FROM {} v
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
              {}
            ORDER BY v.distance ASC
        "#,
            vec_table, filter_clause
        )
    } else {
        format!(
            r#"
            SELECT DISTINCT c.id, v.distance
            FROM {} v
            JOIN code_embeddings e ON e.id = v.rowid
            JOIN chunks c ON c.blob_sha = e.blob_sha
            JOIN files f ON f.id = c.file_id
            JOIN repos r ON r.id = f.repo_id
            WHERE v.embedding MATCH ?1
              AND k = ?3
              AND r.name = ?2
              {}
            ORDER BY v.distance ASC
        "#,
            vec_table, filter_clause
        )
    };

    // Build dynamic parameter list
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    param_values.push(Box::new(query_blob));
    param_values.push(Box::new(repo.to_string()));

    if let Some(wt) = worktree {
        param_values.push(Box::new(wt.to_string()));
    }

    param_values.push(Box::new(limit as i64));

    if let Some(kinds) = kind_filter {
        for kind in kinds {
            param_values.push(Box::new(kind.clone()));
        }
    }

    if let Some(langs) = lang_filter {
        for lang in langs {
            param_values.push(Box::new(lang.clone()));
        }
    }

    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let mut vec_results = Vec::new();

    let rows = stmt.query_map(params_refs.as_slice(), |row| {
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

    Ok(vec_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_to_similarity_identical() {
        // Distance 0 = identical vectors
        let sim = distance_to_similarity(0.0);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "Identical vectors should have similarity 1.0"
        );
    }

    #[test]
    fn test_distance_to_similarity_different() {
        // Distance 1.0
        let sim = distance_to_similarity(1.0);
        assert!(
            (sim - 0.5).abs() < 1e-6,
            "Distance 1.0 should have similarity 0.5"
        );
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

        assert!(
            sim1 > sim2,
            "Smaller distance should have higher similarity"
        );
        assert!(sim2 > sim3, "Similarity should decrease monotonically");
    }

    #[test]
    fn test_distance_to_similarity_range() {
        // All similarities should be in range (0, 1]
        for dist in [0.0, 0.1, 1.0, 5.0, 100.0] {
            let sim = distance_to_similarity(dist);
            assert!(
                sim > 0.0,
                "Similarity should be positive for distance {}",
                dist
            );
            assert!(
                sim <= 1.0,
                "Similarity should be <= 1.0 for distance {}",
                dist
            );
        }
    }
}
