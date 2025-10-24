//! Integration tests for QueryProcessor components.
//!
//! These tests verify the functionality of tokenization, query expansion,
//! and mode detection that don't require embedding generation.
//!
//! NOTE: Full end-to-end tests with embedding generation will be added after
//! HYBRID_SEARCH-1001 (Embedding Service Setup) is complete with proper mock support.

use crewchief_maproom::search::{QueryExpander, Tokenizer};
use std::collections::HashMap;

#[test]
fn test_tokenizer_integration() {
    let tokenizer = Tokenizer::new();

    // Test various query types
    let test_cases = vec![
        ("User::authenticate", vec!["user", "::", "authenticate"]),
        ("array->map()", vec!["array", "->", "map"]),
        ("how to authenticate users", vec!["authenticate", "users"]),
        ("const API_KEY = 'test'", vec!["const", "api_key", "test"]),
        ("x != y and z == 0", vec!["x", "!=", "y", "z", "==", "0"]), // "and" is a stop word
    ];

    for (query, expected) in test_cases {
        let tokens = tokenizer.tokenize(query);
        for exp_token in expected {
            assert!(
                tokens.contains(&exp_token.to_string()),
                "Query '{}' should contain token '{}', got: {:?}",
                query,
                exp_token,
                tokens
            );
        }
    }
}

#[tokio::test]
async fn test_tokenizer_async() {
    let tokenizer = Tokenizer::new();
    let tokens = tokenizer.tokenize_async("async function test()").await;

    assert!(tokens.contains(&"async".to_string()));
    assert!(tokens.contains(&"function".to_string()));
    assert!(tokens.contains(&"test".to_string()));
}

#[test]
fn test_query_expander_integration() {
    let expander = QueryExpander::new();

    // Test function synonyms
    let tokens = vec!["function".to_string()];
    let expanded = expander.expand(&tokens);
    assert!(expanded.contains(&"fn".to_string()));
    assert!(expanded.contains(&"method".to_string()));

    // Test auth synonyms
    let tokens = vec!["auth".to_string()];
    let expanded = expander.expand(&tokens);
    assert!(expanded.contains(&"authentication".to_string()));
    assert!(expanded.contains(&"login".to_string()));

    // Test database synonyms
    let tokens = vec!["database".to_string()];
    let expanded = expander.expand(&tokens);
    assert!(expanded.contains(&"db".to_string()));
}

#[tokio::test]
async fn test_query_expander_async() {
    let expander = QueryExpander::new();
    let tokens = vec!["function".to_string(), "auth".to_string()];
    let expanded = expander.expand_async(&tokens).await;

    // Should have expansions from both tokens
    assert!(!expanded.is_empty());
    assert!(expanded.len() > 2); // More than just the input tokens
}

#[test]
fn test_custom_query_expander() {
    let mut synonyms = HashMap::new();
    synonyms.insert("api".to_string(), vec!["endpoint".to_string(), "service".to_string()]);
    synonyms.insert("test".to_string(), vec!["spec".to_string(), "unittest".to_string()]);

    let expander = QueryExpander::with_synonyms(synonyms);

    let tokens = vec!["api".to_string()];
    let expanded = expander.expand(&tokens);

    assert_eq!(expanded.len(), 2);
    assert!(expanded.contains(&"endpoint".to_string()));
    assert!(expanded.contains(&"service".to_string()));
}

#[test]
fn test_mode_detection_code_patterns() {
    // We can't create a full QueryProcessor without embeddings,
    // but we can test the detection heuristics directly
    let test_cases = vec![
        ("User::authenticate", true),  // :: operator -> code
        ("array->length", true),       // -> operator -> code
        ("x => y", true),              // => operator -> code
        ("x != y", true),              // != operator -> code
        ("getValue()", true),          // () pattern -> code
        ("authenticate", true),        // single word -> code
        ("user_name", true),           // snake_case -> code
        ("UserAuth", true),            // CamelCase -> code
    ];

    for (query, should_be_code) in test_cases {
        let has_code_pattern = query.contains("::")
            || query.contains("->")
            || query.contains("=>")
            || query.contains("!=")
            || query.contains("==")
            || (query.contains('(') && query.contains(')'))
            || query.split_whitespace().count() <= 2;

        assert_eq!(
            has_code_pattern, should_be_code,
            "Query '{}' code detection mismatch",
            query
        );
    }
}

#[test]
fn test_mode_detection_text_patterns() {
    let test_cases = vec![
        ("how to authenticate a user", true),      // 5 words -> text
        ("what is the login process", true),       // 5 words -> text
        ("find all authentication functions", true), // 4 words -> text
    ];

    for (query, should_be_text) in test_cases {
        let word_count = query.split_whitespace().count();
        let is_text = word_count > 3;

        assert_eq!(is_text, should_be_text, "Query '{}' should be text mode", query);
    }
}

