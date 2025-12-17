# Maproom Search Best Practices

## Introduction

This reference provides detailed examples and patterns for effective semantic code search using Maproom. It supplements the core SKILL.md documentation with concrete query transformation examples, task-based search strategies, and common anti-patterns to avoid.

Use this reference to:
- Learn how to transform natural language questions into effective search queries
- Understand how SearchMode auto-detection works
- Apply task-specific search strategies
- Avoid common search mistakes

## Query Transformation Examples

The key to effective search is extracting 2-3 core technical terms from natural language questions. Here are concrete examples:

| Natural Language Query | Transformed Query | SearchMode | Rationale |
|------------------------|-------------------|------------|-----------|
| "How does authentication work in this codebase?" | "authentication" | Code | Single word, likely code identifier/concept |
| "Find the user profile API endpoint" | "user profile api" | Auto | 3 words, mixed concepts (user profile + API) |
| "Explain how to handle database connections" | "database connection" | Auto | 2 words, conceptual pattern |
| "Where is UserAuth::login() implemented?" | "UserAuth::login()" | Code | Code pattern detected (::) indicates method |
| "What are the best practices for error handling?" | "error handling" | Auto | 2 words, conceptual pattern |
| "Show me WebSocket disconnect logic" | "WebSocket disconnect" | Auto | 2 words, feature + action |
| "Find the shopping cart validation function" | "cart validation" | Auto | 2 words, domain + function type |
| "How is the checkout process implemented?" | "checkout process" | Auto | 2 words, business logic concept |
| "Where are HTTP middleware defined?" | "HTTP middleware" | Auto | 2 words, technical concept |
| "Find JWT token generation code" | "JWT token generation" | Auto | 3 words, specific technical operation |
| "Show me rate limiting implementation" | "rate limiting" | Auto | 2 words, feature concept |
| "Where is processPayment called from?" | "processPayment" | Code | camelCase identifier detected |
| "Find error handler middleware" | "error handler" | Auto | 2 words, conceptual pattern |
| "Show me database migration scripts" | "database migration" | Auto | 2 words, infrastructure concept |
| "Where is user_session_store used?" | "user_session_store" | Code | snake_case identifier detected |

### Transformation Principles

1. **Extract core concepts**: Remove "how", "what", "where", "show me", "find the"
2. **Keep it short**: 2-3 words maximum
3. **Use technical terms**: Prefer code-like terminology over descriptions
4. **Preserve identifiers**: Keep camelCase/snake_case names intact
5. **Trust auto-detection**: Don't manually set SearchMode unless needed

## Search Strategy Patterns

Different tasks require different search approaches. Here are proven patterns organized by goal:

### Architecture Exploration

**Goal**: Understand how a system component works

**Strategy**:
1. Start with broad concept search: `"authentication"`
2. Use `mode: "vector"` for conceptual understanding
3. Review top results to identify key files/symbols
4. Use `context` tool on relevant chunks to explore relationships
5. Follow callers/callees to understand data flow

**Example workflow**:
```
1. Search query: "authentication"
2. Find AuthService in results
3. Get context for AuthService chunk (includes callers, callees)
4. Discover login flow, token validation, session management
```

### Debugging

**Goal**: Find the source of an error or unexpected behavior

**Strategy**:
1. Search for error/exception handling: `"error handler"`
2. Use `mode: "hybrid"` to find both identifiers and patterns
3. Search for similar code patterns if error is unclear
4. Check test coverage to understand expected behavior

**Example workflow**:
```
1. Search query: "error handling" or specific error message concept
2. Find error handling middleware and utilities
3. Search for test cases: "error test"
4. Identify where error should be caught
```

### Feature Discovery

**Goal**: Find existing implementations to learn from or reuse

**Strategy**:
1. Search by feature name or concept: `"rate limiting"`
2. Use `mode: "fts"` if you know exact identifier names
3. Use `mode: "vector"` to find similar implementations
4. Use filters to narrow by file type: `filters: {file_type: "ts"}`

**Example workflow**:
```
1. Search query: "rate limiting"
2. Filter to implementation files: filters: {file_type: "ts"}
3. Review multiple implementations
4. Use context to see how they're integrated
```

