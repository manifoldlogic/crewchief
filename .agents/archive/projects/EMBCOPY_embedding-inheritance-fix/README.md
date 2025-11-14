# EMBCOPY: Embedding Inheritance Fix

## Problem

Scans of new worktrees are slow because the embedding pipeline generates new embeddings for chunks that already have embeddings in the `code_embeddings` table. The BLOBSHA deduplication infrastructure exists but isn't being used during embedding generation.

## Impact

- New worktree scans take hours instead of seconds
- Genetic optimizer unusable (5 variants × hours = impractical)
- Wasted API costs generating duplicate embeddings
- Users switching branches experience slow indexing

## Solution

Add embedding inheritance step: before generating embeddings, check `code_embeddings` table and copy existing embeddings for matching blob SHAs.

## Agents

- rust-indexer-engineer (implementation)
- unit-test-runner (validation)
- verify-ticket (acceptance)
- commit-ticket (finalization)

## Planning Documents

- [Analysis](planning/analysis.md) - Problem investigation
- [Architecture](planning/architecture.md) - Solution design
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security assessment
- [Plan](planning/plan.md) - Execution strategy
