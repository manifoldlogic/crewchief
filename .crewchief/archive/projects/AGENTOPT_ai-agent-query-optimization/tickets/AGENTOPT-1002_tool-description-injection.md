# Ticket: AGENTOPT-1002 - Implement Tool Description Variant Injection via Worktrees

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Implement mechanism to provide different tool description variants to agents by creating worktrees of the crewchief repo and modifying the MCP tool description source code directly in each worktree.

## Background

For agent competitions to test tool description variants, each agent needs to see a different variant of the maproom `search` tool description.

**Key Insight**: Since this repo (crewchief) contains the MCP server code (`packages/maproom-mcp/`), we can leverage worktrees to create isolated copies with different tool descriptions:

1. Create worktree of crewchief repo
2. Modify `packages/maproom-mcp/src/tools/search.ts` in that worktree
3. Agent running in that worktree uses its local MCP server with the variant

**Benefits**:
- ✅ No SDK limitations (SDK doesn't support tool description overrides)
- ✅ No config file complexity
- ✅ Simple source code changes
- ✅ True isolation via worktrees (existing infrastructure)
- ✅ Easy to reproduce and debug
- ✅ Transparent - variant is in source code, not hidden in config

## Acceptance Criteria

- [x] Function to create variant worktree with modified tool description
- [x] Support for all variant types from AGENTOPT-0002
- [x] Verification test showing 2 agents with different descriptions
- [x] Documentation of worktree-based variant injection
- [x] Cleanup mechanism to remove variant worktrees after competition

## Technical Requirements

**Worktree Creation with Variant Injection**:
```typescript
// packages/cli/src/sdk/variant-injection.ts
import { Variant } from '../search-optimization/types'
import { WorktreeService } from '../git/worktree'
import { writeFileSync, readFileSync } from 'fs'
import { join } from 'path'

export async function createVariantWorktree(
  variant: Variant,
  basePath: string = process.cwd()
): Promise<{ path: string; cleanup: () => Promise<void> }> {
  // 1. Create worktree of crewchief repo
  const worktreeService = new WorktreeService(basePath)
  const branchName = `variant-${variant.id}-${Date.now()}`
  const worktree = await worktreeService.create(branchName)

  // 2. Modify tool description in worktree
  const toolFilePath = join(
    worktree.path,
    'packages/maproom-mcp/src/tools/search.ts'
  )

  // Read current file
  let content = readFileSync(toolFilePath, 'utf-8')

  // Replace description (find the description field and replace)
  content = content.replace(
    /description:\s*`[^`]+`/,
    `description: \`${variant.description}\``
  )

  // Write back
  writeFileSync(toolFilePath, content)

  // 3. Return worktree info with cleanup function
  return {
    path: worktree.path,
    cleanup: async () => {
      await worktreeService.remove(branchName)
    }
  }
}
```

**Integration with SDK Spawner**:
```typescript
// packages/cli/src/sdk/spawner.ts extension
export async function spawnAgentWithVariant(
  task: string,
  variant: Variant
): Promise<AgentResult> {
  // Create variant worktree
  const variantWorktree = await createVariantWorktree(variant)

  try {
    // Spawn agent in variant worktree
    const result = await spawnAgent({
      task,
      worktreePath: variantWorktree.path,
      permissionMode: 'bypassPermissions',
      hooks: {
        onToolUse: (event) => {
          // Capture search metrics
        }
      }
    })

    return result
  } finally {
    // Cleanup variant worktree
    await variantWorktree.cleanup()
  }
}
```

**MCP Tool Description Location**:
The tool description to modify is likely in:
- `packages/maproom-mcp/src/tools/search.ts` (if tools are separate files)
- OR `packages/maproom-mcp/src/index.ts` (if tools are defined inline)

Need to verify exact location and structure in implementation.

**Verification Test**:
```typescript
// packages/cli/tests/sdk/variant-injection.test.ts
describe('Variant Injection via Worktrees', () => {
  it('should create worktrees with different tool descriptions', async () => {
    const variantA = loadVariant('variant-a-detailed.json')
    const variantB = loadVariant('variant-b-minimal.json')

    const worktreeA = await createVariantWorktree(variantA)
    const worktreeB = await createVariantWorktree(variantB)

    // Verify tool descriptions are different
    const descA = await readToolDescription(worktreeA.path)
    const descB = await readToolDescription(worktreeB.path)

    expect(descA).toContain(variantA.description)
    expect(descB).toContain(variantB.description)
    expect(descA).not.toEqual(descB)

    // Cleanup
    await worktreeA.cleanup()
    await worktreeB.cleanup()
  })

  it('should spawn agents with different variants', async () => {
    const task = 'Search for authentication code'

    const resultA = await spawnAgentWithVariant(task, variantA)
    const resultB = await spawnAgentWithVariant(task, variantB)

    // Both should succeed
    expect(resultA.success).toBe(true)
    expect(resultB.success).toBe(true)

    // Both should have search metrics
    expect(resultA.metrics?.searchCount).toBeGreaterThan(0)
    expect(resultB.metrics?.searchCount).toBeGreaterThan(0)
  }, 120000)
})
```

## Implementation Notes

**Worktree Isolation**:
- Each variant gets its own worktree of the entire crewchief repo
- Changes to tool description are isolated to that worktree
- Agent spawned in that worktree uses its local MCP server
- No interference between concurrent agents

**Source Code Modification**:
- Need to locate the exact tool description in MCP server code
- Use regex or AST parsing to replace description reliably
- Preserve all other code structure
- Could also use template files if description is complex

**Build Requirements**:
- May need to rebuild MCP server in each worktree (check if needed)
- OR: If MCP server is TypeScript, might work without build
- Test to see if changes take effect immediately or need rebuild

**Cleanup Strategy**:
- Remove variant worktrees after competition completes
- Keep worktrees during competition for debugging
- Option to preserve worktrees with `--keep-variants` flag

**Alternative Approaches Considered**:
1. ❌ SDK tool description overrides - NOT SUPPORTED by SDK
2. ❌ Config files - Requires MCP server changes, adds complexity
3. ❌ Environment variables - Requires MCP server changes
4. ✅ **Worktree source modification** - Simple, transparent, leverages existing infrastructure

## Dependencies

- AGENTOPT-1001 (SDK integration) must be complete
- AGENTOPT-0002 (variant generation) provides variants to use
- Existing WorktreeService in crewchief CLI

## Risk Assessment

**Risk**: MCP server needs rebuild after source modification
**Mitigation**: Test if rebuild is needed, automate if necessary

**Risk**: Regex replacement breaks code
**Mitigation**: Use robust regex, add validation, test thoroughly

**Risk**: Worktree cleanup fails, leaves orphans
**Mitigation**: Use try/finally, add cleanup command for manual cleanup

**Risk**: Concurrent access to git repo
**Mitigation**: Worktrees already handle this, no additional concern

## Files/Packages Affected

- packages/cli/src/sdk/variant-injection.ts (new)
- packages/cli/src/sdk/spawner.ts (extend with spawnAgentWithVariant)
- packages/cli/tests/sdk/variant-injection.test.ts (new)
- packages/maproom-mcp/src/tools/search.ts (modified per variant, in worktrees)

## Planning References

- Replan Analysis: `../replan-analysis.md` (will be updated)
- Variant System: AGENTOPT-0002
- SDK Integration: AGENTOPT-1001
