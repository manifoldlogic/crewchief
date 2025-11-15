# Variant: Control (Current Baseline)

## Metadata
- **Variant ID**: variant-control
- **Generation**: 0 (Initial baseline variant)
- **Performance Score**: 17.7% (Baseline comparison)
- **Token Count**: 350
- **Status**: Control - Production baseline for comparison
- **Mutation Type**: N/A (Original production description)

## Key Features

This variant represents the production tool description before optimization. It establishes the baseline for measuring improvement.

1. **Clear Use Case Framing** - Opens with "BEST FOR" to set context
2. **Comparison Anchors** - "FASTER THAN" helps agents understand tool positioning
3. **Concrete Examples** - Provides search query examples
4. **Search Mode Explanations** - Documents fts/vector/hybrid modes
5. **Negative Guidance** - Clear "NOT FOR" and "USE GREP/GLOB WHEN" sections
6. **Best Practices List** - Simple bullet points for good vs. bad queries

### Patterns Used
- **Concrete Examples** (TOOLOPT-PG-003): Query examples provided
- **Visual Hierarchy** (TOOLOPT-PG-002): Uses ⚠️ and ✅ symbols for emphasis
- **Comparison Positioning** (TOOLOPT-PG-007): "FASTER THAN Grep" and tool comparison sections

See [Pattern Catalog](../patterns-catalog.md) for detailed pattern descriptions.

## Original Variant Content

```
Semantic code search - BEST FOR: finding functions/classes by concept, understanding code relationships,
exploring unfamiliar codebases. FASTER THAN: Grep for conceptual searches. USE WHEN: searching for
functionality rather than exact text matches. EXAMPLES: "authentication flow", "error handling",
"database connection", "React component state". TIP: Start with simple terms (1-3 words), then refine.
Use status tool first to see what's indexed.

⚠️ NOT FOR:
- Exact string matching: "TODO", "FIXME", "⚠️", "console.log"
- Special characters or symbols in the query
- File paths or file names (use Glob instead)
- Very long queries (>4 words) or implementation-specific names

✅ USE GREP WHEN:
- You know the exact text to search for
- Searching for literal patterns, comments, or markers
- Finding special characters (emojis, symbols, punctuation)
- Need regex pattern matching
- Performance is critical for simple searches

✅ USE GLOB WHEN:
- Finding files by name pattern: "*.test.ts", "components/**/*.tsx"
- Discovering files in specific directories
- File extension or path-based searches

SEARCH MODES:
- "fts" (full-text search): Best for exact keyword matches, identifiers, specific terms
- "vector" (semantic search): Best for conceptual queries, finding similar code
- "hybrid" (default): Combines FTS and vector search for best overall results

QUERY BEST PRACTICES:
- Keep it simple: 1-3 words works best
- Use concepts: "auth" not "authentication_service_implementation_v2"
- Think "what does this do" not "what is it called"
- Good: "error handling", "message bus", "state management"
- Avoid: "TODO comments", "find all ⚠️ markers", "src/components/Button.tsx"

FILTERS: Optionally narrow results by file_type, recency, repo_id, or worktree_id
DEBUG: Set debug=true to see score breakdowns and fusion details
```

## Annotations

### Section 1: Opening Statement
> **Strength**: Clear framing with "BEST FOR", "FASTER THAN", "USE WHEN" creates context.
>
> **Limitation**: Doesn't explicitly signal this is optimized for AI agents. Agents might not recognize agent-specific guidance.
>
> **Pattern**: Comparison Positioning (TOOLOPT-PG-007)

### Section 2: Examples and Tips
> **Strength**: Provides 4 concrete query examples ("authentication flow", "error handling", etc.) showing the expected query style.
>
> **Limitation**: Examples are standalone without transformation context. No guidance on HOW to convert user questions into these queries.
>
> **Missing**: No "before → after" transformation examples that show the conversion process.

### Section 3: NOT FOR Section
> **Strength**: Clear negative guidance with ⚠️ symbol and concrete examples of what NOT to search for.
>
> **Effective**: Concrete anti-patterns ("TODO", "FIXME", file paths) are easy for agents to recognize.
>
> **Pattern**: Visual Hierarchy (TOOLOPT-PG-002)

