//! Token counting utilities using tiktoken.

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use tiktoken_rs::CoreBPE;

/// Global tokenizer instance using cl100k_base encoding (GPT-4, GPT-3.5-turbo).
///
/// This encoding is the most common for modern LLMs and provides accurate
/// token counts for TypeScript, JavaScript, Rust, and other languages.
static TOKENIZER: Lazy<CoreBPE> = Lazy::new(|| {
    tiktoken_rs::cl100k_base().expect("Failed to initialize cl100k_base tokenizer")
});

/// Token counter for accurate estimation of LLM token consumption.
///
/// Uses tiktoken with cl100k_base encoding, which is used by:
/// - GPT-4 and GPT-4 Turbo
/// - GPT-3.5-turbo
/// - text-embedding-ada-002
///
/// # Example
///
/// ```
/// use crewchief_maproom::context::TokenCounter;
///
/// let counter = TokenCounter::new();
/// let code = "fn main() { println!(\"Hello, world!\"); }";
/// let tokens = counter.count(code).unwrap();
/// println!("Code uses {} tokens", tokens);
/// ```
#[derive(Debug, Clone)]
pub struct TokenCounter;

impl TokenCounter {
    /// Create a new token counter.
    pub fn new() -> Self {
        Self
    }

    /// Count tokens in the given text using cl100k_base encoding.
    ///
    /// # Errors
    ///
    /// Returns error if tokenization fails (extremely rare, usually indicates
    /// invalid UTF-8 or internal tiktoken issues).
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::TokenCounter;
    ///
    /// let counter = TokenCounter::new();
    /// assert_eq!(counter.count("hello").unwrap(), 1);
    /// assert_eq!(counter.count("hello world").unwrap(), 2);
    /// ```
    pub fn count(&self, text: &str) -> Result<usize> {
        let tokens = TOKENIZER
            .encode_with_special_tokens(text)
            .len();
        Ok(tokens)
    }

    /// Count tokens with a fallback estimate if tokenization fails.
    ///
    /// Uses a simple heuristic (1 token ≈ 4 characters) as fallback.
    /// This is less accurate but ensures we never fail due to tokenization errors.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::TokenCounter;
    ///
    /// let counter = TokenCounter::new();
    /// let tokens = counter.count_with_fallback("hello world");
    /// assert!(tokens > 0);
    /// ```
    pub fn count_with_fallback(&self, text: &str) -> usize {
        self.count(text)
            .unwrap_or_else(|_| self.estimate_tokens(text))
    }

    /// Estimate token count using a simple heuristic.
    ///
    /// Uses the rule of thumb: 1 token ≈ 4 characters.
    /// This is a rough approximation and should only be used as a fallback.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::TokenCounter;
    ///
    /// let counter = TokenCounter::new();
    /// assert_eq!(counter.estimate_tokens("hello"), 2); // 5 chars / 4 ≈ 2 tokens
    /// assert_eq!(counter.estimate_tokens("hello world"), 3); // 11 chars / 4 ≈ 3 tokens
    /// ```
    pub fn estimate_tokens(&self, text: &str) -> usize {
        // Rule of thumb: 1 token ≈ 4 characters
        // Round up to avoid underestimating
        (text.len() + 3) / 4
    }

    /// Count tokens for multiple text segments and return the total.
    ///
    /// This is more efficient than counting each segment individually
    /// when you need the total.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::TokenCounter;
    ///
    /// let counter = TokenCounter::new();
    /// let segments = vec!["hello", "world", "!"];
    /// let total = counter.count_multiple(&segments).unwrap();
    /// assert!(total > 0);
    /// ```
    pub fn count_multiple(&self, texts: &[&str]) -> Result<usize> {
        texts
            .iter()
            .map(|text| self.count(text))
            .sum::<Result<usize>>()
            .context("Failed to count tokens for multiple texts")
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_simple_text() {
        let counter = TokenCounter::new();

        // Simple English words
        let count = counter.count("hello").unwrap();
        assert!(count > 0 && count < 3, "Expected 1-2 tokens for 'hello', got {}", count);

        let count = counter.count("hello world").unwrap();
        assert!(count > 0 && count < 5, "Expected 2-4 tokens for 'hello world', got {}", count);
    }

    #[test]
    fn test_count_code() {
        let counter = TokenCounter::new();

        // Simple function
        let code = "fn main() { println!(\"Hello\"); }";
        let count = counter.count(code).unwrap();
        assert!(count > 5, "Expected >5 tokens for simple function, got {}", count);
    }

    #[test]
    fn test_count_multiline_code() {
        let counter = TokenCounter::new();

        let code = r#"
fn fibonacci(n: u32) -> u32 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}
"#;
        let count = counter.count(code).unwrap();
        assert!(count > 20, "Expected >20 tokens for fibonacci function, got {}", count);
    }

    #[test]
    fn test_count_with_fallback() {
        let counter = TokenCounter::new();

        let text = "hello world";
        let count = counter.count_with_fallback(text);
        assert!(count > 0);
    }

    #[test]
    fn test_estimate_tokens() {
        let counter = TokenCounter::new();

        // 5 characters / 4 = 1.25 -> rounds up to 2
        assert_eq!(counter.estimate_tokens("hello"), 2);

        // 11 characters / 4 = 2.75 -> rounds up to 3
        assert_eq!(counter.estimate_tokens("hello world"), 3);

        // 20 characters / 4 = 5
        assert_eq!(counter.estimate_tokens("12345678901234567890"), 5);
    }

    #[test]
    fn test_count_multiple() {
        let counter = TokenCounter::new();

        let segments = vec!["hello", "world", "test"];
        let total = counter.count_multiple(&segments).unwrap();
        assert!(total > 0);

        // Total should be approximately sum of individual counts
        let sum: usize = segments
            .iter()
            .map(|s| counter.count(s).unwrap())
            .sum();
        assert_eq!(total, sum);
    }

    #[test]
    fn test_empty_string() {
        let counter = TokenCounter::new();
        assert_eq!(counter.count("").unwrap(), 0);
        assert_eq!(counter.estimate_tokens(""), 0);
    }

    #[test]
    fn test_unicode_handling() {
        let counter = TokenCounter::new();

        // Unicode characters may use multiple tokens
        let emoji = "👍";
        let count = counter.count(emoji).unwrap();
        assert!(count > 0);

        let chinese = "你好世界";
        let count = counter.count(chinese).unwrap();
        assert!(count > 0);
    }
}
