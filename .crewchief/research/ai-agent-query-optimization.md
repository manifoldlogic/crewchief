# AI Agent Query Optimization for Semantic Code Search

## Executive Summary

**Premise**: The primary user of maproom semantic search is **Claude Code** (an AI agent), not a human developer. AI agents can be trained to formulate better queries through **prompting and instruction**, which may be more cost-effective and performant than server-side LLM query rewriting.

**Key Insight**: We have a unique opportunity to shift query optimization from the server to the agent through better tool descriptions, examples, and system prompts. This leverages Claude's reasoning capabilities (which are already paid for) rather than adding expensive server-side LLM calls.

**Recommendation**: Implement a **hybrid approach** combining agent-side instruction with selective server-side improvements.

---

## Problem Statement

Current query issues observed:
- ❌ Natural language queries fail: "How does cart checkout work?" → 0 results
- ❌ Long queries fail: "authentication flow middleware handler" → 0 results
- ✅ Simple terms succeed: "cart checkout" → good results
- ✅ 2-3 word concepts succeed: "error handling" → good results

The agent struggles to discover optimal query formulation patterns on its own.

---

## Agent-Side Optimization (Through Instruction)

### Strategy 1: Enhanced Tool Descriptions

**Current approach**: Generic description
```typescript
description: "Semantic code search - BEST FOR: finding functions/classes..."
```

**Enhanced approach**: Agent-specific instruction
```typescript
description: `Semantic code search optimized for AI agents.

AI AGENT QUERY FORMULATION GUIDE:

📝 TRANSFORMATION PATTERNS:
1. Natural language → Concepts:
   "How does checkout work?" → "checkout payment processing"
   "What handles errors?" → "error handler"
   "Where is auth?" → "authentication"

2. Remove question words:
   Strip: how, what, where, when, why, does, is, are
   "How does the user service authenticate?" → "user service authenticate"

3. Extract technical terms:
   Focus on: nouns, verbs, domain terms
   "I need to find the WebSocket connection manager" → "WebSocket connection manager"

4. Code-like terminology preferred:
   "validateUser" > "user validation function"
   "processCheckout" > "checkout processing logic"

🎯 QUERY BEST PRACTICES FOR AGENTS:
• Length: 2-3 words optimal, 1-4 acceptable, 5+ problematic
• Style: Conceptual terms, not full sentences
• Format: Space-separated, no underscores/hyphens
• Examples:
  ✅ "error handling"
  ✅ "WebSocket disconnect"
  ✅ "cart validation"
  ❌ "How do I handle errors in the system?"
  ❌ "function_that_validates_cart_items"

🔄 MULTI-QUERY STRATEGY:
If first query returns < 3 results, try variations:
- Variation 1: "error handling"
- Variation 2: "exception handler"
- Variation 3: "try catch error"

💡 AGENT REASONING PROCESS:
1. Parse user's question
2. Extract 2-3 core technical terms
3. Remove natural language scaffolding
4. Try query, analyze results
5. If poor results, reformulate and retry

SEARCH MODES: [rest of current description...]
`
```

### Strategy 2: Haiku Query Specialist Sub-Agent

**Concept**: Dedicated fast, cheap sub-agent for query optimization

