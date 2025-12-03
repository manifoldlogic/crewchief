//! Graph traversal module for chunk relationships
//!
//! Provides recursive traversal of chunk_edges for:
//! - Caller/callee relationships (function calls)
//! - Import relationships (module imports)
//! - Extension relationships (class inheritance)
//!
//! Uses SQLite recursive CTEs for transitive closure queries
//! with cycle detection to handle circular dependencies.

use anyhow::Result;
use rusqlite::{params, Connection};

/// Default maximum depth for graph traversal
pub const DEFAULT_MAX_DEPTH: usize = 3;

/// Hard maximum depth to prevent runaway recursion
pub const HARD_MAX_DEPTH: usize = 10;

/// Result from graph traversal
#[derive(Debug, Clone)]
pub struct GraphResult {
    /// Target chunk ID found in traversal
    pub chunk_id: i64,
    /// Depth from source (1 = direct relationship)
    pub depth: usize,
    /// Path from source to this chunk (list of chunk IDs)
    pub path: Vec<i64>,
    /// Type of relationship (calls, imports, extends)
    pub edge_type: String,
}

/// Direction for import relationship queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportDirection {
    /// Find chunks that import the target (who imports this?)
    Incoming,
    /// Find chunks that the source imports (what does this import?)
    Outgoing,
}

