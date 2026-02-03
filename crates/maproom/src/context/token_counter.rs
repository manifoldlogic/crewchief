//! Token counting utilities using tiktoken.

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use tiktoken_rs::CoreBPE;

/// Global tokenizer instance using cl100k_base encoding (GPT-4, GPT-3.5-turbo).
///
/// This encoding is the most common for modern LLMs and provides accurate
/// token counts for TypeScript, JavaScript, Rust, and other languages.
static TOKENIZER: Lazy<CoreBPE> =
    Lazy::new(|| tiktoken_rs::cl100k_base().expect("Failed to initialize cl100k_base tokenizer"));

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
        let tokens = TOKENIZER.encode_with_special_tokens(text).len();
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
        text.len().div_ceil(4)
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

    /// Truncate text to fit within a maximum token count.
    ///
    /// Returns the original string if within limit, otherwise truncates
    /// at a token boundary. Uses character-based fallback if decoding fails.
    ///
    /// # Example
    ///
    /// ```
    /// use crewchief_maproom::context::TokenCounter;
    ///
    /// let counter = TokenCounter::new();
    /// let long_text = "word ".repeat(1000);
    /// let truncated = counter.truncate_to_limit(&long_text, 100);
    /// assert!(truncated.len() < long_text.len());
    /// ```
    pub fn truncate_to_limit(&self, text: &str, max_tokens: usize) -> String {
        let tokens = TOKENIZER.encode_with_special_tokens(text);

        if tokens.len() <= max_tokens {
            return text.to_string();
        }

        // Truncate tokens and decode back to string
        let truncated_tokens: Vec<usize> = tokens.into_iter().take(max_tokens).collect();
        TOKENIZER.decode(truncated_tokens).unwrap_or_else(|_| {
            // Fallback: character-based truncation (4 chars per token estimate)
            let max_chars = max_tokens * 4;
            text.char_indices()
                .take_while(|(i, _)| *i < max_chars)
                .map(|(_, c)| c)
                .collect()
        })
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
        assert!(
            count > 0 && count < 3,
            "Expected 1-2 tokens for 'hello', got {}",
            count
        );

        let count = counter.count("hello world").unwrap();
        assert!(
            count > 0 && count < 5,
            "Expected 2-4 tokens for 'hello world', got {}",
            count
        );
    }

    #[test]
    fn test_count_code() {
        let counter = TokenCounter::new();

        // Simple function
        let code = "fn main() { println!(\"Hello\"); }";
        let count = counter.count(code).unwrap();
        assert!(
            count > 5,
            "Expected >5 tokens for simple function, got {}",
            count
        );
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
        assert!(
            count > 20,
            "Expected >20 tokens for fibonacci function, got {}",
            count
        );
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
        let sum: usize = segments.iter().map(|s| counter.count(s).unwrap()).sum();
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

    // ===== HAPPY PATH TESTS - TEXT WITHIN LIMITS =====

    #[test]
    fn test_truncate_short_text_unchanged() {
        // Short text under limit should be returned unchanged
        let counter = TokenCounter::new();
        let text = "This is a short sentence.";
        let result = counter.truncate_to_limit(text, 100);
        assert_eq!(result, text, "Short text should be unchanged");
    }

    #[test]
    fn test_truncate_at_exact_limit_unchanged() {
        // Text exactly at token limit should be returned unchanged
        let counter = TokenCounter::new();
        let text = "a ".repeat(50); // ~50 tokens
        let token_count = TOKENIZER.encode_with_special_tokens(&text).len();
        let result = counter.truncate_to_limit(&text, token_count);
        assert_eq!(result, text, "Text at exact limit should be unchanged");
    }

    #[test]
    fn test_truncate_medium_text_under_limit() {
        // Medium text under limit should be returned unchanged
        let counter = TokenCounter::new();
        let text = "word ".repeat(200); // ~200 tokens
        let result = counter.truncate_to_limit(&text, 1000);
        assert_eq!(result, text, "Medium text under limit should be unchanged");
    }

    // ===== TRUNCATION TESTS - TEXT OVER LIMITS =====

    #[test]
    fn test_truncate_over_limit_reduces_size() {
        // Text over limit should be truncated to correct token count
        let counter = TokenCounter::new();
        let text = "word ".repeat(1000); // ~1000 tokens
        let result = counter.truncate_to_limit(&text, 100);
        assert!(
            result.len() < text.len(),
            "Truncated text should be shorter"
        );

        // Verify truncated result is within token limit
        let result_tokens = TOKENIZER.encode_with_special_tokens(&result);
        assert!(
            result_tokens.len() <= 100,
            "Truncated text should be within token limit"
        );
    }

    #[test]
    fn test_truncate_very_large_text() {
        // Very large text (20k+ tokens) should truncate to limit
        let counter = TokenCounter::new();
        let text = "word ".repeat(20000); // ~20k tokens (over Vertex AI limit)
        let result = counter.truncate_to_limit(&text, 19_000);

        let result_tokens = TOKENIZER.encode_with_special_tokens(&result);
        assert!(
            result_tokens.len() <= 19_000,
            "Very large text should truncate to limit"
        );
        assert!(
            result.len() < text.len(),
            "Truncated text should be shorter than original"
        );
    }

    #[test]
    fn test_truncate_result_fewer_tokens() {
        // Truncated result should have fewer tokens than original
        let counter = TokenCounter::new();
        let text = "word ".repeat(500);
        let original_tokens = TOKENIZER.encode_with_special_tokens(&text).len();
        let result = counter.truncate_to_limit(&text, 100);
        let result_tokens = TOKENIZER.encode_with_special_tokens(&result);

        assert!(
            result_tokens.len() < original_tokens,
            "Truncated result should have fewer tokens than original"
        );
        assert!(
            result_tokens.len() <= 100,
            "Truncated result should be within limit"
        );
    }

    #[test]
    fn test_truncate_result_is_valid_utf8() {
        // Truncated result should decode to valid UTF-8 string
        let counter = TokenCounter::new();
        let text = "word ".repeat(1000);
        let result = counter.truncate_to_limit(&text, 100);

        // If this doesn't panic, UTF-8 is valid
        assert!(
            std::str::from_utf8(result.as_bytes()).is_ok(),
            "Result should be valid UTF-8"
        );
    }

    // ===== EDGE CASE TESTS =====

    #[test]
    fn test_truncate_empty_string() {
        // Empty string should return empty string
        let counter = TokenCounter::new();
        let result = counter.truncate_to_limit("", 100);
        assert_eq!(result, "", "Empty string should return empty string");
    }

    #[test]
    fn test_truncate_single_token() {
        // Single token text should be handled correctly
        let counter = TokenCounter::new();
        let text = "word";
        let result = counter.truncate_to_limit(text, 1);
        assert!(
            !result.is_empty(),
            "Single token truncation should return non-empty string"
        );
    }

    #[test]
    fn test_truncate_one_over_limit() {
        // Text with exactly max_tokens + 1 (boundary condition)
        let counter = TokenCounter::new();
        let text = "word ".repeat(100);
        let token_count = TOKENIZER.encode_with_special_tokens(&text).len();
        let result = counter.truncate_to_limit(&text, token_count - 1);

        let result_tokens = TOKENIZER.encode_with_special_tokens(&result);
        assert!(
            result_tokens.len() <= token_count - 1,
            "Text one token over should truncate"
        );
    }

    #[test]
    fn test_truncate_zero_max_tokens() {
        // Zero max_tokens parameter (edge case)
        let counter = TokenCounter::new();
        let text = "Some text";
        let result = counter.truncate_to_limit(text, 0);

        // Should return empty or very short string, should not panic
        assert!(
            result.len() <= text.len(),
            "Zero limit should not panic and result should not be longer"
        );
    }

    // ===== UNICODE TESTS =====

    #[test]
    fn test_truncate_emoji_text() {
        // Unicode text (emoji) should truncate correctly
        let counter = TokenCounter::new();
        let text = "Hello 👋 World 🌍 ".repeat(100);
        let result = counter.truncate_to_limit(&text, 50);

        assert!(result.len() < text.len(), "Emoji text should truncate");
        assert!(
            std::str::from_utf8(result.as_bytes()).is_ok(),
            "Emoji result should be valid UTF-8"
        );
    }

    #[test]
    fn test_truncate_cjk_characters() {
        // CJK characters (Chinese/Japanese/Korean) should truncate correctly
        let counter = TokenCounter::new();
        let text = "你好世界 こんにちは 안녕하세요 ".repeat(100);
        let result = counter.truncate_to_limit(&text, 50);

        assert!(result.len() < text.len(), "CJK text should truncate");
        assert!(
            std::str::from_utf8(result.as_bytes()).is_ok(),
            "CJK result should be valid UTF-8"
        );
    }

    #[test]
    fn test_truncate_mixed_unicode_ascii() {
        // Mixed Unicode and ASCII should truncate correctly
        let counter = TokenCounter::new();
        let text = "English 中文 Emoji 🎉 ".repeat(100);
        let result = counter.truncate_to_limit(&text, 50);

        assert!(
            result.len() < text.len(),
            "Mixed Unicode/ASCII should truncate"
        );
        assert!(
            std::str::from_utf8(result.as_bytes()).is_ok(),
            "Mixed result should be valid UTF-8"
        );
    }

    #[test]
    fn test_truncate_unicode_no_panic() {
        // Unicode should not cause panics or invalid UTF-8
        let counter = TokenCounter::new();
        // Various Unicode edge cases
        let texts = vec![
            "🎉".repeat(1000),
            "你".repeat(1000),
            "a🎉b".repeat(1000),
            "test\u{FFFD}".repeat(100), // Replacement character
        ];

        for text in texts {
            let result = counter.truncate_to_limit(&text, 10);
            // Should not panic, should return valid string
            assert!(
                !result.is_empty() || text.is_empty(),
                "Unicode truncation should not panic"
            );
        }
    }
}
