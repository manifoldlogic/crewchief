# Ticket: AGENTOPT-1002 - Implement Tool Description Variant Injection

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Implement mechanism to inject different tool description variants into agents spawned with the SDK, enabling each agent in a competition to use a different variant without code changes.

## Background

For agent competitions to test tool description variants, each agent needs to see a different variant of the maproom `search` tool description. This must happen at runtime without modifying the MCP server code.

**Challenge**: The SDK needs to override the tool description that comes from the MCP server.

**Solution Options**:
1. SDK `mcpServers` option with description overrides (if supported)
2. Per-worktree MCP server configuration files
3. Environment variables read by MCP server
4. Runtime MCP server customization

## Acceptance Criteria

- [ ] Mechanism to specify tool description variant per agent
- [ ] Variant injection works with SDK-spawned agents
- [ ] Multiple agents can run simultaneously with different variants
- [ ] Verification test showing 2 agents with different descriptions
- [ ] Documentation of injection mechanism
- [ ] **SDK LIMITATION ACKNOWLEDGED**: Document if MCP server changes are required (if SDK doesn't support overrides)

## Technical Requirements

**Preferred Approach** (SDK-based if supported):
```typescript
import { query } from '@anthropic-ai/claude-agent-sdk'

export async function spawnAgentWithVariant(
  task: string,
  variant: Variant,
  worktreePath: string
) {
  return query({
    prompt: task,
    options: {
      workingDirectory: worktreePath,

      // Option 1: SDK tool description override (check if supported)
      mcpServers: {
        maproom: {
          toolOverrides: {
            search: {
              description: variant.description
            }
          }
        }
      }
    }
  })
}
```

**Alternative Approach** (Config file per worktree):
```typescript
// Write variant-specific config to worktree
const configPath = path.join(worktreePath, '.maproom-config.json')
writeFileSync(configPath, JSON.stringify({
  toolDescriptions: {
    search: variant.description
  }
}))

// MCP server reads this config at runtime
```

**Integration with Variants** (from AGENTOPT-0002):
```typescript
import { loadVariant } from '../test/tool-description-optimization/variants'

const variant = loadVariant('variant-a-detailed.json')
const agent = await spawnAgentWithVariant(task, variant, worktreePath)
```

**Verification Test**:
1. Load 2 different variants
2. Spawn 2 agents simultaneously
3. Verify each sees different tool description
4. Check both can execute tasks successfully

## Implementation Notes

**Research Required** (Depends on AGENTOPT-1001 SDK research):
1. Check SDK docs for tool description override capabilities
2. Test if MCP servers can be customized per query
3. Fallback to config file approach if SDK doesn't support

**IMPORTANT**: If SDK doesn't support runtime tool description overrides:
- **Option A (Config File)**: Write per-worktree MCP config files
  - Requires MCP server to read config at startup
  - May need MCP server code changes in `packages/maproom-mcp/src/index.ts`
- **Option B (Environment Variables)**: Pass variant via ENV
  - MCP server reads `MAPROOM_SEARCH_DESCRIPTION` env var
  - Requires MCP server code changes
- **Option C (Multiple MCP Instances)**: Spawn separate MCP server per variant
  - More complex, but cleanest separation
  - May require SDK support for custom MCP server paths

**Recommendation**: Start with Option A (config file), as it requires minimal MCP server changes

**Key Files**:
- `packages/cli/src/sdk/variant-injection.ts` - Injection logic
- `packages/cli/tests/sdk/variant-injection.test.ts` - Verification tests

**Design Principles**:
- Transparent to agent (agent doesn't know it has custom description)
- No MCP server code changes required
- Works with parallel execution
- Supports all variant types from AGENTOPT-0002

## Dependencies

- AGENTOPT-1001 (SDK integration) must be complete
- AGENTOPT-0002 (variant generation) provides variants to use

## Risk Assessment

**Risk**: SDK doesn't support tool description overrides
**Mitigation**: Use config file approach or MCP server customization

**Risk**: Variants not properly isolated between agents
**Mitigation**: Test with parallel execution, verify separation

**Risk**: MCP server caches descriptions
**Mitigation**: Use separate MCP server instances per agent if needed

## Files/Packages Affected

- packages/cli/src/sdk/variant-injection.ts (new)
- packages/cli/src/sdk/spawner.ts (update to use injection)
- packages/cli/tests/sdk/variant-injection.test.ts (new)
- packages/maproom-mcp/ (may need config file support)

## Planning References

- Replan Analysis: `../replan-analysis.md`
- Variant System: AGENTOPT-0002
