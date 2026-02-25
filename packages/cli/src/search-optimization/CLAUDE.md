# search-optimization

Genetic algorithm system for optimizing MCP tool descriptions to improve agent search behavior.

**Key insight**: Tool descriptions significantly impact which tools agents select. This subsystem finds optimal descriptions through evolutionary search.

## Benchmark Tiers

| Tier                 | Purpose                                           | Weight |
| -------------------- | ------------------------------------------------- | ------ |
| **tier1-impossible** | Tasks impossible for grep/glob (semantic queries) | 0.3    |
| **tier2-hard**       | Difficult for text search (ambiguous names)       | 0.3    |
| **tier3-realworld**  | Actual developer tasks (debugging, refactoring)   | 0.4    |

Convergence threshold: improvement < 0.01 for 3 consecutive generations.

## When to Run Validation

- **Before MCP tool description changes** — ensure no regression
- **After adding new task categories** — verify discoverability
- **After search algorithm changes** — measure impact

## Gotchas

- **Cost awareness** — full validation makes API calls; use `estimateCost()` first
- **Flaky tasks** — some tasks are inherently probabilistic; use statistical analysis
- **Task definitions are code** — tasks return expected files/chunks, not free-form answers
