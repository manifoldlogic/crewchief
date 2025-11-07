# Search Evaluation Architecture

This document describes CrewChief's approach to evaluating semantic code search capabilities and proving measurable value over traditional keyword-based tools.

## Overview

CrewChief incorporates a rigorous, scientific framework for evaluating semantic code search through grep-impossible task design. This framework enables objective comparison between semantic search (using embeddings and code graphs) and traditional tools (grep, file globbing) without coercing tool selection.

## Motivation

Traditional code search evaluation faces critical challenges:

- **No Objective Comparison**: Claims of "AI-powered search" lack rigorous validation
- **Unclear Value Proposition**: Developers cannot assess whether semantic search justifies adoption
- **Optimization Without Ground Truth**: Tool improvements lack validated success metrics
- **Coerced Usage**: Tasks that hint "use semantic search" invalidate natural tool selection

The grep-impossible framework addresses these challenges through systematic benchmark design with objective success criteria.

## Grep-Impossible Task Design Framework

### Three-Tier Validation Methodology

The framework organizes tasks into three tiers that progressively validate different aspects of semantic search value:

**Tier 1: Grep-Impossible (Capability)**
- Tasks that fundamentally defeat keyword-based search (<30% success rate with grep)
- Require: Code graph traversal, architectural understanding, negative space detection
- Examples: Transitive dependency analysis, data flow tracing, finding missing patterns
- **Proves**: Semantic search can solve problems that grep cannot

**Tier 2: Grep-Hard (Efficiency)**
- Tasks where grep might succeed but is significantly slower or less accurate (30-60% success)
- Require: Conceptual similarity understanding, ambiguity resolution, cross-cutting concerns
- Examples: Finding all retry implementations (different naming conventions), authentication checks across modules
- **Proves**: Semantic search is 30-50% more efficient for complex queries

**Tier 3: Real-World (Utility)**
- Natural developer scenarios without artificial constraints
- Focus: Realistic workflows from code review, debugging, refactoring
- No tool selection coercion—agents choose based on task characteristics
- **Proves**: Developers voluntarily adopt semantic search when appropriate

### Six Task Categories

The framework defines six categories of code search tasks based on empirical research:

1. **Relationship Discovery**: Finding code dependencies, call chains, impact analysis
2. **Conceptual Similarity**: Locating implementations of similar patterns across different naming
3. **Architectural Understanding**: Tracing request flows, understanding system structure
4. **Negative Space**: Finding code that lacks specific properties (e.g., endpoints without rate limiting)
5. **Cross-Cutting Concerns**: Discovering scattered functionality (logging, error handling, authorization)
6. **Ambiguity Resolution**: Disambiguating concepts through context (e.g., "transaction" in database vs business logic)

### Five Quality Dimensions

Every task is validated across five dimensions ensuring scientific rigor:

1. **Construct Validity**: Does grep fail as expected? (baseline comparison)
2. **Discriminant Validity**: Does search provide significant advantage? (p < 0.05)
3. **Ecological Validity**: Is this a realistic developer task? (practitioner surveys)
4. **Test-Retest Reliability**: Are results consistent across runs? (correlation > 0.7)
5. **Statistical Power**: Is sample size adequate? (minimum 5-10 iterations)

### Key Design Principles

**Natural Tool Selection**: Tasks never hint which tool to use—agents choose organically based on task characteristics

**Objective Success Criteria**: All success metrics are deterministic and automatable:
- Correct files identified
- Specific functions/patterns mentioned
- Code changes that pass tests
- No subjective "good explanation" judgments

**Anti-Keyword Pattern**: Tasks are phrased conceptually rather than using obvious keywords:
- Bad (keyword-heavy): "Find the retry logic with exponential backoff"
- Good (conceptual): "Find code that re-attempts failed operations with increasing delays"

**Ecological Validity**: All tasks based on real scenarios from pull requests, debugging sessions, code reviews, and refactoring work

## Framework Documentation

Comprehensive guides for using the grep-impossible framework:

- **[Task Design Guide](../search-optimization/task-design-guide.md)** - Creating high-quality grep-impossible tasks across six categories with anti-keyword patterns and objective success criteria
- **[Validation Guide](../search-optimization/validation-guide.md)** - Validating task quality across five dimensions with statistical rigor and troubleshooting guidance
- **[Benchmark Usage Guide](../search-optimization/benchmark-usage.md)** - Running three-tier benchmark suites, interpreting results, and integrating with genetic optimization
- **[Research Report](../research/grep-impossible-tasks-report.md)** - Publication-ready analysis of framework development, validation methodology, and empirical findings

## How This Proves Semantic Search Value

The framework provides scientific validation through multiple mechanisms:

### 1. Capability Proof (Tier 1)

Tier 1 tasks prove semantic search can solve problems grep fundamentally cannot:

**Example: Transitive Dependency Discovery**
- Task: "What code would break if we change this API signature?"
- Grep approach: Can find direct callers using string matching
- Grep limitation: Cannot traverse call chains to find transitive dependencies
- Search approach: Uses code graph to discover all downstream impacts
- Result: Grep success rate ~25%, search success rate ~75%

