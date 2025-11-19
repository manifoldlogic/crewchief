# Analysis: Semantic Entry Point Ranking

## Problem Definition

### Current State

**Note:** TypeScript MCP search tool (`/packages/maproom-mcp/src/tools/search.ts`) **does not exist**. This is a critical blocker that must be resolved in Phase 0 before semantic enhancements. Rust FTS implementation exists at `/crates/maproom/src/search/fts.rs`.

Maproom's Rust FTS implementation uses PostgreSQL's `ts_rank_cd()` function, which scores documents based on:
- Term frequency (how many times query terms appear)
- Term position in tsvector
- Document length normalization
- Small exact bonus (+0.2 if symbol_name ILIKE '%query%')

This works well for traditional text search but fails for code search because:

1. **Test files mention functions more frequently than implementations**
   - Test: imports function, uses in test name, 5+ assertions, setup/teardown
   - Implementation: function definition appears once, some parameter usage
   - Result: Test scores higher due to term frequency

2. **FTS doesn't understand code semantics**
   - No awareness of "definition vs reference"
   - No understanding of "implementation vs test vs documentation"
   - Treats all text equally regardless of code role

3. **Context tool requires correct entry points**
   - If search returns test chunk_id, context() traverses "tested-by" relationships
   - If search returns implementation chunk_id, context() traverses "calls/called-by/configures" relationships
   - Wrong entry point = wrong relationship graph = AI misunderstands the system

### Real-World Failure Case

**Query:** "validate_provider"

**Current Results (FTS only):**
1. `crates/maproom/tests/provider_test.rs:test_validate_provider` (score: 0.89)
2. `docs/architecture/providers.md` (mentions validate_provider 3 times) (score: 0.76)
3. `crates/maproom/src/config.rs:validate_provider` (actual implementation) (score: 0.45)

**Why This Breaks Workflow:**
- User gets chunk_id for test file
- Calls `context(test_chunk_id)`
- Receives: test setup, test assertions, tested functions
- Missing: actual callers of validate_provider, configuration it validates, error handling paths
- AI incorrectly understands the system from test perspective

## Industry Solutions Analysis

### Traditional Code Search (Sourcegraph, OpenGrok)

**Approach:**
- Regex-based exact matching with ranking by file type
- Manual boosting of certain directories (src/ over test/)
- Popularity signals (edit frequency, git blame)

**Strengths:**
- Fast exact matches
- Good for known-item lookup

**Weaknesses:**
- Not designed for AI consumption
- No concept of "entry point for graph traversal"
- Limited semantic understanding

### IDE-Based Code Intelligence (LSP, IntelliJ)

**Approach:**
- Symbol table for definitions
- AST-based navigation (find references, go to definition)
- Type-aware searching

**Strengths:**
- Perfect for developer navigation
- Understands code structure deeply

**Weaknesses:**
- Single-language focused
- No cross-file conceptual search
- Not optimized for batch queries (AI agent use case)

### Semantic Code Search (Sourcegraph Code Search, GitHub Copilot)

**Approach:**
- Vector embeddings for similarity search
- AST parsing for structural search
- Hybrid ranking (lexical + semantic)

**Strengths:**
- Conceptual understanding
- Cross-language support

**Weaknesses:**
- Expensive (embedding generation)
- Slower than text search
- Often treats all code equally (no entry point bias)

### Grep/Ripgrep

**Approach:**
- Exact text matching optimized for speed
- No ranking beyond match quality

**Strengths:**
- Fastest possible exact matches
- Universal applicability

**Weaknesses:**
- Returns file:line:text (not chunk_id for graph)
- No understanding of code role
- Requires manual filtering (implementation vs test)

## Maproom's Current State

### Architecture

