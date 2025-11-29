# Ticket: AGENTOPT-2002: Automated Variant Deployment

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create an automated deployment command that takes a winning variant from the leaderboard and deploys it to the live MCP server by updating the tool description in the source code, rebuilding, and optionally restarting the server.

## Background
Currently, deploying a winning variant to production is a manual 4-step process:
1. Promote variant with `crewchief optimization promote variant-abc123` (marks it in tracking system)
2. Manually read the variant JSON from `.crewchief/production/variants/variant-abc123.json`
3. Manually edit `packages/maproom-mcp/src/index.ts` to update the search tool description
4. Manually rebuild with `pnpm build` and restart the MCP server

This manual process is error-prone and creates deployment friction. The tracking system (AGENTOPT-2001) tracks which variant is "production" in metadata, but doesn't actually deploy it to the live MCP server.

**Gap**: The `promoteToProduction()` function only updates tracking metadata. It doesn't modify the actual MCP server source code that AI assistants use.

This ticket implements the deployment automation component of Phase 2 - Winner Tracking and Production Management. By automating the deployment process, we reduce the friction from variant selection to production deployment, enabling faster iteration cycles and reducing human error in the deployment process.

## Acceptance Criteria
- [ ] `deployVariant()` function implemented in `packages/cli/src/search-optimization/tracking/deployment.ts`
- [ ] `crewchief optimization deploy <variantId>` CLI command added
- [ ] Function reads variant JSON from `.crewchief/production/variants/` or leaderboard
- [ ] Function updates `packages/maproom-mcp/src/index.ts` tool description automatically
- [ ] Function rebuilds MCP server with `pnpm build` in maproom-mcp package
- [ ] Optional: Detects if MCP server is running and offers to restart it
- [ ] Dry-run mode with `--dry-run` flag to preview changes without applying
- [ ] Backup of previous tool description saved to `.crewchief/production/backups/`
- [ ] Integration with production tracking (auto-calls `promoteToProduction()` if not already promoted)
- [ ] Unit tests for deployment logic (source code patching, backup creation)
- [ ] Documentation in `docs/architecture/optimization-tracking-system.md`

## Technical Requirements

### 1. Source Code Patching

Parse `packages/maproom-mcp/src/index.ts` to find the `search` tool definition:
- Locate the `description` field (around line 118 in toolSchemas array)
- Replace description with variant's optimized description
- Preserve all other tool properties (inputSchema, etc.)
- Maintain formatting and code style

Tool Description Location:
```typescript
// File: packages/maproom-mcp/src/index.ts
const toolSchemas = [
  {
    name: 'search',
    description: 'REPLACE THIS TEXT WITH VARIANT DESCRIPTION',
    inputSchema: { ... }
  }
]
```

### 2. Deployment Function Signature

```typescript
export async function deployVariant(
  variantId: string,
  options: {
    dryRun?: boolean
    skipBuild?: boolean
    autoRestart?: boolean
  } = {}
): Promise<DeploymentResult>

interface DeploymentResult {
  success: boolean
  variantId: string
  previousDescription: string
  newDescription: string
  backupPath: string
  buildSuccess: boolean
  serverRestarted: boolean
}
```

### 3. Backup System

- Save previous description to `.crewchief/production/backups/description-{timestamp}.txt`
- Keep last 10 backups (prune older ones)
- Include metadata: timestamp, variant ID, deployment reason

### 4. Build Integration

- Execute `pnpm build` in `packages/maproom-mcp` directory
- Capture build output and errors
- Return success/failure status
- On build failure, rollback source code changes

### 5. Server Restart Detection (Optional)

- Check if MCP server process is running (look for node process with maproom-mcp)
- If running, ask user if they want to restart
- Provide instructions for manual restart if declined

### 6. CLI Commands

```bash
# Basic deployment
crewchief optimization deploy variant-abc123

# Dry run (preview changes)
crewchief optimization deploy variant-abc123 --dry-run

# Skip rebuild (for testing)
crewchief optimization deploy variant-abc123 --skip-build

# Auto-restart server if running
crewchief optimization deploy variant-abc123 --auto-restart

# Deploy current production variant (from tracking system)
crewchief optimization deploy --production
```

## Implementation Notes

### Source Code Patching Strategy

Use **regex with validation** (simpler, faster approach):

```typescript
const fileContent = fs.readFileSync('packages/maproom-mcp/src/index.ts', 'utf-8')

// Find search tool in toolSchemas
const searchToolRegex = /{\s*name:\s*'search',\s*description:\s*'([^']*)',/s

if (!searchToolRegex.test(fileContent)) {
  throw new Error('Could not find search tool definition in index.ts')
}

// Replace description
const newContent = fileContent.replace(
  searchToolRegex,
  `{\n    name: 'search',\n    description: '${escapeString(variant.description)}',`
)

// Validate the replacement worked
if (!newContent.includes(variant.description)) {
  throw new Error('Failed to update tool description')
}

fs.writeFileSync('packages/maproom-mcp/src/index.ts', newContent)
```

**Alternative Option: TypeScript AST** (more robust, handles edge cases)
- Use `ts-morph` library for AST manipulation
- Find toolSchemas array declaration
- Locate object with name: 'search'
- Update description property value
- Preserve formatting with ts-morph pretty printer

### Workflow Integration

When user runs `crewchief optimization deploy variant-abc123`:

1. Check if variant exists in leaderboard or production variants
2. If not in production tracking, prompt: "Promote to production first? (Y/n)"
3. Create backup of current tool description
4. Update source code
5. Run build
6. If build fails, rollback source code from backup
7. Update production tracking (if not already promoted)
8. Check if server is running
9. Prompt for restart if needed
10. Display success message with next steps

### Error Handling

- **Variant not found**: Clear error with suggestion to run `crewchief optimization leaderboard`
- **Build failure**: Automatic rollback, display build errors
- **File not found**: Check workspace structure, suggest running from repo root
- **Permission errors**: Clear message about file permissions

### Safety Checks

- Verify we're in the correct repository (check for packages/maproom-mcp/)
- Confirm variant description is non-empty
- Validate TypeScript syntax after patching (compile check)
- Create backup before any modifications
- Provide clear rollback instructions on failure

## Dependencies
- AGENTOPT-2001 (Winner Tracking) - COMPLETED
- packages/maproom-mcp source code structure

## Risk Assessment

- **Risk**: Source code patching breaks syntax
  - **Mitigation**: AST-based approach + validation, automatic rollback on build failure

- **Risk**: User deploys untested variant
  - **Mitigation**: Require confirmation, show variant metadata before deployment

- **Risk**: MCP server doesn't pick up changes
  - **Mitigation**: Provide clear restart instructions, detect running server

- **Risk**: Breaking changes to index.ts structure
  - **Mitigation**: Validate tool schema structure before patching, fail gracefully

## Files/Packages Affected
- `/workspace/packages/cli/src/search-optimization/tracking/deployment.ts` (new)
- `/workspace/packages/cli/src/cli/optimization.ts` (modify - add deploy command)
- `/workspace/packages/cli/src/search-optimization/tracking/index.ts` (modify - export deployment)
- `/workspace/packages/cli/src/search-optimization/tracking/__tests__/deployment.test.ts` (new)
- `/workspace/packages/maproom-mcp/src/index.ts` (modified by deployment automation)
- `/workspace/docs/architecture/optimization-tracking-system.md` (update with deployment docs)

## Success Metrics
- Deploy variant with single command instead of 4 manual steps
- Zero syntax errors in deployed code (validated before commit)
- Automatic rollback on build failure
- Clear success/failure messaging
- Deployment completes in <30 seconds
