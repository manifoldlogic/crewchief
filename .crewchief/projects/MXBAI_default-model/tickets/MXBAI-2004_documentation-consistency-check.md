# Ticket: [MXBAI-2004]: Documentation Consistency Check

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
- documentation-writer
- verify-ticket
- commit-ticket

## Summary
Run final consistency check across all documentation to ensure no conflicting default references remain, verify only expected "nomic-embed-text" occurrences exist, and confirm migration guide and active docs are aligned.

## Background
This ticket implements Phase 2, Deliverable 4 from plan.md. After all documentation updates, we must verify consistency across all docs to prevent user confusion. This is the final quality gate for Phase 2.

Reference: plan.md Phase 2, Deliverable 4 "Documentation Consistency Check"

## Acceptance Criteria
- [ ] Grep scan completed for "nomic-embed-text" in active documentation
- [ ] All remaining references are expected (migration guide, backward compat sections)
- [ ] No conflicting default references found (e.g., one doc says nomic, another says mxbai)
- [ ] Active documentation files show mxbai-embed-large as default
- [ ] Archived documentation untouched (verified)
- [ ] Example commands tested and working
- [ ] Consistency report documented in ticket

## Technical Requirements
**Grep Scans** to run:

**Scan 1: Active documentation for nomic-embed-text**
```bash
# Search active docs (exclude .crewchief/)
grep -rn "nomic-embed-text" \
  docs/ \
  crates/maproom/CLAUDE.md \
  README.md \
  packages/vscode-maproom/README.md \
  packages/maproom-mcp/README.md
```

**Expected Results**:
- `docs/guides/migrating-to-mxbai.md`: Multiple references (migration guide explains old model)
- Other files: Only in "backward compatibility" or "legacy" sections

**Unexpected Results** (fail if found):
- Default examples using nomic-embed-text
- Setup instructions with nomic-embed-text
- Quickstart with nomic-embed-text

**Scan 2: Active documentation for dimension values**
```bash
# Find 768 and 1024 references
grep -rn "768\|1024" \
  docs/ \
  crates/maproom/CLAUDE.md \
  README.md \
  packages/vscode-maproom/README.md \
  packages/maproom-mcp/README.md
```

**Expected Results**:
- Default examples show 1024
- Backward compat sections may show 768 for nomic-embed-text
- Migration guide shows both (comparison table)

**Scan 3: Verify archived docs untouched**
```bash
# Confirm no accidental updates to archived projects
git diff --name-only | grep "\.crewchief/archive/"
# Should return empty (no changes to archive/)

git diff --name-only | grep "\.crewchief/projects/DIM1024_"
# Should return empty (no changes to DIM1024 project)
```

**Scan 4: Check for conflicting statements**
Search for phrases that might indicate contradictory information:
```bash
# Look for "default model" statements
grep -rn "default.*model" docs/ README.md packages/*/README.md crates/maproom/CLAUDE.md

# All should point to mxbai-embed-large, not nomic-embed-text
```

**Manual Verification**:
Test example commands from updated documentation:
1. Pick 3 example commands from different docs
2. Run each command to verify it works
3. Document results in ticket

## Implementation Notes
**Purpose**:
- Catch any inconsistencies from MXBAI-2002 updates
- Verify migration guide (MXBAI-2003) aligns with active docs
- Confirm no accidental updates to archived docs
- Provide final quality assurance before project completion

**Consistency Criteria**:
- All active docs show mxbai-embed-large as default
- Backward compat notes mention nomic-embed-text as option (not default)
- No doc says "default is nomic-embed-text"
- Migration guide comprehensive and aligned with active docs

**Failure Handling**:
- If conflicts found: Update relevant doc in MXBAI-2002
- If archived docs modified: Revert those changes
- If examples fail: Fix commands in relevant documentation

**Success Report Format**:
Document in ticket completion notes:
1. Grep scan results (expected vs unexpected references)
2. Archived docs status (untouched confirmed)
3. Example commands tested (3+ examples, all passed)
4. Any issues found and resolved

## Dependencies
- **Critical dependencies**:
  - MXBAI-2001 (documentation audit)
  - MXBAI-2002 (active docs updated)
  - MXBAI-2003 (migration guide created)
- **External dependency**: None

## Risk Assessment
- **Risk**: False positives in grep (valid references flagged as errors)
  - **Mitigation**: Review each grep result for context (migration guide refs are expected)

- **Risk**: Missing subtle inconsistencies
  - **Mitigation**: Check for "default model" phrases, not just model names

- **Risk**: Broken example commands
  - **Mitigation**: Test representative examples from each major doc

## Files/Packages Affected
- All documentation files from MXBAI-2002 and MXBAI-2003 (verification only, no changes expected)
- Archived documentation (verification only, must remain untouched)

## Verification Notes
Tests pass: N/A (documentation verification, no executable code changes)

verify-ticket agent should check:
- [ ] Grep scan results documented in ticket completion notes
- [ ] All "nomic-embed-text" references in active docs are expected (migration guide, backward compat)
- [ ] No conflicting default statements found
- [ ] Archived documentation untouched (git diff confirms)
- [ ] Example commands tested and working (3+ examples)
- [ ] Consistency report is comprehensive and clear
- [ ] Any issues found were resolved
- [ ] Documentation is ready for user consumption
