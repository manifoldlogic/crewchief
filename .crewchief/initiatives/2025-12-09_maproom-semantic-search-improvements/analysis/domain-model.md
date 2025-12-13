# Domain Model: Semantic Search System

## Core Entities

### SearchQuery

Represents a user's search intent with execution parameters.

**Attributes:**
- `query_text`: Original user input (natural language or keywords)
- `normalized_query`: Processed form (case-folded, symbol normalization)
- `search_mode`: Execution strategy (fts | vector | hybrid)
- `repository`: Target repo identifier
- `filters`: Type/path/recency constraints
- `result_limit`: Maximum results to return (k)
- `debug_mode`: Whether to include diagnostic metadata

**Relationships:**
- Produces → SearchResults
- Validated by → QueryValidator
- Executed by → SearchEngine

**Key Behaviors:**
- Normalization (camelCase → snake_case, whitespace handling)
- Validation (parameter checks, syntax validation)
- Mode selection (based on query characteristics)

### SearchResult

Individual match returned from search operation.

**Attributes:**
- `chunk_id`: Unique chunk identifier
- `symbol_name`: Function/class/module name
- `kind`: Chunk type (func, class, test, heading, etc.)
- `file_path`: Relative path within repository
- `content`: Actual code/text content
- `start_line`: Beginning line number
- `end_line`: Ending line number
- `score`: Final ranking score
- `confidence`: Normalized quality indicator (0-100)
- `score_breakdown`: Debug information (optional)

**Relationships:**
- Grouped in → SearchResults
- Related to → SearchResult (via edges)
- Ranked by → RankingEngine

**Key Behaviors:**
- Confidence calculation from raw score
- Score normalization across modes
- Metadata enrichment (kind multipliers, exact match indicators)

### SearchResults

Collection of results with query metadata.

**Attributes:**
- `results`: Ordered list of SearchResult
- `total_matches`: Count before limit applied
- `query_metadata`: Understanding feedback
- `execution_time`: Latency in milliseconds
- `mode_used`: Actual execution mode
- `filters_applied`: Active filtering rules

**Relationships:**
- Produced by → SearchQuery
- Contains → SearchResult[]
- May include → RelatedResults

**Key Behaviors:**
- Progressive filtering (confidence-based cutoff)
- Result clustering (by relationship)
- Deduplication (across worktrees)

### SearchFilter

Constraints applied to narrow result set.

**Attributes:**
- `type_filter`: Content type (code | docs | tests | all)
- `path_patterns`: Inclusion/exclusion glob patterns
- `file_extensions`: Allowed file types
- `recency_threshold`: Modified-since date
- `archived_excluded`: Whether to filter .crewchief/archive/

**Relationships:**
- Applied to → SearchQuery
- Modifies → SearchResults

**Key Behaviors:**
- Path matching (glob pattern evaluation)
- Type inference (from file path and chunk kind)
- Smart defaults (code-first, exclude archived)

### ConfidenceScore

Quality assessment for search results.

**Attributes:**
- `raw_score`: Unnormalized score from ranking
- `normalized_score`: 0-100 confidence percentage
- `confidence_band`: High (80-100) | Medium (50-80) | Low (0-50)
- `components`: Breakdown (FTS, vector, kind, exact match)

**Relationships:**
- Computed for → SearchResult
- Based on → RankingScore

**Key Behaviors:**
- Normalization (score → confidence percentage)
- Band classification (threshold-based bucketing)
- Component explanation (debug mode)

### RelatedResults

Semantically related chunks for a result.

**Attributes:**
- `primary_result`: Main search result
- `related_chunks`: Ordered list of related SearchResult
- `relationship_type`: Import | Call | SameFile | SameModule
- `strength`: Relationship strength score

**Relationships:**
- Extends → SearchResult
- Derived from → EdgeRelationships

**Key Behaviors:**
- Graph traversal (follow edges)
- Relationship scoring (weighted by edge type)
- Clustering (group by proximity)

## Supporting Entities

### RankingScore

Raw scoring components before normalization.

**Attributes:**
- `fts_score`: Full-text search rank
- `vector_score`: Cosine similarity (if applicable)
- `kind_multiplier`: Semantic ranking boost (0.3-2.5x)
- `exact_match_multiplier`: Symbol name match bonus (1.0x or 3.0x)
- `fusion_score`: RRF combined score (hybrid mode)

**Relationships:**
- Produces → ConfidenceScore
- Applied to → SearchResult

### QueryUnderstanding

Diagnostic feedback about query processing.

**Attributes:**
- `original_query`: Raw user input
- `normalized_form`: Processed query terms
- `mode_selection_rationale`: Why fts/vector/hybrid chosen
- `detected_patterns`: Symbol names, camelCase, etc.
- `validation_warnings`: Non-fatal issues (e.g., very broad query)

**Relationships:**
- Produced for → SearchQuery
- Included in → SearchResults (metadata)

### ErrorDiagnostic

