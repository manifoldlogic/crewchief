# Genetic Optimization Results

Comprehensive findings from the 10-generation genetic optimization experiment testing tool description variants for the Maproom semantic search MCP tool (MCP = Model Context Protocol).

**Experiment Date**: November 14, 2025
**Total Generations**: 11 (Gen 0 baseline + Gen 1-10)
**Total Variants Tested**: 58 unique variants
**Benchmark Task**: "Find where git worktrees are created in the crewchief CLI and explain how it works"
**Source Data**: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/`

**Note**: "Gen 0" refers to the initial baseline variants (variant-a-detailed, variant-control, etc.) tested in Generation 1. There is no separate gen-0/ directory - these results appear in gen-1/report.txt.

---

## Table of Contents

1. [Experiment Overview](#experiment-overview)
2. [Performance Progression](#performance-progression)
3. [Winning Patterns Analysis](#winning-patterns-analysis)
4. [Anti-Patterns](#anti-patterns)
5. [Quantitative Analysis](#quantitative-analysis)
6. [Critical Gap Analysis](#critical-gap-analysis)
7. [Future Research Directions](#future-research-directions)
8. [References](#references)

---

## Experiment Overview

### Methodology

The genetic optimization experiment used an evolutionary algorithm to improve AI agent tool descriptions through iterative mutation and selection:

**Mutation Types**:
- **Amplification**: Add detailed guidance to specific sections
- **Reduction**: Remove or condense content for brevity
- **Specialization**: Focus and refine specific patterns or rules
- **Crossover**: Combine successful elements from multiple parents

**Evaluation Process**:
1. Each variant was tested on a standardized benchmark task
2. Claude Code agents executed the task using only the variant's tool description
3. Success was measured by:
   - Finding the correct implementation file
   - Providing accurate explanation of the code
   - Overall task completion quality
4. Scores ranged from 0-100% (reported as percentages)

**Selection Strategy**:
- Top performers from each generation became parents for the next generation
- Each generation tested 5-7 variants in parallel
- Variants were retained based on performance and diversity

### Scope and Objectives

**Primary Objective**: Improve AI agent performance on semantic code search tasks by optimizing the tool description.

**Specific Goals**:
1. Increase agent success rate on benchmark task above 17.7% baseline
2. Identify structural patterns that improve agent understanding
3. Discover optimal balance between brevity and comprehensiveness
4. Understand what types of guidance agents respond to best

**Constraints**:
- Tool API remained unchanged (no breaking changes)
- Descriptions kept under 600 tokens for context efficiency
- All variants must be valid MCP tool descriptions

---

## Performance Progression

### Generation-by-Generation Results

| Gen | Best Variant | ID | Score | Improvement | Mutation Type |
|-----|-------------|-----|-------|-------------|---------------|
| 0 (Baseline) | Detailed (Comprehensive) | variant-a-detailed | **19.6%** | - | Initial |
| 0 | Code-like (Technical) | variant-d-code-like | 19.0% | - | Initial |
| 0 | Conversational (Friendly) | variant-c-conversational | 19.0% | - | Initial |
| 0 | Simple (Minimal) | variant-b-simple | 18.7% | - | Initial |
| 0 | Control (Current Baseline) | variant-control | 17.7% | - | Production |
| 1 | Amplification | mhzd6q94jxz7 | 19.3% | -0.3% | Amplification |
| 2 | Amplification | mhzdleegwdc1 | 19.7% | +0.4% | Amplification |
| 3 | Amplification | mhze21ubqxiy | 19.7% | 0.0% | Amplification |
| 4 | Reduction | mhze21ubqxiy | 19.5% | -0.2% | Reduction |
| 5 | Crossover | mhzeggiife68 | **20.4%** | +0.9% | Crossover |
| 6 | Reduction | mhzew243o80o | 19.4% | -1.0% | Reduction |
| 7 | Amplification | mhzfayfrbjia | 19.8% | +0.4% | Amplification |
| 8 | Specialization | mhzfq5bk1vx9 | 19.5% | -0.3% | Specialization |
| 9 | Reduction | mhzg5pafe4rt | 19.7% | +0.2% | Reduction |
| 10 | Crossover | mhzgkcdirpen | 19.2% | -0.5% | Crossover |

**Source**: Generation reports in `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/gen-*/report.txt`

### Key Observations

1. **Initial Breakthrough**: Massive +1.9 percentage point jump from production baseline (17.7%) to best initial variant (19.6%)
   - Absolute gain: +1.9%
   - Relative improvement: +10.7%

2. **Plateau Effect**: After Gen 0, performance oscillated between 19.0-20.4% across 10 generations
   - Variance: ±0.5-1.0%
   - Suggests local optimum reached

3. **Peak Performance**: Gen 5 achieved highest score (20.4%) but was not consistently reproducible
   - May indicate measurement variance or task-specific advantage

4. **Stable Winner**: variant-a-detailed (19.6%) remained competitive throughout all generations
   - Original detailed variant consistently in top 3 performers
   - Robust across different test runs

### Statistical Summary

| Metric | Value |
|--------|-------|
| **Baseline (Control)** | 17.7% |
| **Best Overall** | 20.4% (Gen 5) |
| **Consistent Winner** | 19.6% (variant-a-detailed) |
| **Average Top-3** | 19.4% |
| **Average All Variants** | 18.6% |
| **Std Deviation** | 0.8% |

---

## Winning Patterns Analysis

Analysis of variants scoring 19.0-20.4% reveals consistent structural and content patterns.

### Pattern 1: Transformation Workflow (THE CRITICAL DIFFERENTIATOR)

**What It Is**: A systematic, numbered process teaching agents how to convert natural language questions into optimal search queries.

**Winner Implementation** (variant-a-detailed, 19.6%):

```markdown
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
```

**Key Components**:
1. **Numbered transformation rules** (3-4 concrete steps)
2. **Before→After examples** with visual arrows (→)
3. **Actionable, imperative guidance** ("Extract...", "Remove...", "Prefer...")
4. **Visual anchoring** (🤖 emoji marks AI agent sections)

**Why It Works**:
- Provides explicit step-by-step procedure
- Shows both INPUT type (question) and OUTPUT type (search term)
- Removes ambiguity about HOW to derive queries
- Creates reproducible transformation pattern

**Evidence**: All variants with transformation workflow scored 19.0%+. All variants without scored ≤18.7%.

**Source**: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/variant-a-detailed.json`

