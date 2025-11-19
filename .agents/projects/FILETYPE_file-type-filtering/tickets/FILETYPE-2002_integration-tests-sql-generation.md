# Ticket: FILETYPE-2002: Create Integration Tests for SQL Generation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- typescript-test-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create 10 integration tests in a new test file verifying correct SQL query generation for file_type filter, including single/multi extensions, parameterization safety, and filter combinations.

## Background
Integration tests verify that buildFilterClauses() generates correct SQL for all file_type filter scenarios. These tests ensure SQL structure, parameterization (security), and interaction with other filters are correct before E2E testing.

**Reference:**
- quality-strategy.md - "Integration Tests: SQL Generation (10 tests)" section (lines 258-398)
- quality-strategy.md - "Test File Organization" - NEW FILE in filters/ directory (lines 71-78)

## Acceptance Criteria
- [ ] All 10 integration tests pass
- [ ] Single extension SQL verified (LIKE clause)
- [ ] Multi-extension SQL verified (OR clause with parentheses)
- [ ] Parameterized queries confirmed (SQL injection safe)
- [ ] Filter combination tested (file_type + recency, file_type + worktree_id)
- [ ] Tests run in <2 seconds

## Technical Requirements

**Location:** `packages/maproom-mcp/tests/filters/file-type.int.test.ts` (NEW FILE)

**Action:** CREATE new file in NEW directory `tests/filters/`

**Directory setup:**
```bash
mkdir -p packages/maproom-mcp/tests/filters
```

**Test suite structure:**
```typescript
import { describe, it, expect } from 'vitest'
import { buildFilterClauses } from '../src/index.js'

describe('buildFilterClauses - File Type Integration Tests', () => {
  // Single extension SQL (P0)
  it('generates correct SQL for single extension', () => {
    const args: any[] = [1] // repoId
    const filters = { file_type: 'ts' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain("f.relpath LIKE $2")
    expect(args).toContain('%.ts')
    expect(args.length).toBe(2)
  })

  // Multi-extension SQL (P0)
  it('generates correct OR clause for multiple extensions', () => {
    const args: any[] = [1]
    const filters = { file_type: 'ts,tsx,js' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('(f.relpath LIKE')
    expect(clauses).toContain(' OR ')
    expect(args).toContain('%.ts')
    expect(args).toContain('%.tsx')
    expect(args).toContain('%.js')
    expect(args.length).toBe(4) // repoId + 3 extensions
  })

  // Parameterization (P0)
  it('uses parameterized queries (SQL injection safe)', () => {
    const args: any[] = [1]
    const filters = { file_type: "ts'; DROP TABLE files; --" }
    const clauses = buildFilterClauses(filters, 'all', args)

    // Should NOT contain the malicious string directly in SQL
    expect(clauses).not.toContain("DROP TABLE")
    // Should use parameter placeholders
    expect(clauses).toContain("$")
  })

  // Empty filter (P0)
  it('handles empty file_type gracefully', () => {
    const args: any[] = [1]
    const filters = { file_type: '' }
    const clausesBefore = args.length
    const clauses = buildFilterClauses(filters, 'all', args)

    // No filter added, args unchanged
    expect(args.length).toBe(clausesBefore)
    expect(clauses).not.toContain('f.relpath LIKE')
  })

  // Filter combination - recency (P1)
  it('combines file_type with recency_threshold', () => {
    const args: any[] = [1]
    const filters = {
      file_type: 'ts',
      recency_threshold: '7 days'
    }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('f.relpath LIKE')
    expect(clauses).toContain('f.last_modified >')
    expect(args.length).toBe(3) // repoId, file_type, recency
  })

  // Filter combination - worktree (P1)
  it('combines file_type with worktree_id', () => {
    const args: any[] = [1]
    const filters = {
      file_type: 'ts',
      worktree_id: 42
    }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('f.relpath LIKE')
    expect(clauses).toContain('worktree_id')
  })

  // Legacy filter coexistence (P1)
  it('works with legacy filter parameter', () => {
    const args: any[] = [1]
    const filters = { file_type: 'ts' }
    const clauses = buildFilterClauses(filters, 'code', args)

    // Both filters applied
    expect(clauses).toContain('f.relpath LIKE') // file_type
    expect(clauses).toContain("NOT LIKE '%.md'") // legacy "code" filter
  })

  // Case handling (P0)
  it('normalizes case before SQL generation', () => {
    const args: any[] = [1]
    const filters = { file_type: 'TS,TSX' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(args).toContain('%.ts') // Lowercased
    expect(args).toContain('%.tsx')
    expect(args).not.toContain('%.TS') // Not uppercase
  })

  // Limit enforcement (P1)
  it('truncates >20 extensions gracefully', () => {
    const args: any[] = [1]
    const twentyOne = Array(21).fill('ts').map((_, i) => `ext${i}`).join(',')
    const filters = { file_type: twentyOne }
    const clauses = buildFilterClauses(filters, 'all', args)

    // Should truncate to 20 (or handle gracefully)
    // args.length = repoId (1) + extensions (max 20) = max 21
    expect(args.length).toBeLessThanOrEqual(21)
  })

  // Whitespace/dot handling (P1)
  it('handles dots and whitespace before SQL', () => {
    const args: any[] = [1]
    const filters = { file_type: '  .ts , .tsx  ' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(args).toContain('%.ts')
    expect(args).toContain('%.tsx')
    expect(args).not.toContain('%..ts') // No double dot
  })
})
```

## Implementation Notes

**File organization:**
- NEW directory: `tests/filters/`
- NEW file: `file-type.int.test.ts`
- Pattern: `{feature}.{test-type}.test.ts`

**Naming convention rationale:**
- `.int.test.ts` suffix indicates integration tests
- Distinguishes from unit tests in search_tool.test.ts
- Distinguishes from E2E tests in file-type.e2e.test.ts

**What these tests verify:**
1. Correct SQL structure (LIKE clauses, OR logic, parentheses)
2. Parameterization (security critical)
3. Filter combination (composability)
4. Edge case handling (empty, case, dots)

**Test execution:**
```bash
# Run only integration tests
pnpm test filters/*.int.test.ts

# Run all file-type tests
pnpm test file-type
```

## Dependencies
- **FILETYPE-1002** (parseFileTypeFilter implemented)
- **FILETYPE-1003** (buildFilterClauses updated)

## Risk Assessment
- **Risk**: Tests coupled to SQL implementation details
  - **Mitigation:** Test SQL behavior (LIKE clauses, parameters) not exact string format

- **Risk**: buildFilterClauses not exported (can't import for testing)
  - **Mitigation:** May need to temporarily export for testing or test via handleSearch

## Files/Packages Affected
- `packages/maproom-mcp/tests/filters/` (NEW DIRECTORY)
- `packages/maproom-mcp/tests/filters/file-type.int.test.ts` (NEW FILE)