### Code Navigation

**Goal**: Quickly jump to specific code locations

**Strategy**:
1. Always run `status` first to check embeddings availability
2. Use exact identifier names for FTS: `"processCheckout"`
3. Use `mode: "fts"` for instant identifier lookup
4. Use `open` tool to read specific files
5. Use `context` tool to explore relationships

**Example workflow**:
```
1. Check status (verify embeddings exist)
2. Search query: "processCheckout"
3. Find exact function definition
4. Use context to see callers and dependencies
```

### API Discovery

**Goal**: Find API endpoints, handlers, or routes

**Strategy**:
1. Search for route concepts: `"user routes"` or `"API endpoints"`
2. Combine domain + "api": `"payment api"`
3. Use filters for route files: `filters: {file_type: "ts"}`
4. Follow context to controllers/handlers

**Example workflow**:
```
1. Search query: "user api"
2. Find route definitions
3. Get context to see controller implementations
4. Trace to validation and business logic
```

### Test Coverage Investigation

**Goal**: Find tests for a specific component

**Strategy**:
1. Search for component name + "test": `"AuthService test"`
2. Use file type filter: `filters: {file_type: "test.ts,spec.ts"}`
3. Use vector search to find similar test patterns
4. Check test utilities and fixtures

**Example workflow**:
```
1. Search query: "AuthService test"
2. Filter: filters: {file_type: "test.ts"}
3. Find unit tests, integration tests
4. Review test setup and assertions
```

## SearchMode Detection Patterns

Maproom automatically detects the optimal search mode based on query patterns. Understanding these patterns helps you write effective queries:

### Code Mode Detection

SearchMode is set to **Code** when query contains:
- Single word identifiers: `"processPayment"`, `"UserAuth"`
- camelCase patterns: `"getUserProfile"`, `"handleClick"`
- snake_case patterns: `"user_session_store"`, `"db_connection"`
- Code syntax: `"UserAuth::login()"`, `"user->profile"`, `"auth.validate()"`

**Why Code mode**: These patterns indicate you're looking for specific code identifiers. FTS excels at exact matching.

### Auto Mode Detection

SearchMode is set to **Auto** (hybrid) when query contains:
- 2-3 words without code patterns: `"error handling"`, `"rate limiting"`
- Mixed concepts: `"user profile api"`, `"database connection"`
- Technical phrases: `"JWT token generation"`, `"session management"`

**Why Auto mode**: These are conceptual searches that benefit from both exact matching (FTS) and semantic understanding (vector).

### Text Mode Detection

SearchMode is set to **Text** when query contains:
- Natural language questions: `"how to handle errors"`
- Complete sentences with articles: `"the authentication flow"`
- Documentation-like phrases: `"best practices for validation"`

**Why Text mode**: Long-form queries work better with pure semantic search. However, transforming to 2-3 words (Auto mode) usually works better.

### Override Recommendations

**Rarely needed**: Auto-detection is intelligent. Only override when:
- You want pure vector search for concept exploration: `mode: "vector"`
- You want pure FTS for exact identifier lookup: `mode: "fts"`
- Default mode isn't finding what you need (try other modes)

**Most common override**: `mode: "vector"` for architecture exploration and finding similar implementations.

## Anti-Patterns to Avoid

### 1. Full Sentence Queries

**Problem**: "How do I authenticate users in this application?"

**Why it fails**: Too verbose, dilutes signal with noise words ("How", "do", "I", "in", "this")

**Fix**: Extract core concept → `"authentication"` or `"user authentication"`

### 2. Over-Specific Queries

**Problem**: "UserAuthenticationServiceImplV2Factory"

**Why it fails**: Too specific, might miss related implementations with different names

**Fix**: Start broader (`"authentication"`), then narrow using context and file paths

### 3. Multiple Unrelated Concepts

**Problem**: "authentication database logging middleware"

**Why it fails**: Mixes unrelated concerns, unclear what you're actually looking for

**Fix**: Search each concept separately:
- First: `"authentication"`
- Then: `"logging middleware"`
- Finally: correlate results

