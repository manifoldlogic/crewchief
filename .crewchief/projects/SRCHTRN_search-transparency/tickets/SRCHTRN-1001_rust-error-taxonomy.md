# SRCHTRN-1001: Rust Error Taxonomy

## Title
Create structured error taxonomy with conversion logic and actionable suggestions

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
Create `crates/maproom/src/search/errors.rs` with structured error types (`SearchErrorDetails`, `ErrorType`, `PipelineStage`) and conversion logic to transform `PipelineError` into actionable error diagnostics with context and suggestions.

## Background
Currently, when search pipeline errors occur, only generic error messages are returned to clients. Error context is lost during serialization. This ticket establishes the foundation for structured error reporting by defining a taxonomy that maps the 13 observed error scenarios to 6 high-level error types with actionable suggestions.

**Critical First Step**: Audit existing `PipelineError` types to verify sufficient context is available for extraction. May require minor refactoring of error types to capture necessary context.

## Acceptance Criteria
- [x] `PipelineError` types audited for context availability
- [x] `SearchErrorDetails` struct created with `error_type`, `stage`, `context`, `suggestions` fields
- [x] `ErrorType` enum defined with 6 variants: `embedding_provider`, `database`, `validation`, `timeout`, `not_found`, `unknown`
- [x] `PipelineStage` enum defined with 4 variants: `query_processing`, `search_execution`, `score_fusion`, `result_assembly`
- [x] `from_pipeline_error()` conversion function implemented with pattern matching
- [x] Each error type has 1-2 actionable suggestions (generic suggestions acceptable for limited-context errors)
- [x] Unit tests validate error conversion for all 6 error types
- [x] All tests passing
- [x] Type sync validation: Enum values match expected TypeScript types (manual check)

## Technical Requirements

### File Structure
Create `crates/maproom/src/search/errors.rs` with:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchErrorDetails {
    pub error_type: ErrorType,
    pub stage: PipelineStage,
    pub context: HashMap<String, String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    EmbeddingProvider,
    Database,
    Validation,
    Timeout,
    NotFound,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    QueryProcessing,
    SearchExecution,
    ScoreFusion,
    ResultAssembly,
}

impl SearchErrorDetails {
    pub fn from_pipeline_error(error: &PipelineError) -> Self {
        // Pattern match on PipelineError variants
        // Extract context from error types
        // Map to high-level ErrorType
        // Generate 1-2 actionable suggestions
    }
}
```

### Error Mapping Table
Map 13 observed scenarios → 6 error types:

**ErrorType::EmbeddingProvider** (3 scenarios)
- OpenAI API timeout → Context: provider_error, Suggestions: ["Check credentials", "Verify network", "Try FTS mode"]
- Google credentials missing → Context: provider, Suggestions: ["Set GOOGLE_API_KEY", "Check credentials file"]
- Ollama service not running → Context: provider, Suggestions: ["Start Ollama service", "Check localhost:11434"]

**ErrorType::Database** (4 scenarios)
- Repository not indexed → Context: repo_name, Suggestions: ["Run scan command", "Check index status"]
- Worktree not found → Context: worktree_id, Suggestions: ["Check worktree exists", "Run status command"]
- Corrupted SQLite database → Context: error_message, Suggestions: ["Backup and rebuild index", "Check disk space"]
- Database connection timeout → Context: error_message, Suggestions: ["Check database connectivity", "Restart daemon"]

**ErrorType::Validation** (2 scenarios)
- Empty query → Context: none, Suggestions: ["Provide non-empty query"]
- Query too long → Context: length, max_length, Suggestions: ["Shorten query to <1000 chars"]

**ErrorType::Timeout** (1 scenario)
- Search execution timeout → Context: timeout_ms, Suggestions: ["Narrow search scope", "Try simpler query"]

**ErrorType::NotFound** (2 scenarios)
- Repository not found → Context: repo_name, Suggestions: ["Check repository name", "Run status to list repos"]
- No meaningful content → Context: query, Suggestions: ["Add more specific terms"]

**ErrorType::Unknown** (fallback)
- Unexpected errors → Context: error_message, Suggestions: ["Report issue with error details"]

### Conversion Logic Pattern
```rust
impl SearchErrorDetails {
    pub fn from_pipeline_error(error: &PipelineError) -> Self {
        match error {
            PipelineError::QueryProcessing(query_error) => match query_error {
                QueryProcessorError::Embedding(emb_error) => Self {
                    error_type: ErrorType::EmbeddingProvider,
                    stage: PipelineStage::QueryProcessing,
                    context: extract_embedding_context(emb_error),
                    suggestions: vec![
                        "Check your embedding provider credentials".to_string(),
                        "Verify network connectivity".to_string(),
                        "Try FTS mode while debugging: --mode fts".to_string(),
                    ],
                },
                QueryProcessorError::EmptyQuery => Self {
                    error_type: ErrorType::Validation,
                    stage: PipelineStage::QueryProcessing,
                    context: HashMap::new(),
                    suggestions: vec!["Provide a non-empty search query".to_string()],
                },
                // ... other query processing errors
            },
            PipelineError::Database(db_error) => Self {
                error_type: ErrorType::Database,
                stage: PipelineStage::SearchExecution,
                context: extract_database_context(db_error),
                suggestions: vec![
                    "Check database connectivity".to_string(),
                    "Verify repository is indexed".to_string(),
                ],
            },
            // ... other pipeline errors
            _ => Self {
                error_type: ErrorType::Unknown,
                stage: PipelineStage::QueryProcessing, // best guess
                context: HashMap::from([("error".to_string(), error.to_string())]),
                suggestions: vec!["Please report this error with full details".to_string()],
            },
        }
    }
}
```

### Unit Tests
Create tests for:
- Each `ErrorType` variant conversion
- Context extraction correctness
- Suggestions present for each error
- Pattern matching coverage

Example test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_provider_error_conversion() {
        let pipeline_error = PipelineError::QueryProcessing(
            QueryProcessorError::Embedding(EmbeddingError::Timeout)
        );

        let details = SearchErrorDetails::from_pipeline_error(&pipeline_error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert!(details.suggestions.len() >= 2);
        assert!(details.suggestions.iter().any(|s| s.contains("FTS mode")));
    }

    #[test]
    fn test_all_error_types_have_suggestions() {
        // Test that every error type produces suggestions
    }
}
```

