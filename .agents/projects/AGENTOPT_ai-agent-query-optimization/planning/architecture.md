# Architecture: AI Agent Query Optimization

## Solution Overview

**Approach**: Enhance MCP tool description with agent-specific query transformation guidance

**Key Components**:
1. Enhanced tool description (MCP server)
2. Query transformation patterns (documentation)
3. Multi-query retry strategy (agent behavior)
4. Optional server-side preprocessing (future)

**Design Philosophy**: Leverage agent intelligence rather than adding server infrastructure

## System Architecture

### Current Architecture

```
┌─────────────────┐
│   Claude Code   │
│   (AI Agent)    │
└────────┬────────┘
         │ Generic tool description
         │ No query guidance
         ↓
┌─────────────────┐
│   MCP Server    │
│  (maproom-mcp)  │
└────────┬────────┘
         │ Raw query passthrough
         ↓
┌─────────────────┐
│   PostgreSQL    │
│   FTS + Vector  │
└─────────────────┘
```

**Gaps**:
- Agent doesn't know how to formulate queries
- No preprocessing or optimization
- No feedback loop for learning

### Enhanced Architecture (Phase 1)

```
┌──────────────────────────────┐
│      Claude Code (Agent)     │
│                              │
│  Reads: Enhanced Description │
│  - Transformation patterns   │
│  - Good/bad examples         │
│  - Multi-query strategy      │
│                              │
│  Reasoning:                  │
│  "How does X work?"          │
│  → Extract: X                │
│  → Query: "X" or "X handler" │
└──────────────┬───────────────┘
               │ Optimized query
               ↓
┌──────────────────────────────┐
│        MCP Server            │
│     (No changes needed)      │
└──────────────┬───────────────┘
               │
               ↓
┌──────────────────────────────┐
│        PostgreSQL            │
│      (No changes needed)      │
└──────────────────────────────┘
```

**Benefits**:
- Zero infrastructure changes
- Zero latency added
- Zero cost
- Works immediately

### Full Architecture (Phase 2 + 3, Optional)

```
┌──────────────────────────────┐
│      Claude Code (Agent)     │
│   Enhanced Tool Description  │
└──────────────┬───────────────┘
               │ Better queries
               ↓
┌──────────────────────────────┐
│        MCP Server            │
│                              │
│  Optional Preprocessing:     │
│  - Normalize whitespace      │
│  - Strip question words      │
│  - Lowercase                 │
│                              │
│  Query Analysis:             │
│  - Detect patterns           │
│  - Return suggestions        │
│                              │
│  Optional LLM Fallback:      │
│  - If 0 results              │
│  - Generate 3 variations     │
│  - Retry automatically       │
└──────────────┬───────────────┘
               │
               ↓
┌──────────────────────────────┐
│        PostgreSQL            │
│  Enhanced Scoring:           │
│  - Path-based boosts         │
│  - Name matching bonus       │
│  - Recency signals           │
└──────────────────────────────┘
```

## Component Design

### Component 1: Enhanced Tool Description

**Location**: `packages/maproom-mcp/src/index.ts:117-155`

**Current** (39 lines):
```typescript
{
  name: 'search',
  description: 'Semantic code search - BEST FOR: finding functions/classes...',
  inputSchema: { ... }
}
```

**Enhanced** (~80 lines):
```typescript
{
  name: 'search',
  description: `Semantic code search optimized for AI agents.

🤖 AI AGENT QUERY FORMULATION:

Transform natural language questions into optimal search queries:

TRANSFORMATION PATTERNS:
1. Extract 2-3 core technical terms
2. Remove: how, what, where, when, why, does, is, are, the, a
3. Prefer code-like terminology

Examples:
  "How does checkout work?" → "checkout payment"
  "What handles errors?" → "error handler"
  "Find auth logic" → "authentication"
  "Where is WebSocket disconnect?" → "WebSocket disconnect"

QUERY BEST PRACTICES:
✅ Good: 2-3 words, concepts, code terms
  - "error handling"
  - "cart validation"
  - "WebSocket disconnect"
  - "processCheckout"

