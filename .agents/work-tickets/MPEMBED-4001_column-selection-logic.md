# Ticket: MPEMBED-4001: Implement column selection based on embedding dimension

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement a function to select appropriate database columns based on embedding dimensions: 768-dimensional vectors use *_ollama columns, 1536-dimensional vectors use original columns. Return error for unsupported dimensions.

## Background
This ticket implements Phase 4 (Database and Search Integration) from the MPEMBED multi-provider embeddings plan. The database schema now has two sets of embedding columns (*_ollama for 768-dim and original for 1536-dim). This ticket creates the abstraction layer that maps embedding dimensions to the correct column names, enabling dimension-agnostic code in the upsert and search paths.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-4-database-search-integration.md

## Acceptance Criteria
- [ ] `select_columns_for_dimension()` function created
- [ ] Function returns ColumnSet struct with code and doc column names
- [ ] Dimension 768 → code_embedding_ollama, doc_embedding_ollama
- [ ] Dimension 1536 → code_embedding, doc_embedding
- [ ] Error for unsupported dimensions (not 768 or 1536)
- [ ] Unit tests for all dimension mappings
- [ ] Unit tests for error cases (0, 384, 3072, etc.)
- [ ] Documentation explaining column selection logic

## Technical Requirements
- Create new module: crates/maproom/src/db/columns.rs
- Define ColumnSet struct to hold column name pairs
- Implement dimension validation (must be 768 or 1536)
- Return anyhow::Result with descriptive errors
- Make function const-friendly if possible
- No database dependencies (pure logic function)
- Export publicly from db module

## Implementation Notes
**Module Structure:**
```rust
// crates/maproom/src/db/columns.rs

/// Column names for embedding storage
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnSet {
    pub code_embedding: &'static str,
    pub doc_embedding: &'static str,
}

impl ColumnSet {
    /// Column set for 768-dimensional embeddings (Ollama, Google)
    pub const OLLAMA: Self = Self {
        code_embedding: "code_embedding_ollama",
        doc_embedding: "doc_embedding_ollama",
    };

    /// Column set for 1536-dimensional embeddings (OpenAI)
    pub const OPENAI: Self = Self {
        code_embedding: "code_embedding",
        doc_embedding: "doc_embedding",
    };
}

/// Select database columns based on embedding dimension
///
/// # Arguments
/// * `dimension` - Embedding vector dimension (768 or 1536)
///
/// # Returns
/// * `Ok(ColumnSet)` - Column names for the dimension
/// * `Err` - If dimension is not supported
///
/// # Examples
/// ```
/// let cols = select_columns_for_dimension(768)?;
/// assert_eq!(cols.code_embedding, "code_embedding_ollama");
/// ```
pub fn select_columns_for_dimension(dimension: usize) -> anyhow::Result<ColumnSet> {
    match dimension {
        768 => Ok(ColumnSet::OLLAMA),
        1536 => Ok(ColumnSet::OPENAI),
        _ => Err(anyhow::anyhow!(
            "Unsupported embedding dimension: {}. Supported dimensions: 768 (Ollama/Google), 1536 (OpenAI)",
            dimension
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_768_dimension_selects_ollama_columns() {
        let cols = select_columns_for_dimension(768).unwrap();
        assert_eq!(cols, ColumnSet::OLLAMA);
        assert_eq!(cols.code_embedding, "code_embedding_ollama");
        assert_eq!(cols.doc_embedding, "doc_embedding_ollama");
    }

    #[test]
    fn test_1536_dimension_selects_openai_columns() {
        let cols = select_columns_for_dimension(1536).unwrap();
        assert_eq!(cols, ColumnSet::OPENAI);
        assert_eq!(cols.code_embedding, "code_embedding");
        assert_eq!(cols.doc_embedding, "doc_embedding");
    }

    #[test]
    fn test_unsupported_dimension_returns_error() {
        let result = select_columns_for_dimension(384);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
        assert!(result.unwrap_err().to_string().contains("384"));
    }

    #[test]
    fn test_zero_dimension_returns_error() {
        let result = select_columns_for_dimension(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_large_dimension_returns_error() {
        let result = select_columns_for_dimension(3072);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("768") && msg.contains("1536"));
    }

    #[test]
    fn test_column_set_constants() {
        assert_eq!(ColumnSet::OLLAMA.code_embedding, "code_embedding_ollama");
        assert_eq!(ColumnSet::OLLAMA.doc_embedding, "doc_embedding_ollama");
        assert_eq!(ColumnSet::OPENAI.code_embedding, "code_embedding");
        assert_eq!(ColumnSet::OPENAI.doc_embedding, "doc_embedding");
    }
}
```

**Module Registration:**
```rust
// crates/maproom/src/db/mod.rs
pub mod columns;

pub use columns::{ColumnSet, select_columns_for_dimension};
```

**Usage Example:**
```rust
// In upsert code
let provider = create_provider("ollama")?;
let dimension = provider.dimension(); // 768

let columns = select_columns_for_dimension(dimension)?;
// columns.code_embedding = "code_embedding_ollama"
// columns.doc_embedding = "doc_embedding_ollama"

sqlx::query(&format!(
    "UPDATE chunks SET {} = $1, {} = $2 WHERE id = $3",
    columns.code_embedding, columns.doc_embedding
))
.bind(&code_vec)
.bind(&doc_vec)
.bind(&chunk_id)
.execute(&pool)
.await?;
```

## Dependencies
- MPEMBED-1001 (Database migration must be complete with *_ollama columns available)

## Risk Assessment
- **Risk**: Hard-coded column names may diverge from actual schema
  - **Mitigation**: Integration tests verify column names match database schema, schema validation in migration tests
- **Risk**: Future providers with different dimensions (e.g., 384, 3072) will require schema changes
  - **Mitigation**: Error message guides users, document process for adding new dimensions
- **Risk**: String formatting for SQL may be vulnerable to injection
  - **Mitigation**: Column names are compile-time constants, not user input; parameterized queries for all values

## Files/Packages Affected
- crates/maproom/src/db/columns.rs (create)
- crates/maproom/src/db/mod.rs (modify - export new module)
