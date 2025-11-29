# AGENTOPT Tickets Review Report

**Date**: January 7, 2025
**Reviewer**: Claude Code
**Project**: AGENTOPT - AI Agent Query Optimization
**Review Scope**: All tickets in `.crewchief/projects/AGENTOPT_ai-agent-query-optimization/tickets/`

---

## Executive Summary

**Total Tickets Found**: 25
**Tickets to Keep (Phase 1)**: 7
**Tickets to Remove (Pre-Pivot)**: 18
**Critical Issues Found**: 4
**Recommendations**: Cleanup required before implementation

### Status: ⚠️ **ACTION REQUIRED**

The ticket directory contains tickets from **THREE different strategic plans** that conflict with each other. The January 2025 strategic pivot created new Phase 1 tickets (1001-1007) but left old tickets from the pre-pivot plan intact, creating confusion about which approach to follow.

---

## Critical Issues

### 1. ❌ **Duplicate Ticket Numbers** (CRITICAL)

Multiple tickets share the same numbers with completely different purposes:

| Ticket Number | Pre-Pivot Name | Post-Pivot Name | Conflict |
|---------------|----------------|-----------------|----------|
| **AGENTOPT-1001** | design-enhanced-tool-description | agents-sdk-integration | YES |
| **AGENTOPT-1002** | implement-enhanced-tool-description | tool-description-injection | YES |
| **AGENTOPT-1003** | testing-and-validation | competition-framework-extension | YES |
| **AGENTOPT-1004** | code-review-approval | search-task-library | YES |
| **AGENTOPT-1005** | deploy-enhanced-description-production | evaluation-framework-extension | YES |
| **AGENTOPT-1006** | monitoring-validation | competition-runner | YES |

**Impact**: HIGH - Executing `/single-ticket AGENTOPT-1001` is ambiguous
**Root Cause**: Old tickets not removed during strategic pivot commit (b62ead8)

### 2. ❌ **Orphaned Pre-Pivot Tickets**

The following tickets are from the **old strategic plan** (production tool description optimization) that was replaced in January 2025:

**Pre-Pivot Phase 1** (Tool Description Enhancement):
- `AGENTOPT-1001_design-enhanced-tool-description.md` - Design tool description
- `AGENTOPT-1002_implement-enhanced-tool-description.md` - Implement description
- `AGENTOPT-1003_testing-and-validation.md` - Manual testing
- `AGENTOPT-1004_code-review-approval.md` - Code review
- `AGENTOPT-1005_deploy-enhanced-description-production.md` - Production deployment
- `AGENTOPT-1006_monitoring-validation.md` - Monitor metrics

**Pre-Pivot Phase 2** (Server-Side Preprocessing):
- `AGENTOPT-2001_query-preprocessing-module.md`
- `AGENTOPT-2002_metadata-score-boosting.md`
- `AGENTOPT-2003_phase2-testing-deployment.md`

**Pre-Pivot Phase 3** (LLM Fallback):
- `AGENTOPT-3001_llm-fallback-implementation.md`
- `AGENTOPT-3002_phase3-testing-deployment.md`

**Impact**: HIGH - These are no longer aligned with project strategy
**Evidence**: README.md documents strategic pivot, these tickets reference old approach

### 3. ⚠️ **Obsolete Phase 0 Infrastructure Ticket**

`AGENTOPT-0006_phase-0-integration-first-experiment.md` is an integration ticket for running the first A/B testing experiment.

**Status**: Obsolete - This was the archive/cleanup ticket removed during pivot
**Impact**: MEDIUM - Not aligned with SDK-driven approach
**Note**: User explicitly requested removal of A/B testing work in previous session

### 4. ⚠️ **File Path References to Moved Documents**

**Multiple tickets** reference analysis documents with incorrect paths:

```
INCORRECT: .crewchief/work-in-progress/AGENTOPT-replan-analysis.md
CORRECT:   .crewchief/projects/AGENTOPT_ai-agent-query-optimization/replan-analysis.md

INCORRECT: .crewchief/work-in-progress/search-tasks-deep-thinking.md
CORRECT:   .crewchief/projects/AGENTOPT_ai-agent-query-optimization/search-tasks-analysis.md
```

