# Ticket: AGENTOPT-2001: Winner Tracking and Leaderboard System

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create a comprehensive tracking system for genetic optimization results that maintains a leaderboard across multiple runs, tracks production variants, and captures learnings for future tuning.

## Background
The genetic iterator (genetic-iterator.ts) currently saves results per-run in `.crewchief/genetic-iterations/run-{timestamp}/` but lacks cross-run comparison, production variant designation, historical tracking, and learnings database capabilities.

This work implements Phase 2 of the AGENTOPT project, focusing on Winner Tracking and Production Management rather than the originally planned Server-Side Preprocessing phase. This strategic shift enables:
- Systematic comparison of results from different optimization runs
- Clear production variant promotion workflow
- Historical tracking of best-performing variants over time
- Institutional knowledge capture about effective mutation strategies

Without this tracking infrastructure, teams must manually compare JSON files across runs, cannot systematically decide which variant to promote, and lose valuable insights about what optimization approaches work best.

## Acceptance Criteria
- [x] Global leaderboard JSON file created at `.crewchief/leaderboard.json`
- [x] Leaderboard tracks top 10 variants across all optimization runs with scores, metadata
- [x] Production variant designation system at `.crewchief/production/current.json`
- [x] Run registry at `.crewchief/optimization-runs/index.json` tracking all runs with metadata
- [x] Genetic iterator automatically updates leaderboard after each run
- [x] `saveToLeaderboard()` function implemented
- [x] `promoteToProduction()` function implemented
- [x] `compareRunResults()` function for comparing two runs
- [x] `exportLearnings()` function to capture insights from runs
- [x] CLI commands for viewing leaderboard and managing production variant
- [x] Unit tests for all tracking functions
- [x] Documentation in docs/architecture/ explaining the tracking system

## Technical Requirements

### 1. Leaderboard Schema
```typescript
interface Leaderboard {
  allTimeTopVariants: LeaderboardEntry[]  // Top 10 by composite score
  productionVariant: string | null        // Current production variant ID
  productionDeployedAt: Date | null
  lastUpdated: Date
}

interface LeaderboardEntry {
  rank: number
  variantId: string
  name: string
  compositeScore: number
  tierScores: { tier1: number; tier2: number; tier3: number }
  runId: string
  generation: number
  converged: boolean
  timestamp: Date
  taskCoverage: { total: number; passed: number }
}
```

### 2. Run Registry Schema
```typescript
interface RunRegistry {
  runs: OptimizationRun[]
}

interface OptimizationRun {
  runId: string
  startedAt: Date
  completedAt: Date | null
  status: 'running' | 'completed' | 'failed'
  convergenceReached: boolean
  bestVariant: {
    id: string
    score: number
    generation: number
  }
  config: IterationConfig
  learnings: string[]
}
```

### 3. Production Variant System
- `.crewchief/production/current.json` - Pointer to active variant
- `.crewchief/production/variants/{id}.json` - Copy of production variants
- `.crewchief/production/deployment-log.md` - Change log

### 4. Integration with Genetic Iterator
- Update `runGeneticIterations()` to call `saveToLeaderboard()` on completion
- Add run metadata to registry at start and completion
- Capture learnings during iteration (best mutations, convergence patterns)

### 5. Core Functions
- `saveToLeaderboard(runResults)` - Update global leaderboard with run results
- `promoteToProduction(variantId)` - Copy variant to production and update current.json
- `compareRunResults(runId1, runId2)` - Side-by-side comparison of two runs
- `exportLearnings(runId)` - Extract insights from a completed run

### 6. CLI Integration
Commands should be added to the CLI for:
- Viewing current leaderboard
- Comparing runs
- Promoting variants to production
- Viewing production variant history

## Implementation Notes

### File Structure
```
.crewchief/
├── leaderboard.json                    # Global top 10 variants
├── optimization-runs/
│   ├── index.json                      # Run registry
│   └── run-{timestamp}/                # Existing per-run results
│       ├── generations/
│       ├── final-results.json
│       └── convergence-log.json
└── production/
    ├── current.json                    # Current production pointer
    ├── deployment-log.md               # Historical deployments
    └── variants/
        ├── {variant-id-1}.json
        └── {variant-id-2}.json
```

### Leaderboard Update Logic
1. After each genetic iteration run completes
2. Extract top variant from run results
3. Load current leaderboard
4. Insert new variant if score qualifies for top 10
5. Re-rank and maintain only top 10
6. Write updated leaderboard atomically

### Production Promotion Workflow
1. User identifies winning variant from leaderboard
2. `promoteToProduction(variantId)` copies variant JSON to production/variants/
3. Updates production/current.json with variant ID and timestamp
4. Appends entry to deployment-log.md
5. Returns confirmation with deployment details

### Learnings Extraction
Capture insights such as:
- Which mutation types led to best improvements
- Convergence patterns (generations needed, plateau detection)
- Task coverage trends across generations
- Score improvement velocity
- Successful parameter ranges

## Dependencies
- Genetic iterator implementation (genetic-iterator.ts)
- Benchmark runner and evaluation framework
- File system utilities for atomic writes

## Risk Assessment
- **Risk**: Concurrent writes to leaderboard.json from multiple runs
  - **Mitigation**: Implement file locking or atomic write-then-rename pattern

- **Risk**: Leaderboard grows unbounded over time
  - **Mitigation**: Hard limit of top 10 variants; run registry can be pruned periodically

- **Risk**: Production variant rollback needed after promotion
  - **Mitigation**: deployment-log.md tracks all deployments; implement rollback command

- **Risk**: Schema changes break existing leaderboard/registry files
  - **Mitigation**: Version leaderboard schema; implement migration utilities

## Files/Packages Affected
- `/workspace/packages/cli/src/search-optimization/tracking/leaderboard.ts` (new)
- `/workspace/packages/cli/src/search-optimization/tracking/production.ts` (new)
- `/workspace/packages/cli/src/search-optimization/tracking/run-registry.ts` (new)
- `/workspace/packages/cli/src/search-optimization/tracking/index.ts` (new)
- `/workspace/packages/cli/src/search-optimization/genetic-iterator.ts` (modify)
- `/workspace/packages/cli/src/search-optimization/tracking/__tests__/leaderboard.test.ts` (new)
- `/workspace/packages/cli/src/search-optimization/tracking/__tests__/production.test.ts` (new)
- `/workspace/packages/cli/src/search-optimization/tracking/__tests__/run-registry.test.ts` (new)
- `/workspace/docs/architecture/optimization-tracking-system.md` (new)

## Success Metrics
- Can view top 10 variants across all runs with a single command
- Can promote variant to production with one command
- Can compare two runs side-by-side with detailed metrics
- Can see what mutation strategies performed best historically
- Leaderboard automatically updates after each optimization run
- Zero data loss during concurrent writes
- Production deployment history fully auditable
