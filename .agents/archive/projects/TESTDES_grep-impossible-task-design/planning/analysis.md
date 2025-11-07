# Analysis: Grep-Impossible Task Design & Test Methodology

## Executive Summary

The recent genetic optimization revealed a critical insight: we successfully optimized tool descriptions, but agents never used the tool. This wasn't a failure—it was a measurement of **optimization target misalignment**. The fundamental question is: "How do we design tasks that prove semantic search is genuinely useful without coercing agents to use it?"

This analysis explores test design methodologies from software testing, ML evaluation, and information retrieval research to build a rigorous framework for proving real-world utility.

## Problem Space Analysis

### The Current Misalignment

**What We Measured**: Tool description quality via genetic optimization
**What Actually Happened**: Agents chose Grep/Glob for all tasks (0 maproom searches across 40+ sessions)
**Root Cause**: Task design allowed—and even encouraged—simpler tools

The scoring formula (`searchQuality * 0.4 + taskCompletion * 0.4 + efficiency * 0.2`) assumes agents will use the search tool. When they don't, searchQuality=0, efficiency=0, resulting in ~20% scores even when task completion was 50%.

### Why Agents Chose Grep

Examining the task: "Find the code that creates git worktrees in the crewchief CLI"

**Grep/Glob Advantages**:
- Familiar pattern: "find code" → file search
- Direct path: `**/*worktree*.ts` immediately finds candidates
- Visible success: See file list, open files, explain code
- Low cognitive load: String matching is straightforward

**Semantic Search Barriers**:
- Requires conceptual thinking: "What query captures 'worktree creation'?"
- Uncertainty: "Will this even work better than Grep?"
- Unfamiliar territory: Most developers think file-first
- No obvious win: Task is solvable with simpler tools

**Key Insight**: The agent made the CORRECT choice. For this task, Grep WAS the better tool.

### Research Into Similar Problems

#### 1. Information Retrieval Evaluation (TREC, CLEF)

The Text REtrieval Conference (TREC) has decades of research on evaluating search systems. Key insights:

**Query Difficulty Classification**:
- **Easy queries**: Can be solved with keyword matching (e.g., "find documents about 'climate change'")
- **Hard queries**: Require understanding context, relationships, ambiguity (e.g., "what are the economic implications of climate policy?")

**Relevance Judgments**:
- Binary: relevant/not relevant
- Graded: highly relevant, relevant, somewhat relevant, not relevant
- User-centered: did this help the user accomplish their goal?

**Applied to Our Problem**:
- Current task is "easy": keyword "worktree" suffices
- Need "hard" tasks: require understanding relationships, indirect connections, conceptual similarities

#### 2. Machine Learning Test Set Construction

ML research emphasizes **adversarial test sets** that expose model weaknesses.

**Adversarial NLI (ANLI)**:
- Start with simple examples
- Humans create examples that fool the model
- Train on failures, repeat
- Result: Increasingly sophisticated test set

**Checklist Testing (Ribeiro et al. 2020)**:
- Test specific capabilities independently
- Behavioral tests, not just accuracy
- Example: "Does model understand negation?" "Does it handle rare words?"

**Applied to Our Problem**:
- Create tasks where Grep provably fails
- Test specific semantic search capabilities
- Behavioral validation: "Does agent choose the right tool for the right task?"

#### 3. Software Testing Methodologies

**Property-Based Testing (QuickCheck)**:
- Define properties that must hold
- Generate test cases automatically
- Example: "For any valid query, search results should be ranked by relevance"

**Mutation Testing**:
- Intentionally break code
- Verify tests catch the breakage
- If tests pass with broken code, tests are inadequate

**Applied to Our Problem**:
- Define properties of good search tasks
- Generate task variants automatically
- Mutation: If we remove semantic search, do scores drop significantly?

#### 4. User Study Design (HCI Research)

**Within-Subjects vs Between-Subjects**:
- Within: Same agent performs both conditions (Grep task vs Search task)
- Between: Different agents for each condition
- Trade-offs: learning effects vs individual variance

**Ecological Validity**:
- Lab tasks vs real-world tasks
- "Would developers actually do this in practice?"
- External validity: do results generalize?

**Applied to Our Problem**:
- Tasks should reflect real developer workflows
- Avoid artificial "test the tool" scenarios
- Measure: time saved, quality improvement, confidence

## Current Landscape: Semantic Code Search

### Existing Solutions

**1. GitHub Code Search**
- Uses Elasticsearch + custom ranking
- Supports regex, exact match, symbol search
- Primarily keyword-based with some ML ranking

**2. Sourcegraph**
- Combines literal search with structural patterns
- Symbol navigation, find references
- Growing semantic capabilities via Cody AI

**3. grep.app, Searchcode.com**
- Web-based grep across open source
- Fast literal matching
- No semantic understanding

**4. Research Systems**
- Microsoft CodeQL: structural queries over code graphs
- DeepCode (Snyk): ML-based pattern detection
- Chronicler: retrieval augmented code navigation

### What's Missing: The Semantic Gap