❌ Avoid: Full sentences, questions, special chars
  - "How do I handle errors in the checkout?"
  - "function_that_validates_cart_items"
  - "src/cart/checkout.ts"

MULTI-QUERY STRATEGY:
If first query returns <3 results, try variations:
  Query 1: "error handling"
  → <3 results?
  Query 2: "exception handler"
  → <3 results?
  Query 3: "try catch error"

[Rest of current description...]
`,
  inputSchema: { ... }
}
```

**Size Impact**:
- Added tokens: ~400-500
- Total system prompt increase: <1%
- Acceptable for Claude (100K context)

**Implementation**:
```typescript
// Single string literal, no logic changes
const enhancedDescription = `...` // Enhanced text
```

### Component 2: Query Transformation Logic (Agent-Side)

**Location**: Claude Code's reasoning process

**Not implemented in code** - happens via enhanced tool description

**How it works**:
1. Agent reads tool description (every query)
2. Agent applies transformation patterns (reasoning)
3. Agent formulates optimized query
4. Agent tries query, analyzes results
5. Agent optionally retries with variation

**Example reasoning flow**:
```
User: "How does the shopping cart calculate totals?"

Agent reasoning (from enhanced description):
1. Extract technical terms: shopping, cart, calculate, totals
2. Remove natural language: "How does", "the"
3. Core concepts: cart, calculate, totals
4. Formulate: "cart calculate total" or "cart total calculation"
5. Choose: "cart total calculation" (more specific)

Query sent: "cart total calculation"
```