### 4. No Status Check Before Search

**Problem**: Using `mode: "vector"` without checking embeddings exist

**Why it fails**: Vector search requires embeddings. If unavailable, search fails or falls back poorly

**Fix**: Always run `status` first to check embeddings availability

### 5. Ignoring SearchMode Signals

**Problem**: Forcing `mode: "fts"` for conceptual queries like "error patterns"

**Why it fails**: FTS only matches exact text, misses semantic variations

**Fix**: Trust auto-detection (Auto mode for concepts, Code mode for identifiers)

### 6. Using Maproom for Exact String Matches

**Problem**: Searching for `"TODO: fix this"` or `"FIXME"` in Maproom

**Why it fails**: Semantic search is overkill for exact strings. Slow and less accurate than grep.

**Fix**: Use Grep tool for exact string matching, comments, TODOs, etc.

### 7. Using Maproom for File Path Patterns

**Problem**: Searching for `"*.test.ts"` or `"src/components/*.tsx"`

**Why it fails**: Maproom searches code content, not file paths

**Fix**: Use Glob tool for file pattern matching

### 8. Too Many Results Without Filtering

**Problem**: Searching broad concepts without filters, getting 100+ results

**Why it fails**: Too much to review, signal lost in noise

**Fix**: Use filters to narrow scope:
- `filters: {file_type: "ts,tsx"}` for TypeScript only
- `filters: {recency_threshold: "7 days"}` for recent changes
- `k: 5` to limit result count

### 9. Not Using Context for Understanding

**Problem**: Reading individual chunks without relationships

**Why it fails**: Misses how code fits together (callers, dependencies, tests)

**Fix**: Use `context` tool to get related chunks:
```
context(chunk_id, expand: {callers: true, callees: true, tests: true})
```

### 10. Searching Without Deduplication

**Problem**: Getting duplicate results across worktrees

**Why it fails**: Same code indexed in multiple branches clutters results

**Fix**: Use `deduplicate: true` (default) to group duplicates, or search specific `worktree`

## Advanced Techniques

### Multi-Query Refinement

If first query returns too few results (<3), try semantic variations:

```
Query 1: "error handling"
  → <3 results?
Query 2: "exception handler"
  → <3 results?
Query 3: "try catch error"
```

### Progressive Filtering

Start broad, narrow progressively:

```
1. "authentication" (no filters)
2. "authentication" + filters: {file_type: "ts"}
3. "authentication" + filters: {file_type: "ts", recency_threshold: "30 days"}
```

### Mode Comparison

When uncertain, compare modes:

```
1. Search "payment processing" mode: "fts"
2. Search "payment processing" mode: "vector"
3. Search "payment processing" mode: "hybrid"
4. Compare results, choose best
```

### Context Expansion Strategy

For deep understanding, progressively expand context:

```
1. Search initial concept
2. Get context with expand: {callers: true, callees: true}
3. For each interesting caller, get its context
4. Build mental map of component relationships
```

### File Type Specialization

Different file types need different strategies:

**TypeScript/JavaScript**: `filters: {file_type: "ts,tsx,js,jsx"}`
**Tests**: `filters: {file_type: "test.ts,spec.ts"}`
**Documentation**: `filters: {file_type: "md,mdx"}`
**Configuration**: `filters: {file_type: "json,yaml,toml"}`
**Rust**: `filters: {file_type: "rs"}`

### Recency-Based Investigation

For bug investigation or recent changes:

```
filters: {recency_threshold: "7 days"}  # Last week
filters: {recency_threshold: "1 month"} # Last month
```

Combine with concept search to find recent implementations of a feature.

## Summary

**Effective Maproom search requires**:
1. Extracting 2-3 core technical terms from questions
2. Trusting SearchMode auto-detection
3. Using task-appropriate strategies (architecture vs debugging vs navigation)
4. Avoiding anti-patterns (full sentences, over-specificity, wrong tool)
5. Leveraging filters, context, and progressive refinement

**Remember**:
- Maproom excels at semantic code search and relationships
- Use Grep for exact string matching
- Use Glob for file path patterns
- Always check status before vector search
- Use context to understand how code fits together
