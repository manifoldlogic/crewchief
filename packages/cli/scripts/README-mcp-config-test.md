# MCP Configuration Test Script

Test script to verify MCP configuration is properly loaded by the Claude Agents SDK.

## Purpose

The `test-mcp-config.ts` script verifies that:

1. The `.mcp.json` file is accessible to the Claude Agents SDK
2. MCP tools (specifically maproom) are available to spawned agents
3. The SDK can successfully connect to MCP servers

## Usage

```bash
# Test MCP config in current directory
tsx scripts/test-mcp-config.ts

# Test MCP config in specific worktree
tsx scripts/test-mcp-config.ts /path/to/worktree
```

## What It Tests

1. **File Location**: Reports where `.mcp.json` should be located
2. **Agent Spawn**: Spawns a Claude agent in the specified directory
3. **MCP Tool Access**: Attempts to use `mcp__maproom__status` tool
4. **Success Criteria**: Verifies the agent successfully used MCP tools

## Expected Output

### Success Case

```
🔍 Testing MCP Configuration
   Worktree: /workspace
   .mcp.json should be at: /workspace/.mcp.json

📝 Task: Check if the maproom MCP tool is available...

✅ Agent completed successfully
   Success: true
   Turns: 2
   Duration: 1523ms

✅ SUCCESS: Maproom MCP tool was accessible and used!
```

### Failure Case

```
🔍 Testing MCP Configuration
   Worktree: /workspace
   .mcp.json should be at: /workspace/.mcp.json

📝 Task: Check if the maproom MCP tool is available...

❌ ERROR: Agent failed
Error: MCP server connection failed
```

## How Claude Agents SDK Loads MCP Config

The Claude Agents SDK looks for `.mcp.json` in the working directory (`cwd`) provided to the agent:

```typescript
const result = await spawnAgent({
  task: 'Your task here',
  worktreePath: '/path/to/worktree', // SDK looks for /path/to/worktree/.mcp.json
})
```

**Key Points**:

- The SDK expects `.mcp.json` at the **root of the working directory**
- For git worktrees, this means `/path/to/worktree/.mcp.json`
- The file must be tracked by git to appear in worktrees
- The file should NOT be in subdirectories like `packages/cli/`

## Troubleshooting

### Issue: "Missing .mcp.json file"

**Diagnosis**: Check if the file exists and is tracked by git:

```bash
# Check if file exists
ls -la /path/to/worktree/.mcp.json

# Check if file is tracked by git
git ls-files /path/to/worktree/.mcp.json

# Check if file is in git index
git ls-files --stage .mcp.json
```

**Solution**: Ensure `.mcp.json` is at repository root and force-add to git:

```bash
cd /path/to/repository/root
git add -f .mcp.json
git commit -m "config: add .mcp.json for MCP server configuration"
```

### Issue: "MCP server connection failed"

**Diagnosis**: Check MCP server configuration:

```bash
# Check if maproom MCP container is running
docker ps | grep maproom-mcp

# Check maproom MCP logs
docker logs maproom-mcp

# Test maproom binary directly
/workspace/packages/cli/bin/linux-arm64/maproom --help
```

**Solution**: Ensure MCP servers are running and accessible.

## Configuration Format

The `.mcp.json` file should follow this format:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "docker",
      "args": ["exec", "-i", "maproom-mcp", "node", "/app/dist/index.js"],
      "env": {
        "MAPROOM_EMBEDDING_PROVIDER": "openai",
        "OPENAI_API_KEY": "${OPENAI_API_KEY}"
      }
    }
  }
}
```

For genetic optimizer compatibility, **keep only essential MCP servers** to reduce variables during troubleshooting.

## Related Files

- `/workspace/.mcp.json` - MCP configuration at repository root
- `/workspace/packages/cli/src/sdk/spawner.ts` - Agent spawning logic
- `/workspace/packages/cli/src/search-optimization/validation/pre-flight-validator.ts` - Pre-flight MCP validation

## See Also

- [Claude Agents SDK Documentation](https://github.com/anthropics/claude-agent-sdk)
- [MCP Protocol Specification](https://modelcontextprotocol.io)
- [Genetic Optimizer Pre-Flight Validation](../src/search-optimization/validation/README.md)
