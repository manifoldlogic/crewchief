# Maproom Search Tool Optimization

## Project Overview

Apply learnings from genetic optimization to improve the Maproom semantic search tool description, targeting >20% performance on AI agent code search benchmarks.

## Problem Statement

Genetic optimization discovered that AI agents perform significantly better (19.6% vs 17.7% baseline) when tool descriptions teach **transformation workflows** (how to convert natural language → search queries) rather than just providing examples. However, performance has plateaued at 19-20% across 10 generations.

Analysis identified a critical gap: current descriptions teach question→query transformation but not task→query mapping. Agents receive high-level tasks like "Find where X is implemented" but lack guidance on deriving search strategies from task goals.

## Proposed Solution

1. **Document learnings**: Create permanent documentation capturing genetic optimization insights
2. **Apply top performer**: Update production MCP tool description with variant-a-detailed (19.6% winner)
3. **Implement enhancement**: Add task-to-query mapping section to push beyond 20%
4. **Validate change**: Compare new description against baseline in controlled test

## Relevant Agents

- **Technical writer** (for documentation)
- **MCP server specialist** (for tool description updates)
- **Test engineer** (for validation)

## Planning Documents

- [Analysis](planning/analysis.md) - Genetic optimization findings and pattern analysis
- [Architecture](planning/architecture.md) - Tool description structure and deployment
- [Quality Strategy](planning/quality-strategy.md) - Validation approach
- [Security Review](planning/security-review.md) - Security considerations
- [Plan](planning/plan.md) - Implementation phases and execution

## Success Criteria

- [ ] Documentation published to `docs/optimization/`
- [ ] Production tool description updated
- [ ] Performance validation shows ≥19.6% (maintains or improves)
- [ ] Enhancement variant created for future testing
