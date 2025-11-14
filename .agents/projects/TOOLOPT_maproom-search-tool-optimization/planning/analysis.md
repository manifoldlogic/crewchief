# Analysis: Maproom Search Tool Optimization

## Problem Context

The Maproom semantic search tool enables AI agents to search codebases by concept rather than exact text matching. However, initial tool descriptions showed poor agent performance (17.7% success rate on benchmark tasks).

A genetic optimization experiment ran 10+ generations with 5-7 variants each, testing different description styles and structures. Performance improved to 19.6% but plateaued around 19-20%.

## Genetic Optimization Findings

### Performance Progression

| Generation | Best Variant | Score | Improvement |
|-----------|--------------|-------|-------------|
| 0 (baseline) | Control | 17.7% | - |
| 1 | Detailed (Comprehensive) | 19.6% | +19.6% |
| 2 | Amplification Mutation | 19.3% | -0.3% |
| 7 | Amplification Mutation | 19.8% | +0.5% |
| 8 | Specialization Mutation | 19.5% | -0.3% |

**Key Observation**: Initial jump from 17.7% → 19.6% (+1.9 absolute points) was massive. Subsequent generations oscillated ±0.3-0.5% around 19-20%, suggesting a local optimum.

### Winning Pattern Analysis

#### ✅ **What Top Performers Have** (19.0-19.6%)

**1. Transformation Workflow** (THE CRITICAL DIFFERENTIATOR)

```markdown
🤖 AI AGENT QUERY FORMULATION:

TRANSFORMATION PATTERNS:
1. Extract 2-3 core technical terms
2. Remove: how, what, where, when, why, does, is, are, the, a, an
3. Prefer code-like terminology

EXAMPLES:
  "How does authentication work?" → "authentication"
  "What handles errors?" → "error handler"
  "Find auth logic" → "authentication"
```

This section teaches agents **HOW to convert** natural language questions into search queries using:
- **Numbered transformation rules** (3-4 concrete steps)
- **Before→After examples** with visual arrows (→)
- **Actionable, imperative guidance** ("Extract...", "Remove...", "Prefer...")

**2. Complete Query Lifecycle**

Winners provide end-to-end workflow:
- **Transformation**: Input (question) → Query (search terms)
- **Execution**: Search modes (fts/vector/hybrid)
- **Recovery**: Multi-query retry strategy if <3 results
- **Boundaries**: Anti-patterns (what NOT to search for)

**3. Focused Scope**

Winners dedicate 90% of content to teaching THIS tool vs 10% on alternatives. Control variant spent ~40% explaining when to use Grep/Glob instead, diluting the core message.

**4. Visual Anchoring**

🤖 emoji specifically marks "AI AGENT" sections, creating audience targeting. Agents can quickly identify guidance meant for them vs general users.

#### ❌ **What Losers Have** (17.7-18.7%)

**1. Static Examples Without Transformation**

```markdown
EXAMPLES: "authentication flow", "error handling", "database connection"
```

Shows WHAT to search but not HOW to derive these queries from user questions. Agents must infer the transformation logic themselves.

**2. Alternative Tool Over-Documentation**

Control variant (17.7%) included extensive Grep/Glob documentation:

```markdown
✅ USE GREP WHEN:
- You know the exact text to search for
- Searching for literal patterns, comments, or markers
- Finding special characters (emojis, symbols, punctuation)
...5 more bullets

✅ USE GLOB WHEN:
- Finding files by name pattern
- Discovering files in specific directories
...3 more bullets
```

This consumed ~150 tokens explaining OTHER tools, distracting from learning THIS tool.

**3. Missing Systematic Transformation Guidance**

No numbered rules, no transformation patterns, no before→after examples.

**4. Extreme Brevity or Malformation**

- **variant-b-simple** (220 tokens): Underpowered, missing critical transformation guidance (-0.9% vs detailed)
- **Reduction variants** (Gen 7-8): Duplication bugs creating repeated sections (-1.1% vs winners)

### Quantitative Insights

| Metric | Winners (Avg) | Losers (Avg) | Correlation |
|--------|---------------|--------------|-------------|
| Token Count | 472 | 341 | ⚠️ **WEAK** - Overlapping ranges |
| Transformation Examples (with →) | 5 | 1 | ✅ **STRONG** (+1.9%) |
| Numbered Transformation Rules | 3-4 | 0-1 | ✅ **STRONG** |
| Alternative Tool Documentation | 15 tokens | 120 tokens | ✅ **INVERSE** correlation |
| Multi-Query Strategy Present | 100% | 60% | ⚠️ **MODERATE** |

**Key Finding**: Structure and content type matter far more than raw length. A 580-token variant with duplication issues (19.5%) outperforms a clean 329-token variant (18.1%).

### Qualitative Insights

#### Tone & Language

**Winners use IMPERATIVE commands:**
- "Extract 2-3 core technical terms"
- "Remove: how, what, where..."
- "If first query returns <3 results, try variations"

**Losers use DESCRIPTIVE advice:**
- "Best for finding functions/classes"
- "Use concepts: 'auth' not 'authentication_service_implementation_v2'"

**Implication**: Agents respond better to step-by-step procedures than general principles.

#### Example Quality

**High-Quality (Winners):**
```
"How does authentication work?" → "authentication"
```
Shows INPUT type (natural language) and OUTPUT type (search term) with transformation arrow.

**Low-Quality (Losers):**
```
"authentication flow", "error handling"
```
Shows only outputs. Agent must infer how to create these queries.

