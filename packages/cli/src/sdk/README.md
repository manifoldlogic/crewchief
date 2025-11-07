# Claude Code Agents SDK Integration

This module provides integration with the `@anthropic-ai/claude-agent-sdk` for programmatic agent control within the crewchief CLI.

## Installation

```bash
pnpm add @anthropic-ai/claude-agent-sdk
```

## Usage

### Basic Agent Spawning

```typescript
import { spawnAgent } from './sdk'

const result = await spawnAgent({
  task: 'Create a file test.txt with "Hello World"',
  worktreePath: '/path/to/worktree',
})

console.log('Success:', result.success)
console.log('Session ID:', result.sessionId)
console.log('Tokens used:', result.usage?.inputTokens, result.usage?.outputTokens)
```

### With Hooks

```typescript
const result = await spawnAgent({
  task: 'Find all TypeScript files',
  worktreePath: '/path/to/worktree',
  hooks: {
    onToolUse: (event) => {
      console.log(`Tool used: ${event.tool_name}`)
    },
    onComplete: (result) => {
      console.log(`Agent completed in ${result.performance?.durationMs}ms`)
    },
  },
})
```

### Configuration

```typescript
import { updateSDKConfig } from './sdk'

updateSDKConfig({
  defaultPermissionMode: 'acceptEdits',
  defaultModel: 'claude-sonnet-4',
  verbose: true,
})
```

## SDK Capabilities Research

Based on the official SDK documentation (January 2025), here are the key capabilities:

### 1. Tool Description Overrides

**Finding**: ❌ NOT SUPPORTED

The SDK does NOT provide a mechanism to override MCP tool descriptions at runtime. The SDK configuration includes:

- `mcpServers`: Configure MCP server connections (stdio, HTTP, SSE, or in-process SDK servers)
- `allowedTools` / `disallowedTools`: Control which tools are available
- `canUseTool()`: Custom permission callback

But there is NO `toolOverrides`, `toolDescriptions`, or similar option to modify tool descriptions.

**Implications for AGENTOPT-1002**:

- Must use alternative approach (config file, environment variables, or multiple MCP instances)
- Recommend: Per-worktree `.mcp.json` config file approach
- May require minimal MCP server code changes to read variant from config

### 2. Event Hooks

**Finding**: ✅ FULLY SUPPORTED

Available lifecycle events:

- `PreToolUse` - Before tool execution (can modify input, deny permission)
- `PostToolUse` - After tool execution (can see result, track metrics)
- `SessionStart` - Session initialization
- `SessionEnd` - Session termination
- `UserPromptSubmit` - User input capture
- `Notification` - Alert messages
- `Stop` / `SubagentStop` - Interruption events
- `PreCompact` - Context compaction

**Hook Structure**:

```typescript
{
  match: (input) => boolean,  // Filter which events to handle
  callback: async (input) => {
    // Process event
    return { continue: true }  // or { async: true }
  }
}
```

**Event Data Available**:

- `session_id` - Unique session identifier
- `transcript_path` - Path to conversation transcript
- `cwd` - Current working directory
- Tool-specific fields (tool_name, tool_input, tool_result, is_error)

**Implications for AGENTOPT-1003/1005**:

- Can capture ALL tool usage with `PreToolUse` and `PostToolUse`
- Can track search queries, results, timing
- Can implement custom logging in hooks
- No need to parse agent logs

### 3. Async Patterns

**Finding**: ✅ ASYNC ITERATOR (for await...of)

The `query()` function returns an async generator (`Query` type) that yields `SDKMessage` objects:

```typescript
const agentQuery = query({ prompt, options })

for await (const message of agentQuery) {
  // Process message
  if (message.type === 'result') {
    // Final result
  }
}
```

**Message Types**:

- `assistant` - Agent's text response
- `user` - User input
- `result` - Final outcome with usage/performance data
- `partial` - Streaming partial content
- `system` - System messages

**Control Methods**:

- `agentQuery.interrupt()` - Stop agent execution
- `agentQuery.setPermissionMode(mode)` - Change permissions mid-execution