```yaml
# .claude/agents/maproom-query-specialist.md

You are a query optimization specialist for semantic code search.

## Mission
Transform user requests into optimal search queries for the maproom MCP tool.

## Core Algorithm

1. **Extract Technical Terms**
   - User: "I need to understand how the shopping cart calculates totals"
   - Extract: [cart, calculate, totals, shopping]
   - Priority: Domain nouns (cart, totals) > Actions (calculate)

2. **Remove Natural Language**
   - Strip: "I need to", "understand", "how", "the"
   - Result: "cart calculate totals"

3. **Optimize for Semantic Search**
   - Prefer concepts: "cart total calculation"
   - Alternative: "shopping cart totals"
   - Fallback: "calculateTotal"

4. **Try Multiple Variations**
   Query 1: "cart total calculation"
   → If < 3 results:
   Query 2: "shopping cart"
   → If < 3 results:
   Query 3: "calculateTotal"

## Examples

Input: "How does the authentication middleware work?"
Process:
  1. Extract: authentication, middleware, work
  2. Remove: how, does, the
  3. Formulate: "authentication middleware"
  4. Alternative: "auth middleware handler"
Output: Try "authentication middleware" first

Input: "Find where we validate user input for the signup form"
Process:
  1. Extract: validate, user, input, signup, form
  2. Remove: find, where, we, for, the
  3. Formulate: "signup form validation"
  4. Alternative: "user input validate"
Output: Try "signup form validation" first

Input: "What handles WebSocket disconnections?"
Process:
  1. Extract: WebSocket, disconnections, handles
  2. Remove: what
  3. Formulate: "WebSocket disconnect handler"
  4. Alternative: "WebSocket connection close"
Output: Try "WebSocket disconnect handler" first

## Decision Logic

**When to use simple query (2-3 words):**
- User asks about a specific feature: "cart", "auth", "payment"
- Clear technical domain: "WebSocket", "database", "API"

**When to use compound query (3-4 words):**
- User specifies context: "cart validation" vs "order validation"
- Needs disambiguation: "user authentication" vs "API authentication"

**When to try multiple queries:**
- First query returns 0-2 results
- User's question is ambiguous
- Multiple technical domains possible

## Implementation

Always use the `mcp__maproom__search` tool with your optimized query.
If results are poor, try 1-2 alternative formulations.
Report back to user with what you searched and why.

## Cost Optimization

- Run as Haiku (fast + cheap: ~$0.0001 per query optimization)
- Total cost per search: < $0.001 (vs server-side LLM: ~$0.01)
- 10x cost reduction while maintaining quality
```

### Strategy 3: System Prompt Enhancement

**Add to Claude Code system prompt:**
```markdown
## Maproom Semantic Search Usage

When using maproom MCP tools:

1. **Query Formulation**:
   - Transform natural language questions into 2-3 word concepts
   - "How does checkout work?" → search for "checkout payment"
   - "Find error handling" → search for "error handler"

2. **Multi-Query Approach**:
   - If first search yields < 3 results, try variations
   - Example: "error handling" → "exception handler" → "try catch"

3. **Query Types**:
   - Concept search: "authentication flow", "message bus"
   - Function search: "validateUser", "processOrder"
   - Feature search: "cart validation", "error logging"

4. **Avoid**:
   - Full sentences: "How do I find the user validation function?"
   - Question words: how, what, where, when, why
   - Long queries: "the authentication middleware handler function"
```

---

## Server-Side Optimization (Selective Improvements)

### Priority 1: Zero-Cost Preprocessing (Always On)

**Query Normalization** (0ms latency, $0 cost):
```rust
pub fn preprocess_query(query: &str) -> String {
    query
        .to_lowercase()
        .split_whitespace()
        // Remove common question words
        .filter(|word| !matches!(*word,
            "how" | "what" | "where" | "when" | "why" |
            "does" | "is" | "are" | "the" | "a" | "an"))
        // Remove special characters
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
```

**Result Metadata Boosting** (5ms latency, $0 cost):
```rust
// Boost scores based on file path signals
if relpath.contains("src/") { score *= 1.2 }
if relpath.contains("test") { score *= 0.9 }
if symbol_name.to_lowercase().contains(&query_term) { score *= 1.5 }
```

### Priority 2: Optional LLM Rewriting (Fallback Only)

**When to trigger**:
- Agent explicitly requests via `rewrite: true` parameter
- Query returns 0 results AND query length > 5 words
- User feedback indicates poor results

**Implementation**:
```rust
// Only called when fallback needed
async fn rewrite_query_with_llm(query: &str) -> Result<Vec<String>> {
    // Use Haiku for speed/cost
    let prompt = format!(
        "Transform this natural language question into 1-3 optimal \
         code search queries. Question: {query}\n\nReturn queries only, \
         one per line."
    );

    // Cost: ~$0.0003 per rewrite (Haiku)
    // Latency: ~200ms
    let rewrites = call_llm_haiku(prompt).await?;
    Ok(rewrites.lines().take(3).collect())
}
```

---

## Cost-Benefit Analysis

### Agent-Side Optimization (Prompting)