Actionable error information for failed searches.

**Attributes:**
- `error_code`: Specific error type (not generic RPC_ERROR)
- `message`: Human-readable description
- `cause`: Root cause details
- `suggested_actions`: List of fixes to try
- `related_docs`: Links to relevant documentation

**Relationships:**
- Produced by → SearchQuery (on failure)
- Replaces → Generic RPC_ERROR

## Relationships and Edges

### Import Relationships

Code-level dependencies between chunks.

**Attributes:**
- `source_chunk`: Importing chunk
- `target_chunk`: Imported chunk
- `import_type`: ES6 import | Python import | Rust use | etc.

**Usage:**
- Cluster related chunks
- Boost results in same module
- Surface implementation dependencies

### Call Relationships

Function/method invocations between chunks.

**Attributes:**
- `caller_chunk`: Invoking function
- `callee_chunk`: Called function
- `call_context`: Direct | Indirect (via variable)

**Usage:**
- Find usage examples
- Discover related functionality
- Identify cross-cutting concerns

### Proximity Relationships

Chunks in same file or nearby files.

**Attributes:**
- `chunk_a`: First chunk
- `chunk_b`: Second chunk
- `distance`: Line distance (same file) or file distance (same directory)

**Usage:**
- Cluster implementation details
- Group related code sections
- Improve context understanding

## Boundaries

### Search Core (In Scope)

- Query processing and validation
- Result ranking and scoring
- Filtering and constraints
- Confidence assessment
- Relationship clustering

### Indexing System (Out of Scope)

- Code parsing (tree-sitter)
- Chunk extraction
- Embedding generation
- FTS index maintenance
- Edge relationship extraction

**Boundary:** Search operates on pre-indexed data. Indexing improvements are separate initiatives.

### Client Presentation (Out of Scope)

- UI rendering of results
- User interaction tracking
- Search history management
- Personalization and preferences

**Boundary:** Search returns structured data. Clients decide presentation and interaction.

### Analytics and ML (Out of Scope - Phase 2)

- Query expansion via ML
- Result re-ranking based on user behavior
- Personalized search
- Search quality analytics

**Boundary:** Foundation features first. ML enhancements in future phases.

## Interactions

### Search Pipeline Architecture

```
User Query
    ↓
QueryValidator
    ↓ (validation passed)
QueryNormalizer
    ↓ (normalized query)
SearchModeSelector
    ↓ (mode: fts|vector|hybrid)
SearchFilterApplier
    ↓ (constraints applied)
SearchEngine (Rust daemon)
    ↓ (raw results)
RankingEngine (semantic scoring)
    ↓ (scored results)
ConfidenceScorer
    ↓ (confidence-enriched results)
ProgressiveFilter (cutoff low-confidence)
    ↓ (high-quality results)
RelationshipClusterer
    ↓ (grouped results)
SearchResults
    ↓
User
```

### Key Interaction Patterns

**Query Processing Flow:**
1. User submits query via MCP tool
2. QueryValidator checks parameters
3. QueryNormalizer processes text (camelCase → snake_case)
4. SearchModeSelector chooses execution strategy
5. SearchFilterApplier adds constraints
6. Query sent to Rust daemon via JSON-RPC

**Result Production Flow:**
1. SearchEngine executes FTS/vector/hybrid search
2. RankingEngine applies semantic multipliers
3. ConfidenceScorer normalizes scores
4. ProgressiveFilter excludes low-confidence results
5. RelationshipClusterer groups related chunks
6. SearchResults returned to client

**Error Handling Flow:**
1. Exception caught at any pipeline stage
2. ErrorDiagnostic created with specific error code
3. Suggested actions derived from error type
4. Enriched error returned (not generic RPC_ERROR)

## Key Domain Invariants

1. **Score Monotonicity**: Higher scores always indicate better matches
2. **Confidence Correlation**: Confidence bands align with user-perceived quality
3. **Filter Composition**: Multiple filters combine via AND logic (all must match)
4. **Backward Compatibility**: New metadata fields are always optional
5. **Performance Bound**: Search latency remains sub-100ms p95
6. **Deduplication**: No duplicate (same file/symbol/line) across worktrees
7. **Progressive Quality**: Results returned in descending confidence order

## Domain Concepts Summary

**Query Processing:**
- QueryValidator, QueryNormalizer, QueryUnderstanding
- SearchFilter, SearchModeSelector

**Result Production:**
- SearchEngine, RankingEngine, ConfidenceScorer
- SearchResult, SearchResults

**Result Enhancement:**
- RelatedResults, RelationshipClusterer
- ProgressiveFilter

**Observability:**
- ErrorDiagnostic, QueryUnderstanding
- ConfidenceScore breakdown

**Relationships:**
- Import/Call/Proximity edges
- Clustering algorithms
- Graph traversal

This domain model provides the conceptual foundation for the search improvements initiative, focusing on transparency, control, and quality assessment while maintaining performance and compatibility constraints.