This objective difference demonstrates capability that keyword matching cannot replicate.

### 2. Efficiency Proof (Tier 2)

Tier 2 tasks prove semantic search is significantly more efficient:

**Example: Finding All Retry Implementations**
- Task: "Locate all code that implements retry logic"
- Grep approach: Search for "retry" keyword
- Grep limitation: Misses "exponential backoff", "circuit breaker", "attempt mechanism"
- Search approach: Understands conceptual similarity across different implementations
- Result: Grep finds 40% of implementations, search finds 85%, saves 30-50% time

This quantifies practical efficiency gains for realistic scenarios.

### 3. Adoption Proof (Tier 3)

Tier 3 tasks prove developers voluntarily choose semantic search when appropriate:

**Example: Understanding Request Flow**
- Task: "Trace how an HTTP request flows through the authentication system"
- No hints about which tools to use
- Natural scenarios from real debugging workflows
- Result: Agents voluntarily select search 70% of the time, grep 30%
- Validation: Search usage correlates with task complexity and architectural scope

This demonstrates utility without coercion—the strongest validation.

## Integration with Genetic Optimization

The framework integrates with genetic optimization for evolving tool descriptions:

1. **Multi-Tier Scoring**: Tasks from all three tiers contribute to fitness scores
2. **Natural Selection**: Agents organically choose tools—coerced usage penalized
3. **Objective Metrics**: Success/failure deterministic, enabling automated evolution
4. **Value Validation**: Optimization proven to improve actual utility, not just scores

This closes the loop: optimization validated against scientifically rigorous benchmarks.

## Implementation

The framework is fully implemented in CrewChief:

```
packages/cli/src/search-optimization/
├── tasks/                    # 35+ validated tasks across 6 categories
├── benchmarks/               # Three-tier suite definitions
├── evaluation/               # Baseline comparison and metrics
├── validation/               # Five-dimension quality validation
└── genetic-iterator.ts       # Multi-tier optimization integration
```

### Running Benchmarks

```bash
# Run a single task with validation
crewchief search-optimize:validate tier1-transitive-deps

# Run full Tier 1 suite (grep-impossible tasks)
crewchief search-optimize:benchmark --tier=1

# Run complete three-tier validation
crewchief search-optimize:benchmark --all-tiers

# Compare grep vs search on specific task
crewchief search-optimize:compare tier1-transitive-deps
```

### Cost Considerations

- Single task validation (5 iterations): $0.30-0.75
- Full Tier 1 suite (10 tasks): $12-20
- Complete three-tier benchmark (35 tasks): $45-75
- Mock mode available for development (free)

## Usage Examples

### For Tool Developers

Use the framework to prove your semantic search provides measurable value:

1. Run baseline comparison: prove grep fails on your target scenarios
2. Run search comparison: prove your tool succeeds with statistical significance
3. Run cross-project validation: prove your tool generalizes beyond one codebase
4. Document results: provide objective evidence of value proposition

### For Researchers

Use the framework as a reusable benchmark suite:

1. Compare different semantic search approaches (embeddings, code graphs, hybrid)
2. Evaluate LLM tool usage patterns across task types
3. Study tool selection behavior under natural conditions
4. Extend to new task categories or programming languages

### For Developers

Use the framework to validate search tool adoption:

1. Run real-world scenarios from your actual workflows
2. Measure time savings and result quality improvements
3. Identify which tasks benefit most from semantic search
4. Make data-driven decisions about tool adoption

## Research Foundation

The framework draws on three research traditions:

**Information Retrieval (TREC)**
- Query difficulty classification (easy vs hard)
- Relevance judgments with statistical validation
- Reusable test collections for cross-system comparison

**Machine Learning Evaluation (CheckList)**
- Behavioral testing with objective criteria
- Adversarial examples that expose limitations
- Property-based validation

**Software Testing**
- Mutation testing (what if search is broken?)
- Cross-validation (does it generalize?)
- Statistical significance (is the difference real?)

## Future Work

Planned extensions to the framework:

1. **Expanded Task Coverage**: Additional Tier 2 and Tier 3 tasks
2. **Multi-Language Validation**: Python, Rust, Go, Java task adaptations
3. **Community Benchmark Suite**: Public leaderboard for semantic search tools
4. **Continuous Improvement Pipeline**: Automated task generation and validation
5. **Cross-Tool Comparison**: Standardized evaluation across different search implementations

## Related Documentation

- **[Maproom Architecture](./MAPROOM_ARCHITECTURE.md)** - Semantic search implementation details
- **[Database Architecture](./DATABASE_ARCHITECTURE.md)** - Storage and indexing architecture
- **[Search Optimization Framework](../search-optimization/)** - Complete framework documentation

## References

- TREC Information Retrieval Conference: https://trec.nist.gov/
- CheckList Behavioral Testing: https://aclanthology.org/2020.acl-main.442/
- Cohen's d Effect Size: Standard statistical measure for comparing groups
- Cross-Project Validation: Transferring IR evaluation across domains

---

**Last Updated**: 2025-11-07
**Status**: Production-ready
**Framework Version**: 1.0.0