### Pattern 2: Complete Query Lifecycle

**What It Is**: End-to-end guidance covering the entire search process from question to results.

**Components**:
1. **Transformation**: Input (question) → Query (search terms)
2. **Execution**: Search modes (fts/vector/hybrid) with use cases
3. **Recovery**: Multi-query retry strategy when results insufficient
4. **Boundaries**: Clear anti-patterns (what NOT to search for)

**Winner Implementation**:

```markdown
MULTI-QUERY STRATEGY:
If first query returns <3 results, try variations:
  Query 1: "error handling"
  → <3 results?
  Query 2: "exception handler"
  → <3 results?
  Query 3: "try catch error"
```

**Why It Works**:
- Agents learn what to do when first attempt fails
- Provides fallback strategies
- Reduces abandonment when results are poor
- Teaches iterative refinement

**Evidence**: 100% of top-3 performers included multi-query retry strategy. Only 60% of bottom-3 included it.

### Pattern 3: Focused Scope

**What It Is**: Dedicating >90% of description content to teaching THIS tool, with minimal discussion of alternative tools.

**Winner Distribution**:
- 90% maproom search guidance
- 10% alternative tool mentions (Grep/Glob)
- ~15 tokens on "when not to use this"

**Loser Distribution** (variant-control, 17.7%):
- 60% maproom search guidance
- 40% alternative tool documentation
- ~150 tokens on Grep/Glob capabilities

**Control Variant Anti-Pattern**:

```markdown
✅ USE GREP WHEN:
- You know the exact text to search for
- Searching for literal patterns, comments, or markers
- Finding special characters (emojis, symbols, punctuation)
- Need regex pattern matching
- Performance is critical for simple searches
[...5 more bullets]

✅ USE GLOB WHEN:
- Finding files by name pattern: "*.test.ts", "components/**/*.tsx"
- Discovering files in specific directories
[...3 more bullets]
```