### Section 4: USE GREP/GLOB WHEN
> **Strength**: Explicit guidance on when to use alternative tools. Helps agents choose the right tool.
>
> **Effective**: Clear decision boundaries between maproom search, Grep, and Glob.

### Section 5: Search Modes
> **Strength**: Concise mode explanations with use cases for each (fts/vector/hybrid).
>
> **Clear**: Default mode explicitly marked.

### Section 6: Query Best Practices
> **Strength**: Provides both principles ("Keep it simple: 1-3 words") and concrete examples.
>
> **Limitation**: Examples are mixed with principles. No structured transformation workflow.
>
> **Missing**: No guidance on what to do when queries fail or return insufficient results.

## What's Missing (vs. variant-a-detailed)

### 1. AI Agent Query Formulation Section
**Impact**: -1.2 percentage points estimated

Control lacks the dedicated 🤖 section that provides:
- Step-by-step transformation rules
- Explicit word removal patterns (how, what, where, etc.)
- Algorithmic guidance agents can follow

### 2. Transformation Workflow
**Impact**: -0.4 percentage points estimated

Control shows examples but doesn't explain the transformation process:
- No "before → after" query conversions
- No numbered steps for transforming natural language
- No explicit rules for extracting technical terms

### 3. Multi-Query Retry Strategy
**Impact**: -0.3 percentage points estimated

Control doesn't provide fallback guidance:
- No explicit pattern for when first query fails
- No progression through query variations
- Agents must invent their own retry strategy

### 4. Visual Agent Marker
**Impact**: Minor, but aids recognition

Control lacks the 🤖 emoji that signals agent-specific content. This reduces visual parsing efficiency.

### 5. Token Efficiency Trade-off

variant-control: 350 tokens → 17.7% performance
variant-a-detailed: 450 tokens → 19.6% performance

**ROI**: 100 additional tokens → +1.9 percentage points
**Cost per point**: ~53 tokens per percentage point improvement

## Strengths to Preserve

Despite lower performance, variant-control has several strengths:

1. **Conciseness** - At 350 tokens, it's 22% shorter than variant-a-detailed
2. **Clear tool boundaries** - Strong USE GREP/GLOB WHEN sections prevent tool confusion
3. **Production-tested** - Has been in production, so all guidance is validated
4. **Good examples** - Query examples are concrete and realistic
5. **Clean structure** - Flows logically from use cases → negative guidance → best practices

## Evolution Notes

variant-control served as the baseline for all optimization runs:
- Generation 1: variant-control scored 17.7%, ranking 5th of 5 initial variants
- All mutations and crossovers were compared against this baseline
- The +19.64% improvement metric in Gen 1 report is calculated against this variant's pre-optimization score

Later generations didn't directly evolve from variant-control, but it remained the reference point for measuring progress.

## Lessons Learned

1. **Baseline value**: Even "lower-performing" baselines provide essential comparison data
2. **Token efficiency matters**: variant-control's conciseness makes it suitable for token-constrained scenarios
3. **Production stability**: This variant was stable in production, proving that highest performance isn't always the only criterion
4. **Missing patterns revealed**: Comparison with variant-a-detailed revealed the value of transformation workflows and retry strategies
5. **Strengths to preserve**: Tool boundary guidance (USE GREP/GLOB) should be retained in all variants

## When to Use This Variant

Consider variant-control (or derivatives) when:
- **Token budget is constrained** - At 350 tokens, it's 22% more efficient
- **Production stability matters** - This variant has real-world validation
- **Simplicity is preferred** - Some agents may perform better with less guidance
- **Legacy compatibility** - Existing agents trained on this description

However, for maximum performance, variant-a-detailed's transformation workflow provides measurable improvement.

## References

- Source: `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/variant-control.json`
- Generation 1 Report: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/gen-1/report.txt`
- Current Production: `/workspace/packages/maproom-mcp/src/index.ts` (line 117)
- Pattern Catalog: [../patterns-catalog.md](../patterns-catalog.md)
- Optimization Summary: [../TOOLOPT-summary.md](../TOOLOPT-summary.md)