**Cost**: $0 (uses Claude's existing reasoning, already paid for)

### Component 3: Optional Server Preprocessing (Phase 2)

**Location**: New module `crates/maproom/src/query/preprocessor.rs`

**Purpose**: Zero-cost baseline improvements for all MCP clients

**Implementation**:
```rust
pub fn preprocess_query(query: &str) -> String {
    let mut tokens: Vec<&str> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|word| !is_stop_word(word))
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();

    tokens.join(" ")
}

fn is_stop_word(word: &str) -> bool {
    matches!(word,
        "how" | "what" | "where" | "when" | "why" |
        "does" | "is" | "are" | "the" | "a" | "an" |
        "find" | "get" | "show" | "list"
    )
}
```

**Examples**:
```rust
preprocess_query("How does the authentication work?")
→ "authentication work"

preprocess_query("find all error handling")
→ "error handling"

preprocess_query("cart validation")
→ "cart validation"  // No change (already optimal)
```

**Performance**: <1ms per query

### Component 4: Metadata Score Boosting (Phase 2)

**Location**: Modify `crates/maproom/src/query/scoring.rs` (or inline in search)

**Purpose**: Boost results based on code structure signals

**Implementation**:
```rust
fn apply_metadata_boosts(
    score: f32,
    relpath: &str,
    symbol_name: &str,
    query_terms: &[&str],
) -> f32 {
    let mut boosted = score;

    // Path signals
    if relpath.starts_with("src/") {
        boosted *= 1.2;  // Prefer src/ over root
    }
    if relpath.contains("/test") || relpath.ends_with(".test.ts") {
        boosted *= 0.9;  // Slightly de-rank tests
    }

    // Name matching
    let name_lower = symbol_name.to_lowercase();
    for term in query_terms {
        if name_lower.contains(&term.to_lowercase()) {
            boosted *= 1.5;  // Strong signal if name matches query
            break;
        }
    }

    // Recency (already in database, just apply)
    // boosted *= recency_score;

    boosted
}
```

**Impact**: +15-25% average score improvement

### Component 5: Optional LLM Fallback (Phase 3)

**Location**: New module `crates/maproom/src/query/rewriter.rs`

**Purpose**: Handle edge cases when agent + preprocessing fail

**When to trigger**:
- Result count = 0 AND query length > 5 words
- Top score < 0.3 (very low confidence)
- Agent explicitly sets `rewrite: true` parameter

**Implementation**:
```rust
async fn rewrite_query_with_llm(query: &str) -> Result<Vec<String>> {
    let client = AnthropicClient::new()?;

    let prompt = format!(
        "Transform this search query into 1-3 optimal code search terms.\n\
         Query: {query}\n\
         Return only the search terms, one per line, no explanation."
    );

    let response = client.call_haiku(prompt, max_tokens: 100).await?;
    let rewrites = response
        .lines()
        .take(3)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(rewrites)
}
```

**Cost**: ~$0.0003 per rewrite (Haiku, ~60 tokens)
**Latency**: ~200-500ms
**Usage**: Only 5-15% of queries (fallback only)

## Data Flow

### Phase 1: Enhanced Description Only

```
User Question
    ↓
Claude reads enhanced tool description
    ↓
Claude transforms query
    ↓
Optimized query sent to MCP
    ↓
MCP passes to PostgreSQL
    ↓
Results returned
```

**Flow time**: +0ms (agent thinking happens anyway)

### Phase 2: With Server Preprocessing

```
User Question
    ↓
Claude transforms query
    ↓
Query: "error handling"
    ↓
MCP preprocessing:
  - Normalize: "error handling"
  - No stop words to remove
  - Result: "error handling"
    ↓
Metadata boosting:
  - Base FTS score: 0.8
  - Path boost (src/): 0.8 * 1.2 = 0.96
  - Name match: 0.96 * 1.5 = 1.44
    ↓
Results returned (sorted by boosted score)
```

**Flow time**: +3-5ms

### Phase 3: With LLM Fallback

```
User Question: "How does the really complex authentication middleware work?"
    ↓
Claude transforms (best effort):
  "authentication middleware complex"
    ↓
Preprocessing:
  "authentication middleware complex" → "authentication middleware complex"
    ↓
PostgreSQL search
    ↓
Results: 0 hits ❌
    ↓
Fallback trigger (0 results AND query >5 words)
    ↓
LLM rewriting:
  "authentication middleware complex"
  → ["authentication middleware", "auth handler", "login service"]
    ↓
Try variation 1: "authentication middleware"
    ↓
Results: 5 hits ✓
    ↓
Return best results
```

**Flow time**: +200-500ms (only on fallback)

## Interface Design

### Tool Description Schema (MCP)

**Input** (unchanged):
```typescript
{
  repo: string
  query: string
  k?: number
  mode?: 'fts' | 'vector' | 'hybrid'
  filter?: 'all' | 'code' | 'docs' | 'config'
  debug?: boolean
}
```

**Output** (unchanged):
```typescript
{
  hits: Array<{
    chunk_id: string
    relpath: string
    symbol_name: string
    kind: string
    start_line: number
    end_line: number
    score: number
  }>
  debug?: {
    mode: string
    query_terms: string
    total_results: number
  }
}
```

**Optional enhancement** (Phase 2):
```typescript
{
  hits: [...],
  queryAnalysis?: {  // NEW
    termCount: number
    hasQuestionWords: boolean
    recommendation: string
    suggestedReformulation?: string
  }
}
```

### Agent Interaction Pattern

**Pattern 1: Simple Query**
```
User: "Find the error handler"
Agent: [reads tool description]
Agent: Extract "error handler" → already optimal
Agent: search(repo="crewchief", query="error handler")
Result: 8 hits ✓
```

**Pattern 2: Natural Language Query**
```
User: "How does authentication work?"
Agent: [reads tool description]
Agent: Transform "How does authentication work?"
       → Remove "How does", "work"
       → Extract "authentication"
       → Query: "authentication"
Agent: search(repo="crewchief", query="authentication")
Result: 5 hits ✓
```

**Pattern 3: Multi-Query Retry**
```
User: "Find cart validation"
Agent: search(repo="crewchief", query="cart validation")
Result: 2 hits (< 3)
Agent: [tool description says try variations]
Agent: search(repo="crewchief", query="cart validate")
Result: 7 hits ✓
```

## Technology Stack

### Phase 1 (Enhanced Description)

**Language**: TypeScript
**Framework**: None (string literal)
**Dependencies**: None
**Build**: None (part of existing build)

**Changes**:
- 1 file: `packages/maproom-mcp/src/index.ts`
- Lines modified: 117-155 (tool description)
- New dependencies: 0

### Phase 2 (Server Preprocessing)

**Language**: Rust
**Dependencies**: None (uses std library)
**Location**: `crates/maproom/src/query/`

**New modules**:
- `preprocessor.rs`: Query normalization
- `scoring.rs`: Metadata boosting
- `analyzer.rs`: Query pattern detection

**Integration point**: Search handler in MCP server

### Phase 3 (LLM Fallback)

**Language**: Rust
**Dependencies**:
- `reqwest`: HTTP client
- `serde_json`: JSON parsing
- `tokio`: Async runtime (already present)

**Environment**:
- `ANTHROPIC_API_KEY`: Required for Haiku calls
- `MAPROOM_ENABLE_LLM_FALLBACK`: Feature flag (default: false)

## Deployment Strategy

### Phase 1: Zero-Downtime Deployment

**Steps**:
1. Update tool description in code
2. Build MCP server (`pnpm build`)
3. Restart MCP server (or let Claude Code restart on next use)
4. No database changes
5. No infrastructure changes

**Rollback**: Git revert + rebuild (< 5 minutes)

### Phase 2: Additive Deployment

**Steps**:
1. Add new Rust modules (preprocessing, scoring)
2. Integrate into search handler
3. Feature flag: `MAPROOM_ENABLE_PREPROCESSING=true`
4. Deploy with flag off initially
5. A/B test: flag on for 10% of queries
6. Monitor metrics for 48 hours
7. Enable for 100% if successful

**Rollback**: Set feature flag to false (instant)

### Phase 3: Gated Deployment

**Steps**:
1. Add LLM integration (requires API key)
2. Feature flag: `MAPROOM_ENABLE_LLM_FALLBACK=false` (default)
3. Optional opt-in for power users
4. Monitor costs carefully
5. Gradual rollout based on cost analysis

**Rollback**: Remove API key or set flag to false

## Scaling Considerations

### Phase 1: Tool Description

**Scalability**: Perfect
- Static string, no runtime cost
- Works for unlimited queries
- No server resources consumed
- Scales with Claude Code (Anthropic's problem)

### Phase 2: Server Preprocessing

**Scalability**: Excellent
- Pure CPU (string processing)
- <1ms per query
- No database calls
- No network calls
- Can handle 1000+ QPS on single core

**Bottleneck**: None (preprocessing is trivial)

### Phase 3: LLM Fallback

**Scalability**: Limited by cost, not performance

**Cost Analysis**:
- 150 queries/day typical usage
- 10% fallback rate = 15 fallback queries/day
- 15 * $0.0003 = $0.0045/day = $1.35/month per user

**At scale** (100 users):
- $135/month for LLM fallbacks
- Acceptable if users are paying customers
- Not acceptable for free tier

**Mitigation**:
- Cache query rewrites (dedupe similar queries)
- Rate limit fallbacks (max 10/day per user)
- Fallback only for complex queries (>7 words)

## Error Handling

### Phase 1: Tool Description

**Errors**: None possible (static string)

### Phase 2: Server Preprocessing

**Error scenarios**:
1. Empty query → Return empty string (caught by validation)
2. All words are stop words → Return empty (trigger validation error)
3. Special characters → Strip them (safe)

**Handling**:
```rust
fn preprocess_query(query: &str) -> Result<String> {
    let processed = /* ... processing ... */;

    if processed.is_empty() {
        return Err(Error::EmptyQuery(
            "Query contains only stop words or invalid characters"
        ));
    }

    Ok(processed)
}
```

### Phase 3: LLM Fallback

**Error scenarios**:
1. API key missing → Skip fallback, return original results
2. API timeout → Skip fallback, return original results
3. API rate limit → Skip fallback, log warning
4. Invalid response → Skip fallback, return original results

**Handling**:
```rust
async fn try_llm_fallback(query: &str) -> Vec<String> {
    match rewrite_query_with_llm(query).await {
        Ok(rewrites) => rewrites,
        Err(e) => {
            warn!("LLM fallback failed: {}", e);
            vec![query.to_string()]  // Return original
        }
    }
}
```

**Principle**: Fallback failures never break search (graceful degradation)

## Testing Strategy

### Phase 1: Manual Testing

**Test suite**: 20 diverse queries
- 10 natural language questions
- 5 simple 2-3 word queries
- 5 complex multi-word queries

**Metrics**:
- Results count (before/after)
- Top-3 relevance (manual evaluation)
- Agent retry rate

**Pass criteria**:
- Natural language queries: 70% find ≥3 results (vs 10% baseline)
- Simple queries: No degradation (maintain 80% success)
- Overall improvement: +40 percentage points

### Phase 2: Unit + Integration Tests

**Unit tests**:
```rust
#[test]
fn test_preprocess_removes_question_words() {
    assert_eq!(
        preprocess_query("How does authentication work?"),
        "authentication work"
    );
}

#[test]
fn test_preprocess_handles_empty() {
    assert!(preprocess_query("how what where").is_err());
}
```

**Integration tests**:
```rust
#[tokio::test]
async fn test_search_with_preprocessing() {
    let result = search("crewchief", "How does cart checkout work?").await?;
    assert!(result.hits.len() >= 3);
    assert!(result.hits[0].relpath.contains("cart") ||
            result.hits[0].relpath.contains("checkout"));
}
```

### Phase 3: Cost Monitoring

**Metrics to track**:
- Fallback trigger rate
- Cost per fallback
- Total monthly cost
- Fallback success rate (did rewrites help?)

**Alerts**:
- Cost > $10/day → Email alert
- Fallback rate > 20% → Investigate (agent not working?)
- Fallback success < 50% → Review prompts

## Performance Benchmarks

### Target Performance

**Phase 1** (Enhanced description):
- Agent thinking time: <2s (including transformation)
- Server latency: 0ms added
- End-to-end: No change

**Phase 2** (Server preprocessing):
- Preprocessing: <1ms
- Score boosting: <2ms
- Total added: <5ms
- End-to-end: ~50-100ms (vs ~50ms baseline)

**Phase 3** (LLM fallback):
- Haiku call: 200-500ms
- Only on fallback: 5-15% of queries
- Average impact: +10-75ms across all queries

### Acceptable Thresholds

**p50 latency**: <100ms (excellent)
**p95 latency**: <500ms (acceptable)
**p99 latency**: <2s (acceptable for complex queries with fallback)

## Monitoring and Observability

### Metrics to Collect

**Query Quality**:
- `query_success_rate`: % of queries returning ≥3 results
- `natural_language_success_rate`: % of questions finding results
- `average_result_count`: Mean results per query
- `top_score_distribution`: Histogram of top result scores

**Agent Behavior**:
- `multi_query_rate`: % of queries where agent retries
- `average_queries_per_question`: Mean tries before success
- `transformation_detected`: % of queries with agent transformation

**Performance**:
- `preprocessing_latency_ms`: Time spent in preprocessing
- `total_search_latency_ms`: End-to-end search time
- `fallback_trigger_rate`: % of queries using LLM fallback
- `fallback_latency_ms`: Time spent in fallback

**Cost**:
- `llm_calls_count`: Number of Haiku calls
- `llm_cost_usd`: Daily/monthly LLM spend
- `cost_per_successful_query`: ROI metric

### Logging

**Structured logs**:
```rust
info!(
    query_original = %original_query,
    query_preprocessed = %preprocessed,
    results_count = hits.len(),
    top_score = hits.first().map(|h| h.score),
    fallback_used = used_fallback,
    "Search completed"
);
```

### Dashboards

**Query Quality Dashboard**:
- Success rate over time (line chart)
- Results distribution (histogram)
- Top failed queries (table)

**Performance Dashboard**:
- Latency percentiles (line chart)
- Fallback rate (line chart)
- Cost over time (line chart)

## Testing Infrastructure Architecture

### Data-Driven Optimization Framework

**Principle**: Use empirical measurement and competitive testing to discover optimal tool descriptions, not "vibes-based" development.

**Core Components**:
1. Variant generation and mutation system
2. Automated testing harness with agent simulation
3. Statistical analysis framework
4. Production A/B testing infrastructure
5. Continuous improvement pipeline

### Variant Generation System

```
┌─────────────────────────────────────┐
│   Variant Generator                 │
│                                     │
│   Initial: Manual variants (4-5)   │
│   - Detailed patterns (~500 tok)   │
│   - Simple bullets (~200 tok)      │
│   - Conversational (~300 tok)      │
│   - Code-like (~400 tok)           │
│   - Control (current ~350 tok)     │
│                                     │
│   Genetic: Mutations from winners  │
│   - Crossover (combine winners)    │
│   - Amplification (more examples)  │
│   - Reduction (simplify)           │
│   - Reframing (same content)       │
│   - Specialization (query types)   │
└─────────────────────────────────────┘
```

**Variant Structure**:
```typescript
interface Variant {
  id: string                    // "variant-a-detailed"
  name: string                  // "Detailed Patterns"
  description: string           // Full tool description text
  tokens: number                // Token count
  generation: number            // 0 = manual, 1+ = mutation
  parent_ids: string[]          // For genetic tracking
  mutation_type?: string        // "crossover", "amplification", etc.
  created_at: Date
}
```

### Testing Harness Architecture

```
┌────────────────────────────────────────┐
│         Test Query Set (100)           │
│                                        │
│   Natural Language (40):               │
│   - "How does X work?"                 │
│   - "What handles Y?"                  │
│                                        │
│   Simple Queries (30):                 │
│   - "error handling"                   │
│   - "auth middleware"                  │
│                                        │
│   Complex Queries (20):                │
│   - "cart checkout validation"         │
│                                        │
│   Edge Cases (10):                     │
│   - "processPayment" (camelCase)       │
│   - "src/cart.ts" (file paths)        │
└────────────────┬───────────────────────┘
                 │
                 ↓
┌────────────────────────────────────────┐
│      Variant Tester (Parallel)         │
│                                        │
│   For each variant:                    │
│     For each query:                    │
│       1. Simulate agent transformation │
│       2. Execute search                │
│       3. Measure results               │
│       4. Record metrics                │
└────────────────┬───────────────────────┘
                 │
                 ↓
┌────────────────────────────────────────┐
│      Statistical Analyzer              │
│                                        │
│   - Success rate per variant           │
│   - Pairwise t-tests (p<0.05)         │
│   - Confidence intervals               │
│   - Clear winner detection             │
│   - Mutation recommendations           │
└────────────────────────────────────────┘
```

### Agent Simulation Strategies

**Three approaches for simulating agent query transformation**:

**1. API-Based (Most Accurate)**
```typescript
async function simulateAgentTransformation(
  query: string,
  toolDescription: string
): Promise<string> {
  const response = await anthropic.messages.create({
    model: "claude-sonnet-4",
    messages: [{
      role: "user",
      content: `You are using a search tool with this description:

${toolDescription}

User asked: "${query}"

What search query will you use? Return ONLY the transformed query, no explanation.`
    }],
    max_tokens: 50
  })

  return response.content[0].text.trim()
}
```
**Cost**: ~$0.0003 per query → $0.05 per variant (100 queries) → $1.00 per experiment (5 variants + control)

**2. LLM-Based (Cheaper)**
```typescript
// Use Haiku instead of Sonnet
// Cost: ~$0.01 per variant → $0.10 per experiment
```

**3. Rule-Based (Free, Less Accurate)**
```typescript
function ruleBasedTransformation(
  query: string,
  variant: Variant
): string {
  // Extract patterns from variant description
  // Apply heuristics
  // Less accurate but zero cost
}
```

**MVP Recommendation**: Start with rule-based for rapid iteration, validate winners with API-based.

### Test Query Set Design

**Structure**:
```json
{
  "test_queries": [
    {
      "id": "NL-001",
      "category": "natural_language",
      "query": "How does authentication work?",
      "expected_terms": ["authentication", "auth"],
      "min_results": 3,
      "gold_standard_files": ["auth.ts", "middleware/auth.ts"],
      "notes": "Common natural language pattern"
    },
    {
      "id": "SIMPLE-001",
      "category": "simple",
      "query": "error handling",
      "expected_terms": ["error handling"],
      "min_results": 5,
      "gold_standard_files": ["error.ts", "try-catch"],
      "notes": "Already optimal, should not degrade"
    }
  ]
}
```

**Categories**:
- Natural language (40): Test transformation effectiveness
- Simple queries (30): Test for degradation
- Complex multi-word (20): Test nuanced transformations
- Edge cases (10): Test robustness

### Automated Testing Flow

```
Experiment N (Week N)
├─ Generate variants (manual or mutations)
├─ For each variant:
│  ├─ Run 100 test queries
│  ├─ Simulate agent transformation
│  ├─ Execute searches
│  └─ Collect metrics:
│     ├─ Success rate (queries with ≥3 results)
│     ├─ Avg result count
│     ├─ Top-3 relevance (manual spot check)
│     └─ Transformation consistency
├─ Statistical analysis:
│  ├─ Pairwise comparisons (t-tests)
│  ├─ Confidence intervals
│  └─ Winner selection (p<0.05)
├─ Generate mutations from winner
└─ Repeat or deploy
```

**Output**:
```json
{
  "experiment_id": "exp-003",
  "date": "2025-01-20",
  "variants_tested": 5,
  "results": [
    {
      "variant_id": "variant-a-detailed",
      "success_rate": 0.78,
      "ci_95": [0.74, 0.82],
      "natural_language_success": 0.85,
      "simple_query_success": 0.83,
      "avg_result_count": 7.2
    }
  ],
  "winner": "variant-a-detailed",
  "statistical_significance": true,
  "p_value": 0.003,
  "recommendation": "Deploy variant-a-detailed to production A/B test"
}
```

### Production A/B Testing Infrastructure

**Architecture**:
```
┌────────────────────────────────────┐
│        User Request                │
└────────────┬───────────────────────┘
             │
             ↓
┌────────────────────────────────────┐
│     Variant Assigner               │
│                                    │
│  hash(user_id) % 100:              │
│    0-49 → Variant A                │
│   50-99 → Variant B                │
└────────────┬───────────────────────┘
             │
             ↓
┌────────────────────────────────────┐
│   MCP Server (with variant)        │
│                                    │
│  Load tool description for variant │
│  Return to agent                   │
└────────────┬───────────────────────┘
             │
             ↓
┌────────────────────────────────────┐
│     Metrics Collector              │
│                                    │
│  Log: user_id, variant, query,     │
│       result_count, success        │
└────────────────────────────────────┘
```

**Implementation**:
```typescript
class VariantAssigner {
  private variants: Map<string, Variant>

  assign(userId: string): string {
    const hash = hashUserId(userId)
    const bucket = hash % 100

    // 50/50 split initially
    if (bucket < 50) return 'variant-a'
    return 'variant-b'
  }
}

class MetricsCollector {
  async recordQuery(
    userId: string,
    variant: string,
    query: string,
    results: any[]
  ): Promise<void> {
    await db.insert('ab_test_metrics', {
      timestamp: Date.now(),
      user_id: userId,
      variant: variant,
      query_original: query,
      result_count: results.length,
      success: results.length >= 3,
      session_id: getSessionId()
    })
  }
}
```

**Analysis** (at n=1000 per variant):
```sql
SELECT
  variant,
  COUNT(*) as total_queries,
  AVG(CASE WHEN success THEN 1.0 ELSE 0.0 END) as success_rate,
  AVG(result_count) as avg_results
FROM ab_test_metrics
WHERE timestamp > NOW() - INTERVAL '7 days'
GROUP BY variant;
```

**Statistical Test**:
```typescript
function analyzeABTest(variantA: Metrics, variantB: Metrics) {
  const ttest = performTTest(variantA.success_rate, variantB.success_rate)

  return {
    winner: ttest.p_value < 0.05
      ? (variantA.success_rate > variantB.success_rate ? 'A' : 'B')
      : 'no_significant_difference',
    p_value: ttest.p_value,
    confidence: 1 - ttest.p_value,
    effect_size: variantA.success_rate - variantB.success_rate,
    recommendation: ttest.p_value < 0.05
      ? 'Deploy winner to 100% of traffic'
      : 'Continue testing or try new variants'
  }
}
```

### Multi-Armed Bandit (Advanced)

**For continuous optimization**:

```typescript
class ThompsonSampling {
  private variants: Map<string, { alpha: number, beta: number }>

  constructor() {
    this.variants = new Map([
      ['variant-a', { alpha: 1, beta: 1 }],
      ['variant-b', { alpha: 1, beta: 1 }],
      ['variant-c', { alpha: 1, beta: 1 }]
    ])
  }

  selectArm(userId: string): string {
    // Sample from beta distributions
    const samples = Array.from(this.variants.entries())
      .map(([id, params]) => ({
        id,
        sample: betaRandom(params.alpha, params.beta)
      }))

    // Select highest sample (exploration vs exploitation)
    return samples.sort((a, b) => b.sample - a.sample)[0].id
  }

  update(variant: string, success: boolean): void {
    const params = this.variants.get(variant)!
    if (success) {
      params.alpha += 1
    } else {
      params.beta += 1
    }
  }
}
```

**Benefits**:
- Automatically favors better-performing variants
- Reduces traffic to poor variants
- Continuous learning
- Faster convergence than fixed A/B split

### Continuous Improvement Pipeline

**Weekly Automated Process**:
```
Week N:
  1. Collect production metrics
  2. Analyze current winner performance
  3. Generate 3-5 mutations from winner
  4. Test mutations offline (100 queries each)
  5. Select best mutation
  6. Deploy to 10% traffic (A/B test)
  7. Monitor for 1 week

Week N+1:
  8. Analyze A/B results
  9. If mutation wins (p<0.05):
     → Deploy to 100%
     → Archive old variant
  10. Else:
     → Keep current
  11. Repeat
```

**Convergence Detection**:
```typescript
function detectConvergence(history: Experiment[]): boolean {
  const recentImprovements = history
    .slice(-5)
    .map(exp => exp.improvement_over_previous)

  const avgImprovement = mean(recentImprovements)

  // Converged if last 5 experiments show <2% improvement
  return avgImprovement < 0.02
}
```

**When converged**: Switch from weekly testing to monthly, focus on other optimizations.

## Success Criteria

### Phase 1 Acceptance Criteria

- [ ] Tool description updated with transformation guidance
- [ ] 20 test queries show improvement (manual evaluation)
- [ ] Natural language query success: 70% (vs 10% baseline)
- [ ] Simple query success: No degradation (maintain 80%)
- [ ] No increase in average latency
- [ ] Git tagged and deployed

### Phase 2 Acceptance Criteria

- [ ] Query preprocessing function implemented
- [ ] Metadata score boosting integrated
- [ ] Unit tests pass (100% coverage for new code)
- [ ] Integration tests pass
- [ ] Performance: <5ms preprocessing latency (p95)
- [ ] A/B test shows +15% quality improvement
- [ ] Feature flag deployed

### Phase 3 Acceptance Criteria

- [ ] LLM integration implemented
- [ ] Fallback logic working correctly
- [ ] Cost monitoring in place
- [ ] Alerts configured
- [ ] Fallback success rate >50%
- [ ] Monthly cost <$150 at 100 users
- [ ] Feature flag deployed (default: off)

## Future Enhancements

### Possible Improvements

1. **Query caching**: Cache preprocessed queries + results
2. **Personalization**: Learn user's query patterns over time
3. **Context-aware transformation**: Use conversation history
4. **Multi-language support**: Handle queries in different languages
5. **Query suggestions**: Return "Did you mean?" alternatives
6. **Semantic query expansion**: Add related terms automatically
7. **Cross-encoder reranking**: Use model to rerank results

### Not Recommended

1. **Client-side preprocessing**: Violates MCP server abstraction
2. **Custom query language**: Adds complexity for marginal benefit
3. **Per-user models**: Too expensive, poor ROI
4. **Interactive refinement**: Breaks agent flow