**Affected Tickets**: AGENTOPT-1001, 1002, 1003, 1004
**Impact**: LOW - Documentation references only, doesn't affect code

---

## Ticket-by-Ticket Analysis

### Phase 0 Tickets (Infrastructure - COMPLETE)

| Ticket | Status | Notes |
|--------|--------|-------|
| AGENTOPT-0001 | ✅ **KEEP** | Test query set - foundation for Phase 1 tasks |
| AGENTOPT-0002 | ✅ **KEEP** | Variant generation system - used by Phase 1 |
| AGENTOPT-0003 | ✅ **KEEP** | Testing harness - may inform Phase 1 validators |
| AGENTOPT-0004 | ✅ **KEEP** | Statistical analysis - needed for genetic iteration |
| AGENTOPT-0005 | ✅ **KEEP** | Production A/B testing - COMPLETED but code removed |
| AGENTOPT-0006 | ❌ **DELETE** | Phase 0 integration - obsolete, A/B testing approach |

**Recommendation**: Keep Phase 0 tickets as completed work (historical record), delete AGENTOPT-0006.

---

### Phase 1 Tickets (SDK Approach - CURRENT FOCUS)

#### ✅ AGENTOPT-1001: Agents SDK Integration

**Files**: `AGENTOPT-1001_agents-sdk-integration.md` ← **CORRECT**
**Duplicate**: `AGENTOPT-1001_design-enhanced-tool-description.md` ← **DELETE**

**Assessment**: SOUND ✓
- Aligns with strategic pivot to SDK-driven approach
- Clear acceptance criteria
- No breaking changes to existing functionality
- Adds new `packages/cli/src/sdk/` module

**Issues Found**:
- ⚠️ References outdated path `.crewchief/work-in-progress/AGENTOPT-replan-analysis.md`
- ✅ No conflicts with existing code (SDK not yet integrated)

**Dependencies**: None (foundational)

**Recommendations**:
1. ✅ **KEEP** this ticket
2. ❌ **DELETE** duplicate `AGENTOPT-1001_design-enhanced-tool-description.md`
3. 🔧 **FIX** documentation reference paths

---

#### ✅ AGENTOPT-1002: Tool Description Variant Injection

**Files**: `AGENTOPT-1002_tool-description-injection.md` ← **CORRECT**
**Duplicate**: `AGENTOPT-1002_implement-enhanced-tool-description.md` ← **DELETE**

**Assessment**: SOUND with CONCERNS ⚠️

**Strengths**:
- Correctly identifies need for runtime variant injection
- Proposes multiple approaches (SDK override, config file, environment variables)
- Acknowledges research needed for SDK capabilities

**Concerns**:
1. **Assumes SDK supports tool description overrides** - This needs verification
   - SDK docs may not support `mcpServers.toolOverrides`
   - May require MCP server code changes (contradicts design goal)

2. **Alternative approach may require MCP server changes**:
   ```typescript
   // If SDK doesn't support overrides, may need:
   // packages/maproom-mcp/src/index.ts modifications
   ```

3. **Risk**: "No MCP server code changes required" may be unrealistic
   - MCP server defines tool schemas at startup (lines 114-264 in `src/index.ts`)
   - Dynamic per-request overrides not standard in MCP protocol
   - May need per-agent MCP server instances or config file approach

**Integration with Existing Code**:
- ✅ Correctly references AGENTOPT-0002 variant system
- ✅ Doesn't break existing MCP server functionality
- ⚠️ May require more MCP server work than anticipated

**Recommendations**:
1. ✅ **KEEP** this ticket
2. ❌ **DELETE** duplicate `AGENTOPT-1002_implement-enhanced-tool-description.md`
3. 🔧 **ADD** acceptance criterion: "Document SDK limitations and chosen approach"
4. ⚠️ **RESEARCH FIRST**: Verify SDK tool override capabilities before implementation
5. 🔧 **UPDATE** ticket to acknowledge MCP server changes may be needed

