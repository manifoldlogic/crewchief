# Ticket: [WTSCAN-2001]: Add Documentation and Migration Guide

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass - N/A** - documentation-only ticket
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- docs-writer
- verify-ticket
- commit-ticket

## Summary
Document the auto-scan configuration feature, explain trade-offs, provide migration guide for existing users, and create a prominent changelog entry for this breaking change.

## Background
Phase 1 implemented the technical changes (config schema + conditional logic + tests). This ticket completes Phase 2 by ensuring users understand the breaking change, know how to migrate, and can make informed decisions about enabling/disabling auto-scan.

Clear documentation is critical because this is a breaking change that affects all users' workflows. The default behavior is changing from "auto-scan always" to "auto-scan opt-in".

## Acceptance Criteria
- [x] README includes new "Auto-Scan Configuration" section
- [x] Trade-offs clearly explained (speed vs convenience)
- [x] Migration example shows exact config to restore old behavior
- [x] Changelog entry prominently notes breaking change
- [x] Documentation is accurate and grammatically correct
- [x] Example config snippets are copy-paste ready
- [x] JSDoc comments updated if relevant

## Technical Requirements
- Add section to `packages/cli/README.md` after "Semantic Code Search" section
- Create changelog entry in appropriate changelog file
- Use clear, user-friendly language
- Include code examples in JavaScript (matching existing config format)
- Explain both enabling and disabling scenarios
- Document manual scanning alternative

## Implementation Notes
**README Section** (add after line ~70 in `packages/cli/README.md`):

```markdown
#### Auto-Scan Configuration

By default, worktree operations do NOT automatically trigger maproom indexing. This keeps worktree creation fast.

To enable auto-scan when creating worktrees, add to your `crewchief.config.js`:

\`\`\`javascript
export default {
  worktree: {
    autoScanOnWorktreeUse: true, // Enable auto-indexing
  },
}
\`\`\`

**Trade-offs**:
- **Auto-scan enabled**: New worktrees are immediately searchable, but creation is slower (5-30s depending on repo size)
- **Auto-scan disabled** (default): Fast worktree operations, but you must manually run `crewchief maproom scan` when needed

**Manual scanning**:
\`\`\`bash
# After creating a worktree, index it manually:
crewchief maproom scan
\`\`\`

**Migration from older versions**: If you relied on automatic scanning, simply add `autoScanOnWorktreeUse: true` to your config.
```

**Changelog Entry** (create or update `CHANGELOG.md` in packages/cli):

```markdown
## [Unreleased]

### Breaking Changes

#### Auto-Scan Now Opt-In

**What Changed**: Worktree creation no longer automatically triggers maproom scanning by default.

**Why**: This change dramatically improves worktree creation speed (from 5-30s to <1s) and gives users control over when indexing happens.

**Migration**: To restore automatic scanning, add one line to your `crewchief.config.js`:

\`\`\`javascript
export default {
  worktree: {
    autoScanOnWorktreeUse: true, // Restore auto-scan behavior
  },
}
\`\`\`

**Alternative**: Manually scan when needed: `crewchief maproom scan`

**Impact**: Users relying on automatic indexing must update config or manually scan.
```

**JSDoc Updates** (if applicable):
- Update JSDoc for `runMaproomScan()` if it mentions automatic scanning
- Update JSDoc for `createWorktree()` to mention conditional scan behavior
- Add JSDoc for `autoScanOnWorktreeUse` field in config schema

**Tone and Style**:
- Be direct and honest about the breaking change
- Emphasize the performance benefit
- Make migration trivial (copy-paste config)
- Provide alternatives (manual scan)
- Use examples, not just explanations

## Dependencies
- **Prerequisite**: WTSCAN-1001 (config schema exists)
- **Prerequisite**: WTSCAN-1002 (conditional logic implemented)
- **Prerequisite**: WTSCAN-1003 (tests verify behavior)
- Phase 1 must be complete before documentation

## Risk Assessment
- **Risk**: Insufficient migration guidance causes user confusion
  - **Mitigation**: Include explicit config example in multiple places (README, changelog). Make it copy-paste ready.
- **Risk**: Users don't understand trade-offs
  - **Mitigation**: Clearly explain speed vs convenience in bullet points. Provide manual scan alternative.
- **Risk**: Breaking change not prominent enough
  - **Mitigation**: Put breaking change at top of changelog. Use "Breaking Changes" heading. Explain impact clearly.

## Files/Packages Affected
- `packages/cli/README.md` - Add Auto-Scan Configuration section
- `packages/cli/CHANGELOG.md` - Add breaking change entry (create if doesn't exist)
- `packages/cli/src/config/schema.ts` - Add JSDoc comments (if needed)
- `packages/cli/src/git/worktrees.ts` - Update JSDoc comments (if needed)

## Verification Notes
**Tests pass**: N/A (documentation-only ticket)

**verify-ticket agent should check**:
1. README section exists and is in the right location
2. Code examples are syntactically correct JavaScript
3. Migration guide provides exact config snippet
4. Changelog entry exists and is prominent
5. Trade-offs are clearly explained
6. Language is clear and user-friendly
7. No spelling or grammar errors
8. Examples are copy-paste ready (no placeholders)
9. Both enabling and manual alternatives documented

**Manual verification**:
- Read documentation as a new user - is it clear?
- Copy-paste config example - does it work?
- Follow migration guide - is it easy?
- Breaking change warning - is it prominent?

**Documentation Quality Standards**:
- Accuracy: Technical details are correct
- Clarity: User can understand without prior knowledge
- Completeness: All scenarios covered (enable, disable, migrate, manual)
- Actionability: User knows exactly what to do
