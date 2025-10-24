//! Query tokenization for FTS compatibility.
//!
//! This tokenizer produces tokens compatible with PostgreSQL FTS indexing,
//! ensuring that query tokens match the indexed tsvector tokens.

use std::collections::HashSet;

/// Query tokenizer that produces FTS-compatible tokens.
///
/// This tokenizer implements simple tokenization rules that align with
/// PostgreSQL's 'simple' text search configuration, which:
/// - Splits on whitespace and punctuation
/// - Preserves code operators (::, ->, etc.)
/// - Normalizes to lowercase
/// - Removes stop words
pub struct Tokenizer {
    /// Stop words to filter out (common words with low signal)
    stop_words: HashSet<String>,
}

impl Tokenizer {
    /// Create a new tokenizer with default stop words.
    pub fn new() -> Self {
        Self {
            stop_words: Self::default_stop_words(),
        }
    }

    /// Create a tokenizer with custom stop words.
    pub fn with_stop_words(stop_words: HashSet<String>) -> Self {
        Self { stop_words }
    }

    /// Default stop words for English.
    fn default_stop_words() -> HashSet<String> {
        [
            "a", "an", "and", "are", "as", "at", "be", "by", "for", "from",
            "has", "he", "in", "is", "it", "its", "of", "on", "that", "the",
            "to", "was", "will", "with",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Tokenize a query string into FTS-compatible tokens.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crewchief_maproom::search::Tokenizer;
    ///
    /// let tokenizer = Tokenizer::new();
    /// let tokens = tokenizer.tokenize("authenticate user");
    /// assert_eq!(tokens, vec!["authenticate", "user"]);
    ///
    /// let tokens = tokenizer.tokenize("User::authenticate");
    /// assert_eq!(tokens, vec!["user", "::", "authenticate"]);
    /// ```
    pub fn tokenize(&self, query: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let chars: Vec<char> = query.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            match ch {
                // Handle multi-character operators
                ':' | '-' | '>' | '<' | '=' | '!' | '&' | '|' => {
                    // Check if we have a preceding token that was just saved
                    let had_preceding_token = !current_token.is_empty();

                    // Save current token if any
                    if !current_token.is_empty() {
                        self.add_token(&mut tokens, &current_token);
                        current_token.clear();
                    }

                    // Build multi-char operator
                    let mut operator = String::from(ch);
                    let mut is_multi_char = false;
                    if i + 1 < chars.len() {
                        let next = chars[i + 1];
                        // Check for multi-character operators
                        match (ch, next) {
                            (':', ':') | ('-', '>') | ('=', '>') | ('<', '-') |
                            ('!', '=') | ('=', '=') | ('<', '=') | ('>', '=') => {
                                operator.push(next);
                                i += 1; // Skip next char
                                is_multi_char = true;
                            }
                            _ => {}
                        }
                    }
                    // Only add operators if they're:
                    // 1. Multi-character operators (always keep)
                    // 2. Between alphanumeric chars (like a-b, not word!)
                    // 3. Before alphanumeric (like &mut, not after like world!)
                    let next_is_alnum = i + 1 < chars.len() && chars[i + 1].is_alphanumeric();
                    if is_multi_char || (!had_preceding_token && next_is_alnum) {
                        self.add_token(&mut tokens, &operator);
                    }
                    i += 1;
                    continue;
                }
                // Word boundaries (including operators we don't preserve)
                ' ' | '\t' | '\n' | '\r' | ',' | ';' | '(' | ')' | '[' | ']' |
                '{' | '}' | '"' | '\'' | '*' | '/' => {
                    if !current_token.is_empty() {
                        self.add_token(&mut tokens, &current_token);
                        current_token.clear();
                    }
                }
                // Preserve underscores and dots in identifiers
                '_' | '.' => {
                    current_token.push(ch);
                }
                // Normal alphanumeric characters
                _ if ch.is_alphanumeric() => {
                    current_token.push(ch.to_ascii_lowercase());
                }
                // Skip other characters
                _ => {
                    if !current_token.is_empty() {
                        self.add_token(&mut tokens, &current_token);
                        current_token.clear();
                    }
                }
            }
            i += 1;
        }

        // Add final token
        if !current_token.is_empty() {
            self.add_token(&mut tokens, &current_token);
        }

        tokens
    }

    /// Tokenize asynchronously (for consistency with async pipeline).
    pub async fn tokenize_async(&self, query: &str) -> Vec<String> {
        self.tokenize(query)
    }

    /// Add a token to the list, filtering stop words and empty strings.
    fn add_token(&self, tokens: &mut Vec<String>, token: &str) {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return;
        }

        // Don't filter operators
        if trimmed.starts_with(|c: char| !c.is_alphanumeric() && c != '_') {
            tokens.push(trimmed.to_string());
            return;
        }

        // Filter stop words (only for pure alphabetic tokens)
        if trimmed.chars().all(|c| c.is_alphabetic()) && self.stop_words.contains(trimmed) {
            return;
        }

        tokens.push(trimmed.to_string());
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokenization() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("hello world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_code_operators() {
        let tokenizer = Tokenizer::new();

        let tokens = tokenizer.tokenize("User::authenticate");
        assert_eq!(tokens, vec!["user", "::", "authenticate"]);

        let tokens = tokenizer.tokenize("array->length");
        assert_eq!(tokens, vec!["array", "->", "length"]);

        // Note: "a" and "b" are stop words but preserved in code context with operators
        let tokens = tokenizer.tokenize("value => result");
        assert_eq!(tokens, vec!["value", "=>", "result"]);
    }

    #[test]
    fn test_stop_word_filtering() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("the user is authenticated");
        assert_eq!(tokens, vec!["user", "authenticated"]);
    }

