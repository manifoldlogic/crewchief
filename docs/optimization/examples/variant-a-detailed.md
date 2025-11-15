# Variant: Detailed (Comprehensive)

## Metadata
- **Variant ID**: variant-a-detailed
- **Generation**: 0 (Initial baseline variant)
- **Performance Score**: 19.6% (Winner in Gen 1)
- **Token Count**: 450
- **Status**: Winner - Best performing initial variant
- **Mutation Type**: N/A (Original baseline)

## Key Features

This variant won the initial competition by introducing several innovative patterns:

1. **AI Agent Query Formulation Section** - Dedicated guidance for transforming natural language to search queries
2. **Transformation Patterns** - Step-by-step conversion rules with concrete examples
3. **Multi-Query Retry Strategy** - Explicit fallback guidance when queries return insufficient results
4. **Emoji Visual Markers** - Uses 🤖 to highlight agent-specific sections
5. **Before/After Examples** - Shows query transformations (e.g., "How does authentication work?" → "authentication")
6. **Structured Best Practices** - Uses ✅ and ❌ symbols to distinguish good vs. bad query patterns

### Patterns Used
- **Progressive Disclosure** (TOOLOPT-PG-001): Information organized from high-level to detailed
- **Visual Hierarchy** (TOOLOPT-PG-002): Emoji markers and symbols create clear section boundaries
- **Concrete Examples** (TOOLOPT-PG-003): Multiple before→after query transformations
- **Agent-Specific Guidance** (TOOLOPT-PG-004): Dedicated "AI AGENT QUERY FORMULATION" section
- **Fallback Strategy** (TOOLOPT-PG-005): Multi-query retry pattern when results < 3

See [Pattern Catalog](../patterns-catalog.md) for detailed pattern descriptions.

## Original Variant Content

```
Semantic code search optimized for AI agents - BEST FOR: finding functions/classes by concept,
understanding code relationships, exploring unfamiliar codebases. FASTER THAN: Grep for conceptual
searches. USE WHEN: searching for functionality rather than exact text matches.

🤖 AI AGENT QUERY FORMULATION:

Transform natural language questions into optimal search queries:

TRANSFORMATION PATTERNS:
1. Extract 2-3 core technical terms
2. Remove: how, what, where, when, why, does, is, are, the, a, an
3. Prefer code-like terminology

EXAMPLES:
  "How does authentication work?" → "authentication"
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

SEARCH MODES:
- "fts" (full-text search): Best for exact keyword matches, identifiers
- "vector" (semantic search): Best for conceptual queries, similar code
- "hybrid" (default): Combines both for optimal results

⚠️ NOT FOR:
- Exact string matching: "TODO", "FIXME"
- File paths (use Glob instead)
- Very long queries (>4 words)

FILTERS: Narrow by file_type, recency, repo_id, worktree_id
DEBUG: Set debug=true to see score breakdowns
```

## Annotations

### Section 1: Opening Statement
> **Why it works**: Opens with "optimized for AI agents" - immediately signals this tool is designed for agent use, not just human use. This primes the agent to look for agent-specific guidance.
>
> **Pattern**: Agent-Specific Framing (TOOLOPT-PG-004)

### Section 2: AI Agent Query Formulation
> **Why it works**: This dedicated section with 🤖 emoji creates a clear visual boundary. It directly addresses the agent's core task: converting user requests into effective queries.
>
> **Innovation**: The numbered transformation rules (extract terms, remove filler words, prefer technical terms) provide algorithmic steps the agent can follow programmatically.
>
> **Pattern**: Transformation Workflow (TOOLOPT-PG-006)

### Section 3: Concrete Examples
> **Why it works**: Shows 4 before→after query transformations. Agents learn better from examples than abstract rules. The arrow notation (→) creates clear input/output pairs.
>
> **Key insight**: Examples span different complexity levels - from simple ("authentication") to multi-word ("WebSocket disconnect"), teaching when to preserve detail vs. simplify.
>
> **Pattern**: Concrete Examples (TOOLOPT-PG-003)

