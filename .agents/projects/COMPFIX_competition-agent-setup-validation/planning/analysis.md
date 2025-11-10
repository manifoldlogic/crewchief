# Analysis: Competition Agent Setup and Validation

## Problem Definition

The AGENTOPT genetic optimizer competition framework is currently broken due to inadequate agent environment setup and validation. Analysis of the ultra-run-1762742953256 reveals systematic failures across all generations:

### Observed Failures

**Generation 1-6 Results:**
- Search tool usage: 0% (0 searches across ALL variants)
- Task completion: 0% (no agent completed the task)
- Success rate: 0% (complete failure)
- Only efficiency scoring (91-97%) from basic operation timing

**Root Causes Identified:**

1. **Missing MCP Tools**: Agents don't have access to `mcp__maproom__search`, `mcp__maproom__open`, `mcp__maproom__context`, or `mcp__maproom__status` tools
2. **Permission Denials**: Agents blocked from reading files in their own worktrees
3. **Unscanned Worktrees**: Worktrees created but never indexed by maproom
4. **No Pre-flight Validation**: Tests start before environment is ready
5. **Silent Failures**: No feedback when setup fails, tests run anyway

### What We're Actually Testing

The competition framework has a dual purpose that's currently conflated:

**What we WANT to test:**
- Do better tool descriptions cause agents to CHOOSE semantic search over grep?
- Which description variants lead to higher search adoption?
- How do descriptions affect search query quality when agents DO use search?

**What we DON'T want to test:**
- How well agents use search when forced to
- Agent performance without tool availability
- Whether agents can work around missing tools

The current setup fails to test either properly because agents don't have the choice - the tools simply aren't available.

## Current State Analysis

### Competition Runner Flow (`packages/cli/src/search-optimization/competition-runner.ts`)

```typescript
export async function runCompetition(config: CompetitionConfig): Promise<CompetitionResult> {
  // 1. Create competition directory
  // 2. Copy variants to competition dir
  // 3. FOR EACH VARIANT:
  //    a. Create isolated worktree (via crewchief SDK)
  //    b. Inject variant description into worktree MCP config
  //    c. Spawn agent in worktree
  //    d. Wait for completion
  //    e. Evaluate results
  // 4. Compare results and determine winner
}
```

**Missing Steps:**
- ❌ Scan worktree for maproom indexing
- ❌ Wait for scan completion
- ❌ Verify MCP tools are accessible
- ❌ Test tool availability before agent spawn
- ❌ Fail fast if environment invalid

### Agent SDK Integration

The competition runner uses `@anthropic-ai/claude-agent-sdk` which:
- Creates isolated worktrees automatically
- Spawns Claude agents with isolated environments
- Provides tool access via MCP server configuration

**Current MCP Config Flow:**
1. SDK creates worktree at `.crewchief/worktrees/variant-{variant-id}-{timestamp}`
2. Variant injection modifies `.mcp.json` with custom tool description
3. Agent spawns with MCP config
4. ❌ **No verification that MCP server is running or accessible**
5. ❌ **No verification that worktree is indexed**

### Maproom Scanning Requirements

For semantic search to work:
1. Worktree must be scanned: `crewchief-maproom scan <worktree-path>`
2. Scan must complete successfully
3. Embeddings must be generated (or reused from base branch)
4. Database must contain indexed chunks for the worktree

**Current scan status:**
- Base branch: Likely already scanned
- Worktrees: Never scanned (explains 0% search usage)

**Scan time estimates:**
- First scan (base): 30-60s for typical codebase
- Subsequent scans (worktrees): 5-15s (reuses existing embeddings)
- Parallel scans: Possible if using same database (shared embeddings)

### Database Isolation

**Current architecture:**
- Single maproom database: `postgresql://maproom:maproom@localhost:5432/maproom`
- All scans write to same database
- Worktrees identified by: `repo_id` + `worktree` + `commit`

**Implications:**
- ✅ Multiple worktrees CAN share same database
- ✅ Embeddings are reused (fast subsequent scans)
- ✅ No isolation needed at database level
- ⚠️ Each agent needs unique MCP config pointing to same database

### Parallel Execution Constraints

**What needs to be sequential:**
1. Worktree creation (fast, ~1-2s each)
2. Worktree scanning (5-15s each)
3. Pre-flight validation (1-2s each)

**What can be parallel:**
1. Agent execution (30-180s each) - main bottleneck
2. Multiple scans to same database (PostgreSQL handles concurrency)