/// Parse path string into vector of chunk IDs
///
/// Path format: "/id1/id2/id3" where each ID is separated by /
fn parse_path(path_str: &str) -> Vec<i64> {
    path_str
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Find all chunks that call the target chunk (directly or transitively)
///
/// # Arguments
/// * `conn` - SQLite connection
/// * `target_chunk_id` - The chunk to find callers for
/// * `max_depth` - Maximum traversal depth (default 3, max 10)
///
/// # Returns
/// Vector of GraphResult ordered by depth (closest first)
pub fn find_callers(
    conn: &Connection,
    target_chunk_id: i64,
    max_depth: Option<usize>,
) -> Result<Vec<GraphResult>> {
    let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH).min(HARD_MAX_DEPTH);

    let sql = r#"
        WITH RECURSIVE callers(chunk_id, depth, path) AS (
            -- Base case: direct callers
            SELECT src_chunk_id, 1, '/' || src_chunk_id
            FROM chunk_edges
            WHERE dst_chunk_id = ?1 AND type = 'calls'

            UNION ALL

            -- Recursive case: callers of callers
            SELECT e.src_chunk_id, c.depth + 1,
                   c.path || '/' || e.src_chunk_id
            FROM chunk_edges e
            JOIN callers c ON e.dst_chunk_id = c.chunk_id
            WHERE c.depth < ?2
              AND e.type = 'calls'
              -- Cycle detection: don't revisit chunks in path
              AND c.path NOT LIKE '%/' || e.src_chunk_id || '/%'
              AND c.path NOT LIKE '%/' || e.src_chunk_id
        )
        SELECT DISTINCT chunk_id, depth, path
        FROM callers
        ORDER BY depth, chunk_id
    "#;

    let mut stmt = conn.prepare(sql)?;
    let results = stmt.query_map(params![target_chunk_id, depth], |row| {
        let chunk_id: i64 = row.get(0)?;
        let depth: i64 = row.get(1)?;
        let path_str: String = row.get(2)?;
        Ok(GraphResult {
            chunk_id,
            depth: depth as usize,
            path: parse_path(&path_str),
            edge_type: "calls".to_string(),
        })
    })?;

    results
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Find all chunks called by the source chunk (directly or transitively)
///
/// # Arguments
/// * `conn` - SQLite connection
/// * `source_chunk_id` - The chunk to find callees for
/// * `max_depth` - Maximum traversal depth (default 3, max 10)
///
/// # Returns
/// Vector of GraphResult ordered by depth (closest first)
pub fn find_callees(
    conn: &Connection,
    source_chunk_id: i64,
    max_depth: Option<usize>,
) -> Result<Vec<GraphResult>> {
    let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH).min(HARD_MAX_DEPTH);

    let sql = r#"
        WITH RECURSIVE callees(chunk_id, depth, path) AS (
            -- Base case: direct callees
            SELECT dst_chunk_id, 1, '/' || dst_chunk_id
            FROM chunk_edges
            WHERE src_chunk_id = ?1 AND type = 'calls'

            UNION ALL

            -- Recursive case: callees of callees
            SELECT e.dst_chunk_id, c.depth + 1,
                   c.path || '/' || e.dst_chunk_id
            FROM chunk_edges e
            JOIN callees c ON e.src_chunk_id = c.chunk_id
            WHERE c.depth < ?2
              AND e.type = 'calls'
              -- Cycle detection: don't revisit chunks in path
              AND c.path NOT LIKE '%/' || e.dst_chunk_id || '/%'
              AND c.path NOT LIKE '%/' || e.dst_chunk_id
        )
        SELECT DISTINCT chunk_id, depth, path
        FROM callees
        ORDER BY depth, chunk_id
    "#;

    let mut stmt = conn.prepare(sql)?;
    let results = stmt.query_map(params![source_chunk_id, depth], |row| {
        let chunk_id: i64 = row.get(0)?;
        let depth: i64 = row.get(1)?;
        let path_str: String = row.get(2)?;
        Ok(GraphResult {
            chunk_id,
            depth: depth as usize,
            path: parse_path(&path_str),
            edge_type: "calls".to_string(),
        })
    })?;

    results
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Find import relationships for a chunk
///
/// # Arguments
/// * `conn` - SQLite connection
/// * `chunk_id` - The chunk to find imports for
/// * `direction` - Incoming (who imports this) or Outgoing (what this imports)
/// * `max_depth` - Maximum traversal depth (default 3, max 10)
///
/// # Returns
/// Vector of GraphResult ordered by depth (closest first)
pub fn find_imports(
    conn: &Connection,
    chunk_id: i64,
    direction: ImportDirection,
    max_depth: Option<usize>,
) -> Result<Vec<GraphResult>> {
    let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH).min(HARD_MAX_DEPTH);

    let sql = match direction {
        ImportDirection::Incoming => {
            // Find chunks that import the target
            r#"
                WITH RECURSIVE importers(chunk_id, depth, path) AS (
                    -- Base case: direct importers
                    SELECT src_chunk_id, 1, '/' || src_chunk_id
                    FROM chunk_edges
                    WHERE dst_chunk_id = ?1 AND type = 'imports'

                    UNION ALL

                    -- Recursive case: importers of importers
                    SELECT e.src_chunk_id, i.depth + 1,
                           i.path || '/' || e.src_chunk_id
                    FROM chunk_edges e
                    JOIN importers i ON e.dst_chunk_id = i.chunk_id
                    WHERE i.depth < ?2
                      AND e.type = 'imports'
                      -- Cycle detection
                      AND i.path NOT LIKE '%/' || e.src_chunk_id || '/%'
                      AND i.path NOT LIKE '%/' || e.src_chunk_id
                )
                SELECT DISTINCT chunk_id, depth, path
                FROM importers
                ORDER BY depth, chunk_id
            "#
        }
        ImportDirection::Outgoing => {
            // Find chunks that the source imports
            r#"
                WITH RECURSIVE imported(chunk_id, depth, path) AS (
                    -- Base case: direct imports
                    SELECT dst_chunk_id, 1, '/' || dst_chunk_id
                    FROM chunk_edges
                    WHERE src_chunk_id = ?1 AND type = 'imports'

                    UNION ALL

                    -- Recursive case: imports of imports
                    SELECT e.dst_chunk_id, i.depth + 1,
                           i.path || '/' || e.dst_chunk_id
                    FROM chunk_edges e
                    JOIN imported i ON e.src_chunk_id = i.chunk_id
                    WHERE i.depth < ?2
                      AND e.type = 'imports'
                      -- Cycle detection
                      AND i.path NOT LIKE '%/' || e.dst_chunk_id || '/%'
                      AND i.path NOT LIKE '%/' || e.dst_chunk_id
                )
                SELECT DISTINCT chunk_id, depth, path
                FROM imported
                ORDER BY depth, chunk_id
            "#
        }
    };

    let mut stmt = conn.prepare(sql)?;
    let results = stmt.query_map(params![chunk_id, depth], |row| {
        let chunk_id: i64 = row.get(0)?;
        let depth: i64 = row.get(1)?;
        let path_str: String = row.get(2)?;
        Ok(GraphResult {
            chunk_id,
            depth: depth as usize,
            path: parse_path(&path_str),
            edge_type: "imports".to_string(),
        })
    })?;

    results
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Find extension/inheritance relationships for a chunk
///
/// # Arguments
/// * `conn` - SQLite connection
/// * `chunk_id` - The chunk to find extensions for
/// * `direction` - Incoming (what extends this) or Outgoing (what this extends)
/// * `max_depth` - Maximum traversal depth (default 3, max 10)
///
/// # Returns
/// Vector of GraphResult ordered by depth (closest first)
pub fn find_extensions(
    conn: &Connection,
    chunk_id: i64,
    direction: ImportDirection, // Reusing enum for direction
    max_depth: Option<usize>,
) -> Result<Vec<GraphResult>> {
    let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH).min(HARD_MAX_DEPTH);

    let sql = match direction {
        ImportDirection::Incoming => {
            // Find chunks that extend the target (subclasses)
            r#"
                WITH RECURSIVE subclasses(chunk_id, depth, path) AS (
                    -- Base case: direct subclasses
                    SELECT src_chunk_id, 1, '/' || src_chunk_id
                    FROM chunk_edges
                    WHERE dst_chunk_id = ?1 AND type = 'extends'

                    UNION ALL

                    -- Recursive case: subclasses of subclasses
                    SELECT e.src_chunk_id, s.depth + 1,
                           s.path || '/' || e.src_chunk_id
                    FROM chunk_edges e
                    JOIN subclasses s ON e.dst_chunk_id = s.chunk_id
                    WHERE s.depth < ?2
                      AND e.type = 'extends'
                      -- Cycle detection
                      AND s.path NOT LIKE '%/' || e.src_chunk_id || '/%'
                      AND s.path NOT LIKE '%/' || e.src_chunk_id
                )
                SELECT DISTINCT chunk_id, depth, path
                FROM subclasses
                ORDER BY depth, chunk_id
            "#
        }
        ImportDirection::Outgoing => {
            // Find chunks that the source extends (superclasses)
            r#"
                WITH RECURSIVE superclasses(chunk_id, depth, path) AS (
                    -- Base case: direct superclasses
                    SELECT dst_chunk_id, 1, '/' || dst_chunk_id
                    FROM chunk_edges
                    WHERE src_chunk_id = ?1 AND type = 'extends'

                    UNION ALL

                    -- Recursive case: superclasses of superclasses
                    SELECT e.dst_chunk_id, s.depth + 1,
                           s.path || '/' || e.dst_chunk_id
                    FROM chunk_edges e
                    JOIN superclasses s ON e.src_chunk_id = s.chunk_id
                    WHERE s.depth < ?2
                      AND e.type = 'extends'
                      -- Cycle detection
                      AND s.path NOT LIKE '%/' || e.dst_chunk_id || '/%'
                      AND s.path NOT LIKE '%/' || e.dst_chunk_id
                )
                SELECT DISTINCT chunk_id, depth, path
                FROM superclasses
                ORDER BY depth, chunk_id
            "#
        }
    };

    let mut stmt = conn.prepare(sql)?;
    let results = stmt.query_map(params![chunk_id, depth], |row| {
        let chunk_id: i64 = row.get(0)?;
        let depth: i64 = row.get(1)?;
        let path_str: String = row.get(2)?;
        Ok(GraphResult {
            chunk_id,
            depth: depth as usize,
            path: parse_path(&path_str),
            edge_type: "extends".to_string(),
        })
    })?;

    results
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Get all direct edges from or to a chunk (without recursion)
///
/// # Arguments
/// * `conn` - SQLite connection
/// * `chunk_id` - The chunk to find edges for
/// * `direction` - Incoming (edges pointing to chunk) or Outgoing (edges from chunk)
///
/// # Returns
/// Vector of GraphResult with depth=1 for all direct relationships
pub fn get_direct_edges(
    conn: &Connection,
    chunk_id: i64,
    direction: ImportDirection,
) -> Result<Vec<GraphResult>> {
    let sql = match direction {
        ImportDirection::Incoming => {
            "SELECT src_chunk_id, type FROM chunk_edges WHERE dst_chunk_id = ?1"
        }
        ImportDirection::Outgoing => {
            "SELECT dst_chunk_id, type FROM chunk_edges WHERE src_chunk_id = ?1"
        }
    };

    let mut stmt = conn.prepare(sql)?;
    let results = stmt.query_map(params![chunk_id], |row| {
        let related_chunk_id: i64 = row.get(0)?;
        let edge_type: String = row.get(1)?;
        Ok(GraphResult {
            chunk_id: related_chunk_id,
            depth: 1,
            path: vec![related_chunk_id],
            edge_type,
        })
    })?;

    results
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("{}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path_basic() {
        let path = parse_path("/1/2/3");
        assert_eq!(path, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_path_single() {
        let path = parse_path("/42");
        assert_eq!(path, vec![42]);
    }

    #[test]
    fn test_parse_path_empty() {
        let path = parse_path("");
        assert!(path.is_empty());
    }

    #[test]
    fn test_parse_path_no_slashes() {
        let path = parse_path("123");
        assert_eq!(path, vec![123]);
    }

    #[test]
    fn test_parse_path_trailing_slash() {
        let path = parse_path("/1/2/3/");
        assert_eq!(path, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_path_invalid_elements() {
        let path = parse_path("/1/abc/2");
        assert_eq!(path, vec![1, 2]); // abc is filtered out
    }

    #[test]
    fn test_default_max_depth() {
        assert_eq!(DEFAULT_MAX_DEPTH, 3);
    }

    #[test]
    fn test_hard_max_depth() {
        assert_eq!(HARD_MAX_DEPTH, 10);
    }

    #[test]
    fn test_import_direction_variants() {
        let incoming = ImportDirection::Incoming;
        let outgoing = ImportDirection::Outgoing;
        assert_ne!(incoming, outgoing);
    }

    #[test]
    fn test_graph_result_construction() {
        let result = GraphResult {
            chunk_id: 42,
            depth: 2,
            path: vec![1, 2, 42],
            edge_type: "calls".to_string(),
        };
        assert_eq!(result.chunk_id, 42);
        assert_eq!(result.depth, 2);
        assert_eq!(result.path.len(), 3);
        assert_eq!(result.edge_type, "calls");
    }

    #[test]
    fn test_depth_clamping() {
        // Test that depth is properly limited
        let clamped = Some(100_usize)
            .unwrap_or(DEFAULT_MAX_DEPTH)
            .min(HARD_MAX_DEPTH);
        assert_eq!(clamped, HARD_MAX_DEPTH);

        let default = None::<usize>
            .unwrap_or(DEFAULT_MAX_DEPTH)
            .min(HARD_MAX_DEPTH);
        assert_eq!(default, DEFAULT_MAX_DEPTH);
    }
}