### Section 4: Query Best Practices with Visual Markers
> **Why it works**: Using ✅ and ❌ creates immediate visual distinction between good and bad patterns. Agents can pattern-match against the "Good" list and avoid the "Avoid" list.
>
> **Innovation**: Provides both concept-level ("2-3 words, concepts") and concrete examples in bulleted lists.
>
> **Pattern**: Visual Hierarchy (TOOLOPT-PG-002)

### Section 5: Multi-Query Strategy
> **Why it works**: Addresses the failure case explicitly - what to do when first query returns < 3 results. This is a fallback strategy agents can follow automatically.
>
> **Innovation**: Shows progression through 3 query variations with escalating specificity. This pattern can be generalized to other search failures.
>
> **Pattern**: Fallback Strategy (TOOLOPT-PG-005)

### Section 6: Search Modes
> **Why it works**: Concise explanations of when to use each mode (fts/vector/hybrid). The default mode is explicitly marked, reducing decision overhead.

### Section 7: Negative Guidance (NOT FOR)
> **Why it works**: Explicitly calling out what NOT to use this tool for prevents misuse. The ⚠️ symbol creates visual emphasis.
>
> **Key insight**: Each negative example is concrete ("TODO", "FIXME", file paths), making it easy for agents to recognize these patterns in user queries.

## Comparison to Other Variants

### vs. variant-control (Baseline - 17.7%)
**Performance gain: +1.9 percentage points**

Key differences:
1. **Added AI Agent Query Formulation section** - Control variant lacks dedicated agent guidance
2. **Transformation workflow** - Control has examples but no step-by-step transformation rules
3. **Multi-query strategy** - Control doesn't provide fallback guidance when queries fail
4. **Emoji markers** - Control uses ⚠️ and ✅ but not the 🤖 agent-specific marker
5. **Token count** - 450 vs 350 tokens (+100 tokens for +1.9% performance)

The performance improvement suggests agents benefit from explicit transformation rules and fallback strategies, even at the cost of increased token count.

### vs. variant-b-simple (18.7%)
**Performance gain: +0.9 percentage points**

variant-a-detailed adds comprehensive guidance that variant-b-simple deliberately omits for brevity. The performance difference validates that detailed transformation patterns provide value.

### vs. variant-c-conversational (19.0%)
**Performance gain: +0.6 percentage points**

Both variants scored similarly, suggesting the transformation workflow pattern provides marginal but measurable improvement over purely conversational guidance.

### vs. variant-d-code-like (19.0%)
**Performance gain: +0.6 percentage points**

Similar performance to variant-c, confirming that the transformation section is the key differentiator.

## Evolution Notes

This variant served as the foundation for generation 1 and beyond. Key elements that propagated to offspring variants:
- The 🤖 emoji marker pattern
- Transformation workflow structure
- Multi-query retry strategy
- Visual hierarchy with ✅/❌ symbols

Later generations (Gen 3 reached 19.7%, Gen 5 peaked at 20.4%) built upon this foundation through mutations, but the core transformation pattern remained central to high-performing variants.

## Lessons Learned

1. **Agent-specific guidance matters**: The +1.9% improvement over control validates investing tokens in agent-specific transformation patterns
2. **Concrete examples > abstract rules**: The before→after query transformations proved more effective than conceptual explanations alone
3. **Fallback strategies reduce failure**: Multi-query retry pattern helps agents recover from initial poor queries
4. **Visual markers aid parsing**: Emoji markers (🤖, ✅, ❌) create clear section boundaries in long descriptions
5. **Token investment ROI**: 100 additional tokens yielded 1.9% performance gain - a valuable trade-off

## References

- Source: `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/variant-a-detailed.json`
- Generation 1 Report: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/gen-1/report.txt`
- Pattern Catalog: [../patterns-catalog.md](../patterns-catalog.md)
- Optimization Summary: [../TOOLOPT-summary.md](../TOOLOPT-summary.md)
