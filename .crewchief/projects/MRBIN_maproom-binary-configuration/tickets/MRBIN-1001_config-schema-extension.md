# Ticket: [MRBIN-1001]: Add maproomBinaryPath to Config Schema

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
- typescript-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Extend the RepositorySchema in packages/cli/src/config/schema.ts to include an optional maproomBinaryPath field, allowing users to configure a custom maproom binary path in their crewchief.config.js file.

## Background
This project consolidates three separate maproom binary resolution implementations into a single shared utility and adds config-based binary path configuration. The first step is to extend the existing config schema to support the new maproomBinaryPath field.

This ticket implements the schema foundation required by all subsequent tickets. The field is optional to maintain backwards compatibility with existing configurations.

## Acceptance Criteria
- [ ] RepositorySchema includes maproomBinaryPath field as optional string
- [ ] TypeScript types exported correctly from schema.ts
- [ ] Zod validation accepts valid string paths
- [ ] Zod validation accepts undefined (optional field)
- [ ] Existing config validation tests still pass
- [ ] No breaking changes to existing config files

## Technical Requirements
- Add `maproomBinaryPath: z.string().optional()` to RepositorySchema
- Field must be optional (backwards compatible)
- No format validation at schema level (handled at runtime)
- String type for file path
- Export updated RepositoryConfig type

## Implementation Notes
Follow the existing pattern used for worktreeBasePath field. The schema should validate type but not file existence - path validation happens at runtime in the binary resolution utility.

Example schema change:
```typescript
export const RepositorySchema = z.object({
  mainBranch: z.string().default('main'),
  worktreeBasePath: z.string().default('.crewchief/worktrees'),
  maproomBinaryPath: z.string().optional(), // NEW
})
```

This is a minimal additive change with zero impact on existing configurations.

## Dependencies
None - This is the foundation ticket for the project.

## Risk Assessment
- **Risk**: Breaking existing config parsing
  - **Mitigation**: Field is optional, all existing configs remain valid, run existing config tests
- **Risk**: Type export issues breaking downstream code
  - **Mitigation**: Verify TypeScript compilation succeeds, check type exports

## Files/Packages Affected
- packages/cli/src/config/schema.ts

## Verification Notes
Verify that:
1. The schema accepts configs with maproomBinaryPath
2. The schema accepts configs without maproomBinaryPath (backwards compatible)
3. TypeScript types are exported and available
4. All existing config-related tests pass
5. No TypeScript compilation errors
