# AGENTOPT Project Replan - Analysis & Strategy

## User Requirements (Clarified)

### What Was Misunderstood
- **NOT**: Live user A/B testing with real production traffic
- **NOT**: Statistical significance with thousands of real user queries

### What Is Actually Wanted
- **Automated agent-based testing** using Claude Code Agents SDK
- **Agents execute real tasks** with different tool description variants
- **Success measured** by task completion quality
- **Reusable framework** for ongoing tuning as agent capabilities evolve
- **Leverage existing** crewchief CLI infrastructure (worktrees, competitions)
- **Strategic positioning** - what outcomes matter for this tool?
- **Future-looking** - transparent data collection for enterprise, ongoing adaptation

## Existing Infrastructure (Discovery)

### 1. CrewChief CLI - Agent Competition Framework ✅

**Location**: `packages/cli/src/orchestrator/competition.ts`

**Capabilities**:
- Start competitions with multiple agents on same task
- Each agent gets isolated worktree
- Automatic evaluation with scoring (0-1 scale)
- Winner selection based on highest score
- Auto-merge capability for winners

**Evaluation System** (`packages/cli/src/evaluation/checks.ts`):
```typescript
interface EvaluationSummary {
  results: CheckResult[]
  score: number // 0..1
}
```

Current checks:
- Environment validation (pnpm available)
- Agent event logging
- Config-driven quality checks (extensible!)

**Key Insight**: This is 80% of what we need! Just need to:
1. Extend evaluation to measure search task success
2. Integrate tool description variants
3. Define realistic search tasks

### 2. Worktree Management ✅

**Location**: `packages/cli/src/git/`

- Create isolated worktrees per agent
- Parallel execution without conflicts
- Clean merge back to main

### 3. Agent Orchestration ✅

**Commands available**:
```bash
crewchief spawn claude,gemini "task-description"
crewchief agent list
crewchief agent message <name> <message>
```

### 4. Message Bus ✅

**Location**: `packages/cli/src/bus/`

- JSONL event logging
- Agent communication
- Run tracking

## What's Missing

### 1. Agents SDK Integration
- Current orchestrator uses iTerm2 + CLI commands
- Need to integrate `@anthropic-ai/claude-agent-sdk` for programmatic control
- SDK provides `query()` with custom tool configurations

### 2. Tool Description Variant Injection ✅ SOLVED

**Solution**: Worktree-based source code modification

**Key Insight**: This repo IS the maproom-mcp repo! We can create worktrees of crewchief itself and modify the tool description source code directly.

**Approach**:
1. Create worktree of crewchief repo (contains `packages/maproom-mcp/`)
2. Modify `packages/maproom-mcp/src/tools/search.ts` directly in that worktree
3. Agent running in that worktree uses its local MCP server with variant