## Implementation Notes
1. **First**: Audit `PipelineError` in `crates/maproom/src/search/pipeline.rs` or wherever it's defined
2. **Verify**: Error variants have sufficient context for extraction
3. **Refactor if needed**: Add context fields to error types if missing (e.g., repo name, timeout duration)
4. **Keep MVP scope**: Hardcoded string suggestions are acceptable, no complex suggestion engine
5. **Pragmatic approach**: Generic suggestions for errors with limited context

**Note on Suggestions**: It's acceptable to have 1-2 suggestions instead of 2-3 if error context is limited. Quality over quantity.

**Error Context Whitelist** (security constraint):
Only extract these whitelisted context keys from errors to prevent accidental exposure of sensitive data:
- `provider_error` - Embedding provider error details
- `provider` - Provider name (OpenAI, Google, Ollama)
- `error` - Generic error message
- `message` - Human-readable message
- `length` - Query length for validation errors
- `max_length` - Maximum allowed query length
- `repo_name` - Repository name for not found errors
- `worktree_id` - Worktree identifier
- `timeout_ms` - Timeout duration

Do NOT extract other fields (e.g., API keys, tokens, file paths, user data) into error context.

## Dependencies
- **SRCHTRN-1000**: Performance baseline measured (parallel work acceptable)

## Risk Assessment
**Risk Level**: Medium

**Risks**:
- PipelineError may lack context for specific error scenarios
- May need to refactor error types for better context extraction
- Pattern matching may miss edge cases

**Mitigations**:
- Audit error types first before implementation
- Use `Unknown` error type as fallback for unmapped scenarios
- Generic suggestions acceptable for MVP
- Comprehensive unit tests for coverage

## Files/Packages Affected
- **New file**: `crates/maproom/src/search/errors.rs`
- **Modified**: `crates/maproom/src/search/mod.rs` (add `pub mod errors;`)
- **Modified**: `crates/maproom/src/lib.rs` (export error types if needed)
- **Reference**: `crates/maproom/src/search/pipeline.rs` (PipelineError definition)

## Estimated Effort
6-8 hours

**Breakdown**:
- PipelineError audit: 1-2 hours
- Error taxonomy implementation: 2-3 hours
- Conversion logic: 2-3 hours
- Unit tests: 1-2 hours

**Note**: May exceed 8 hours if significant error type refactoring needed. Flag if approaching limit.

## Planning References
- [plan.md](../planning/plan.md) - Phase 1 ticket breakdown
- [architecture.md](../planning/architecture.md) - Error taxonomy design
- [quality-strategy.md](../planning/quality-strategy.md) - Unit testing approach