**FTS Implementation** (`packages/maproom-mcp/src/index.ts:551-617`):
```typescript
// Current query construction
const query = tokens.map(t => `${t}:*`).join(' & ');

// Current scoring
SELECT
  c.id,
  c.symbol_name,
  c.kind,
  ts_rank_cd(c.ts_doc, to_tsquery('simple', $query)) AS fts_score
FROM maproom.chunks c
WHERE c.ts_doc @@ to_tsquery('simple', $query)
ORDER BY fts_score DESC
```

**Boosting Strategy** (Currently):
- Heading 1-2: 2.0x boost (via tsvector weights during indexing)
- Heading 3: 1.5x boost
- JSON keys: 1.3x boost
- No code-specific boosting

**Metadata Available** (Not Currently Used for Ranking):
- `symbol_name`: Extracted function/class/method names
- `kind`: Enum (function, class, method, test, doc, config, etc.)
- `relpath`: File path (could infer test/ vs src/)
- Graph edges: Caller/callee relationships (in relationships table)

### Gap Analysis

**What We Have But Don't Use:**
1. Chunk `kind` field with semantic labels (function, test, doc)
2. Symbol `symbol_name` field for exact matching
3. Relationship graph data (could boost "high centrality" chunks)
4. File path patterns (src/ vs test/ vs docs/)

**Why FTS Fails:**
- Pure frequency-based ranking rewards verbose code (tests, docs)
- No leverage of semantic metadata
- No concept of "entry point quality"

**What Needs to Change:**
- Add kind-based multiplier to scoring
- Add symbol_name exact match detection
- Preserve FTS base score (still valuable signal)
- Combine via multiplication: `final = fts × kind × exact`

## Research Findings

### Code Search Ranking Papers

**"Learning to Rank Code Examples" (Li et al., 2019)**
- Insight: Implementation code has higher value as examples than tests
- Approach: Boost by file path heuristics (src/ > test/)
- Limitation: Doesn't use AST metadata

**"Structural Search and Replace" (JetBrains Research)**
- Insight: Definitions are more valuable entry points than usages
- Approach: AST-based pattern matching distinguishes roles
- Limitation: Single-language, requires full parse

**"Hybrid Search in Code Repositories" (GitHub Research, 2023)**
- Insight: Combining lexical + semantic + structural signals beats any single approach
- Approach: RRF fusion of FTS, vector, and graph signals
- Relevance: Maproom has **operational** RRF fusion (`/crates/maproom/src/search/fusion/rrf.rs` with 18 tests)
- **Current Impact**: SEMRANK improves hybrid search quality immediately, not in future

### Key Insights for Maproom

1. **Entry Point Bias is Valuable**
   - Research confirms: implementations > tests > docs for code understanding
   - Maproom already extracts this (kind field)
   - Industry tools use heuristics (file paths); we have ground truth (AST labels)

2. **Exact Match Still Matters**
   - Even with semantic search, exact symbol matches should rank high
   - Users often know the name and want quick navigation
   - Hybrid mode benefits from strong lexical signal

3. **Don't Abandon FTS**
   - FTS provides fast, reliable baseline
   - Semantic search has latency and cold-start issues
   - Hybrid approaches always outperform single-signal
   - **Maproom's hybrid search is operational now**: Improving FTS strengthens lexical component of active RRF fusion

4. **Metadata is Free Signal**
   - We already extract kind, symbol_name during indexing
   - Zero additional cost to use in ranking
   - High impact on result quality

## Problem Space Summary

**Core Issue:** FTS treats all text equally, but code has inherent structure that matters for entry point quality.

**Root Cause:** PostgreSQL ts_rank_cd() optimized for prose, not code. Tests mention functions frequently (good for prose), but implementations define systems (good for code exploration).

**Impact:** Search → context → graph workflow breaks because wrong entry point leads to wrong relationship traversal.

**Solution Space:** Leverage semantic metadata (kind, symbol_name) already extracted during indexing to boost entry point quality without abandoning FTS speed/reliability.

**Strategic Positioning:** Not competing with grep (speed), competing on correctness of entry points for graph-based code understanding (uniquely maproom).