**Benefits**:
- ✅ No SDK limitations (SDK doesn't support tool description overrides)
- ✅ No config file complexity
- ✅ Simple source code changes
- ✅ True isolation via worktrees (existing infrastructure)
- ✅ Easy to reproduce and debug
- ✅ Transparent - variant visible in source code

**Alternative Approaches Discarded**:
- ❌ SDK tool description overrides - NOT SUPPORTED by SDK (verified in AGENTOPT-1001)
- ❌ Environment variables - Requires MCP server changes
- ❌ Per-worktree config files - Requires MCP server changes, adds complexity
- ❌ Runtime MCP server customization - Too complex

### 3. Search-Specific Evaluation
- Current checks are generic (env, events)
- Need search task success metrics:
  - Did agent find relevant code?
  - How many search attempts?
  - Quality of results used
  - Task completion success

### 4. Realistic Task Definitions
- Need library of real-world search tasks
- Examples:
  - "Find where authentication is implemented"
  - "Locate database connection code"
  - "Find all API endpoints"
  - "Understand error handling flow"

## Strategic Positioning Questions

### What Outcomes Actually Matter?

1. **Task Completion Rate**
   - Did agent complete the task correctly?
   - Most important metric

2. **Search Efficiency**
   - Number of searches to success
   - Time to relevant results
   - Query reformulation behavior

3. **Result Quality**
   - Were top results actually relevant?
   - Did agent use the results effectively?

4. **User Experience Proxies**
   - Low frustration (few failed searches)
   - High confidence (agent doesn't need to guess)
   - Fast resolution (fewer iterations)

### What Real-World Work Should We Mimic?

**Tier 1: Code Navigation** (Most Common)
- Find implementation of feature
- Locate error handling
- Understand architecture

**Tier 2: Bug Investigation**
- Find where error occurs
- Trace execution path
- Locate related code

**Tier 3: Feature Planning**
- Survey existing patterns
- Find similar implementations
- Understand constraints

**Decision**: Start with Tier 1, expand later

## Proposed Architecture

### High-Level Flow

```
1. Define Search Tasks
   ├─ Real-world scenarios
   ├─ Clear success criteria
   └─ Verifiable outcomes

2. Generate Tool Description Variants
   ├─ Use existing generator (AGENTOPT-0002)
   ├─ Start with 3-5 variants
   └─ Genetic mutations for iteration

3. Launch Agent Competition (via SDK)
   ├─ Spawn N agents with SDK
   ├─ Each agent gets different tool description
   ├─ Each agent gets isolated worktree
   └─ All agents work on same search task

4. Execute Tasks in Parallel
   ├─ Agents use maproom MCP tool
   ├─ Capture all tool calls
   ├─ Log search queries and results
   └─ Track task completion

5. Evaluate and Score
   ├─ Task completion (binary: success/fail)
   ├─ Search efficiency (# queries, time)
   ├─ Result relevance (check if used)
   └─ Calculate composite score

6. Determine Winner
   ├─ Highest score wins
   ├─ Winner becomes new baseline
   └─ Generate new mutated variants

7. Iterate (Optional)
   ├─ Use winner as parent
   ├─ Create new generation
   └─ Repeat competition
```

### Component Breakdown

#### A. Task Definition Framework
```typescript
interface SearchTask {
  id: string
  description: string  // "Find authentication implementation"
  acceptanceCriteria: string[]  // Specific files/functions to find
  context: string  // Background info
  successValidator: (result: AgentOutput) => boolean
}
```

#### B. SDK-Based Agent Spawner with Worktree Variant Injection
```typescript
import { query } from '@anthropic-ai/claude-agent-sdk'
import { WorktreeService } from './git/worktree'
import { writeFileSync, readFileSync } from 'fs'

async function spawnAgentWithVariant(
  task: SearchTask,
  variant: Variant
) {
  // 1. Create worktree with modified tool description
  const worktreeService = new WorktreeService(process.cwd())
  const branchName = `variant-${variant.id}-${Date.now()}`
  const worktree = await worktreeService.create(branchName)

  // 2. Modify tool description in worktree's MCP server source
  const toolFile = join(worktree.path, 'packages/maproom-mcp/src/tools/search.ts')
  let content = readFileSync(toolFile, 'utf-8')
  content = content.replace(
    /description:\s*`[^`]+`/,
    `description: \`${variant.description}\``
  )
  writeFileSync(toolFile, content)

  try {
    // 3. Spawn agent in variant worktree
    return query({
      prompt: task.description,
      options: {
        // Agent uses local MCP server from this worktree
        workingDirectory: worktree.path,
        // Hooks for capturing behavior
        hooks: {
          PostToolUse: [{
            hooks: [(event) => logToolUse(event)]
          }]
        }
      }
    })
  } finally {
    // 4. Cleanup variant worktree
    await worktreeService.remove(branchName)
  }
}
```

#### C. Extended Evaluation
```typescript
interface SearchEvaluationResult {
  taskCompleted: boolean
  foundTargets: string[]  // Files/functions found
  searchCount: number
  avgResultsPerSearch: number
  retryCount: number
  timeToSuccess: number
  score: number  // Composite: 0-1
}
```

#### D. Competition Orchestrator Extension
- Integrate with existing `CompetitionManager`
- Add search-specific scoring
- Hook into SDK agents instead of iTerm2 spawning

## Implementation Phases

### Phase 0: Foundation (Current - COMPLETE)
- ✅ Test query set (AGENTOPT-0001)
- ✅ Variant generator (AGENTOPT-0002)
- ✅ Testing harness basics (AGENTOPT-0003)
- ✅ Statistical analysis (AGENTOPT-0004)
- ✅ A/B infrastructure (AGENTOPT-0005) - needs reframing

### Phase 1: Agent SDK Integration (NEW)
**Goal**: Replace manual testing with SDK-driven agent spawning

1. Install and configure Agents SDK
2. Create SDK-based agent spawner
3. Inject tool description variants
4. Capture tool usage events
5. Test with single agent/variant

**Output**: Working SDK agent that uses custom tool description

### Phase 2: Competition Integration
**Goal**: Leverage existing competition framework

1. Extend `CompetitionManager` for search tasks
2. Create search-specific evaluation
3. Integrate SDK agents with competition lifecycle
4. Parallel execution in separate worktrees

**Output**: Multiple agents competing on same search task

### Phase 3: Task Library & Realism
**Goal**: Define meaningful search tasks

1. Create 10-15 realistic search scenarios
2. Define clear success criteria per task
3. Build validation logic
4. Test task difficulty calibration

**Output**: Validated task set with known solutions

### Phase 4: Evaluation & Iteration
**Goal**: Measure and improve

1. Run initial competition with baseline variants
2. Analyze results and scoring
3. Generate next generation from winners
4. Iterate and improve

**Output**: Improved tool description, validated framework

### Phase 5: CLI Integration (Optional)
**Goal**: Make framework accessible via CLI

1. Add `crewchief maproom:compete` command
2. Integrate with existing competition commands
3. Report generation
4. Dashboard for monitoring

**Output**: Easy-to-use CLI for ongoing optimization

## Future Considerations

### Ongoing Tuning Infrastructure
- Collect real usage data (with transparency)
- Periodic re-evaluation as agents evolve
- Version tracking for tool descriptions
- A/B testing for enterprise (when users exist)

### Enterprise Data Collection
**Requirements**:
- Explicit opt-in
- Clear privacy policy
- Data anonymization
- Usage disclosure in tool description

**Implementation** (future):
```typescript
// Tool description includes:
"Data Collection: This tool logs queries for quality improvement.
Opt-out: Set MAPROOM_NO_TELEMETRY=1 to disable."
```

### Adaptation Strategy
1. **Quarterly evaluation**: Re-run competitions with new Claude versions
2. **User feedback loop**: Collect real-world failure cases
3. **Task expansion**: Add new search patterns as product evolves
4. **Variant evolution**: Continuous genetic algorithm

## Recommendations

### Immediate Actions

1. **Archive current Phase 0 work**
   - Move AGENTOPT-0005 (A/B testing) to research
   - Keep AGENTOPT-0001 through 0004 (still useful)
   - Document why pivot happened

2. **Create new ticket set**
   - AGENTOPT-1001: Install & configure Agents SDK
   - AGENTOPT-1002: SDK-based agent spawner
   - AGENTOPT-1003: Extend competition framework
   - AGENTOPT-1004: Search task library
   - AGENTOPT-1005: Initial competition run
   - AGENTOPT-1006: Iteration framework

3. **Update project planning docs**
   - Revise architecture.md
   - Update plan.md with new phases
   - Document decision to pivot

### Don't Reinvent

**Reuse from CrewChief**:
- ✅ Competition orchestration
- ✅ Worktree management
- ✅ Evaluation framework (extend, don't replace)
- ✅ Message bus and logging

**New Components** (focused additions):
- SDK integration layer
- Tool description injection
- Search-specific metrics
- Task validation logic

### Quick Win Path

**Week 1**: Get one agent running one task with one variant
- Install SDK
- Create spawner
- Run simple search task
- Capture results

**Week 2**: Get competition working
- Multiple variants
- Parallel execution
- Basic scoring
- Winner selection

**Week 3**: Realistic tasks & iteration
- Task library
- Validation logic
- Run first real competition
- Analyze and iterate

## Key Insights

1. **80% solution exists** in crewchief CLI - don't rebuild
2. **SDK provides control** - use it instead of iTerm2 spawning
3. **Real tasks matter** - synthetic queries won't cut it
4. **Iterate quickly** - genetic algorithm is the long-term play
5. **Future-proof** - build for ongoing adaptation

## Questions for User

1. **Task realism**: What are the top 5 search tasks you actually perform?
2. **Success criteria**: How do we know an agent "succeeded" at a search task?
3. **Iteration strategy**: Run once and deploy winner, or continuous evolution?
4. **Enterprise timeline**: When do we need transparent data collection?
