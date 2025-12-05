# Ticket: [MRMIGNR-1006]: Documentation Update

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

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
Update CLAUDE.md documentation with comprehensive .maproomignore usage examples, pattern syntax, precedence rules, and limitations to guide future development and user understanding.

## Background
The .maproomignore feature is complete and tested. Documentation needs to capture:
- How to use .maproomignore files
- Pattern syntax and examples
- Pattern precedence (.maproomignore vs .gitignore vs defaults)
- Integration points in scan and watch operations
- Known limitations (hot-reload, pattern validation)

This documentation serves both as user guidance and as developer reference for future maintenance.

Reference: Phase 1 Deliverable (plan.md lines 19-20), Architecture Migration Strategy (architecture.md lines 311-314)

## Acceptance Criteria
- [x] `.maproomignore` section added to `crates/maproom/CLAUDE.md`
- [x] Example `.maproomignore` file provided with realistic patterns
- [x] Pattern syntax documented (gitignore-style globs, comments, blank lines)
- [x] Pattern precedence clearly explained (.maproomignore > .gitignore > defaults)
- [x] Integration points documented (scan via OverrideBuilder, watch via event filter)
- [x] Limitations documented (hot-reload not supported, pattern validation on startup)
- [x] Code examples show how patterns are loaded and applied
- [x] CLI help text mentions .maproomignore (if applicable to scan/watch commands)
- [x] Documentation builds successfully: `cargo doc --no-deps`
- [x] Documentation reviewed for clarity and completeness

## Technical Requirements

**Location**: `crates/maproom/CLAUDE.md`

**Required sections**:

1. **Feature Overview**:
   - What .maproomignore does (exclude files from indexing without .gitignore)
   - Why it's useful (separate indexing concerns from version control)
   - Where the file goes (repository root only)

2. **Example .maproomignore File**:
```
# Example .maproomignore
# Exclude test fixtures from indexing
test-fixtures/**
tests/data/**

# Exclude build artifacts
build/
dist/
target/

# Exclude temporary files
*.tmp
*.bak
*.swp

# Exclude large data files
*.sql
*.csv
data/**
```

3. **Pattern Syntax**:
   - Gitignore-style glob patterns
   - Patterns relative to repository root
   - Comments start with `#`
   - Blank lines ignored
   - Leading `/` means relative to root (git semantics)
   - Examples: `test/**`, `*.tmp`, `build/`, `/specific-root-file.txt`

4. **Pattern Precedence**:
   - `.maproomignore` patterns applied as additional exclusions
   - `.gitignore` patterns still apply independently
   - Both exclusion sets are additive (file excluded if matches either)
   - Default patterns (e.g., `.git/`) always apply
   - Precedence order: .maproomignore OR .gitignore OR defaults

5. **Integration Details**:
   - **Scan**: Patterns loaded via `load_ignore_patterns()`, applied via `OverrideBuilder` to `WalkBuilder`
   - **Watch**: Patterns loaded at watcher startup, events filtered via `should_ignore()` in `event_conversion_task()`
   - Pattern loading happens once (scan start or watcher start)

6. **Known Limitations**:
   - `.maproomignore` must be at repository root (not supported in subdirectories)
   - Pattern changes require watcher restart (hot-reload not supported)
   - Invalid patterns cause startup failure (fail-fast behavior)
   - Programmatic `exclude` parameter is internal-only (not exposed via CLI in current version)

7. **Error Handling**:
   - Invalid glob patterns fail scan/watch startup
   - Missing .maproomignore is fine (uses defaults)
   - File read errors propagate with clear messages

**CLI help text updates** (if applicable):
- `crewchief-maproom scan --help` should mention .maproomignore support
- `crewchief-maproom watch --help` should mention .maproomignore support
- Help text should reference CLAUDE.md for detailed pattern documentation

## Implementation Notes

**Documentation style**:
- Clear, concise language (audience: developers and power users)
- Code examples with comments
- Real-world use cases (exclude test data, build artifacts, etc.)
- Link to architecture.md for implementation details if needed

**Structure in CLAUDE.md**:
```markdown
## Ignore Patterns

Maproom supports custom ignore patterns via `.maproomignore` files...

### Usage

Create a `.maproomignore` file in your repository root...

### Pattern Syntax

Patterns use gitignore-style globs...

### Example

(Example .maproomignore file here)

### Pattern Precedence

(Explain how .maproomignore, .gitignore, and defaults interact)

### Integration

(Explain scan and watch behavior)

### Limitations

(Document known limitations)
```

**Verification approach**:
- Read CLAUDE.md to verify all sections present
- Check for broken links or references
- Verify code examples are syntactically correct
- Run `cargo doc --no-deps` to ensure documentation builds

**Order of work**:
1. Add `.maproomignore` section to CLAUDE.md
2. Include all required subsections
3. Add example .maproomignore file
4. Document pattern syntax and precedence
5. Document integration points
6. Document limitations
7. Update CLI help text if applicable
8. Build documentation to verify
9. Review for clarity and completeness

## Dependencies
- **Prerequisite**: MRMIGNR-1001, MRMIGNR-1002, MRMIGNR-1003 (implementation complete)
- **Prerequisite**: MRMIGNR-1005 (integration tests passing, can reference them)
- **Blocks**: None (final ticket in phase)
- **External dependencies**: None

## Risk Assessment
- **Risk**: Documentation becomes outdated as implementation evolves
  - **Impact**: Medium - developer confusion
  - **Mitigation**: Link to architecture.md and plan.md for rationale. Keep CLAUDE.md focused on "how to use" not "how it works internally".

- **Risk**: Examples don't match actual behavior
  - **Impact**: High - users get wrong expectations
  - **Mitigation**: Test examples manually before committing. Reference integration tests.

- **Risk**: Limitations not clearly communicated
  - **Impact**: Medium - user frustration
  - **Mitigation**: Dedicated "Limitations" section with rationale (e.g., why hot-reload not supported in MVP).

## Files/Packages Affected
- `crates/maproom/CLAUDE.md` (add .maproomignore section)
- Potentially CLI help text files (if help text stored separately)

## Verification Notes
The verify-ticket agent should confirm:
1. `.maproomignore` section exists in `crates/maproom/CLAUDE.md`
2. All required subsections present (Usage, Syntax, Example, Precedence, Integration, Limitations)
3. Example .maproomignore file is realistic and well-commented
4. Pattern syntax clearly explained with examples
5. Pattern precedence clearly explained (additive behavior)
6. Integration details reference correct functions and modules
7. Limitations section documents hot-reload and subdirectory restrictions
8. Documentation builds: `cargo doc --no-deps` succeeds
9. No broken links or references
10. Formatting is consistent with rest of CLAUDE.md

**Manual review checklist**:
- [ ] Example patterns are realistic (test-fixtures, build artifacts, etc.)
- [ ] Syntax explanation matches actual glob implementation
- [ ] Precedence explanation matches architecture.md
- [ ] Integration points reference correct code locations
- [ ] Limitations include all known restrictions from plan.md
- [ ] Language is clear and accessible

**Tests pass**: N/A (documentation-only ticket)