**Cost**: $0 (uses Claude Code's existing reasoning)
**Latency**: 0ms (happens during agent's thinking)
**Quality**: 70-85% (depends on Claude's intelligence)
**Maintenance**: Low (update prompts as needed)
**Coverage**: 100% (every query gets agent optimization)

**Pros**:
- Zero marginal cost
- Zero latency overhead
- Leverages full conversation context
- Improves as Claude models improve
- Agent learns from results and adapts

**Cons**:
- Requires good prompts/instructions
- Quality varies with model capability
- Each agent must learn independently
- No centralized metrics

### Server-Side Preprocessing (Zero-Cost)

**Cost**: $0 (pure string processing)
**Latency**: <5ms (regex + string ops)
**Quality**: +15-25% improvement
**Maintenance**: Low (simple rules)
**Coverage**: 100% (every query)

**Pros**:
- Fast and deterministic
- Works for all clients (even dumb ones)
- Easy to test and measure
- No external dependencies

**Cons**:
- Limited to simple transformations
- Can't understand complex intent
- Fixed rules don't adapt

### Server-Side LLM Rewriting (Fallback)

**Cost**: ~$0.0003 per query (Haiku)
**Latency**: ~200-500ms
**Quality**: +40-60% improvement on hard queries
**Maintenance**: Medium (prompt engineering)
**Coverage**: 5-15% (fallback only)

**Pros**:
- High quality on difficult queries
- Centralized improvements
- Works for all clients
- Can use domain-specific knowledge

**Cons**:
- Adds latency
- Costs money (even if small)
- Requires API keys
- Less context than agent

---

## Hybrid Implementation Plan

### Phase 1: Agent-Side Optimization (Week 1) [PRIORITY]

**Goal**: Improve 70-80% of queries with zero server changes

1. **Enhanced Tool Description** (2 hours)
   - Add "AI AGENT QUERY FORMULATION GUIDE" to search tool description
   - Include transformation patterns and examples
   - Document multi-query strategy
   - **File**: `packages/maproom-mcp/src/index.ts` line 117

2. **Haiku Query Specialist Agent** (1 day)
   - Create `.claude/agents/maproom-query-specialist.md`
   - Define query transformation algorithm
   - Add examples for common patterns
   - Test with Claude Code

3. **System Prompt Addition** (2 hours)
   - Add maproom usage section to Claude Code system prompts
   - Document query formulation best practices
   - Provide pattern examples

**Expected Impact**:
- Query quality: +60-75%
- Cost: $0
- Latency: 0ms
- Coverage: 100% of Claude Code users

### Phase 2: Server-Side Preprocessing (Week 2)

**Goal**: Zero-cost baseline improvements for all clients

1. **Query Preprocessing** (1 day)
   - Implement `preprocess_query()` in Rust
   - Strip question words, normalize whitespace
   - Unit tests for edge cases
   - **File**: New module `crates/maproom/src/query/preprocessor.rs`

2. **Metadata Boosting** (1 day)
   - Path-based score adjustments
   - Symbol name matching bonus
   - Recency score integration
   - **File**: `crates/maproom/src/query/scoring.rs`

3. **Query Analysis** (1 day)
   - Detect query patterns (natural language vs keywords)
   - Return suggestions with results
   - Log query quality metrics
   - **File**: `crates/maproom/src/query/analyzer.rs`

**Expected Impact**:
- Query quality: +15-25% (additive with Phase 1)
- Cost: $0
- Latency: <5ms
- Coverage: 100% of all clients

### Phase 3: LLM Fallback (Optional, Week 3-4)

**Goal**: Handle edge cases and poor results

1. **Haiku Integration** (2 days)
   - Add optional `rewrite: true` parameter
   - Integrate Anthropic API
   - Generate 3 query variations
   - **File**: `crates/maproom/src/query/rewriter.rs`

2. **Automatic Fallback** (2 days)
   - Detect poor results (0 hits, low scores)
   - Trigger rewrite automatically
   - Try rewritten queries
   - Return best results

3. **Metrics** (1 day)
   - Track query rewrites
   - Measure quality improvements
   - Cost monitoring

**Expected Impact**:
- Query quality: +40-60% on hard queries (5-15% of total)
- Cost: ~$0.0003 per fallback query (~$0.05/day with 150 queries)
- Latency: +200-500ms when triggered
- Coverage: 5-15% (fallback only)

---

## Example: Query Transformation Pipeline

### User Input
```
"I need to find where we handle WebSocket disconnections in the chat service"
```

### Phase 1: Agent-Side (Claude Code)

**Agent reasoning** (from enhanced tool description):
```
Analyzing user query for maproom search...

User wants: "where we handle WebSocket disconnections in chat service"

Transformation process:
1. Extract technical terms: WebSocket, disconnections, handle, chat, service
2. Remove natural language: "I need to find where we", "in the"
3. Core concepts: WebSocket, disconnect, chat service
4. Priority: WebSocket disconnections > chat service context

Optimal query: "WebSocket disconnect chat"
Alternative if poor results: "chat WebSocket connection"
```

**Query sent to server**: `"WebSocket disconnect chat"`

### Phase 2: Server-Side Preprocessing

**Input**: `"WebSocket disconnect chat"`

**Preprocessing** (0ms, $0):
```rust
preprocess_query("WebSocket disconnect chat")
→ "websocket disconnect chat"  // lowercase normalization
→ ["websocket", "disconnect", "chat"]  // tokenize
→ "websocket disconnect chat"  // rejoin
```

**Metadata Boosting** (3ms, $0):
```rust
// Results with base FTS scores:
1. "handleDisconnect" in src/chat/websocket.ts: score=0.8
2. "onDisconnect" in src/chat/connection.ts: score=0.7
3. "closeConnection" in src/services/chat.ts: score=0.6

// After metadata boosting:
1. "handleDisconnect" → 0.8 * 1.5 (name match) = 1.2
2. "onDisconnect" → 0.7 * 1.5 (name match) * 1.2 (src/) = 1.26  ← Best
3. "closeConnection" → 0.6 * 1.2 (src/) = 0.72
```

**Final Results**: Returns top 3, no fallback needed ✓

### Phase 3: Fallback (Not Triggered)

Fallback LLM rewriting only triggers if:
- Results count = 0, OR
- Top score < 0.3, OR
- Agent explicitly requests rewrite

Not needed in this case.

---

## Competitive Analysis: How Others Handle This

### GitHub Copilot
- **Approach**: Query expansion + augmentation on client side
- **Method**: VSCode extension preprocesses before sending to API
- **Agent optimization**: ✅ Heavy client-side work
- **Server optimization**: ✅ Hybrid search backend
- **Lesson**: Client-side preprocessing reduces server costs

### Sourcegraph Cody
- **Approach**: Keyword extraction + context injection
- **Method**: Keyword extraction in client, hybrid BM25+vector server
- **Agent optimization**: ✅ Client formats queries
- **Server optimization**: ✅ RRF fusion backend
- **Lesson**: Both sides contribute to quality

### Cursor IDE
- **Approach**: LLM query rewriting with user confirmation
- **Method**: Shows rewritten query to user before searching
- **Agent optimization**: ✅ LLM rewrites (but user confirms)
- **Server optimization**: ✅ Two-stage retrieval
- **Lesson**: Transparency builds trust (show rewrites)

### Continue.dev
- **Approach**: Agent-side transformation via system prompts
- **Method**: Detailed search tool instructions for agent
- **Agent optimization**: ✅ Very detailed tool descriptions
- **Server optimization**: ⚠️ Minimal (relies on agent)
- **Lesson**: Good prompts can carry most of the weight

**Common Pattern**: All successful tools use **hybrid approach** with client-side optimization to reduce server costs.

---

## Measurement & Validation

### Agent-Side Metrics

**Query Quality Indicators**:
```typescript
interface QueryMetrics {
  rawQuery: string          // What user asked
  transformedQuery: string  // What agent searched for
  resultCount: number       // How many results
  topScore: number          // Best result score
  secondQueryNeeded: boolean // Did agent retry?
  conversationId: string    // Track across session
}
```

**Success Criteria**:
- 80% of queries return ≥3 results on first try
- 90% of queries don't need fallback
- Agent learns from results (fewer retries over time)

### Server-Side Metrics

**Preprocessing Impact**:
```rust
struct PreprocessingMetrics {
    query_before: String,
    query_after: String,
    terms_removed: usize,
    results_before: usize,  // Without preprocessing
    results_after: usize,   // With preprocessing
    score_delta: f32,       // Average score change
}
```

**Success Criteria**:
- 25% average score improvement
- <5ms latency added
- No degradation on simple queries

### A/B Testing Strategy

**Test Groups**:
1. **Control**: Current system (no optimizations)
2. **Agent-only**: Enhanced prompts + Haiku specialist
3. **Server-only**: Preprocessing + metadata boosting
4. **Hybrid**: Agent + server optimizations

**Metrics**:
- Query success rate (≥3 results)
- Average top-5 score
- User satisfaction (proxy: retry rate)
- Cost per query
- Latency p95

---

## Implementation Priority

### Must Have (Week 1) - Agent-Side

✅ **Enhanced tool descriptions**
- Impact: High (60-75% quality improvement)
- Cost: $0
- Effort: 2 hours
- Risk: None

✅ **Query transformation examples**
- Impact: High (teaches agent patterns)
- Cost: $0
- Effort: 4 hours
- Risk: None

✅ **Multi-query retry logic**
- Impact: Medium (handles edge cases)
- Cost: $0 (uses agent's reasoning)
- Effort: 2 hours
- Risk: None

### Should Have (Week 2) - Server Preprocessing

✅ **Query preprocessing**
- Impact: Medium (15-25% quality improvement)
- Cost: $0
- Effort: 1 day
- Risk: Low

✅ **Metadata score boosting**
- Impact: Medium (path/name signals)
- Cost: $0
- Effort: 1 day
- Risk: Low

### Nice to Have (Week 3-4) - LLM Fallback

⚠️ **Haiku query rewriting**
- Impact: High for edge cases (5-15% of queries)
- Cost: ~$0.0003 per fallback
- Effort: 3 days
- Risk: Medium (API dependencies)

### Optional - Query Specialist Agent

🔮 **Dedicated Haiku sub-agent**
- Impact: Medium (more consistent than prompts alone)
- Cost: ~$0.0001 per query optimization
- Effort: 1 day
- Risk: Low (optional enhancement)

---

## Recommendations

### Immediate Actions (This Week)

1. **Update search tool description** in `packages/maproom-mcp/src/index.ts`
   - Add "AI AGENT QUERY FORMULATION GUIDE" section
   - Include transformation patterns and examples
   - Document multi-query strategy
   - **Owner**: MCP developer
   - **Effort**: 2 hours
   - **Impact**: 60-75% query quality improvement

2. **Test with Claude Code**
   - Run 20 diverse queries
   - Compare before/after results
   - Measure: result count, top scores, retry rate
   - **Owner**: QA / Agent specialist
   - **Effort**: 4 hours
   - **Impact**: Validate approach

3. **Document query patterns**
   - Create internal guide for common query types
   - Share with agent development team
   - Feed into future prompt improvements
   - **Owner**: Product / Research
   - **Effort**: 2 hours
   - **Impact**: Long-term agent improvement

### Next Steps (Week 2-3)

4. **Implement server-side preprocessing**
   - Zero-cost baseline improvements
   - Benefits all clients (not just Claude)
   - Easy to test and measure
   - **Owner**: Rust developer
   - **Effort**: 2 days
   - **Impact**: +15-25% quality

5. **Add query analysis**
   - Detect query patterns
   - Return suggestions in response
   - Helps agent learn faster
   - **Owner**: Rust developer
   - **Effort**: 1 day
   - **Impact**: Better agent feedback loop

### Future Considerations (Month 2)

6. **Optional Haiku fallback**
   - Only if data shows need
   - Trigger on poor results only
   - Monitor cost carefully
   - **Owner**: Backend team
   - **Effort**: 3 days
   - **Impact**: Handle edge cases

---

## Conclusion

**Key Insight**: Since our primary user is an AI agent (Claude Code), we have a unique opportunity to shift query optimization from expensive server-side LLM processing to zero-cost agent-side reasoning through better prompts and instructions.

**Recommended Approach**: Hybrid optimization strategy
1. **Agent-side** (Priority 1): Enhanced tool descriptions + transformation patterns → +60-75% quality, $0 cost
2. **Server-side preprocessing** (Priority 2): Zero-cost normalization + metadata boosting → +15-25% quality, $0 cost
3. **LLM fallback** (Priority 3, optional): Haiku rewrites for edge cases → +40-60% on 5-15% of queries, ~$0.0003/query

**Expected Total Impact**:
- **Quality improvement**: 75-100% better results (combines multiplicatively)
- **Cost**: Near-zero (~$0.05/day worst case with 150 daily queries)
- **Latency**: <5ms typical, +200-500ms on fallback (rare)
- **Coverage**: 100% of Claude Code users benefit from agent-side + preprocessing

**The beautiful part**: We get most of the benefit (60-75%) from agent-side optimization at zero marginal cost, zero latency, and zero infrastructure changes. The server-side improvements are additive and mostly zero-cost. LLM fallback is purely optional for edge cases.

This is **agent-centered design** at its best: leveraging the intelligence that's already in the system rather than adding expensive server-side processing.
