# Ticket: WTSRCH-2001: Implement Three-Tier Worktree Resolution Logic

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-nodejs-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the core resolution logic that determines which worktree to search based on a three-tier hierarchy: explicit parameter > auto-detection > fallback. This includes database lookup with caching to map worktree names to IDs.

## Background
Users need search results scoped to their current context (the branch they're working in), but the system must gracefully handle cases where:
- User explicitly specifies a worktree (backward compatibility)
- Current branch isn't indexed yet (fallback to main)
- No worktrees are indexed (fallback to all)

The three-tier resolution provides robust behavior across all scenarios while maintaining backward compatibility with existing code that passes explicit worktree parameters.

This ticket implements **Phase 2: Worktree Resolution Logic** from the WTSRCH project plan, specifically implementing Architecture Decision AD-2 (Three-tier resolution strategy).

## Acceptance Criteria
- [ ] `resolveWorktreeId()` function implemented with three resolution tiers
- [ ] **Tier 1 (Explicit):** When `worktree` parameter is provided (string or null), use it directly - no auto-detection
- [ ] **Tier 2 (Auto-detect):** When `worktree` is undefined, call `getCurrentBranch()` and lookup worktree ID
- [ ] **Tier 3 (Fallback):** When auto-detection fails or branch not indexed, fallback to "main" worktree
- [ ] **Tier 4 (Last resort):** When "main" not found, fallback to null (search all worktrees)
- [ ] `lookupWorktreeId()` function queries database with parameterized query
- [ ] LRU cache implemented for worktree ID lookups (5-minute TTL)
- [ ] Clear error messages provided for each failure mode
- [ ] Return value includes both worktree ID and metadata (mode, fallback reason, detected branch)
- [ ] Unit tests pass for all three tiers and edge cases

## Technical Requirements

### 1. Function Signatures

```typescript
async function resolveWorktreeId(
  repo: string,
  explicitWorktree: string | null | undefined,
  client: Client
): Promise<{ id: number | null; metadata: ResolutionMetadata }>

async function lookupWorktreeId(
  client: Client,
  repo: string,
  worktreeName: string
): Promise<number>
```

### 2. Resolution Metadata Type

```typescript
interface ResolutionMetadata {
  mode: 'explicit' | 'auto' | 'fallback' | 'all'
  auto_detected?: boolean
  detected_branch?: string
  fallback?: boolean
  fallback_reason?: string
  worktree?: string
}
```

### 3. Database Query

Use existing pattern from `src/index.ts:656-660`:

```sql
SELECT id, name FROM maproom.worktrees
WHERE repo_id=$1 AND name=$2
```

### 4. Cache Configuration

```typescript
const worktreeIdCache = new LRUCache<string, number>({
  max: 500,          // Max 500 repo+worktree combinations
  ttl: 300_000,      // 5 minute TTL
})
// Cache key: `${repo}:${worktreeName}`
```

### 5. Resolution Logic Flow

```typescript
// Tier 1: Explicit parameter
if (explicitWorktree !== undefined) {
  if (explicitWorktree === null) {
    return { id: null, metadata: { mode: 'all' } }
  }
  const id = await lookupWorktreeId(client, repo, explicitWorktree)
  return { id, metadata: { mode: 'explicit', worktree: explicitWorktree } }
}

// Tier 2: Auto-detect
try {
  const branch = await getCurrentBranch()
  const id = await lookupWorktreeId(client, repo, branch)
  return { id, metadata: { mode: 'auto', auto_detected: true, detected_branch: branch, worktree: branch } }
} catch (error) {
  // Log warning, proceed to fallback
}

// Tier 3: Fallback to main
try {
  const id = await lookupWorktreeId(client, repo, 'main')
  return {
    id,
    metadata: {
      mode: 'fallback',
      fallback: true,
      fallback_reason: 'Current branch not indexed or detection failed',
      worktree: 'main'
    }
  }
} catch (error) {
  // main not found, proceed to tier 4
}

// Tier 4: Search all worktrees
return {
  id: null,
  metadata: {
    mode: 'all',
    fallback: true,
    fallback_reason: 'No indexed worktrees found'
  }
}
```

## Implementation Notes

### File Location
**File:** `packages/maproom-mcp/src/index.ts` (add functions to search handler file)

**Location:** Add these functions before the search tool handler function (around line 600).

### Database Lookup Pattern
Reuse the existing pattern from lines 656-660:

```typescript
const { rows: wt } = await client.query(
  'SELECT id, name FROM maproom.worktrees WHERE repo_id=$1 AND name=$2',
  [repoId, worktreeName]
)
if (wt.length === 0) {
  throw new Error(`Worktree '${worktreeName}' not found for repo '${repo}'`)
}
return wt[0].id
```

### Error Handling
- Use try/catch blocks for each tier
- Log warnings (not errors) for expected fallback scenarios
- Provide helpful error messages that guide users to solutions

### Test File
**New file:** `packages/maproom-mcp/tests/unit/worktree-resolution.test.ts`

Test scenarios using Vitest:
1. Tier 1: Explicit string parameter returns that worktree ID
2. Tier 1: Explicit null parameter returns null (all worktrees)
3. Tier 2: Undefined parameter triggers auto-detection
4. Tier 2: Auto-detected branch found in database returns its ID
5. Tier 3: Auto-detected branch not in database falls back to main
6. Tier 4: Main not in database falls back to null (all)
7. Cache: Second lookup for same repo+worktree uses cache (mock database query, verify called once)
8. Metadata: Each tier returns correct metadata structure

## Dependencies
- **WTSRCH-1001** (Git branch detection) - MUST be completed first (provides `getCurrentBranch()` function)

## Risk Assessment
- **Risk**: Database lookup performance impact
  - **Mitigation**: LRU cache with 5-minute TTL reduces queries by >95%
- **Risk**: Fallback logic confuses users
  - **Mitigation**: Clear metadata and helpful hints in search results (Phase 3)

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` - Add `resolveWorktreeId()` and `lookupWorktreeId()` functions
- `packages/maproom-mcp/tests/unit/worktree-resolution.test.ts` - NEW: Unit tests for resolution logic