**Gap 1: Conceptual Queries**
- "How does authentication work?" → No existing tool handles this well
- Grep returns 100+ files with "auth" in them
- Developer must synthesize understanding from fragments

**Gap 2: Relationship Discovery**
- "What depends on this function without importing it directly?"
- Requires call graph + transitive dependencies
- Not a simple string match

**Gap 3: Similarity Without Exact Match**
- "Find similar error handling patterns"
- Requires understanding what makes handlers "similar"
- Semantic embedding similarity, not string matching

**Gap 4: Ambiguity Resolution**
- "Where is rate limiting implemented?"
- Could be middleware, decorators, manual checks, external service
- Need context to find all implementations

### Proving Real-World Value

To prove semantic search is useful, we must demonstrate:

1. **Time Savings**: Tasks take less time with semantic search
2. **Quality Improvement**: Better/more complete results
3. **Confidence**: Developer trusts results more
4. **Generalization**: Works across different codebases/domains
5. **Natural Usage**: Developers choose it voluntarily when appropriate

## Prior Art in Tool Evaluation

### Developer Tool Studies

**1. Code Completion (Tabnine, Copilot)**
- Metric: Acceptance rate of suggestions
- Benchmark: Time to write code
- Finding: 30-40% time savings for boilerplate, minimal for complex logic

**2. Static Analysis (ESLint, SonarQube)**
- Metric: Bugs caught, false positive rate
- Benchmark: Manual code review
- Finding: High false positives reduce adoption despite catching real bugs

**3. Refactoring Tools (IntelliJ, ReSharper)**
- Metric: Usage frequency, task completion rate
- Benchmark: Manual refactoring time
- Finding: Used when *vastly* faster than manual (e.g., rename), ignored when marginal

**Key Lesson**: Tools must be **10x better** in some dimension to overcome switching costs. Marginal improvements aren't enough.

### Information Retrieval Benchmarks

**TREC Tasks**:
- Ad-hoc retrieval: "find documents about X"
- Question answering: "what is X?"
- Interactive: user refines query based on results

**Metrics**:
- Precision@K: How many of top K results are relevant?
- Recall: Did we find all relevant documents?
- Mean Reciprocal Rank: How quickly do we show the right answer?
- nDCG: Normalized discounted cumulative gain (graded relevance)

**Applied to Code Search**:
- Precision@3: Are top 3 results useful?
- Success@K: Is the right file/function in top K?
- Time-to-answer: How long until developer finds what they need?

## The Fundamental Testing Challenge

### Why "Grep-Impossible" Matters

If a task is grep-solvable, agents will use Grep. This is rational behavior. To prove semantic search value, we need tasks where:

1. **Grep Fails Objectively**: Produces too many results, wrong results, or no results
2. **Semantic Search Succeeds**: Finds the right answer efficiently
3. **Task is Real**: Developers actually encounter this problem
4. **Victory is Measurable**: Clear success criteria

### Taxonomy of Grep Failures

**Type 1: Too Many Results**
- Query: "error" in large codebase → 1000+ files
- Grep succeeds technically but fails practically
- Semantic search: rank by relevance, filter noise

**Type 2: No Obvious Keywords**
- Task: "Find circular dependency detection code"
- Implementation might use graph traversal terms, not "circular" or "dependency"
- Grep: requires guessing keywords
- Semantic search: understands concept regardless of exact wording

**Type 3: Relationship Queries**
- Task: "What calls this function indirectly?"
- Grep: can find direct calls
- Grep fails: can't follow call chains
- Semantic search: leverages code graph

**Type 4: Ambiguity Resolution**
- Task: "Where are database transactions managed?"
- Could be ORM methods, decorators, context managers, manual SQL
- Grep: finds all mentions of "transaction"
- Semantic search: understands different implementation patterns

**Type 5: Cross-Cutting Concerns**
- Task: "Find all error handling in async operations"
- Scattered across codebase, various patterns
- Grep: can't distinguish async error handling from sync
- Semantic search: understands async context

### Task Design Principles

Based on research and current findings:

**Principle 1: Specificity Without Keywords**
- Bad: "Find error handling" (keyword: "error")
- Good: "Find code that handles unexpected responses from external APIs"
- Why: No single keyword captures this, requires understanding

**Principle 2: Relationship Discovery**
- Bad: "Find the auth module" (direct file search)
- Good: "What code depends on the auth module without importing it directly?"
- Why: Grep can't follow transitive dependencies

**Principle 3: Conceptual Similarity**
- Bad: "Find the retry logic in network.ts"
- Good: "Find all retry implementations across the codebase"
- Why: Different files might use different terms/patterns

**Principle 4: Context-Dependent Interpretation**
- Bad: "Find worker threads"
- Good: "Find background processing that runs independently of HTTP requests"
- Why: Implementation might not use "worker" or "thread"

**Principle 5: Negative Space**
- Bad: "Find database queries"
- Good: "Find code that modifies state without database persistence"
- Why: Searching for absence/violation requires understanding

## Research Questions

To validate this framework, we need to answer:

### RQ1: Tool Selection Behavior
**Question**: Under what conditions do agents voluntarily choose semantic search over Grep/Glob?

