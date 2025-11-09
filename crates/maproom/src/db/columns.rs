//! Column selection logic for embedding storage.
//!
//! This module provides an abstraction layer for selecting the correct database columns
//! based on embedding dimensions. Different embedding providers produce vectors of different
//! dimensions, which are stored in different column sets:
//!
//! - 768-dimensional embeddings (Ollama, Google) → `*_embedding_ollama` columns
//! - 1536-dimensional embeddings (OpenAI) → original `*_embedding` columns
//!
//! This enables dimension-agnostic code in the upsert and search paths.

/// Column names for embedding storage.
///
/// This struct holds the database column names used to store code and text embeddings.
/// Different embedding providers use different column sets based on their vector dimensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSet {
    /// Column name for code embeddings
    pub code_embedding: &'static str,
    /// Column name for text embeddings
    pub text_embedding: &'static str,
}

impl ColumnSet {
    /// Column set for 768-dimensional embeddings (Ollama, Google).
    ///
    /// These providers use dedicated `*_ollama` columns to avoid dimension conflicts
    /// with the original OpenAI columns.
    pub const OLLAMA: Self = Self {
        code_embedding: "code_embedding_ollama",
        text_embedding: "text_embedding_ollama",
    };

    /// Column set for 1536-dimensional embeddings (OpenAI).
    ///
    /// These are the original embedding columns, maintained for backward compatibility
    /// with existing OpenAI-based embeddings.
    pub const OPENAI: Self = Self {
        code_embedding: "code_embedding",
        text_embedding: "text_embedding",
    };
}

/// Select database columns based on embedding dimension.
///
/// This function maps embedding vector dimensions to the appropriate database column names.
/// It ensures that embeddings from different providers are stored in the correct columns,
/// preventing dimension mismatches and enabling mixed-provider deployments.
///
/// # Arguments
///
/// * `dimension` - Embedding vector dimension (768 or 1536)
///
/// # Returns
///
/// * `Ok(ColumnSet)` - Column names for the specified dimension
/// * `Err` - If the dimension is not supported
///
/// # Supported Dimensions
///
/// - `768` - Maps to `code_embedding_ollama` and `text_embedding_ollama`
///   - Used by: Ollama providers (e.g., nomic-embed-text), Google Gemini
/// - `1536` - Maps to `code_embedding` and `text_embedding`
///   - Used by: OpenAI text-embedding-3-small, text-embedding-ada-002
///
/// # Errors
///
/// Returns an error for unsupported dimensions with a helpful message listing
/// the supported dimensions and their corresponding providers.
///
/// # Examples
///
/// ```
/// use maproom::db::select_columns_for_dimension;
///
/// // Select columns for Ollama embeddings
/// let cols = select_columns_for_dimension(768)?;
/// assert_eq!(cols.code_embedding, "code_embedding_ollama");
/// assert_eq!(cols.text_embedding, "text_embedding_ollama");
///
/// // Select columns for OpenAI embeddings
/// let cols = select_columns_for_dimension(1536)?;
/// assert_eq!(cols.code_embedding, "code_embedding");
/// assert_eq!(cols.text_embedding, "text_embedding");
///
/// // Unsupported dimension returns error
/// let result = select_columns_for_dimension(384);
/// assert!(result.is_err());
/// # Ok::<(), anyhow::Error>(())
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
        assert_eq!(cols.text_embedding, "text_embedding_ollama");
    }

    #[test]
    fn test_1536_dimension_selects_openai_columns() {
        let cols = select_columns_for_dimension(1536).unwrap();
        assert_eq!(cols, ColumnSet::OPENAI);
        assert_eq!(cols.code_embedding, "code_embedding");
        assert_eq!(cols.text_embedding, "text_embedding");
    }

    #[test]
    fn test_unsupported_dimension_returns_error() {
        let result = select_columns_for_dimension(384);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Unsupported"));
        assert!(err_msg.contains("384"));
        assert!(err_msg.contains("768"));
        assert!(err_msg.contains("1536"));
    }

    #[test]
    fn test_zero_dimension_returns_error() {
        let result = select_columns_for_dimension(0);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Unsupported"));
        assert!(err_msg.contains("0"));
    }

    #[test]
    fn test_large_dimension_returns_error() {
        let result = select_columns_for_dimension(3072);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("3072"));
        assert!(msg.contains("768") && msg.contains("1536"));
    }

    #[test]
    fn test_column_set_constants() {
        assert_eq!(ColumnSet::OLLAMA.code_embedding, "code_embedding_ollama");
        assert_eq!(ColumnSet::OLLAMA.text_embedding, "text_embedding_ollama");
        assert_eq!(ColumnSet::OPENAI.code_embedding, "code_embedding");
        assert_eq!(ColumnSet::OPENAI.text_embedding, "text_embedding");
    }

    #[test]
    fn test_column_set_equality() {
        let cols1 = select_columns_for_dimension(768).unwrap();
        let cols2 = select_columns_for_dimension(768).unwrap();
        assert_eq!(cols1, cols2);

        let cols3 = select_columns_for_dimension(1536).unwrap();
        assert_ne!(cols1, cols3);
    }

    #[test]
    fn test_error_message_helpful() {
        let result = select_columns_for_dimension(512);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // Error message should contain dimension that was attempted
        assert!(err_msg.contains("512"));

        // Error message should list supported dimensions
        assert!(err_msg.contains("768"));
        assert!(err_msg.contains("1536"));

        // Error message should mention providers
        assert!(
            err_msg.contains("Ollama") || err_msg.contains("Google") || err_msg.contains("OpenAI")
        );
    }
}
