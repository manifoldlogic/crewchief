# Ticket: AGENTOPT-1003 - Extend Competition Framework for Search Tasks

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Extend the existing `CompetitionManager` in crewchief CLI to support search-specific tasks and integrate with SDK-based agent spawning.

## Background

The crewchief CLI already has a competition framework at `packages/cli/src/orchestrator/competition.ts` that:
- Manages competitions between multiple agents
- Tracks participants and scores
- Evaluates and selects winners
- Supports auto-merge of winning work

**What's Missing**:
- Search-specific task definitions
- SDK agent integration (currently uses iTerm2)
- Search metrics capture
- Tool description variant management

## Acceptance Criteria

- [x] Extend `Competition` interface for search tasks
- [x] Integrate with SDK spawner (from AGENTOPT-1001)
- [x] Support variant assignment per participant
- [x] Capture search tool usage metrics
- [x] Update `CompetitionManager` to handle search competitions
- [x] Verification test with 2 agents on same search task

## Technical Requirements

**Extended Types** (add to `packages/cli/src/orchestrator/competition.ts`):
```typescript
import { SearchTask, Variant } from '../search-optimization/types'

export interface SearchCompetitionParticipant extends CompetitionParticipant {
  variant?: Variant          // Tool description variant
  searchMetrics?: {
    searchCount: number
    avgResultsPerSearch: number
    queriesIssued: string[]
    toolCallCount: number
    durationSeconds: number
  }
}

export interface SearchCompetition extends Competition {
  task: SearchTask           // From AGENTOPT-1004
  participants: SearchCompetitionParticipant[]
  metrics?: CompetitionMetrics
}
```

**Integration with SDK**:
```typescript
// packages/cli/src/orchestrator/search-competition.ts
import { spawnAgentWithVariant } from '../sdk/spawner'
import { CompetitionManager } from './competition'

export class SearchCompetitionManager extends CompetitionManager {
  async startSearchCompetition(
    task: SearchTask,
    variants: Variant[]
  ): Promise<SearchCompetition> {
    // Create competition
    const comp = this.createCompetition(task, variants)

    // Spawn agents with different variants
    for (const participant of comp.participants) {
      const worktree = await this.createWorktree(participant.id)

      // Spawn SDK agent with variant
      // Note: spawnAgentWithVariant creates its own variant worktree
      // and modifies the tool description source code
      const agent = await spawnAgentWithVariant(
        task.description,
        participant.variant!,
        {
          onToolUse: (event) => this.recordToolUse(participant.id, event),
          onComplete: (result) => this.recordCompletion(participant.id, result)
        }
      )

      participant.agentProcess = agent
      participant.worktreePath = worktree.path
    }

    return comp
  }

  private recordToolUse(participantId: string, event: ToolUseEvent) {
    // Capture search metrics
    if (event.toolName === 'search') {
      // Track query, results, etc.
    }
  }
}
```

**File Structure**:
```
packages/cli/src/orchestrator/
├── competition.ts (existing - extend types)
├── search-competition.ts (new - search-specific logic)
└── task.types.ts (existing - add SearchTask)
```

## Implementation Notes

**Extend, Don't Replace**:
- Keep existing `CompetitionManager` intact
- Create `SearchCompetitionManager` that extends it
- Reuse worktree management, scoring, merging logic

**Metrics Capture** (Depends on SDK event model from AGENTOPT-1001):
- Hook into SDK's tool use events (verify event name/structure)
- Track all maproom tool calls
- Store in participant's `searchMetrics`
- Feed into evaluation (AGENTOPT-1005)

**NOTE**: The exact event hook mechanism depends on SDK capabilities researched in AGENTOPT-1001.
- If SDK provides `PostToolUse`: use that
- If SDK provides generic message stream: filter for tool use events
- If SDK doesn't provide events: may need to parse agent logs

**Parallel Execution**:
- Each agent runs in isolated worktree (existing capability)
- SDK handles concurrent queries
- No manual process management needed

## Dependencies

- AGENTOPT-1001 (SDK integration) - provides spawner
- AGENTOPT-1002 (variant injection) - provides variant assignment
- AGENTOPT-1004 (task library) - provides SearchTask definitions

## Risk Assessment

**Risk**: SDK agents interfere with each other
**Mitigation**: Worktree isolation (already works), test thoroughly

**Risk**: Breaking existing competition features
**Mitigation**: Extend via subclass, don't modify CompetitionManager

**Risk**: Metrics capture overhead slows agents
**Mitigation**: Lightweight event logging, async processing

## Files/Packages Affected

- packages/cli/src/orchestrator/competition.ts (extend types)
- packages/cli/src/orchestrator/search-competition.ts (new)
- packages/cli/src/orchestrator/task.types.ts (add SearchTask)
- packages/cli/tests/orchestrator/search-competition.test.ts (new)

## Planning References

- Existing Competition: `packages/cli/src/orchestrator/competition.ts`
- Replan Analysis: `../replan-analysis.md`
