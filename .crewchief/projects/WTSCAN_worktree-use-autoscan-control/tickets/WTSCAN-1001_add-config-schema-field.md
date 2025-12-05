# Ticket: [WTSCAN-1001]: Add Config Schema Field for Auto-Scan Control

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
- typescript-dev
- test-runner
- verify-ticket
- commit-ticket

## Summary
Add `autoScanOnWorktreeUse` boolean field to `WorktreeSchema` in the config system with default value of `false` to enable opt-in auto-scanning behavior.

## Background
CrewChief currently performs automatic maproom scanning after every `worktree create` operation, causing 5-30 second delays. This change begins the implementation of user-controlled scanning by adding the configuration field that will gate scan behavior.

This implements the config schema portion of Phase 1 from the execution plan, establishing the foundation for conditional scan logic.

## Acceptance Criteria
- [ ] `WorktreeSchema` contains `autoScanOnWorktreeUse` field defined as `z.boolean().default(false)`
- [ ] Schema validation accepts `true` and `false` values
- [ ] Schema validation rejects non-boolean values (strings, numbers, objects, arrays)
- [ ] Default value is `false` when field is undefined or omitted
- [ ] TypeScript types are automatically inferred from schema
- [ ] Unit tests verify schema validation behavior
- [ ] All tests pass

## Technical Requirements
- Add field to `WorktreeSchema` in `packages/cli/src/config/schema.ts`
- Use Zod's `z.boolean().default(false)` for validation and default
- Follow existing pattern: similar to `copyIgnoredFiles` but simpler (just boolean)
- Place field after existing `overwriteStrategy` field for logical grouping
- Ensure TypeScript type inference works automatically

## Implementation Notes
The config schema is defined using Zod in `packages/cli/src/config/schema.ts` (lines 54-58). Add the new field to the `WorktreeSchema` object:

```typescript
export const WorktreeSchema = z.object({
  copyIgnoredFiles: z.array(z.string()).optional(),
  copyFromPath: z.string().default('.'),
  overwriteStrategy: z.enum(['skip', 'overwrite', 'backup']).default('skip'),
  autoScanOnWorktreeUse: z.boolean().default(false), // NEW FIELD
})
```

**Key Decisions**:
- Field name `autoScanOnWorktreeUse` is specific and unambiguous
- Default `false` makes fast operations the default (breaking change, but necessary)
- Boolean type prevents injection attacks and type confusion
- Zod handles validation automatically - no manual checks needed

**Testing Approach**:
Create unit tests in the config schema test file (if one exists) or in a new test file:
- Test valid boolean values (true, false)
- Test invalid values are rejected ("yes", 1, null, undefined treated as default)
- Test default behavior when field omitted
- Test TypeScript type inference

## Dependencies
- No ticket dependencies - first ticket in Phase 1
- External: Zod library (already in use)
- Pattern dependency: Follows `copyIgnoredFiles` config pattern

## Risk Assessment
- **Risk**: Schema validation edge cases
  - **Mitigation**: Zod is battle-tested, handles edge cases automatically. Add comprehensive unit tests.
- **Risk**: Type inference breaks
  - **Mitigation**: TypeScript compiler will catch at build time. Verify with `pnpm build`.

## Files/Packages Affected
- `packages/cli/src/config/schema.ts` - Add field to WorktreeSchema
- `packages/cli/src/config/__tests__/schema.test.ts` - Add validation tests (create if doesn't exist)

## Verification Notes
**verify-ticket agent should check**:
1. Field exists in WorktreeSchema with correct Zod type
2. Default value is `false`
3. Unit tests exist and pass
4. TypeScript compiles without errors
5. No regressions in existing config tests
6. Schema rejects invalid values (try passing "true" as string, should fail)