    #[test]
    fn test_underscores_preserved() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("user_name auth_token");
        assert_eq!(tokens, vec!["user_name", "auth_token"]);
    }

    #[test]
    fn test_dots_preserved() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("console.log user.name");
        assert_eq!(tokens, vec!["console.log", "user.name"]);
    }

    #[test]
    fn test_case_normalization() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("UserAuthentication Login");
        assert_eq!(tokens, vec!["userauthentication", "login"]);
    }

    #[test]
    fn test_punctuation_boundaries() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("hello, world! (test)");
        assert_eq!(tokens, vec!["hello", "world", "test"]);
    }

    #[test]
    fn test_mixed_query() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("authenticate the user with OAuth2.0");
        assert_eq!(tokens, vec!["authenticate", "user", "oauth2.0"]);
    }

    #[test]
    fn test_empty_query() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("   \t\n  ");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_custom_stop_words() {
        let mut stop_words = HashSet::new();
        stop_words.insert("custom".to_string());
        stop_words.insert("stop".to_string());

        let tokenizer = Tokenizer::with_stop_words(stop_words);
        let tokens = tokenizer.tokenize("custom word stop test");
        assert_eq!(tokens, vec!["word", "test"]);
    }

    #[test]
    fn test_code_patterns() {
        let tokenizer = Tokenizer::new();

        // Pointer dereferencing (* is a boundary character, not preserved)
        let tokens = tokenizer.tokenize("*ptr");
        assert_eq!(tokens, vec!["ptr"]);

        // Reference (&mut - & is preserved when next to alphanumeric)
        let tokens = tokenizer.tokenize("&mut value");
        assert_eq!(tokens, vec!["&", "mut", "value"]);

        // Comparison
        let tokens = tokenizer.tokenize("x == y");
        assert_eq!(tokens, vec!["x", "==", "y"]);

        // Not equal
        let tokens = tokenizer.tokenize("x != y");
        assert_eq!(tokens, vec!["x", "!=", "y"]);
    }

    #[tokio::test]
    async fn test_async_tokenization() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize_async("hello world").await;
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_multiple_operators() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("User::Auth->login");
        assert_eq!(tokens, vec!["user", "::", "auth", "->", "login"]);
    }

    #[test]
    fn test_numeric_identifiers() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("user123 auth2fa");
        assert_eq!(tokens, vec!["user123", "auth2fa"]);
    }
}
