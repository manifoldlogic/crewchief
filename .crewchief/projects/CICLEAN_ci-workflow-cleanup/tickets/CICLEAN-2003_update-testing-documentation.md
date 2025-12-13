# Ticket: CICLEAN-2003: Update testing documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only changes)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- "Tests pass - N/A" for documentation-only changes

## Agents
- code-editor
- verify-ticket
- commit-ticket

## Summary
Update SQLite integration testing documentation to remove `--features sqlite` flag from fixture generation instructions.

## Background
The documentation file `docs/testing/SQLITE_INTEGRATION_TESTS.md` instructs developers to generate test fixtures using:
```bash
cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture
```

This command fails because the `sqlite` feature doesn't exist in Cargo.toml. Developers following the documentation will encounter errors and be unable to set up their test environment correctly.

**Planning Reference**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/architecture.md`

## Acceptance Criteria
- [x] Line 62 updated to remove `--features sqlite` from fixture generation command
- [x] Line 148 updated to remove `--features sqlite` from fixture generation command
- [x] Documentation provides correct, working commands
- [x] Markdown formatting remains valid

## Technical Requirements

### 1. Update first fixture generation instruction
**File**: `docs/testing/SQLITE_INTEGRATION_TESTS.md`
**Line**: 62

```markdown
# Before
cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture

# After
cargo test --test create_sqlite_fixture -- --ignored --nocapture
```

### 2. Update second fixture generation instruction
**File**: `docs/testing/SQLITE_INTEGRATION_TESTS.md`
**Line**: 148

```markdown
# Before
cargo test --features sqlite --test create_sqlite_fixture -- --ignored --nocapture

# After
cargo test --test create_sqlite_fixture -- --ignored --nocapture
```

### 3. Validate markdown formatting
After changes, verify markdown is well-formed:
```bash
# If markdownlint is available
markdownlint docs/testing/SQLITE_INTEGRATION_TESTS.md

# Or manually review in markdown preview
```

## Implementation Notes

**Why documentation accuracy matters**:
- Documentation is the first place developers look for guidance
- Incorrect commands waste time and create frustration
- Outdated docs erode trust in the codebase
- This aligns docs with actual build process (no feature flags)

**Context of changes**:
- Line 62: Initial setup instructions for test environment
- Line 148: Troubleshooting section for regenerating fixtures

**Consistency across documentation**:
This change ensures consistency with:
- CI workflow fixture generation (CICLEAN-1002)
- E2E test script error messages (CICLEAN-2001)
- Test helper error messages (CICLEAN-2002)

**Testing approach**:
1. Make documentation changes
2. Verify markdown syntax is correct
3. Optionally: Follow documented steps to verify they work

## Dependencies
- Depends on: CICLEAN-1002 (CI workflow updated)
- Depends on: CICLEAN-2001 (E2E script updated)
- Depends on: CICLEAN-2002 (Test helpers updated)

## Risk Assessment

- **Risk**: Documentation still incorrect
  - **Mitigation**: Command matches actual Cargo.toml (no features exist)
  - **Impact**: Low (better than current incorrect docs)
  - **Probability**: Very low (removing non-existent flag is correct)

- **Risk**: Breaking markdown formatting
  - **Mitigation**: Simple text change in code block; markdown structure unchanged
  - **Impact**: Very low (cosmetic only)
  - **Probability**: Very low (code block text replacement)

- **Risk**: Documentation out of sync with code
  - **Mitigation**: All changes in same project/PR
  - **Impact**: Low (temporary until PR merges)
  - **Probability**: Low (coordinated change set)

## Files/Packages Affected
- `docs/testing/SQLITE_INTEGRATION_TESTS.md` - Update fixture generation commands (lines 62, 148)