**Concurrency**:

- Single `query()` call = one agent session
- For parallel agents: spawn multiple `query()` calls
- Each runs in separate process/session
- No built-in pooling or rate limiting

**Implications for AGENTOPT-1006**:

- Parallel execution: `await Promise.all([spawnAgent(...), spawnAgent(...)])`
- Sequential execution: `for` loop with `await spawnAgent(...)`
- No special handling needed for concurrency
- May need manual resource limits for large-scale parallel execution

### 4. Tool Usage Logging

**Finding**: ⚠️ PARTIAL - Hooks Required

The SDK does NOT automatically log tool calls to files. However:

**Via Hooks** (Recommended):

```typescript
const toolLogs: ToolUseEvent[] = []

spawnAgent({
  hooks: {
    onToolUse: (event) => {
      toolLogs.push(event)
      // Or write to file
    },
  },
})
```

**Via Transcript**:

- SDK creates transcript files at `transcript_path`
- Contains full conversation history
- Format: JSONL (JSON Lines)
- Includes all tool uses and results

**Via Messages**:

- Each message in the async iterator contains tool use data
- Can collect and process messages array

**Implications for AGENTOPT-1005**:

- Implement custom logging via `PreToolUse`/`PostToolUse` hooks
- Store tool logs in run directory (`.crewchief/runs/{runId}/`)
- Use transcript as backup/reference
- Need consistent log format for evaluation framework

## Implementation Notes

### Permission Modes

- `default` - Prompts user for tool permissions (not suitable for automated testing)
- `acceptEdits` - Auto-accepts edits, prompts for destructive operations
- `bypassPermissions` - **Recommended for automated testing** - no prompts
- `plan` - Planning mode, no actual execution

### Session Management

- Each `spawnAgent()` call creates a new session
- Session ID available in result
- Transcripts saved automatically
- Can resume sessions with `resume` option (not used in our implementation)

### Error Handling

- Failures are reflected in `result.success = false`
- Final message type will indicate error
- No exceptions thrown unless SDK itself crashes

### Performance

- First call may be slower (model loading, initialization)
- Subsequent calls are faster
- Token usage tracked automatically
- Cache metrics available (cache creation, cache read tokens)

## Limitations Discovered

1. **No Tool Description Overrides**: Cannot modify MCP tool descriptions via SDK configuration
2. **No Auto-Logging**: Must implement tool logging via hooks
3. **Single Session Per Call**: No built-in multi-agent orchestration
4. **Stdin Not Supported for Prompts**: If using default permission mode, will fail in automated contexts

## Recommendations for Downstream Tickets

### AGENTOPT-1002 (Variant Injection)

- Use `.mcp.json` config file approach in each worktree
- Add minimal MCP server code to read `SEARCH_TOOL_DESCRIPTION` from config
- Alternative: environment variable per agent (pass via `env` option)

### AGENTOPT-1003 (Competition Framework)

- Use `PreToolUse` and `PostToolUse` hooks for metrics capture
- Store hook data in `searchMetrics` field of participant
- No need to parse logs or transcripts

### AGENTOPT-1005 (Evaluation Framework)

- Implement `ToolUseLog` format in hooks
- Write logs to `.crewchief/runs/{runId}/tool-usage.jsonl`
- Use for validation and scoring

### AGENTOPT-1006 (Competition Runner)

- Parallel execution: `Promise.all()` with multiple `spawnAgent()` calls
- Each agent gets isolated worktree (existing capability)
- May need resource limits if running >5 agents concurrently

### AGENTOPT-1007 (Genetic Iteration)

- Sequential competitions recommended initially
- Add parallel execution after Phase A working

## Examples

See `tests/sdk/spawner.test.ts` for working examples.

## References

- SDK Documentation: https://docs.claude.com/en/api/agent-sdk/typescript
- MCP Configuration: https://docs.claude.com/en/api/agent-sdk/mcp
- Agent SDK Release: https://www.anthropic.com/engineering/building-agents-with-the-claude-agent-sdk