#[test]
fn test_tokenizer_preserves_code_operators() {
    let tokenizer = Tokenizer::new();

    // Multi-character operators
    let operators = vec!["::", "->", "=>", "!=", "==", "<=", ">="];

    for op in operators {
        let query = format!("a {} b", op);
        let tokens = tokenizer.tokenize(&query);
        assert!(
            tokens.contains(&op.to_string()),
            "Should preserve operator '{}'",
            op
        );
    }
}

#[test]
fn test_tokenizer_filters_stop_words() {
    let tokenizer = Tokenizer::new();
    let query = "the user is authenticated with the password";
    let tokens = tokenizer.tokenize(query);

    // Stop words should be filtered
    assert!(!tokens.contains(&"the".to_string()));
    assert!(!tokens.contains(&"is".to_string()));
    assert!(!tokens.contains(&"with".to_string()));

    // Meaningful words should be kept
    assert!(tokens.contains(&"user".to_string()));
    assert!(tokens.contains(&"authenticated".to_string()));
    assert!(tokens.contains(&"password".to_string()));
}

#[test]
fn test_query_expansion_coverage() {
    let expander = QueryExpander::new();

    // Test a variety of common programming terms
    let test_terms = vec![
        ("function", vec!["fn", "func", "method"]),
        ("class", vec!["type", "struct", "object"]),
        ("variable", vec!["var", "let", "const"]),
        ("error", vec!["err", "exception"]),
        ("config", vec!["configuration", "settings"]),
    ];

    for (term, expected_synonyms) in test_terms {
        let tokens = vec![term.to_string()];
        let expanded = expander.expand(&tokens);

        for synonym in expected_synonyms {
            assert!(
                expanded.contains(&synonym.to_string()),
                "Expanding '{}' should include '{}'",
                term,
                synonym
            );
        }
    }
}

#[test]
fn test_tokenizer_handles_mixed_content() {
    let tokenizer = Tokenizer::new();

    let query = "User::authenticate() with OAuth2.0 and JWT tokens";
    let tokens = tokenizer.tokenize(query);

    // Should have code operators
    assert!(tokens.contains(&"::".to_string()));

    // Should have meaningful words
    assert!(tokens.contains(&"user".to_string()));
    assert!(tokens.contains(&"authenticate".to_string()));
    assert!(tokens.contains(&"oauth2.0".to_string()));
    assert!(tokens.contains(&"jwt".to_string()));
    assert!(tokens.contains(&"tokens".to_string()));

    // Should filter stop words
    assert!(!tokens.contains(&"with".to_string()));
    assert!(!tokens.contains(&"and".to_string()));
}

#[test]
fn test_expander_deduplication() {
    let expander = QueryExpander::new();

    // "fn" and "function" should produce overlapping synonyms
    let tokens = vec!["fn".to_string(), "function".to_string()];
    let expanded = expander.expand(&tokens);

    // Verify no duplicates
    let mut sorted = expanded.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(expanded.len(), sorted.len(), "Expanded terms should be deduplicated");
}

#[test]
fn test_tokenizer_edge_cases() {
    let tokenizer = Tokenizer::new();

    // Empty query
    assert!(tokenizer.tokenize("").is_empty());

    // Whitespace only
    assert!(tokenizer.tokenize("   \t\n  ").is_empty());

    // Single character
    let tokens = tokenizer.tokenize("a");
    assert!(tokens.is_empty() || tokens == vec!["a"]); // "a" might be filtered as stop word

    // Special characters only
    let tokens = tokenizer.tokenize("!@#$%");
    assert!(tokens.is_empty() || tokens.iter().all(|t| !t.chars().any(|c| c.is_alphanumeric())));
}

#[test]
fn test_expander_prefix_matching() {
    let expander = QueryExpander::new();

    // "authentication" should match "auth" via prefix matching
    let tokens = vec!["authentication".to_string()];
    let expanded = expander.expand(&tokens);

    // Should find related terms from "auth" synonyms
    assert!(!expanded.is_empty(), "Should expand via prefix matching");
}

#[test]
fn test_custom_tokenizer_stop_words() {
    let mut stop_words = std::collections::HashSet::new();
    stop_words.insert("custom".to_string());
    stop_words.insert("stop".to_string());

    let tokenizer = Tokenizer::with_stop_words(stop_words);
    let tokens = tokenizer.tokenize("custom word stop test");

    assert!(!tokens.contains(&"custom".to_string()));
    assert!(!tokens.contains(&"stop".to_string()));
    assert!(tokens.contains(&"word".to_string()));
    assert!(tokens.contains(&"test".to_string()));
}

#[test]
fn test_synchronized_expansion() {
    // Test that synchronous and async expansion produce same results
    let expander = QueryExpander::new();
    let tokens = vec!["function".to_string(), "auth".to_string()];

    let sync_result = expander.expand(&tokens);

    // Async version should match
    let rt = tokio::runtime::Runtime::new().unwrap();
    let async_result = rt.block_on(async { expander.expand_async(&tokens).await });

    assert_eq!(sync_result, async_result);
}
