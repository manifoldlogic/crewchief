# Ticket: MPEMBED-5002: Update MCP scan/upsert tools for provider flag

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- typescript-test-runner
- verify-ticket
- commit-ticket

## Summary
Modify scan.ts and upsert.ts MCP tools to detect provider using provider-detection module and pass --provider flag to the Rust binary. Handle errors when provider is unavailable.

## Background
This ticket extends Phase 5 (MCP Integration) by integrating the provider detection logic from MPEMBED-5001 into the actual MCP tools. The scan and upsert operations need to pass the correct provider to the Rust binary so embeddings are generated with the appropriate dimension and stored in the correct database columns.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-5-mcp-documentation.md

## Acceptance Criteria
- [x] scan.ts imports and uses getProviderConfig() (handleScan in index.ts)
- [x] upsert.ts imports and uses getProviderConfig()
- [x] --provider flag passed to Rust binary in both tools
- [x] Error handling for provider detection failures
- [x] User-friendly error messages with setup instructions
- [x] Tool descriptions updated to mention multi-provider support
- [x] Unit tests for provider flag passing
- [x] Integration test with mocked provider detection

## Technical Requirements
- Import getProviderConfig from utils/provider-detection
- Call getProviderConfig() before spawning Rust binary
- Append --provider=<name> to binary arguments
- Wrap provider detection in try-catch for error handling
- Log provider selection for debugging
- Maintain backward compatibility (tools work without explicit provider)
- Update TypeScript types for tool parameters

## Implementation Notes
**Updated scan.ts:**
```typescript
// packages/maproom-mcp/src/tools/scan.ts
import { getProviderConfig } from '../utils/provider-detection';

export async function scan(params: ScanParams): Promise<ScanResult> {
  try {
    // Detect provider
    const providerConfig = await getProviderConfig();
    console.log(`Scanning with ${providerConfig.provider} provider (${providerConfig.dimension} dimensions)`);

    // Build arguments for Rust binary
    const args = [
      'scan',
      '--repo', params.repo,
      '--worktree', params.worktree,
      '--root', params.root,
      '--provider', providerConfig.provider,
    ];

    if (params.generateEmbeddings) {
      args.push('--generate-embeddings');
    }

    // Spawn Rust binary
    const result = await spawnMaproomBinary(args);

    return {
      success: true,
      filesScanned: result.filesScanned,
      chunksCreated: result.chunksCreated,
      provider: providerConfig.provider,
      dimension: providerConfig.dimension,
    };
  } catch (error) {
    if (error.message.includes('No embedding provider available')) {
      throw new Error(
        'Cannot scan with embeddings: No provider available.\n' +
        error.message
      );
    }
    throw error;
  }
}
```

**Updated upsert.ts:**
```typescript
// packages/maproom-mcp/src/tools/upsert.ts
import { getProviderConfig } from '../utils/provider-detection';

export async function upsert(params: UpsertParams): Promise<UpsertResult> {
  try {
    // Detect provider
    const providerConfig = await getProviderConfig();
    console.log(`Upserting with ${providerConfig.provider} provider (${providerConfig.dimension} dimensions)`);

    const args = [
      'upsert',
      '--repo', params.repo,
      '--worktree', params.worktree,
      '--root', params.root,
      '--commit', params.commit,
      '--provider', providerConfig.provider,
      '--paths', ...params.paths,
    ];

    const result = await spawnMaproomBinary(args);

    return {
      success: true,
      chunksUpdated: result.chunksUpdated,
      provider: providerConfig.provider,
      dimension: providerConfig.dimension,
    };
  } catch (error) {
    if (error.message.includes('No embedding provider available')) {
      throw new Error(
        'Cannot upsert with embeddings: No provider available.\n' +
        error.message
      );
    }
    throw error;
  }
}
```

## Dependencies
- MPEMBED-5001 (Provider detection module must exist)

## Risk Assessment
- **Risk**: Provider detection adds latency to tool calls
  - **Mitigation**: Detection is cached, only runs once per session
- **Risk**: Error messages may not be visible in all MCP clients
  - **Mitigation**: Return structured error responses with setup instructions

## Files/Packages Affected
- packages/maproom-mcp/src/tools/upsert.ts (modified)
- packages/maproom-mcp/src/index.ts (modified handleScan function)
- packages/maproom-mcp/src/types.ts (modified UpsertResult interface)
- packages/maproom-mcp/tests/tools/upsert.test.ts (modified)
- packages/maproom-mcp/tests/tools/scan.test.ts (created)

## Implementation Notes

Successfully integrated provider detection into both scan and upsert MCP tools:

**Changes Made:**
1. **upsert.ts**:
   - Imported `getProviderConfig` from provider-detection module
   - Added provider detection call before binary spawning
   - Added provider flag (`--provider`) to binary arguments
   - Enhanced error handling for NO_PROVIDER errors with helpful setup instructions
   - Updated result to include provider name and dimension

2. **index.ts (handleScan)**:
   - Added provider detection before spawning scan binary
   - Added provider flag to scan binary arguments
   - Enhanced error handling with user-friendly messages
   - Updated scan result to include provider info

3. **types.ts**:
   - Extended UpsertResult interface with optional `provider` and `dimension` fields

4. **Tool Descriptions**:
   - Updated both scan and upsert tool descriptions to mention multi-provider support
   - Noted that provider selection is cached for session performance

**Tests:**
- Created comprehensive provider integration tests in `tests/tools/scan.test.ts`
- Updated `tests/tools/upsert.test.ts` with provider integration tests
- All tests pass (84 passed in provider/scan/upsert test suites)

**Error Handling:**
- Both tools throw `NO_PROVIDER` error with helpful setup instructions when no provider available
- Error messages include options for Ollama, OpenAI, and Google Vertex AI setup
- Maintains backward compatibility (tools work without explicit provider config if auto-detection succeeds)

**Key Features:**
- Provider detection is cached per MCP session (no performance impact on repeated calls)
- Supports explicit provider override via EMBEDDING_PROVIDER env var
- Auto-detects Ollama → OpenAI → Google in priority order
- Both tools log provider selection for debugging
- Results include provider metadata for transparency
