# SRCHTRN-3001: Enhanced Error Suggestions

## Title
Improve error suggestions with context-specific recommendations

## Status
- [x] **Implementation Complete**
- [x] **Tests Passing**
- [x] **Verified**
- [x] **Committed**

## Agents
- **Primary**: rust-engineer
- **unit-test-runner**: Execute tests
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Enhance error suggestion logic in `crates/maproom/src/search/errors.rs` to provide more context-specific recommendations based on error context (provider type, database error details, etc.).

## Background
Phase 1 established basic error suggestions (often generic). Phase 3 improves suggestion quality by using error context to provide more specific, actionable recommendations. For example, OpenAI errors suggest different fixes than Ollama errors.

**Focus**: Improve what we can with available context. Generic suggestions remain acceptable for errors with limited context.

## Acceptance Criteria
- [x] Provider-specific suggestions for embedding errors (OpenAI vs Ollama vs Google)
- [x] Database-specific suggestions based on error message patterns
- [x] At least 2 refinement suggestions per error type
- [x] Context-aware suggestion selection (e.g., suggest Ollama restart if provider is Ollama)
- [x] Unit tests validate context-specific suggestions
- [x] Manual test: OpenAI timeout suggests credential check, Ollama suggests service start
- [x] All tests passing

## Technical Requirements

### Enhance Error Conversion: `crates/maproom/src/search/errors.rs`