**Hypothesis**: Agents choose semantic search when:
- Task description emphasizes conceptual understanding
- Initial Grep attempt yields too many/few results
- Query involves relationships, not file location

**Measurement**:
- Tool usage logs
- Sequence of tool calls
- Query success rate per tool

### RQ2: Task Difficulty Calibration
**Question**: Can we reliably create tasks that are Grep-hard but Search-easy?

**Hypothesis**: Tasks requiring conceptual understanding or relationship discovery are systematically harder for Grep.

**Measurement**:
- Success rate: Grep-only vs Search-available
- Time to completion
- Result quality (precision, completeness)

### RQ3: Real-World Validity
**Question**: Do our designed tasks reflect actual developer workflows?

**Hypothesis**: Tasks based on code review, debugging, and refactoring scenarios have high ecological validity.

**Measurement**:
- Developer survey: "Would you actually do this?"
- Frequency in real projects
- Comparison to IDE usage patterns

### RQ4: Generalization
**Question**: Do results generalize across codebases and domains?

**Hypothesis**: Task patterns (relationship discovery, conceptual similarity) work across different projects.

**Measurement**:
- Cross-project validation
- Domain-specific vs domain-general tasks
- Codebase size/complexity effects

### RQ5: Value Proposition
**Question**: What specific benefits does semantic search provide over baseline tools?

**Hypothesis**:
- Time savings for complex queries (30-50% reduction)
- Higher precision@3 (70% vs 40% for Grep)
- Lower cognitive load (fewer dead ends)

**Measurement**:
- Time-to-success comparison
- Precision/recall metrics
- Agent reasoning complexity (tool call count, query refinements)

## Gaps in Current Approach

### Gap 1: Single Task Type
**Problem**: Only testing "find implementation" tasks
**Impact**: Limited understanding of where semantic search excels
**Solution**: Create task taxonomy covering different categories

### Gap 2: No Baseline Comparison
**Problem**: No explicit Grep-only control condition
**Impact**: Can't attribute success to semantic search vs agent capability
**Solution**: A/B test with/without search tool available

### Gap 3: Subjective Scoring
**Problem**: "Explanation quality" is fuzzy, hard to validate
**Impact**: Variance in scores, unclear success criteria
**Solution**: Objective metrics (correct file found, specific function mentioned)

### Gap 4: No Ecological Validation
**Problem**: Synthetic tasks may not reflect real usage
**Impact**: Results might not generalize to production
**Solution**: Base tasks on real issues, code reviews, Stack Overflow questions

### Gap 5: Missing Failure Analysis
**Problem**: When tasks fail, we don't know why
**Impact**: Can't improve task design systematically
**Solution**: Detailed failure mode analysis, categorization

## Synthesis: Path Forward

### Key Insight from Analysis

The genetic optimization worked perfectly—we optimized what we measured. The problem was we measured the wrong thing. Instead of "how good is the tool description?", we should measure "does the tool provide unique value?"

### Three-Tier Validation Framework

**Tier 1: Grep-Impossible Tasks** (Prove Capability)
- Tasks where Grep objectively fails
- Semantic search must succeed
- Demonstrates technical superiority

**Tier 2: Grep-Hard Tasks** (Prove Efficiency)
- Tasks where Grep works but is inefficient
- Semantic search provides 30-50% time savings
- Demonstrates practical value

**Tier 3: Real-World Tasks** (Prove Utility)
- Tasks from actual development workflows
- Natural tool selection (no coercion)
- Demonstrates adoption viability

### Success Criteria

A semantic search tool is proven useful when:

1. **Tier 1 Pass Rate > 80%**: Successfully solves grep-impossible tasks
2. **Tier 2 Time Savings > 30%**: Demonstrable efficiency gain
3. **Tier 3 Voluntary Adoption > 40%**: Agents choose it when appropriate
4. **Cross-Project Generalization**: Works on unfamiliar codebases
5. **Low False Positive Rate < 20%**: Doesn't mislead developers

### Implementation Strategy

1. **Build Task Taxonomy**: Categorize by Grep failure mode
2. **Create Reference Implementation**: Grep-only baseline for each task
3. **Design Objective Metrics**: Clear pass/fail, no subjective judgment
4. **Validate Ecologically**: Survey developers, analyze real workflows
5. **Iterate Systematically**: Failure analysis → task improvement → retest

## Conclusion

The learning from the genetic optimization experiment is profound: **optimize for the right outcome, not just improved metrics**. Tool descriptions got better, but agents didn't use the tool because the tasks didn't require it.

The path forward is clear: design tasks that prove semantic search provides unique, measurable value without coercing agents to use it. This requires rigorous test methodology borrowed from IR research, ML evaluation, and HCI user studies.

Success means creating a framework where:
- Tasks are demonstrably grep-impossible or grep-inefficient
- Semantic search provides clear, measurable benefits
- Results generalize to real-world development scenarios
- Agents naturally choose the right tool for the task

This is not about forcing adoption—it's about proving value so compelling that adoption is inevitable.
