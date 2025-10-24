# Ticket: HYBRID_SEARCH-2001: Query Processing Pipeline

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- search-quality-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement a comprehensive query processing pipeline that tokenizes search queries, generates embeddings, expands query terms, and detects the appropriate search mode (Code, Text, or Auto) to prepare queries for hybrid search execution.

## Background
This is Phase 2, Week 2, Task 1 of the Hybrid Search implementation plan. The query processor is a critical component that sits at the entry point of the search pipeline, transforming raw user queries into optimized, multi-faceted representations suitable for FTS, vector, and graph-based search strategies.

The query processor must handle diverse query types (code symbols, natural language questions, mixed queries) and prepare them for parallel execution across multiple search subsystems. It uses heuristics to detect query intent and applies appropriate preprocessing strategies.

## Acceptance Criteria
- [ ] Query tokenizer working - produces tokens compatible with FTS indexing
- [ ] Query expansion logic implemented - expands terms with synonyms/related concepts
- [ ] Embedding generation for queries - integrates with EmbeddingService to generate query vectors
- [ ] Search mode detection functional - accurately classifies queries as Code, Text, or Auto
- [ ] Parallel processing implemented - uses tokio::join! for concurrent tokenization, embedding, expansion
- [ ] Unit tests pass for all query processor components
- [ ] Integration tests validate query processing with real search pipeline

## Technical Requirements
- Create `QueryProcessor` struct with fields for tokenizer, embedder, and expander
- Implement async `process()` method that returns `ProcessedQuery` struct
- Use `tokio::join!` for parallel processing of tokenization, embedding, and expansion
- Implement `detect_mode()` method with query heuristics:
  - Code mode: queries containing `::`, `->`, or code-like syntax
  - Text mode: queries with more than 3 words
  - Auto mode: fallback for ambiguous queries
- Integrate with `EmbeddingService` (from HYBRID_SEARCH-1001) for query embedding generation
- Define types: `ProcessedQuery`, `SearchMode` enum (Code, Text, Auto)
- Implement tokenizer compatible with existing FTS token processing
- Implement query expander with synonym/concept expansion logic

## Implementation Notes

### Architecture Reference
See `/workspace/crewchief_context/maproom/HYBRID_SEARCH/HYBRID_SEARCH_ARCHITECTURE.md`:
- Query Processor section (lines 44-81) for struct design
- Query Pipeline section (lines 296-340) for integration context

### Core Components

**1. Query Processor Struct**
```rust
pub struct QueryProcessor {
    tokenizer: Tokenizer,
    embedder: EmbeddingClient,
    expander: QueryExpander,
}
```

**2. Processed Query Output**
```rust
pub struct ProcessedQuery {
    original: String,
    tokens: Vec<String>,
    embedding: Vec<f32>,
    expanded_terms: Vec<String>,
    mode: SearchMode,
}
```

**3. Search Mode Detection**
Implement heuristic-based detection:
- Look for code patterns (`::`), (`->`)
- Count word tokens for natural language detection
- Default to Auto for ambiguous cases

**4. Parallel Processing**
Use `tokio::join!` to run tokenization, embedding, and expansion concurrently for optimal performance.

**5. Integration Points**
- Embedding generation via `EmbeddingService` (dependency HYBRID_SEARCH-1001)
- Tokenizer should align with FTS token processing from Phase 1
- Query expansion should consider domain-specific synonyms (e.g., "function" -> "fn", "method")

### Testing Strategy
- Unit tests for each component (tokenizer, expander, mode detector)
- Integration tests with mock EmbeddingService
- End-to-end tests with sample queries (code, text, mixed)
- Performance tests for parallel processing efficiency

## Dependencies
- **HYBRID_SEARCH-1001** (Embedding Service Setup) - Required for query embedding generation
- Existing FTS tokenization logic from Phase 1 (for consistency)

## Risk Assessment
- **Risk**: Query expansion may introduce noise or irrelevant terms
  - **Mitigation**: Start with conservative expansion rules; implement configurable expansion strategies; add quality metrics to measure expansion effectiveness

- **Risk**: Search mode detection heuristics may misclassify queries
  - **Mitigation**: Log detected modes for analysis; implement override mechanism in SearchOptions; gather real-world query data to refine heuristics

- **Risk**: Embedding generation latency may slow query processing
  - **Mitigation**: Parallel processing with tokio::join!; consider query embedding caching for repeated queries; implement timeout fallbacks

- **Risk**: Tokenizer inconsistencies between query and index tokenization
  - **Mitigation**: Reuse existing tokenization logic; comprehensive integration tests; document tokenization strategy

## Files/Packages Affected
- `crates/maproom/src/search/query_processor.rs` - Main QueryProcessor implementation (CREATE)
- `crates/maproom/src/search/tokenizer.rs` - Query tokenization logic (CREATE)
- `crates/maproom/src/search/expander.rs` - Query expansion logic (CREATE)
- `crates/maproom/src/search/types.rs` - ProcessedQuery, SearchMode types (CREATE)
- `crates/maproom/src/search/mod.rs` - Module exports (MODIFY)
- `crates/maproom/tests/search/query_processor_test.rs` - Unit tests (CREATE)
- `crates/maproom/tests/integration/query_pipeline_test.rs` - Integration tests (CREATE)