```rust
impl SearchErrorDetails {
    pub fn from_pipeline_error(error: &PipelineError) -> Self {
        match error {
            PipelineError::QueryProcessing(query_error) => match query_error {
                QueryProcessorError::Embedding(emb_error) => {
                    let (context, suggestions) = Self::embedding_error_details(emb_error);
                    Self {
                        error_type: ErrorType::EmbeddingProvider,
                        stage: PipelineStage::QueryProcessing,
                        context,
                        suggestions,
                    }
                }
                // ... other cases
            },
            PipelineError::Database(db_error) => {
                let (context, suggestions) = Self::database_error_details(db_error);
                Self {
                    error_type: ErrorType::Database,
                    stage: PipelineStage::SearchExecution,
                    context,
                    suggestions,
                }
            }
            // ... other cases
        }
    }

    fn embedding_error_details(error: &EmbeddingError) -> (HashMap<String, String>, Vec<String>) {
        let mut context = HashMap::new();

        let suggestions = match error {
            EmbeddingError::OpenAI(openai_error) => {
                context.insert("provider".to_string(), "OpenAI".to_string());
                context.insert("error".to_string(), openai_error.to_string());

                if openai_error.contains("timeout") {
                    vec![
                        "Check your network connectivity".to_string(),
                        "Verify OpenAI API status: https://status.openai.com".to_string(),
                        "Try increasing timeout in config".to_string(),
                        "Fallback to FTS mode: --mode fts".to_string(),
                    ]
                } else if openai_error.contains("unauthorized") || openai_error.contains("invalid") {
                    vec![
                        "Check OPENAI_API_KEY environment variable".to_string(),
                        "Verify API key is valid and not expired".to_string(),
                        "Check account billing status".to_string(),
                    ]
                } else {
                    vec![
                        "Check OpenAI API credentials".to_string(),
                        "Try FTS mode: --mode fts".to_string(),
                    ]
                }
            }
            EmbeddingError::Ollama(ollama_error) => {
                context.insert("provider".to_string(), "Ollama".to_string());
                context.insert("error".to_string(), ollama_error.to_string());

                if ollama_error.contains("connection") {
                    vec![
                        "Start Ollama service: ollama serve".to_string(),
                        "Verify Ollama is running: curl http://localhost:11434".to_string(),
                        "Check OLLAMA_HOST environment variable".to_string(),
                    ]
                } else if ollama_error.contains("model") {
                    vec![
                        "Pull required model: ollama pull llama2".to_string(),
                        "List available models: ollama list".to_string(),
                        "Check model name in config".to_string(),
                    ]
                } else {
                    vec![
                        "Check Ollama service status".to_string(),
                        "Try FTS mode: --mode fts".to_string(),
                    ]
                }
            }
            EmbeddingError::Google(google_error) => {
                context.insert("provider".to_string(), "Google".to_string());
                context.insert("error".to_string(), google_error.to_string());

                vec![
                    "Check GOOGLE_API_KEY environment variable".to_string(),
                    "Verify Google Cloud credentials".to_string(),
                    "Check API quota limits".to_string(),
                ]
            }
            _ => {
                context.insert("error".to_string(), error.to_string());
                vec![
                    "Check embedding provider configuration".to_string(),
                    "Try FTS mode: --mode fts".to_string(),
                ]
            }
        };

        (context, suggestions)
    }

    fn database_error_details(error: &DatabaseError) -> (HashMap<String, String>, Vec<String>) {
        let mut context = HashMap::new();
        context.insert("message".to_string(), error.to_string());

        let error_str = error.to_string().to_lowercase();
        let suggestions = if error_str.contains("not found") || error_str.contains("does not exist") {
            vec![
                "Check repository name and path".to_string(),
                "Run 'crewchief status' to list indexed repositories".to_string(),
                "Index repository: 'crewchief scan <path>'".to_string(),
            ]
        } else if error_str.contains("connection") || error_str.contains("timeout") {
            vec![
                "Check database file exists: ~/.maproom/maproom.db".to_string(),
                "Verify database is not locked by another process".to_string(),
                "Restart daemon: crewchief-maproom daemon restart".to_string(),
            ]
        } else if error_str.contains("corrupt") || error_str.contains("malformed") {
            vec![
                "Backup database: cp ~/.maproom/maproom.db ~/.maproom/maproom.db.backup".to_string(),
                "Rebuild index: crewchief scan --rebuild".to_string(),
                "Check disk space".to_string(),
            ]
        } else {
            vec![
                "Check database connectivity".to_string(),
                "Verify repository is indexed".to_string(),
            ]
        };

        (context, suggestions)
    }
}
```

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_timeout_suggestions() {
        let error = PipelineError::QueryProcessing(
            QueryProcessorError::Embedding(
                EmbeddingError::OpenAI("request timeout".to_string())
            )
        );

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.context.get("provider").unwrap(), "OpenAI");
        assert!(details.suggestions.len() >= 3);
        assert!(details.suggestions.iter().any(|s| s.contains("status.openai.com")));
        assert!(details.suggestions.iter().any(|s| s.contains("FTS mode")));
    }

    #[test]
    fn test_ollama_connection_suggestions() {
        let error = PipelineError::QueryProcessing(
            QueryProcessorError::Embedding(
                EmbeddingError::Ollama("connection refused".to_string())
            )
        );

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert_eq!(details.context.get("provider").unwrap(), "Ollama");
        assert!(details.suggestions.iter().any(|s| s.contains("ollama serve")));
        assert!(details.suggestions.iter().any(|s| s.contains("localhost:11434")));
    }

    #[test]
    fn test_database_not_found_suggestions() {
        let error = PipelineError::Database(
            DatabaseError::new("repository not found")
        );

        let details = SearchErrorDetails::from_pipeline_error(&error);

        assert!(details.suggestions.iter().any(|s| s.contains("crewchief status")));
        assert!(details.suggestions.iter().any(|s| s.contains("scan")));
    }
}
```

## Implementation Notes
1. Extract provider type from error context (if available)
2. Pattern match on error messages for common scenarios
3. Provide 3-4 specific suggestions for well-understood errors
4. Fall back to generic suggestions for unknown error patterns
5. Keep suggestions actionable (specific commands, URLs, config keys)

**Suggestion Quality Tiers**:
- **Best**: Provider-specific with exact commands (e.g., "ollama serve")
- **Good**: Error pattern-specific (e.g., timeout → network suggestions)
- **Acceptable**: Generic but actionable (e.g., "Check credentials")

**Don't Over-Engineer**: Use simple string matching on error messages. No complex error classification needed.

## Dependencies
**Phase 1 and Phase 2 Complete**: Error taxonomy and query understanding established

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Error message patterns may change
- Provider detection may fail
- Too many suggestions may overwhelm users

**Mitigations**:
- Graceful fallback to generic suggestions
- Limit to 3-4 suggestions per error
- Test with real error scenarios

## Files/Packages Affected
- **Modified**: `crates/maproom/src/search/errors.rs` (~80 lines added)
- **Modified**: `crates/maproom/src/search/errors.rs` (unit tests, ~60 lines)

## Estimated Effort
4-5 hours

**Breakdown**:
- Provider-specific logic: 2-3 hours
- Database error patterns: 1 hour
- Unit tests: 1-2 hours

## Planning References
- [plan.md](../planning/plan.md) - Phase 3 ticket breakdown
- [architecture.md](../planning/architecture.md) - Error mapping, suggestion generation
- [quality-strategy.md](../planning/quality-strategy.md) - Manual testing approach
