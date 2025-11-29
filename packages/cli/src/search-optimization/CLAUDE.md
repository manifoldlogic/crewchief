# search-optimization

## What This Is

Genetic algorithm system for optimizing MCP tool descriptions to improve agent search behavior. Uses multi-tier benchmarks to measure search quality and iteratively refine tool descriptions.

**Key insight**: Tool descriptions significantly impact which tools agents select. This subsystem finds optimal descriptions through evolutionary search.

## Benchmark Tiers

| Tier                 | Purpose                        | Examples                            |
| -------------------- | ------------------------------ | ----------------------------------- |
| **tier1-impossible** | Tasks impossible for grep/glob | Semantic queries, concept searches  |
| **tier2-hard**       | Difficult for text search      | Ambiguous names, cross-references   |
| **tier3-realworld**  | Actual developer tasks         | Debugging, refactoring, code review |

Tasks live in `tasks/` subdirectories (e.g., `tasks/realworld/debugging/`).

## Key Scripts

```bash
# Full validation (runs all tiers, generates report)
pnpm search-optimization:validate-full

# Quick test of setup
pnpm search-optimization:test-setup

# Run single competition
pnpm search-optimization:run-example
```

## Module Structure

```
benchmarks/        # Tier suites and runner
evaluation/        # Metrics, statistics, comparison
genetic-iterator.ts   # Core genetic algorithm
multi-tier-scoring.ts # Weighted tier aggregation
competition-runner.ts # Single competition execution
tasks/             # Search tasks organized by category
taxonomy/          # Task categorization and difficulty
security/          # Input validation, limits
reporting/         # Statistics and report generation
```

## When to Run Validation

- **Before MCP tool description changes** - Ensure changes don't regress search quality
- **After adding new task categories** - Verify new tasks are discoverable
- **After search algorithm changes** - Measure impact on agent behavior

## Scoring

Multi-tier score aggregates results with default weights:

- tier1: 0.3 (tests semantic capability)
- tier2: 0.3 (tests edge cases)
- tier3: 0.4 (tests real utility)

Convergence threshold: improvement < 0.01 for 3 consecutive generations.

## Gotchas

- **Cost awareness** - Full validation makes API calls; use `estimateCost()` first
- **Flaky tasks** - Some tasks are inherently probabilistic; use statistical analysis
- **Task definitions are code** - Tasks return expected files/chunks, not free-form answers
