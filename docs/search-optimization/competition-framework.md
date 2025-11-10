# AGENTOPT Competition Framework

Guide for running the AI Agent Query Optimization (AGENTOPT) competition framework to tune maproom-mcp tool descriptions through automated agent competitions.

## Overview

The AGENTOPT framework optimizes tool descriptions by:

1. **Creating variants** - Different phrasings/guidance in tool descriptions
2. **Spawning agents** - Multiple AI agents with different variants (via Claude Code Agents SDK)
3. **Running tasks** - Agents execute real search tasks in isolated worktrees
4. **Scoring performance** - Automated evaluation based on search quality, task completion, and efficiency
5. **Selecting winner** - Best performing variant becomes the new baseline
6. **Iterating genetically** - Mutate winner to create new variants, repeat

### Key Benefits

- **Empirical validation** - Real agent performance data, not guesswork
- **Automated testing** - No manual evaluation needed
- **Continuous improvement** - Genetic algorithm finds optimal descriptions
- **Objective metrics** - Clear scoring across multiple dimensions

## Pre-Flight Validation

Starting with the competition framework improvements, the competition runner includes comprehensive pre-flight validation to ensure 100% of agents have valid tool environments before execution.

### Why Validation is Required

Analysis of early competition runs revealed systematic failures:
- 0% search tool usage (agents didn't have access to maproom tools)
- 0% task completion (agents couldn't complete tasks without tools)
- Wasted API credits (~$15-20 per failed run)

Validation ensures:
- Database is accessible
- Base branch is indexed
- All worktrees are scanned
- MCP tools are configured
- File permissions are correct

### Validation Phases

The competition runner now operates in three distinct phases:

**Phase 1: Setup (Sequential)**
1. Validate database connection
2. Verify base branch indexed
3. Create competition directory
4. Load variants
5. Create worktrees (one per variant)
6. Inject variant tool descriptions
7. **Scan all worktrees** (NEW)

**Phase 2: Validation (Per-Variant)**
1. Check worktree exists
2. Check worktree scanned (chunk_count > 0)
3. Check MCP config valid
4. Check file permissions OK
5. **Fail fast if any check fails** (NEW)

**Phase 3: Execution (Parallel)**
1. Spawn agents (only if validation passed)
2. Collect results
3. Evaluate winner

### Timing Expectations

**For 12 variants (ultra configuration):**
- Setup: ~2-3 minutes (worktree creation + scanning)
- Validation: ~10-20 seconds
- Execution: ~2-5 minutes (parallel agents)
- **Total: ~4-8 minutes** (vs ~2-3 minutes without validation)

**Tradeoff:** +2-3 minutes setup time for 100% success rate (vs 0% without validation)

### Validation Checks

#### Database Connection
- **What**: Tests PostgreSQL connectivity
- **How**: Executes `SELECT 1` query
- **Failure**: "Database connection failed - check MAPROOM_DATABASE_URL"

#### Base Branch Indexed
- **What**: Verifies base branch has chunks in database
- **How**: Runs `maproom status --repo <repo> --worktree <branch>`
- **Failure**: "Base branch not indexed - run: crewchief-maproom scan..."

#### Worktree Scanned
- **What**: Ensures variant worktree has chunks indexed
- **How**: Checks `chunk_count > 0` in database
- **Failure**: "Worktree has 0 chunks indexed"

#### MCP Config Valid
- **What**: Validates .mcp.json structure
- **How**: Parses JSON and checks for maproom server
- **Failure**: "MCP config missing or invalid"

#### File Permissions
- **What**: Tests read/write access
- **How**: Reads package.json and creates test file
- **Failure**: "Permission error: EACCES"

### Validation Workflow Diagram

```
Competition Validation Workflow
=================================

                 Start Competition
                        │
                        ├─▶ Check Database Connection
                        │      ├─ PASS → Continue
                        │      └─ FAIL → Error: "Database connection failed"
                        │
                        ├─▶ Verify Base Branch Indexed
                        │      ├─ PASS → Continue
                        │      └─ FAIL → Error: "Base branch not indexed"
                        │
                        ├─▶ Create Worktrees
                        │      └─▶ For each variant:
                        │             ├─ Create directory
                        │             └─ Copy base files
                        │
                        ├─▶ Inject Variant Descriptions
                        │      └─▶ For each worktree:
                        │             └─ Modify .mcp.json
                        │
                        ├─▶ Scan Worktrees
                        │      └─▶ For each worktree:
                        │             ├─ Run: maproom scan
                        │             ├─ Wait for completion
                        │             └─ FAIL if errors
                        │
                        ├─▶ Validate Environments
                        │      └─▶ For each worktree:
                        │             ├─ Check: Exists
                        │             ├─ Check: Indexed
                        │             ├─ Check: MCP config
                        │             ├─ Check: Permissions
                        │             └─ FAIL if any fails
                        │
                        ├─▶ Spawn Agents (parallel)
                        │      └─▶ Only if ALL validations passed
                        │
                        └─▶ Evaluate Results
```

### Console Output Examples

Example successful run with validation:

```
$ pnpm tsx scripts/run-genetic-optimizer-ultra.ts

🏁 Starting competition with pre-flight validation

📋 Phase 1: Setup
============================================================
✅ Database connection verified
✅ Base branch indexed (1234 chunks)
✅ Competition directory: /tmp/comp-1234567890
✅ Loaded 12 variants

✅ Created worktree for variant-control
✅ Created worktree for variant-a-detailed
...

📊 Scanning worktrees...
============================================================
📊 Scanning worktree: variant-control
   Path: /tmp/comp-1234567890/worktrees/variant-control
   ✅ Scan complete: 567 chunks in 8234ms
...
============================================================
✅ All scans complete in 16.1s
📊 Total chunks indexed: 6804

🔍 Phase 2: Pre-Flight Validation
============================================================
✅ variant-control: All checks passed
✅ variant-a-detailed: All checks passed
...

✅ All variants validated - ready for execution

🚀 Phase 3: Agent Execution
============================================================
[Agents running...]
```

## Troubleshooting

### Database Connection Failed

**Error:**
```
❌ Pre-flight validation failed: Database connection failed
```

**Fix:**
1. Verify PostgreSQL is running:
   ```bash
   docker ps | grep maproom-postgres
   ```

2. Check environment variable:
   ```bash
   echo $MAPROOM_DATABASE_URL
   ```

3. Test connection manually:
   ```bash
   psql $MAPROOM_DATABASE_URL -c "SELECT 1"
   ```

4. Restart PostgreSQL if needed:
   ```bash
   cd packages/maproom-mcp/config
   docker compose down
   docker compose up -d
   ```

### Base Branch Not Indexed

**Error:**
```
❌ Pre-flight validation failed: Base branch 'main' not indexed
```

**Fix:**
Run scan on base branch first (one-time setup):
```bash
crewchief-maproom scan --repo crewchief --worktree main --root /workspace
```

This takes 30-60 seconds initially. Subsequent variant scans will be fast (5-15s) due to embedding reuse.

### Worktree Scan Failed

**Error:**
```
❌ Scan failed for variant-a-detailed: Permission denied
```

**Fix:**
1. Check worktree path exists:
   ```bash
   ls -la .crewchief/worktrees/
   ```

2. Verify binary is in PATH:
   ```bash
   which crewchief-maproom
   ```

3. Check database permissions:
   ```bash
   psql $MAPROOM_DATABASE_URL -c "SELECT * FROM repos LIMIT 1"
   ```

### MCP Config Missing

**Error:**
```
❌ Validation failed: MCP config missing in worktree
```

**Fix:**
This indicates a bug in worktree creation. Check:
1. Variant injection completed:
   ```bash
   cat .crewchief/worktrees/variant-*/.mcp.json
   ```

2. SDK version is compatible:
   ```bash
   pnpm list @anthropic-ai/claude-agent-sdk
   ```

### Permission Denied

**Error:**
```
❌ Validation failed: Permission error: EACCES
```

**Fix:**
1. Check directory ownership:
   ```bash
   ls -la .crewchief/worktrees/
   ```

2. Fix permissions if needed:
   ```bash
   chmod -R u+rw .crewchief/worktrees/
   ```

## Security and Resource Limits

### Resource Limits

The competition runner enforces limits to prevent resource exhaustion:

- **MAX_VARIANTS**: 50 (prevents excessive worktree creation)
- **MAX_PARALLEL_AGENTS**: 10 (limits concurrent API calls)
- **MAX_TIMEOUT**: 600000ms (10 minutes per agent)

To run larger competitions, these limits can be adjusted in code (requires recompilation).

### Security Controls

1. **Variant ID Validation**: Only alphanumeric, dash, underscore allowed (prevents path traversal)
2. **Command Injection Protection**: All subprocess execution uses spawn with args array
3. **Sensitive Data Sanitization**: Database credentials redacted in logs
4. **Fail-Fast Validation**: Stops immediately on setup errors (doesn't waste API credits)

For details, see the project planning documentation in `.agents/projects/COMPFIX_competition-agent-setup-validation/planning/`.

## Prerequisites

### Required Software

```bash
# Node.js 18+
node --version  # Should be >= 18.0.0

# pnpm
pnpm --version

# PostgreSQL with pgvector
# (Can be run via Docker - see Database Setup section)
```

### Required Environment Variables

```bash
# 1. Anthropic API Key (Required)
export ANTHROPIC_API_KEY="sk-ant-..."

# 2. Database Connection (Required)
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"

# 3. Optional: LLM Provider (defaults to Anthropic if ANTHROPIC_API_KEY is set)
export LLM_PROVIDER="anthropic"  # or "openai"

# 4. Optional: Model Selection
export ANTHROPIC_MODEL="claude-3-5-sonnet-latest"
# or
export OPENAI_MODEL="gpt-4o-mini"
export OPENAI_API_KEY="sk-..."
```

### Database Fallback Chain

The system checks environment variables in this order:

1. `MAPROOM_DATABASE_URL` (recommended, current standard)
2. `MAPROOM_DB_HOST` + component variables
3. `PG_DATABASE_URL` (deprecated, backward compatibility)
4. `DATABASE_URL` (generic fallback)

**Best Practice**: Use `MAPROOM_DATABASE_URL` explicitly.

## Setup

### 1. Install Dependencies

```bash
cd packages/cli
pnpm install
```

### 2. Build the CLI

```bash
pnpm build
```

### 3. Start PostgreSQL

#### Option A: Docker (Recommended)

```bash
cd packages/maproom-mcp

# Start PostgreSQL with pgvector
docker compose -f config/docker-compose.yml up -d

# Verify connection
psql $MAPROOM_DATABASE_URL -c "SELECT version();"
```

#### Option B: Local PostgreSQL

Ensure PostgreSQL 14+ is installed with pgvector extension:

```bash
# Install pgvector extension
psql -d maproom -c "CREATE EXTENSION IF NOT EXISTS vector;"

# Verify setup
psql $MAPROOM_DATABASE_URL -c "SELECT * FROM pg_extension WHERE extname='vector';"
```

### 4. Set Up Environment Variables

Create a `.env` file or add to your shell profile:

```bash
# .env file in packages/cli/
ANTHROPIC_API_KEY=sk-ant-your-key-here
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom
LLM_PROVIDER=anthropic
```

Load environment variables:

```bash
# Using dotenv
source .env

# Or add to ~/.bashrc or ~/.zshrc
echo 'export ANTHROPIC_API_KEY="sk-ant-..."' >> ~/.bashrc
echo 'export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"' >> ~/.bashrc
source ~/.bashrc
```

### 5. Verify Setup

```bash
# Check environment variables
echo $ANTHROPIC_API_KEY  # Should show your key
echo $MAPROOM_DATABASE_URL  # Should show connection string

# Test database connection
psql $MAPROOM_DATABASE_URL -c "SELECT 1;"

# Test Anthropic API
curl https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-3-5-sonnet-latest","max_tokens":10,"messages":[{"role":"user","content":"Hi"}]}'
```

## Running Competitions

### Quick Start: Full Validation

Run the complete validation suite (all 30+ tasks):

```bash
cd packages/cli

# Run full validation
pnpm search-optimization:validate-full
```

**⚠️ WARNING**: This is **EXPENSIVE**:
- **Cost**: $20-50 in API costs
- **Time**: 2-4 hours
- **API calls**: 60+ agent spawns (30 tasks × 2 conditions)

The script will:
1. Estimate cost
2. Ask for confirmation
3. Run all tasks in both conditions (grep-only vs search-available)
4. Generate statistical analysis
5. Save reports to `.crewchief/validations/`

### Running Individual Competitions

For testing or specific optimizations, run individual competitions:

```typescript
// packages/cli/src/search-optimization/examples/run-competition.ts

import { runCompetition } from '../competition-runner.js'
import { TASK_FIND_WORKTREE_CREATION } from '../tasks/implementation.js'

// Define variants
const variants = [
  {
    id: 'baseline',
    name: 'Baseline Description',
    searchToolDescription: 'Semantic code search - use for finding code by concept',
  },
  {
    id: 'enhanced-v1',
    name: 'Enhanced with Examples',
    searchToolDescription: `Semantic code search optimized for AI agents.

QUERY FORMULATION:
- Extract 2-3 core technical terms
- Remove question words (how, what, where, why)
- Prefer code-like terminology

Examples:
  "How does checkout work?" → "checkout payment"
  "Find auth logic" → "authentication"`,
  },
]

// Run competition
const result = await runCompetition({
  task: TASK_FIND_WORKTREE_CREATION,
  variants,
  parallelExecution: false, // Set true for faster execution
  timeout: 300, // 5 minutes per agent
  baseDir: '.crewchief/competitions',
})

console.log('Winner:', result.winner.variantName)
console.log('Score:', (result.winner.score * 100).toFixed(1) + '%')
console.log('Report:', result.report)
```

Run it:

```bash
tsx src/search-optimization/examples/run-competition.ts
```

### Running Benchmark Suites

Run specific benchmark tiers:

```typescript
// Run Tier 1: Grep-Impossible tasks
import { TIER1_GREP_IMPOSSIBLE_SUITE } from '../benchmarks/index.js'
import { runBenchmarkSuite } from '../benchmarks/suite-runner.js'

const result = await runBenchmarkSuite(TIER1_GREP_IMPOSSIBLE_SUITE, {
  parallel: false,
  iterations: 5,
})

console.log(formatSuiteSummary(result))
```

Available suites:
- `TIER1_GREP_IMPOSSIBLE_SUITE` - 12 tasks that fundamentally defeat grep
- `TIER2_GREP_HARD_SUITE` - 12 tasks where grep is inefficient
- `TIER3_REALWORLD_SUITE` - 8 realistic developer scenarios

## Configuration Options

### Competition Config

```typescript
interface CompetitionConfig {
  task: SearchTask              // Which task to run
  variants: Variant[]           // Variants to compete
  parallelExecution?: boolean   // Run agents in parallel? (default: false)
  timeout?: number              // Max time per agent in seconds (default: 300)
  baseDir?: string              // Directory for results (default: .crewchief/competitions)
}
```

### Task Configuration

Tasks define what agents should do:

```typescript
interface SearchTask {
  id: string                    // Unique identifier
  name: string                  // Human-readable name
  description: string           // Task prompt for agent
  searchTarget: SearchTarget    // What to find
  followUpTask: FollowUpTask    // What to do after finding
  difficulty: 'easy' | 'medium' | 'hard'
  category: string              // Task type category
  maxSearchAttempts: number     // Max searches allowed
  maxTimeSeconds: number        // Max time allowed
  expectedGrepSuccess: number   // Expected grep success rate (0-1)
  expectedSearchSuccess: number // Expected search success rate (0-1)
}
```

### Variant Configuration

Variants are different tool descriptions:

```typescript
interface Variant {
  id: string                    // Unique identifier
  name: string                  // Human-readable name
  searchToolDescription: string // The tool description to test
  openToolDescription?: string  // Optional: customize open tool
  contextToolDescription?: string // Optional: customize context tool
}
```

## Understanding Results

### Competition Reports

After each competition, a report is generated with:

```
COMPETITION REPORT
==================

Task: Find Worktree Creation Implementation
Difficulty: medium
Category: finding-implementation

RESULTS
-------
1. Enhanced with Examples
   Score: 87.3%
   Search Quality: 100.0%  (found target in top 3)
   Task Completion: 80.0%  (good explanation, minor issues)
   Efficiency: 85.0%       (efficient execution)
   Searches: 2

2. Baseline Description
   Score: 52.1%
   Search Quality: 40.0%   (found target in top 20)
   Task Completion: 60.0%  (partial completion)
   Efficiency: 70.0%       (multiple retries)
   Searches: 5

WINNER
------
Enhanced with Examples (87.3%)

Found target file in 2 searches with high-quality explanation.
Mentioned key files: worktrees.ts, git service.
Efficient execution with minimal retries.

NEXT STEPS
----------
- Use winner as baseline for next generation
- Generate mutations from winner
- Run next competition
```

### Scoring Breakdown

Each participant is scored on three dimensions:

#### 1. Search Quality (40% weight)

Measures how well the agent finds the target:

- **1.0** - Target found in top 3 results
- **0.7** - Target found in top 10 results
- **0.4** - Target found in top 20 results
- **0.0** - Target not found

#### 2. Task Completion (40% weight)

Measures whether the agent completed the task:

- **1.0** - Task fully completed with all criteria met
- **0.5-0.8** - Partially completed
- **0.0** - Task failed

Validated automatically using:
- File mentions (for explanations)
- Pattern matching (for explanations)
- File changes (for code modifications)
- File creation (for new files)

#### 3. Efficiency (20% weight)

Measures how efficiently the agent completed the task:

- Fewer searches (1-10 optimal)
- Fewer tool calls (5-30 optimal)
- Faster execution (30-300s optimal)

**Total Score** = 0.4 × searchQuality + 0.4 × taskCompletion + 0.2 × efficiency

### Validation Reports

Full validation runs generate detailed reports:

```
VALIDATION REPORT
=================

Total Tasks: 32
Grep Success Rate: 23.4%
Search Success Rate: 76.8%
Improvement: +53.4 percentage points
Statistically Significant: Yes (p < 0.001)

PER-TIER SUMMARY
----------------

Tier 1 (Grep-Impossible):
  Tasks: 12
  Grep: 15.2%
  Search: 82.1%
  Improvement: +66.9 pp
  Statistical significance: p < 0.001

Tier 2 (Grep-Hard):
  Tasks: 12
  Grep: 28.4%
  Search: 75.2%
  Improvement: +46.8 pp
  Statistical significance: p < 0.001

Tier 3 (Real-World):
  Tasks: 8
  Grep: 31.2%
  Search: 71.5%
  Improvement: +40.3 pp
  Statistical significance: p < 0.001

TOOL USAGE STATISTICS
---------------------

Search Usage Rate: 78.2% (appropriate for 82.1% of tasks)
Grep Usage Rate: 21.8% (appropriate for 17.9% of tasks)
Appropriate Usage: 89.4%

RECOMMENDATIONS
---------------

✅ Semantic search provides significant value across all tiers
✅ Tool descriptions are effective - agents select appropriate tools
✅ Continue optimization with genetic iteration
⚠️  Consider improving grep guidance for simple queries
```

## Cost Management

### Cost Estimation

```bash
# Estimate before running
cd packages/cli
tsx src/search-optimization/scripts/estimate-cost.ts

# Output:
# Tier 1 (12 tasks): $12-20
# Tier 2 (12 tasks): $14-22
# Tier 3 (8 tasks): $18-28
# Full validation: $44-70
```

### Cost-Saving Strategies

1. **Use Mock Mode for Development**

```typescript
const result = await validateTask({
  task: MY_TASK,
  tier: 'tier1-impossible',
  useMockData: true, // Free, fast, for testing
})
```

2. **Run Individual Tasks First**

```bash
# Test one task before running full suite
tsx src/search-optimization/examples/run-single-task.ts
```

3. **Reduce Iterations**

```typescript
const result = await runBenchmarkSuite(TIER1_SUITE, {
  iterations: 3, // Instead of 10
})
```

4. **Use Smaller Models**

```bash
export ANTHROPIC_MODEL="claude-3-haiku-20240307"  # Cheaper than Sonnet
```

5. **Sequential Execution**

```typescript
const result = await runCompetition({
  parallelExecution: false, // Prevents rate limiting charges
  // ...
})
```

## Troubleshooting

### Database Connection Failed

**Error**: `Connection to database failed`

**Solutions**:

```bash
# 1. Verify MAPROOM_DATABASE_URL is set
echo $MAPROOM_DATABASE_URL

# 2. Test connection manually
psql $MAPROOM_DATABASE_URL -c "SELECT 1;"

# 3. Check PostgreSQL is running
docker ps | grep postgres
# or
pg_isready -h localhost -p 5432

# 4. Start PostgreSQL if needed
cd packages/maproom-mcp
docker compose -f config/docker-compose.yml up -d

# 5. Verify pgvector extension
psql $MAPROOM_DATABASE_URL -c "SELECT * FROM pg_extension WHERE extname='vector';"
```

### Anthropic API Errors

**Error**: `ANTHROPIC_API_KEY is not set`

**Solutions**:

```bash
# 1. Verify API key is set
echo $ANTHROPIC_API_KEY

# 2. Test API key
curl https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -H "content-type: application/json" \
  -d '{"model":"claude-3-5-sonnet-latest","max_tokens":10,"messages":[{"role":"user","content":"test"}]}'

# 3. Check API key permissions
# Visit https://console.anthropic.com/settings/keys
```

**Error**: `Rate limit exceeded`

**Solutions**:

```bash
# 1. Use sequential execution
parallelExecution: false

# 2. Add delays between tasks
# 3. Upgrade API tier at https://console.anthropic.com
```

### Agent Spawn Failed

**Error**: `Failed to spawn agent`

**Solutions**:

```bash
# 1. Verify Claude Code Agents SDK is installed
cd packages/cli
pnpm list @anthropic-ai/claude-agent-sdk

# 2. Reinstall SDK if needed
pnpm install @anthropic-ai/claude-agent-sdk@latest

# 3. Check Node.js version
node --version  # Should be >= 18.0.0
```

### Timeout Errors

**Error**: `Agent execution timed out`

**Solutions**:

```typescript
// Increase timeout
const result = await runCompetition({
  timeout: 600, // 10 minutes instead of 5
  // ...
})
```

### Out of Memory

**Error**: `JavaScript heap out of memory`

**Solutions**:

```bash
# Increase Node.js memory limit
NODE_OPTIONS="--max-old-space-size=4096" pnpm search-optimization:validate-full
```

## Advanced Usage

### Genetic Iteration

Run multiple generations to evolve optimal descriptions:

```typescript
import { runGeneticIterator } from './genetic-iterator.js'

const result = await runGeneticIterator({
  initialVariants: [baselineVariant],
  task: TASK_FIND_WORKTREE_CREATION,
  generations: 10,
  populationSize: 5,
  mutationRate: 0.3,
  convergenceThreshold: 0.01, // Stop when improvement < 1%
})

console.log('Best variant:', result.bestVariant)
console.log('Improvement:', result.improvement)
```

### Custom Validators

Create custom success criteria:

```typescript
const customTask: SearchTask = {
  // ... other fields
  successValidator: (output: AgentOutput) => {
    // Custom scoring logic
    const searchQuality = output.searchResults.some(r => r.path.includes('target.ts'))
      ? 1.0
      : 0.0

    const taskCompletion = output.workResult.success ? 1.0 : 0.0

    const efficiency = Math.min(1.0, 10 / output.searchCount)

    return {
      searchQuality,
      taskCompletion,
      efficiency,
      total: 0.4 * searchQuality + 0.4 * taskCompletion + 0.2 * efficiency,
      details: `Custom scoring: ${output.searchCount} searches`,
    }
  },
}
```

### Cross-Project Validation

Test tasks on different codebases:

```bash
# Index another project
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"
cd /path/to/other/project

# Scan the project
node packages/maproom-mcp/bin/cli.cjs scan . --repo other-project

# Run tasks against it
tsx packages/cli/src/search-optimization/examples/cross-project-validation.ts \
  --repo other-project \
  --tasks tier1
```

## File Locations

### Source Code

```
packages/cli/src/search-optimization/
├── competition-runner.ts       # Main orchestrator
├── genetic-iterator.ts         # Genetic algorithm
├── types.ts                    # Type definitions
├── tasks/                      # Task library
│   ├── implementation.ts       # Finding implementation tasks
│   ├── architecture.ts         # Architecture understanding tasks
│   ├── errors.ts               # Error handling tasks
│   └── ...
├── benchmarks/                 # Benchmark suites
│   ├── tier1-impossible.ts     # Grep-impossible tasks
│   ├── tier2-hard.ts           # Grep-hard tasks
│   └── tier3-realworld.ts      # Real-world tasks
├── evaluation/                 # Scoring and metrics
│   ├── baseline-runner.ts      # Run baseline comparisons
│   ├── comparison.ts           # Compare results
│   └── metrics.ts              # Calculate scores
├── validation/                 # Quality validation
│   └── task-validator.ts       # Validate task quality
└── scripts/                    # Executable scripts
    └── run-full-validation.ts  # Full validation runner
```

### Generated Reports

```
.crewchief/
├── competitions/               # Competition runs
│   ├── comp-1234567890/       # Individual competition
│   │   ├── report.txt         # Competition report
│   │   ├── run-baseline/      # Baseline variant run
│   │   │   ├── agent-result.json
│   │   │   └── tool-usage.log
│   │   └── run-variant-a/     # Variant A run
│   │       ├── agent-result.json
│   │       └── tool-usage.log
│   └── ...
└── validations/                # Full validation runs
    ├── validation-1234567890/  # Individual validation
    │   ├── report.txt          # Validation report
    │   ├── statistics.json     # Statistical analysis
    │   └── tier-results/       # Per-tier results
    └── ...
```

## Best Practices

### 1. Start Small

```bash
# Test setup with one task
tsx src/search-optimization/examples/run-single-task.ts

# Then run small suite
tsx src/search-optimization/examples/run-tier1-sample.ts

# Finally run full validation
pnpm search-optimization:validate-full
```

### 2. Use Version Control

```bash
# Save variants as files
git add src/search-optimization/variants/
git commit -m "feat: add enhanced variant v2"

# Tag successful variants
git tag variant-v2-success
```

### 3. Monitor Costs

```bash
# Track API usage
echo "Competition run: $(date)" >> .crewchief/api-usage.log
echo "Estimated cost: $15-25" >> .crewchief/api-usage.log

# Review monthly
cat .crewchief/api-usage.log
```

### 4. Document Learnings

After each competition, document:
- What worked
- What failed
- Why the winner won
- Ideas for next iteration

### 5. Validate Before Deploying

```bash
# 1. Run validation
pnpm search-optimization:validate-full

# 2. Check statistical significance
# Look for p < 0.05 in reports

# 3. Cross-project validation
# Test on 2-3 different codebases

# 4. Deploy winner
# Update tool descriptions in packages/maproom-mcp/src/index.ts
```

## Further Reading

- **[Task Design Guide](./task-design-guide.md)** - How to create quality tasks
- **[Validation Guide](./validation-guide.md)** - How to validate tasks
- **[Benchmark Usage](./benchmark-usage.md)** - How to run benchmarks
- **[Project Archive](../../.agents/archive/projects/AGENTOPT_ai-agent-query-optimization/)** - Full project documentation

## Questions?

- Open an issue with the `search-optimization` tag
- Check existing discussions in the repository
- Review the project archive for architectural details

---

**Last Updated**: 2025-11-10
**Version**: 1.0.0
**Status**: Production-ready