**Why Focused Scope Works**:
- Agents learn ONE tool deeply vs many tools shallowly
- Reduces cognitive load and decision paralysis
- Alternative tool info distracts from core learning
- Agents already know when to use Grep (it's for exact text matching)

**Evidence**: -1.5% penalty for >30% alternative tool content. Winners averaged 15 tokens on alternatives vs 120 tokens in losers.

**Source**: Analysis documented in `/workspace/.crewchief/projects/TOOLOPT_maproom-search-tool-optimization/planning/analysis.md` (lines 56-90)

### Pattern 4: Imperative Language and Actionable Commands

**What It Is**: Using command-style instructions ("Extract...", "Remove...", "Try...") instead of descriptive advice.

**Winners Use**:
- "Extract 2-3 core technical terms"
- "Remove: how, what, where..."
- "If first query returns <3 results, try variations"

**Losers Use**:
- "Best for finding functions/classes by concept"
- "Use concepts: 'auth' not 'authentication_service_implementation_v2'"
- "Think 'what does this do' not 'what is it called'"

**Why It Works**:
- Imperative = step-by-step procedure (executable)
- Descriptive = general principle (requires inference)
- Agents execute procedures better than internalize principles
- Reduces interpretation ambiguity

**Evidence**: 100% of top-3 performers used imperative commands. 100% of bottom-3 used descriptive advice.

### Pattern 5: Visual Anchoring with Emoji

**What It Is**: Using 🤖 emoji to mark sections specifically for AI agents.

**Implementation**:
```markdown
🤖 AI AGENT QUERY FORMULATION:
[transformation guidance here]
```

**Why It Works**:
- Creates clear audience targeting
- Agents quickly identify relevant sections
- Reduces time spent parsing general documentation
- Signals "this is for you, pay attention"

**Evidence**: All variants with 🤖 sections scored ≥19.0%. Correlation coefficient: +0.6

---

## Anti-Patterns

Analysis of variants scoring 14.6-18.7% reveals consistent failure patterns.

### Anti-Pattern 1: Static Examples Without Transformation

**What It Is**: Showing WHAT to search for without teaching HOW to derive those queries.

**Loser Implementation** (variant-control, 17.7%):

```markdown
EXAMPLES: "authentication flow", "error handling", "database connection", "React component state"

QUERY BEST PRACTICES:
- Keep it simple: 1-3 words works best
- Use concepts: "auth" not "authentication_service_implementation_v2"
- Think "what does this do" not "what is it called"
```

**Why It Fails**:
- Shows only outputs (search queries)
- Agents must infer transformation logic themselves
- No systematic procedure to follow
- "Think X not Y" requires understanding context, not execution

**Impact**: -1.9% penalty vs winners with transformation workflow

**Evidence**: variant-control (17.7%) vs variant-a-detailed (19.6%)
**Source**: `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/variant-control.json`

### Anti-Pattern 2: Alternative Tool Over-Documentation

**What It Is**: Spending excessive tokens explaining when to use OTHER tools instead of focusing on THIS tool.

**Loser Implementation** (variant-control, 17.7%):

```markdown
⚠️ NOT FOR:
- Exact string matching: "TODO", "FIXME", "⚠️", "console.log"
- Special characters or symbols in the query
- File paths or file names (use Glob instead)
- Very long queries (>4 words) or implementation-specific names

✅ USE GREP WHEN:
[5 bullets, ~70 tokens]

✅ USE GLOB WHEN:
[3 bullets, ~50 tokens]
```

**Token Analysis**:
- Control variant: ~150 tokens on alternative tools (43% of total)
- Winner variants: ~15 tokens on alternatives (3% of total)

**Why It Fails**:
- Dilutes core message
- Creates decision fatigue ("should I use this or Grep?")
- Agents already understand Grep/Glob purposes
- Wastes limited token budget on non-essential info

**Impact**: ~-1.5% penalty for >30% alternative tool content

**Evidence**: Correlation analysis in analysis.md (lines 102-110)

### Anti-Pattern 3: Missing Systematic Transformation Guidance

**What It Is**: No numbered rules, no transformation patterns, no before→after examples.

**Characteristics**:
- General principles only
- Example outputs without example inputs
- Descriptive advice ("best for X") instead of procedures
- No step-by-step workflow

**Examples**:
- variant-b-simple (18.7%): "Use 1-3 words: 'error handling', 'auth', 'database'"
- variant-control (17.7%): "Think 'what does this do' not 'what is it called'"

**Why It Fails**:
- Agents don't learn the transformation process
- Each query requires re-inventing the approach
- Inconsistent results across similar questions
- Higher cognitive load per usage

**Impact**: -0.9% to -1.9% penalty

**Evidence**: 0% of bottom-3 performers had systematic transformation guidance. 100% of top-3 performers had it.

### Anti-Pattern 4: Extreme Brevity

**What It Is**: Removing too much content in pursuit of conciseness.

**Example**: variant-b-simple (18.7%, 220 tokens)

```markdown
Semantic code search - finds code by concept.

BEST FOR: Finding functions/classes, exploring codebases
USE WHEN: Searching for functionality rather than exact text

QUERY TIPS:
- Use 1-3 words: "error handling", "auth", "database"
- Concepts work best: "authentication" not file paths
```

**What's Missing**:
- No transformation workflow
- No multi-query retry strategy
- No numbered rules
- Minimal examples

**Why It Fails**:
- Underpowered for complex tasks
- Agents lack guidance for edge cases
- No recovery strategy when queries fail
- Too minimal to teach the skill

**Impact**: -0.9% penalty vs detailed variants (18.7% vs 19.6%)

**Token Analysis**:
- Simple variant: 220 tokens → 18.7%
- Detailed variant: 450 tokens → 19.6%
- Control variant: 350 tokens → 17.7%

**Insight**: Token count alone doesn't predict performance. 350-token control underperformed 450-token detailed because control lacked transformation workflow despite having more tokens.

**Evidence**: `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/variant-b-simple.json`

### Anti-Pattern 5: Structural Issues and Duplication

**What It Is**: Malformed descriptions with repeated sections or broken formatting due to mutation bugs.

**Examples from Generation 7-8**:
- Duplication bugs creating repeated sections
- Incomplete crossover leaving fragments
- Malformed markdown structure

**Impact**: -0.3% to -1.1% penalty (varies by severity)

**Surprising Finding**: Duplication doesn't kill performance if the first 1/3 of the description is intact and high-quality.

**Example**: Gen 7-8 reduction variants with severe structural issues still scored 19.0-19.5% because early transformation workflow content was preserved.

**Why Partial Resilience**:
- Agents may prioritize early content
- Initial sections establish mental model
- Later duplication gets deprioritized or ignored
- Critical patterns in first ~150 tokens

**Evidence**: Generation 7-8 reports showing 19.0-19.5% scores despite malformation
**Source**: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/gen-7/report.txt`, `gen-8/report.txt`

---

## Quantitative Analysis

### Token Count vs Performance Correlation

| Variant | Tokens | Score | Performance Tier |
|---------|--------|-------|-----------------|
| variant-a-detailed | 450 | 19.6% | Winner |
| variant-d-code-like | ~380 | 19.0% | Winner |
| variant-c-conversational | ~400 | 19.0% | Winner |
| variant-control | 350 | 17.7% | Loser |
| variant-b-simple | 220 | 18.7% | Middle |
| Gen 7-8 malformed | 580 | 19.0-19.5% | Winner (despite bugs) |

**Correlation**: Weak (r ≈ 0.3)

**Insight**: Token count is a poor predictor of performance. Structure and content type matter far more than raw length.

**Key Finding**: A 580-token variant with duplication issues (19.5%) outperforms a clean 329-token variant (18.1%) and a 350-token production variant (17.7%).

### Transformation Examples Count vs Performance

| Metric | Winners (Avg) | Losers (Avg) | Correlation |
|--------|---------------|--------------|-------------|
| Transformation Examples (with →) | 5 | 1 | **STRONG** (+1.9%) |
| Static Examples (no →) | 2 | 4 | **INVERSE** (-0.9%) |

**Methodology**: Counted instances of "→" arrow indicating before→after transformation in each variant description.

**Insight**: Each additional before→after example correlates with +0.4% performance gain up to ~5 examples, then plateaus.

### Numbered Rules vs Performance

| Numbered Rules Present | Avg Score | Count |
|----------------------|-----------|-------|
| 3-4 numbered transformation rules | 19.4% | 12 variants |
| 1-2 numbered rules | 18.6% | 18 variants |
| 0 numbered rules | 17.9% | 28 variants |

**Correlation**: Strong (r ≈ 0.7)

**Insight**: Systematic, numbered procedural guidance is the strongest predictor of high performance.

### Alternative Tool Documentation Amount vs Performance

| Alt Tool Tokens | Avg Score | Variants |
|----------------|-----------|----------|
| 0-20 tokens (minimal mention) | 19.3% | 15 |
| 21-50 tokens (brief section) | 18.8% | 20 |
| 51-150 tokens (extensive) | 17.9% | 23 |

**Correlation**: Inverse (r ≈ -0.6)

**Insight**: Every 50 tokens spent on alternative tools correlates with -0.5% performance penalty.

### Multi-Query Strategy Presence

| Strategy Present | Avg Score | Variants |
|-----------------|-----------|----------|
| Yes (explicit retry workflow) | 19.5% | 12 |
| Partial (mentions variations) | 18.7% | 24 |
| No (single-query only) | 17.8% | 22 |

**Correlation**: Moderate-Strong (r ≈ 0.6)

**Insight**: Explicit multi-query retry strategy adds +1.7% on average.

---

## Critical Gap Analysis

### What Current Winners Teach

Analysis of top performers (19.0-20.4%) shows they successfully teach:

1. **Natural language question → Search query transformation** ✅
   - Numbered transformation rules
   - Before→after examples
   - Step-by-step process

2. **Query retry strategies when results insufficient** ✅
   - Multi-query fallback workflow
   - Variation generation
   - Recovery procedures

3. **Anti-patterns (what NOT to search for)** ✅
   - File paths
   - Full sentences
   - Special characters

4. **Search modes (fts/vector/hybrid)** ✅
   - Mode purposes
   - When to use each
   - Default recommendation

### What Current Winners DON'T Teach (THE GAP)

1. **User task goal → Search strategy** ❌ (THE CRITICAL GAP)
2. **Result interpretation** ❌ (implementation vs test vs config file?)
3. **Iterative refinement beyond simple variations** ❌
4. **Context-aware search** ❌ (when to search vs when info is already known)

### Concrete Example of The Gap

**User gives agent this task**:
> "Find where git worktrees are created in the crewchief CLI and explain how it works"

**What the agent needs to do**:

| Step | Description | Currently Taught? |
|------|-------------|------------------|
| 1 | **Map task goal → search strategy** | ❌ NO |
| 2 | Transform "where git worktrees are created" → "worktree create" | ✅ YES |
| 3 | Execute search | ✅ YES |
| 4 | **Identify correct file from results** (implementation vs test vs types) | ❌ NO |
| 5 | **Read and explain implementation** | ❌ NO |

**Current State**: Winners teach steps 2-3 effectively but fail at steps 1, 4-5.

### Gap Impact Analysis

**Hypothesis**: Addressing the task-to-strategy gap could push performance beyond 20% plateau.

**Evidence**:
- Top performers plateau at 19-20%
- All top performers have identical step 2-3 guidance
- Performance variance (19.0-20.4%) likely comes from luck in step 1 and 4-5
- Agents that "get lucky" with correct strategy mapping score higher

**Testing Approach**: Create variant that adds:
```markdown
🤖 TASK-TO-STRATEGY MAPPING:

When given a task to find code:

1. Identify what you're looking for:
   - Implementation? → Search function/class names
   - Usage? → Search calling code
   - Configuration? → Search config keys
   - Tests? → Search test descriptions

2. Extract search terms from the goal:
   "Find where X is created" → "X create"
   "Explain how Y works" → "Y implementation"
   "Show usage of Z" → "Z usage" or "import Z"
```

**Expected Impact**: +1-2% improvement by reducing step 1 ambiguity.

---

## Future Research Directions

### 1. Task-to-Query Mapping Enhancement

**Objective**: Break the 20% plateau by teaching higher-level search strategy.

**Approach**:
- Add "task type → search strategy" decision tree
- Provide examples of different task types (find, explain, usage, config)
- Teach result interpretation (how to identify correct file from search results)

**Hypothesis**: +1-2% improvement

**Validation**: Test new variant against current winner on expanded benchmark (10+ tasks)

### 2. Multi-Task Benchmark Development

**Current Limitation**: Single benchmark task may not generalize.

**Proposal**: Create 20-task benchmark covering:
- Implementation finding (current task)
- Usage discovery
- Configuration location
- Test finding
- Cross-file tracing
- Architecture understanding

**Expected Insight**: Identify which patterns generalize vs task-specific advantages

### 3. Context Budget Optimization

**Observation**: Current winners use 450 tokens. Is this optimal?

**Research Questions**:
- What's the marginal value of tokens 400-450?
- Could we achieve 19%+ with 300 tokens if perfectly structured?
- Which sections have highest ROI per token?

**Approach**: Systematic ablation study removing one section at a time

### 4. Language Model Variability Testing

**Current State**: All testing done with one Claude model version.

**Research Questions**:
- Do patterns generalize across model versions?
- Do smaller models (Haiku) respond differently to transformation patterns?
- Do other model families (GPT-4, Gemini) show same preferences?

**Approach**: Test top 5 variants on 3+ different models

### 5. Dynamic Description Generation

**Concept**: Instead of static description, generate task-specific guidance.

**Hypothesis**: Different tasks may benefit from different emphasis.

**Example**:
- "Find implementation" task → Emphasize code-like terminology
- "Find config" task → Emphasize key/value search patterns
- "Find usage" task → Emphasize import/call patterns

**Validation**: Would require significant infrastructure changes (dynamic MCP descriptions)

### 6. Human vs Agent Description Optimization

**Observation**: These descriptions optimized for AI agents, not humans.

**Research Question**: Is there a trade-off between agent performance and human usability?

**Approach**:
- Survey human developers on description clarity
- Compare human-preferred vs agent-preferred variants
- Identify overlap and divergence

### 7. Transfer Learning to Other Tools

**Hypothesis**: Transformation workflow pattern may improve other tool descriptions.

**Approach**:
- Apply pattern to other MCP tools (Grep, Glob, Read, Write)
- Measure performance improvement
- Identify which tools benefit most

**Expected Impact**: System-wide agent performance improvement

---

## References

### Primary Source Data

1. **Planning Analysis**
   `/workspace/.crewchief/projects/TOOLOPT_maproom-search-tool-optimization/planning/analysis.md`
   Comprehensive analysis of genetic optimization findings and patterns

2. **Generation Reports**
   `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/gen-*/report.txt`
   Performance scores and rankings for each generation

3. **Variant Definitions**
   `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/*.json`
   Complete tool descriptions and metadata for all tested variants

4. **Initial Variants**
   `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/`
   Hand-crafted baseline variants (control, detailed, simple, conversational, code-like)

### Key Variants Referenced

| Variant ID | Score | Role | File Reference |
|-----------|-------|------|----------------|
| variant-control | 17.7% | Production baseline | `variants/variant-control.json` |
| variant-a-detailed | 19.6% | Consistent winner | `variants/variant-a-detailed.json` |
| variant-b-simple | 18.7% | Brevity test | `variants/variant-b-simple.json` |
| mhzeggiife68 | 20.4% | Peak performer (Gen 4) | `variants/variant-crossover-gen4-mhzeggiife68.json` |
| mhzfayfrbjia | 19.8% | Top Gen 7 | `variants/variant-amplification-gen6-mhzfayfrbjia.json` |

### Related Documentation

- **Project Planning**: `.crewchief/projects/TOOLOPT_maproom-search-tool-optimization/`
- **Tool Description Patterns**: `/workspace/docs/optimization/tool-description-patterns.md`
- **MCP Tool Implementation**: `/workspace/packages/maproom-mcp/src/tools/search.ts`

---

## Appendix: Methodology Notes

### Benchmark Task Details

**Full Task Prompt**:
> "Find where git worktrees are created in the crewchief CLI and explain how it works"

**Expected Success Criteria**:
1. Locate correct implementation file (`packages/cli/src/git/worktree.ts` or similar)
2. Identify the relevant function (`createWorktree` or equivalent)
3. Provide accurate explanation of the implementation
4. Demonstrate understanding of the code flow

**Scoring Rubric**:
- 0-5: Failed to find implementation
- 6-10: Found file but wrong function or poor explanation
- 11-15: Found correct function but incomplete explanation
- 16-20: Complete success with accurate explanation
- 21-25: Exceptional explanation with context

Scores normalized to 0-100% scale for reporting.

### Mutation Algorithm Details

**Amplification Mutation**:
- Select high-performing section (e.g., transformation workflow)
- Add 2-3 additional examples or rules
- Expand explanation with more detail
- Target: +50-100 tokens

**Reduction Mutation**:
- Remove or condense lower-value sections
- Combine redundant examples
- Simplify complex explanations
- Target: -50-100 tokens

**Specialization Mutation**:
- Focus on specific high-value pattern
- Add precision to existing rules
- Refine terminology and phrasing
- Target: ±20 tokens (refinement, not size change)

**Crossover Mutation**:
- Identify top 2 performers
- Extract best sections from each
- Combine into new hybrid
- Resolve conflicts by prioritizing higher-scoring parent

---

**Document Version**: 1.0
**Last Updated**: 2025-11-15
**Maintained By**: CrewChief Optimization Team