**Parallelization strategy:**
- Setup phase: Sequential (worktree + scan + validate) - 10-20s per variant
- Execution phase: Parallel (agent runs) - saves most time
- Total setup overhead for 12 variants: ~2-4 minutes (acceptable)

## Existing Solutions Research

### Competition Frameworks

**Similar tools:**
- MLflow experiments: Pre-validates environment before runs
- Kubernetes Jobs: Readiness probes before main container
- GitHub Actions: Setup steps separated from main workflow
- Jest: `beforeAll` hooks for test environment setup

**Common patterns:**
1. **Setup → Validate → Execute** pipeline
2. **Fail-fast on setup failures** (don't waste resources)
3. **Idempotent setup** (safe to retry)
4. **Detailed logging** for debugging failures

### Tool Availability Testing

**How other systems verify tool access:**

1. **Docker health checks:**
   ```dockerfile
   HEALTHCHECK --interval=5s --timeout=3s --retries=3 \
     CMD curl -f http://localhost:8080/health || exit 1
   ```

2. **MCP server readiness:**
   ```typescript
   async function waitForMCP(config: MCPConfig, timeout = 30000): Promise<void> {
     const start = Date.now()
     while (Date.now() - start < timeout) {
       try {
         const tools = await listMCPTools(config)
         if (tools.includes('mcp__maproom__search')) return
       } catch {}
       await new Promise(resolve => setTimeout(resolve, 1000))
     }
     throw new Error('MCP tools not available after ' + timeout + 'ms')
   }
   ```

3. **Database readiness:**
   ```typescript
   async function waitForIndex(repo: string, worktree: string): Promise<void> {
     const result = await execMaproom(['status', '--repo', repo, '--worktree', worktree])
     if (!result.includes('indexed: true')) {
       throw new Error('Worktree not indexed')
     }
   }
   ```

## Research Findings

### MCP Tool Discovery

From `@anthropic-ai/claude-agent-sdk` documentation:
- Agents receive tool list from MCP config on spawn
- Tool availability determined by `.mcp.json` in worktree
- No built-in health checks or readiness probes
- SDK assumes tools are available if in config

**Key insight:** We must validate BEFORE spawning agent, not during.

### Maproom Scan Idempotency

From `crates/maproom/src/commands/scan.rs`:
- `scan` command is idempotent
- Reuses existing embeddings for unchanged files
- Fast incremental updates (~5-10s for typical worktree)
- Safe to run multiple times
- Outputs JSON status on completion

**Validation approach:**
```bash
maproom scan --repo crewchief --worktree variant-abc --root /path
maproom status --repo crewchief --worktree variant-abc --json
# Check: indexed = true, chunk_count > 0
```

### Agent Worktree Permissions

From Claude Code Agents SDK analysis:
- Worktrees created with full read/write access for agent process
- Permission denials indicate incorrect worktree path or ownership
- SDK uses process UID for file operations

**Root cause of permission denials:**
- Agent spawned OUTSIDE worktree directory
- File paths resolved relative to wrong CWD
- Fix: Ensure agent CWD = worktree path

## Problem Space Summary

### Core Issue

The competition framework tests tool description quality by comparing agent behavior across variants. However, it currently fails to:

1. **Provide the tools being described** (maproom search unavailable)
2. **Prepare the environment for tool use** (worktrees not scanned)
3. **Validate environment readiness** (tests start regardless of setup state)
4. **Fail fast on setup errors** (silent failures waste API credits)

### Success Criteria

For the competition framework to work correctly:

1. ✅ All agents have access to maproom search tools
2. ✅ All worktrees are indexed and searchable
3. ✅ Setup failures are detected before agent spawn
4. ✅ Clear error messages explain setup failures
5. ✅ Parallel execution remains possible (minimize total time)
6. ✅ Test only what we intend: tool description effectiveness

### Constraints

1. **Must not force tool usage** - agents should choose based on descriptions
2. **Must validate availability** - but not require usage
3. **Must support parallel agent execution** - sequential setup is acceptable
4. **Must reuse embeddings** - shared database across worktrees
5. **Must fail fast** - don't run incomplete tests

## Next Steps

Based on this analysis, the architecture must address:

1. **Pre-flight validation framework** - Verify environment before agent spawn
2. **Scan orchestration** - Ensure all worktrees are indexed
3. **MCP configuration** - Provide search tools without forcing usage
4. **Error handling** - Fail fast with clear diagnostics
5. **Parallel-safe setup** - Sequential prep, parallel execution