---

#### ✅ AGENTOPT-1003: Competition Framework Extension

**Files**: `AGENTOPT-1003_competition-framework-extension.md` ← **CORRECT**
**Duplicate**: `AGENTOPT-1003_testing-and-validation.md` ← **DELETE**

**Assessment**: SOUND with INTEGRATION CONCERNS ⚠️

**Strengths**:
- Correctly identifies existing `CompetitionManager` at `packages/cli/src/orchestrator/competition.ts`
- Proposes extension via subclass (good design)
- Integrates with SDK (AGENTOPT-1001) and variants (AGENTOPT-1002)
- Preserves existing competition functionality

**Concerns**:
1. **SearchTask type not yet defined** - Depends on AGENTOPT-1004
   - Ticket proposes importing from `../search-optimization/types`
   - That module doesn't exist yet
   - **Fix**: Add explicit dependency note

2. **Metrics capture approach unclear**:
   ```typescript
   onToolUse: (event) => this.recordToolUse(participant.id, event)
   ```
   - Assumes SDK provides `PostToolUse` hook
   - Assumes `ToolUseEvent` structure
   - **Risk**: SDK may not expose tool use events in this format

3. **Parallel execution assumptions**:
   - Ticket states "SDK handles concurrent queries"
   - This needs verification - SDK may be single-agent per process
   - May need multiple SDK processes for parallel execution