### Anti-Patterns Confirmed

1. **Static Examples Only**: -1.9% penalty
2. **Alternative Tool Emphasis**: ~-1.5% penalty
3. **Missing Transformation Workflow**: Largest penalty (-1.9%)
4. **Excessive Brevity**: -0.9% penalty (220 tokens too minimal)
5. **Duplication Bugs**: -0.3 to -1.1% penalty (varies by severity)

**Surprising Finding**: Duplication doesn't kill performance if the first 1/3 of the description is intact and high-quality. Gen 7-8 variants had severe structural issues but still scored 19.0-19.5% because early content was correct.

## The Critical Gap: Task-to-Query Mapping

### What Current Winners Teach

1. ✅ Natural language question → Search query transformation
2. ✅ Query retry strategies when results insufficient
3. ✅ Anti-patterns (what NOT to search for)
4. ✅ Search modes (fts/vector/hybrid)

### What Current Winners DON'T Teach

1. ❌ **User task goal → Search strategy** (THE GAP)
2. ❌ Result interpretation (implementation vs test vs config file?)
3. ❌ Iterative refinement beyond simple variations
4. ❌ Context-aware search (when to search vs when info is known)

### Example of The Gap

**User gives agent this task:**
> "Find where git worktrees are created in the crewchief CLI and explain how it works"

**Agent needs to:**
1. **Map task goal → search strategy** ← NOT TAUGHT
2. Transform "where git worktrees are created" → "worktree create" ← TAUGHT
3. Execute search ← TAUGHT
4. **Identify correct file from results** ← NOT TAUGHT
5. **Read and explain implementation** ← NOT TAUGHT

Current variants teach step 2-3 effectively but fail at steps 1, 4-5.

## Existing Solutions (Industry Approaches)

### Tool Description Best Practices

**OpenAI Function Calling Documentation** recommends:
- Clear, concise descriptions
- Parameter explanations
- Example usage
- When to use vs not use

**Anthropic Tool Use Guide** emphasizes:
- Structured output formats
- Edge case handling
- Chain-of-thought guidance

**LangChain Tool Documentation** focuses on:
- Purpose and capabilities
- Input/output schemas
- Usage examples

**Gap**: None of these provide systematic transformation workflows or task-to-query mapping. They assume agents already know how to formulate tool inputs from user requests.

### Our Innovation

The genetic optimization discovered that **teaching transformation workflows** (how to think) beats **showing examples** (what to do). This is a novel insight not present in existing tool description patterns.

## Current State

### Production Tool Description

Current MCP server uses the "Control" variant (17.7% baseline):
- Location: `packages/maproom-mcp/src/tools/search.ts`
- Token count: ~350
- Missing transformation workflow
- Over-emphasizes alternative tools (Grep/Glob)

### Best Performing Variant

**variant-a-detailed** (19.6%):
- Location: `packages/maproom-mcp/test/tool-description-optimization/variants/variant-a-detailed.json`
- Token count: 450
- Has complete transformation workflow
- Focused on THIS tool
- Includes multi-query retry strategy

### Performance Gap

**+1.9 percentage points** available by simply adopting the proven winner.

## Research Findings

### Primary Research: Genetic Optimization Experiment

- **Duration**: 10+ generations over ~4 hours
- **Variants tested**: 53 total (5-7 per generation)
- **Benchmark**: "Find worktree creation implementation" task
- **Method**: Parallel Claude Code agent execution with scoring
- **Key metric**: Success rate (correct file + explanation quality)

### Secondary Research: Pattern Analysis

Systematic comparison of:
- Structural patterns (headings, lists, examples)
- Content patterns (topics covered, level of detail)
- Instructional patterns (imperative vs descriptive)
- Example quality (transformation vs static)
- Language/tone patterns

### Insights from Failed Mutations

**Reduction mutations** (removing content) consistently underperformed:
- Gen 1: 18.1% (-1.5% from parent)
- Gen 7: 17.8% (-1.7% from parent)

**Lesson**: You can't shrink your way to better performance. Agents need sufficient guidance.

**Amplification mutations** (adding content) showed high variance:
- Best: 19.4% (+0.3% from parent)
- Worst: 17.9% (-1.4% from parent)

**Lesson**: Adding MORE content doesn't guarantee improvement. What you add matters.

**Specialization mutations** (focusing content) performed best:
- Gen 7: 19.5% (top performer in gen 8)
- Added specific guidance: "Keep technical nouns and action verbs"

**Lesson**: Precision beats volume. Specific, actionable rules outperform general advice.

## Implications for Implementation

1. **Quick win available**: Simply adopting variant-a-detailed yields +1.9% improvement
2. **Enhancement opportunity**: Adding task-to-query mapping could push beyond 20%
3. **Documentation value**: Learnings should be preserved for future tool description work
4. **Validation critical**: Changes must be tested to ensure production performance matches experimental results

## Constraints

- **No breaking changes**: Tool API must remain unchanged
- **Backward compatibility**: Existing agent integrations must continue working
- **Performance target**: Must maintain or improve 19.6% benchmark score
- **Token budget**: Keep description under 600 tokens for model context efficiency

## Success Metrics

1. **Performance**: ≥19.6% on benchmark task (maintain winner's score)
2. **Production adoption**: Updated tool description deployed to MCP server
3. **Documentation**: Learnings captured in permanent repo documentation
4. **Future-proofing**: Enhancement variant created for next optimization round