**Integration with Existing Code**:
- ✅ Correctly extends `CompetitionManager` interface (lines 13-36 of competition.ts)
- ✅ Reuses worktree management (`WorktreeService`)
- ✅ Follows existing patterns (inherit, don't replace)
- ⚠️ May conflict with current `AgentRunner` (uses iTerm2, not SDK)

**Recommendations**:
1. ✅ **KEEP** this ticket
2. ❌ **DELETE** duplicate `AGENTOPT-1003_testing-and-validation.md`
3. 🔧 **ADD** explicit dependency: "Requires AGENTOPT-1004 (SearchTask definition)"
4. 🔧 **UPDATE** to clarify SDK event hook requirements
5. ⚠️ **DEFER** parallel execution until SDK capabilities verified

---

#### ✅ AGENTOPT-1004: Search Task Library

**Files**: `AGENTOPT-1004_search-task-library.md` ← **CORRECT**
**Duplicate**: `AGENTOPT-1004_code-review-approval.md` ← **DELETE**

**Assessment**: EXCELLENT ✓✓✓

**Strengths**:
- Comprehensive task type definitions
- Clear 3-level scoring model (Search Quality 40% + Task Completion 40% + Efficiency 20%)
- Realistic task examples based on actual crewchief codebase
- No dependencies on other tickets (foundational)
- No impact on existing functionality (adds new module)

**Example Task Quality**:
```typescript
TASK_FIND_WORKTREE_CREATION: {
  searchTarget: { type: 'file', path: 'packages/cli/src/git/worktree.ts' },
  followUpTask: { type: 'explanation', ... }
  validator: { mentionsFiles: [...], mentionsPattern: /worktree|git/i }
}
```
✅ This file actually exists
✅ Validation criteria are objective
✅ Task difficulty appropriate

**Concerns**:
1. **Validator implementation complexity** - May be more complex than shown
   - File change detection needs git integration
   - Pattern matching needs robust parsing
   - **Risk**: False positives/negatives in validation

2. **AgentOutput interface not defined**:
   ```typescript
   successValidator: (result: AgentOutput) => TaskScore
   ```
   - Ticket doesn't define `AgentOutput` structure
   - Needs coordination with AGENTOPT-1005 (evaluation framework)

**Integration with Existing Code**:
- ✅ Tasks reference real files in codebase
- ✅ New module, no conflicts with existing code
- ✅ Can be developed and tested independently

**Recommendations**:
1. ✅ **KEEP** this ticket - highest quality of all Phase 1 tickets
2. ❌ **DELETE** duplicate `AGENTOPT-1004_code-review-approval.md`
3. 🔧 **ADD** `AgentOutput` interface definition to ticket
4. 🔧 **ADD** note: "Coordinate with AGENTOPT-1005 for output format"

---

#### ✅ AGENTOPT-1005: Evaluation Framework Extension

**Files**: `AGENTOPT-1005_evaluation-framework-extension.md` ← **CORRECT**
**Duplicate**: `AGENTOPT-1005_deploy-enhanced-description-production.md` ← **DELETE**

**Assessment**: SOUND with INTEGRATION NEEDS ⚠️

**Strengths**:
- Correctly identifies existing evaluation framework at `packages/cli/src/evaluation/checks.ts`
- Proposes extension (not replacement) of `EvaluationSummary`
- Integrates with task validators (AGENTOPT-1004)
- Preserves existing `runDefaultChecks()` functionality

**Concerns**:
1. **Tool log format assumptions**:
   ```typescript
   interface ToolUseLog {
     timestamp: number
     toolName: string
     arguments: Record<string, any>
     result: any
   }
   ```
   - Assumes SDK logs tool usage to files
   - Assumes specific log format
   - **Risk**: SDK may not provide this logging automatically
   - **Fix**: May need custom logging in AGENTOPT-1001

2. **Integration with existing evaluation**:
   ```typescript
   interface SearchEvaluationSummary extends EvaluationSummary {
     task: SearchTask
     taskScore: TaskScore
     searchMetrics: {...}
   }
   ```
   - ✅ Good: Extends, doesn't replace
   - ⚠️ Concern: `runDefaultChecks()` returns `EvaluationSummary`, not `SearchEvaluationSummary`
   - **Fix**: Need adapter or separate evaluation path

**Integration with Existing Code**:
- ✅ Correctly extends `packages/cli/src/evaluation/checks.ts`
- ✅ Follows existing scoring pattern (0-1 range)
- ⚠️ May need separate `runSearchTaskEvaluation()` function

**Recommendations**:
1. ✅ **KEEP** this ticket
2. ❌ **DELETE** duplicate `AGENTOPT-1005_deploy-enhanced-description-production.md`
3. 🔧 **ADD** acceptance criterion: "Document tool log format requirements"
4. 🔧 **UPDATE** to show separate evaluation path alongside `runDefaultChecks()`
5. 🔧 **COORDINATE** with AGENTOPT-1001 for tool logging implementation

---

#### ✅ AGENTOPT-1006: Competition Runner

**Files**: `AGENTOPT-1006_competition-runner.md` ← **CORRECT**
**Duplicate**: `AGENTOPT-1006_monitoring-validation.md` ← **DELETE**

**Assessment**: SOUND - Integration Orchestrator ✓

**Strengths**:
- Clear orchestration of all prior components
- Integrates SDK (1001), variants (1002), competition (1003), tasks (1004), evaluation (1005)
- Provides end-to-end competition execution
- Includes reporting and winner selection

**Concerns**:
1. **Heavy dependency load** - Requires all previous tickets working
   - Dependencies: AGENTOPT-1001, 1002, 1003, 1004, 1005
   - **Risk**: Any upstream issues block this ticket
   - **Mitigation**: Defer until all dependencies complete

2. **Agent lifecycle assumptions**:
   ```typescript
   for await (const message of agent) {
     if (message.type === 'tool_use') {...}
   }
   ```
   - Assumes SDK returns async iterable
   - Assumes specific message format
   - **Risk**: SDK may use different patterns (callbacks, events, promises)

3. **Parallel execution complexity**:
   ```typescript
   if (config.parallelExecution) {
     await Promise.all(competition.participants.map(p => executeParticipant(p)))
   }
   ```
   - Each agent spawns separate process
   - Multiple worktrees active simultaneously
   - **Risk**: Resource contention (CPU, memory, disk I/O)
   - **Recommendation**: Start with sequential, add parallel later

**Integration with Existing Code**:
- ✅ Uses existing `CompetitionManager` structure
- ✅ Leverages worktree isolation
- ⚠️ Assumes all prior tickets implemented correctly

**Recommendations**:
1. ✅ **KEEP** this ticket
2. ❌ **DELETE** duplicate `AGENTOPT-1006_monitoring-validation.md`
3. 🔧 **ADD** phased approach: "Sequential first, parallel in follow-up"
4. 🔧 **ADD** acceptance criterion: "Document SDK message/event format used"
5. ⚠️ **DEFER** until AGENTOPT-1001 through 1005 complete and tested

---

#### ✅ AGENTOPT-1007: Genetic Iteration Framework

**Files**: `AGENTOPT-1007_genetic-iteration-framework.md` ← **CORRECT**
**No duplicates** ✓

**Assessment**: SOUND - Final Integration ✓

**Strengths**:
- Implements genetic algorithm approach (winner → baseline → mutate → compete)
- Convergence detection prevents infinite loops
- Clear generation tracking and history
- Reuses mutation logic from AGENTOPT-0002

**Concerns**:
1. **Complex orchestration** - Requires all prior tickets working perfectly
   - Uses competition runner (1006)
   - Uses mutator (0002)
   - Uses task library (1004)
   - **Risk**: Cascading failures from any dependency

2. **Convergence criteria may be too simple**:
   ```typescript
   if (Math.abs(improvement) < config.convergenceThreshold) {
     // Stop iteration
   }
   ```
   - Single threshold may not detect plateaus
   - May stop too early or run too long
   - **Recommendation**: Add "no improvement for N consecutive generations" rule

3. **Population management**:
   ```typescript
   // 1. Keep best variant (elitism)
   nextGen.push(sorted[0])
   // 2. Crossover top 2
   // 3. Mutate best variant
   ```
   - Genetic diversity may be low (all from best variant)
   - **Risk**: Gets stuck in local optimum
   - **Recommendation**: Add occasional random mutations

**Integration with Existing Code**:
- ✅ Reuses AGENTOPT-0002 mutation system
- ✅ Extends AGENTOPT-1006 competition runner
- ✅ No conflicts with existing code

**Recommendations**:
1. ✅ **KEEP** this ticket
2. 🔧 **ADD** convergence criterion: "OR no improvement for 3 consecutive generations"
3. 🔧 **ADD** diversity mechanism: "Include 1 random variant per generation"
4. ⚠️ **DEFER** until AGENTOPT-1006 working and tested
5. 🔧 **ADD** early stopping: "Max 10 generations for MVP"

---

### Pre-Pivot Tickets (OBSOLETE)

All tickets from the old strategic plan should be removed:

#### ❌ Phase 1 (Tool Description) - DELETE ALL
- `AGENTOPT-1001_design-enhanced-tool-description.md`
- `AGENTOPT-1002_implement-enhanced-tool-description.md`
- `AGENTOPT-1003_testing-and-validation.md`
- `AGENTOPT-1004_code-review-approval.md`
- `AGENTOPT-1005_deploy-enhanced-description-production.md`
- `AGENTOPT-1006_monitoring-validation.md`

**Reason**: Replaced by SDK-driven approach in January 2025 pivot

#### ❌ Phase 2 (Server-Side) - DELETE ALL
- `AGENTOPT-2001_query-preprocessing-module.md`
- `AGENTOPT-2002_metadata-score-boosting.md`
- `AGENTOPT-2003_phase2-testing-deployment.md`

**Reason**: Server-side optimization deferred until Phase 1 SDK approach completes

#### ❌ Phase 3 (LLM Fallback) - DELETE ALL
- `AGENTOPT-3001_llm-fallback-implementation.md`
- `AGENTOPT-3002_phase3-testing-deployment.md`

**Reason**: Optional future work, not part of current plan

---

## Breaking Changes Assessment

### ✅ **No Breaking Changes Found**

All Phase 1 tickets add new functionality without modifying existing systems:

| Component | Impact | Risk |
|-----------|--------|------|
| **Competition Framework** | Extended via subclass | LOW - existing code untouched |
| **Evaluation System** | Extended types, new functions | LOW - existing checks preserved |
| **Agent Spawning** | New SDK module alongside existing | LOW - iTerm2 agent runner still works |
| **Worktree Management** | No changes | NONE |
| **MCP Server** | May need variant injection support | MEDIUM - if SDK doesn't support overrides |

**Key Design Principle**: All tickets follow "extend, don't replace" pattern ✓

---

## Dependency Graph Review

```
Foundation (No Dependencies):
├─ AGENTOPT-1001 (SDK Integration)
└─ AGENTOPT-1004 (Search Task Library)

Layer 2 (Depends on Foundation):
├─ AGENTOPT-1002 (Variant Injection) ← requires 1001
└─ AGENTOPT-1005 (Evaluation Extension) ← requires 1004

Layer 3 (Depends on Layer 2):
└─ AGENTOPT-1003 (Competition Framework) ← requires 1001, 1002, 1004

Layer 4 (Integration):
└─ AGENTOPT-1006 (Competition Runner) ← requires 1001, 1002, 1003, 1004, 1005

Layer 5 (Optimization):
└─ AGENTOPT-1007 (Genetic Iteration) ← requires 1006
```

**Assessment**: ✅ Dependency graph is sound
**Risk**: Sequential dependencies mean delays cascade
**Mitigation**: Implement foundation tickets (1001, 1004) first, test thoroughly

---

## Recommended Actions

### 🔴 CRITICAL (Do Before Implementation)

1. **Delete duplicate pre-pivot tickets**:
   ```bash
   cd .crewchief/projects/AGENTOPT_ai-agent-query-optimization/tickets/

   # Delete old Phase 1 tickets
   rm AGENTOPT-1001_design-enhanced-tool-description.md
   rm AGENTOPT-1002_implement-enhanced-tool-description.md
   rm AGENTOPT-1003_testing-and-validation.md
   rm AGENTOPT-1004_code-review-approval.md
   rm AGENTOPT-1005_deploy-enhanced-description-production.md
   rm AGENTOPT-1006_monitoring-validation.md

   # Delete Phase 2 tickets
   rm AGENTOPT-2001_query-preprocessing-module.md
   rm AGENTOPT-2002_metadata-score-boosting.md
   rm AGENTOPT-2003_phase2-testing-deployment.md

   # Delete Phase 3 tickets
   rm AGENTOPT-3001_llm-fallback-implementation.md
   rm AGENTOPT-3002_phase3-testing-deployment.md

   # Delete obsolete Phase 0 integration ticket
   rm AGENTOPT-0006_phase-0-integration-first-experiment.md
   ```

2. **Fix documentation reference paths** in Phase 1 tickets:
   - Update `AGENTOPT-1001`: `.crewchief/work-in-progress/AGENTOPT-replan-analysis.md` → `../replan-analysis.md`
   - Update `AGENTOPT-1002`: Same path fix
   - Update `AGENTOPT-1003`: Same path fix
   - Update `AGENTOPT-1004`: `.crewchief/work-in-progress/search-tasks-deep-thinking.md` → `../search-tasks-analysis.md`

### 🟡 HIGH PRIORITY (Before Starting AGENTOPT-1001)

3. **Research SDK capabilities**:
   - Verify tool description override support
   - Verify tool use event hooks
   - Verify async iteration vs callback patterns
   - Document findings in `packages/cli/src/sdk/README.md`

4. **Update AGENTOPT-1002** to acknowledge:
   - May require MCP server changes if SDK doesn't support overrides
   - Alternative approaches if SDK limitations found

5. **Add AgentOutput interface** to AGENTOPT-1004:
   ```typescript
   interface AgentOutput {
     searchResults: SearchResult[]
     workResult: WorkResult
     searchCount: number
     toolCallCount: number
     durationSeconds: number
   }
   ```

### 🟢 MEDIUM PRIORITY (During Implementation)

6. **Add phased approach to AGENTOPT-1006**:
   - Phase A: Sequential execution only
   - Phase B: Parallel execution (after testing)

7. **Enhance convergence detection in AGENTOPT-1007**:
   - Add: "No improvement for 3 consecutive generations" rule
   - Add: "Max 10 generations" limit for MVP
   - Add: Random variant diversity mechanism

8. **Create SDK research ticket** (AGENTOPT-1008):
   - Investigate SDK capabilities before AGENTOPT-1001 implementation
   - Document findings for variant injection approach
   - Determine if MCP server changes needed

---

## Risk Summary

### HIGH RISK ⚠️
1. **SDK may not support tool description overrides** → May require MCP server modifications
2. **SDK event model unclear** → May need different metrics capture approach
3. **Cascading dependencies** → Delays in foundation tickets block entire project

### MEDIUM RISK ⚠️
4. **Validators may have false positives/negatives** → Need thorough testing
5. **Parallel execution resource contention** → Recommend sequential-first approach
6. **Genetic algorithm local optimum** → Need diversity mechanisms

### LOW RISK ✓
7. **Breaking existing functionality** → All tickets extend, don't replace
8. **Dependency conflicts** → New modules, isolated from existing code

---

## Final Recommendation

### ✅ **APPROVE PHASE 1 TICKETS** (After Cleanup)

The Phase 1 ticket set (AGENTOPT-1001 through 1007) is **well-designed and technically sound** with these caveats:

1. **Critical**: Delete all pre-pivot duplicate tickets immediately
2. **Important**: Research SDK capabilities before starting AGENTOPT-1001
3. **Recommended**: Update documentation paths and add missing interface definitions

### Execution Order (After Cleanup)

```
Week 1: Foundation
├─ SDK Research (new ticket) - 2 days
├─ AGENTOPT-1001 (SDK Integration) - 2 days
└─ AGENTOPT-1004 (Task Library) - 3 days

Week 2: Integration
├─ AGENTOPT-1002 (Variant Injection) - 2 days
├─ AGENTOPT-1005 (Evaluation Extension) - 2 days
└─ AGENTOPT-1003 (Competition Framework) - 3 days

Week 3: Orchestration
├─ AGENTOPT-1006 (Competition Runner) - 3 days
└─ AGENTOPT-1007 (Genetic Iteration) - 2 days
```

---

## Appendix: Complete Ticket Inventory

### ✅ KEEP (13 tickets)

**Phase 0 (Complete)**:
- AGENTOPT-0001_create-test-query-set.md
- AGENTOPT-0002_variant-generation-system.md
- AGENTOPT-0003_implement-automated-testing-harness.md
- AGENTOPT-0004_statistical-analysis-framework.md
- AGENTOPT-0005_production-ab-testing-infrastructure.md

**Phase 1 (Current)**:
- AGENTOPT-1001_agents-sdk-integration.md
- AGENTOPT-1002_tool-description-injection.md
- AGENTOPT-1003_competition-framework-extension.md
- AGENTOPT-1004_search-task-library.md
- AGENTOPT-1005_evaluation-framework-extension.md
- AGENTOPT-1006_competition-runner.md
- AGENTOPT-1007_genetic-iteration-framework.md

### ❌ DELETE (12 tickets)

**Pre-Pivot Phase 0**:
- AGENTOPT-0006_phase-0-integration-first-experiment.md

**Pre-Pivot Phase 1**:
- AGENTOPT-1001_design-enhanced-tool-description.md
- AGENTOPT-1002_implement-enhanced-tool-description.md
- AGENTOPT-1003_testing-and-validation.md
- AGENTOPT-1004_code-review-approval.md
- AGENTOPT-1005_deploy-enhanced-description-production.md
- AGENTOPT-1006_monitoring-validation.md

**Pre-Pivot Phase 2**:
- AGENTOPT-2001_query-preprocessing-module.md
- AGENTOPT-2002_metadata-score-boosting.md
- AGENTOPT-2003_phase2-testing-deployment.md

**Pre-Pivot Phase 3**:
- AGENTOPT-3001_llm-fallback-implementation.md
- AGENTOPT-3002_phase3-testing-deployment.md

---

**Review Complete**: January 7, 2025
**Next Action**: Cleanup duplicate tickets, then proceed with Phase 1 implementation
