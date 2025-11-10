# Execution Plan: CLI-Maproom Alignment

**Project:** CLIMAP - CLI-Maproom Alignment
**Date:** 2025-01-10
**Total Effort:** 1-1.5 days

## Overview

This plan outlines the systematic approach to align the CLI package with maproom, including command structure refactoring, environment validation, and comprehensive documentation updates.

## Phases

### Phase 1: Documentation Fixes (Critical Priority)
**Effort:** 3-4 hours
**Agent:** technical-writer (for documentation) + typescript-engineer (for code examples)
**Goal:** Fix critical documentation errors that cause user connection failures

#### Tasks

**1.1 Update Environment Variable References**
- File: `packages/cli/README.md`
- Replace all `PG_DATABASE_URL` → `MAPROOM_DATABASE_URL` (4 locations)
- Add fallback hierarchy explanation
- Update connection examples to use new variable

**1.2 Add Database Setup Section**
- Document `MAPROOM_DATABASE_URL` format
- Explain 4-tier fallback system
- Add Docker setup instructions
- Include troubleshooting subsection

**1.3 Add Embedding Provider Setup Section**
- Document OpenAI configuration
  - `MAPROOM_EMBEDDING_PROVIDER=openai`
  - `OPENAI_API_KEY` requirement
  - Custom endpoint override
- Document Google Vertex AI configuration
  - `MAPROOM_EMBEDDING_PROVIDER=google`
  - `GOOGLE_PROJECT_ID` requirement
  - `GOOGLE_APPLICATION_CREDENTIALS` path
- Document Ollama configuration
  - `MAPROOM_EMBEDDING_PROVIDER=ollama`
  - Local setup instructions
  - Default endpoint behavior

**1.4 Add Troubleshooting Section**
- Database connection errors
- Embedding provider errors
- Binary not found errors
- Migration state issues

**Acceptance Criteria:**
- [ ] No references to `PG_DATABASE_URL` in README
- [ ] All examples use `MAPROOM_DATABASE_URL`
- [ ] Embedding providers documented with examples
- [ ] Troubleshooting covers common errors

---

### Phase 2: Command Structure Refactoring
**Effort:** 2-3 hours
**Agent:** typescript-engineer
**Goal:** Convert `maproom:` commands to subcommand pattern (clean break, no backward compat)

#### Tasks

**2.1 Refactor Command Registration**
- File: `packages/cli/src/cli/maproom.ts`
- Create parent `maproom` command
- Convert existing commands to subcommands:
  - `maproom:scan` → `maproom scan`
  - `maproom:search` → `maproom search`
  - `maproom:upsert` → `maproom upsert`
  - `maproom:watch` → `maproom watch`
  - `maproom:db` → `maproom db` (with `migrate` subcommand)

**2.2 Add New Commands**
- Register `maproom branch-watch`
- Register `maproom cache`
- Register `maproom generate-embeddings`
- Add appropriate help text for each

**2.3 Update Help Text**
- Update examples to use new syntax
- Add performance flags to scan help
- Document all new commands
- Remove references to old syntax

**Code Changes:**
```typescript
// Before (delete these)
program.command('maproom:scan')...
program.command('maproom:search')...
program.command('maproom:upsert')...
program.command('maproom:watch')...
program.command('maproom:db')...

// After (new structure)
const maproom = program.command('maproom')...
maproom.command('scan')...
maproom.command('search')...
maproom.command('upsert')...
maproom.command('watch')...
maproom.command('db').command('migrate')...
maproom.command('branch-watch')...
maproom.command('cache')...
maproom.command('generate-embeddings')...
```

**Acceptance Criteria:**
- [ ] All new subcommands registered
- [ ] Help text updated
- [ ] All arguments forward correctly
- [ ] Old `maproom:*` commands removed

---

### Phase 3: Environment Validation
**Effort:** 4-6 hours
**Agent:** typescript-engineer
**Goal:** Add pre-flight validation with helpful error messages

#### Tasks

**3.1 Create Validation Module**
- File: `packages/cli/src/cli/maproom-validation.ts` (new)
- Implement `validateMaproomEnvironment()` function
- Return structured result (valid, errors, warnings)
- Check database URL (all variants)
- Check embedding provider configuration
- Provider-specific validation (OpenAI, Google, Ollama)

**3.2 Integrate Validation**
- File: `packages/cli/src/cli/maproom.ts`
- Call validation before forwarding to Rust binary
- Display errors and warnings
- Block on errors, allow on warnings
- Skip validation for help commands

**3.3 Implement Error Display**
- Create `displayValidationResult()` function
- Friendly error messages
- Link to documentation sections
- Actionable next steps

**3.4 Security Audit**
- Ensure no credentials in error messages
- Ensure no connection strings in logs
- Ensure generic messages (reference env var names only)

**Code Structure:**
```typescript
// maproom-validation.ts
export interface ValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}

export function validateMaproomEnvironment(): ValidationResult
export function displayValidationResult(result: ValidationResult): void

// maproom.ts
function runMaproomForward(args: string[]) {
  if (needsValidation(args[0])) {
    const validation = validateMaproomEnvironment()
    displayValidationResult(validation)
    if (!validation.valid) {
      process.exitCode = 1
      return
    }
  }
  // ... forward to binary
}
```

**Acceptance Criteria:**
- [ ] Validation catches missing database URL
- [ ] Validation catches missing provider config
- [ ] Error messages are helpful and link to docs
- [ ] No credentials logged in validation
- [ ] Help commands skip validation

---

### Phase 4: Testing
**Effort:** 2-2.5 hours
**Agent:** unit-test-runner + integration-tester
**Goal:** Comprehensive test coverage for new functionality

#### Tasks

**4.1 Unit Tests**
- File: `packages/cli/tests/unit/maproom-validation.test.ts` (new)
- Test validation with valid config
- Test validation with missing database URL
- Test validation with missing provider
- Test validation with invalid provider config
- Test all provider-specific validations (6+ tests)

**4.2 Integration Tests**
- File: `packages/cli/tests/integration/maproom-commands.int.test.ts` (new)
- Test subcommand registration (help text)
- Test argument forwarding
- Test validation error display
- Test validation doesn't block help

**4.3 Manual Testing**
- Run manual testing checklist (from quality-strategy.md)
- Verify all command combinations work
- Test with real environment setup
- Verify error messages are helpful
- Confirm performance is acceptable

**4.4 Regression Testing**
- Run existing CLI test suite: `pnpm test`
- Verify all 922 tests still pass
- Test other CLI commands (worktree, agent, spawn)
- Ensure no side effects

**Acceptance Criteria:**
- [ ] Unit tests: 6+ tests, all passing
- [ ] Integration tests: 10+ tests, all passing (no backward compat needed)
- [ ] Existing tests: All 922 still passing
- [ ] Manual checklist: All items verified
- [ ] Performance: No significant regression

---

### Phase 5: Documentation Updates
**Effort:** 2-3 hours
**Agent:** technical-writer
**Goal:** Complete documentation with new commands and features

#### Tasks

**5.1 Command Reference**
- Document all maproom subcommands
- Add examples for each command
- Document new flags (`--force`, `--parallel`, `--provider`)
- Show common use cases

**5.2 Performance Optimization Section**
- Explain incremental scanning
- Document parallel processing
- Show batch size tuning
- Provide before/after performance examples

**5.3 Schema & Features Section**
- Explain blob_sha (content addressing)
- Explain code_embeddings (deduplication)
- Explain worktree_ids (branch-aware search)
- Show migration status check

**5.4 Security Best Practices**
- Document secure credential management
- Show `.env` file usage
- Recommend secret managers
- Warn about credential exposure

**Acceptance Criteria:**
- [ ] All commands documented
- [ ] Examples are accurate
- [ ] Performance section clear
- [ ] Security guide complete

---

### Phase 6: Security & Quality Assurance
**Effort:** 1-2 hours
**Agent:** typescript-engineer + verify-ticket
**Goal:** Final security audit and quality checks

#### Tasks

**6.1 Security Audit**
- Review all validation error messages
- Ensure no credential leaks
- Check for shell injection vectors
- Verify dependency security (`pnpm audit`)
- Review security-review.md checklist

**6.2 Code Review**
- Review all changed files
- Verify consistent code style
- Check for edge cases
- Ensure proper error handling

**6.3 Performance Check**
- Measure command startup time
- Compare with baseline
- Ensure <10ms regression
- Verify validation is lightweight

**6.4 Final Verification**
- Run complete test suite
- Execute manual testing checklist
- Verify documentation accuracy
- Confirm all acceptance criteria met

**Acceptance Criteria:**
- [ ] No security vulnerabilities
- [ ] Code review approved
- [ ] Performance acceptable
- [ ] All tests passing
- [ ] Documentation accurate

---

## Agent Assignments

### Phase 1: Documentation Fixes
- **Primary:** technical-writer
- **Support:** typescript-engineer (for code examples)

### Phase 2: Command Refactoring
- **Primary:** typescript-engineer
- **Testing:** unit-test-runner

### Phase 3: Environment Validation
- **Primary:** typescript-engineer
- **Testing:** unit-test-runner

### Phase 4: Testing
- **Primary:** integration-tester
- **Support:** unit-test-runner

### Phase 5: Documentation Updates
- **Primary:** technical-writer
- **Review:** typescript-engineer

### Phase 6: Security & QA
- **Primary:** verify-ticket
- **Support:** typescript-engineer

## Dependencies Between Phases

```
Phase 1 (Docs) ─────────────┐
                            │
Phase 2 (Commands) ─────────┼──→ Phase 4 (Testing) ─→ Phase 6 (QA) ─→ Commit
                            │         ↑
Phase 3 (Validation) ───────┘         │
                                      │
Phase 5 (Docs Updates) ───────────────┘
```

**Critical Path:** Phases 2 & 3 → Phase 4 → Phase 6
**Parallel Work:** Phase 1 can happen early, Phase 5 can overlap with testing

## Rollback Plan

### If Issues Found During Testing

**Scenario:** Integration tests fail or validation has bugs

**Action:**
1. Isolate problematic phase (command refactor vs. validation)
2. Fix issues in place (refactoring is low-risk)
3. Re-run tests
4. If unfixable quickly, revert that phase only

**Rollback Steps:**
```bash
# Revert command structure changes
git checkout HEAD -- packages/cli/src/cli/maproom.ts

# Revert validation layer
git rm packages/cli/src/cli/maproom-validation.ts
git checkout HEAD -- packages/cli/src/cli/maproom.ts

# Keep documentation changes (they're always safe)
```

**Note:** No backward compatibility concerns since tool has no users

## Risk Mitigation

### High-Risk Items

**Validation blocking users:**
- Mitigation: Comprehensive test coverage
- Testing: Integration tests with various env configs
- Fallback: Make validation warnings-only, remove blocks

### Medium-Risk Items

**Command naming change:**
- Mitigation: N/A (no users)
- Testing: Integration tests verify new commands work
- Fallback: Easy to revert if needed

**Documentation accuracy:**
- Mitigation: Manual testing of all examples
- Testing: Try each documented command
- Fallback: Quick doc fixes (no code changes)

**Performance regression:**
- Mitigation: Benchmark before/after
- Testing: Time command startup
- Fallback: Remove validation if too slow (unlikely)

## Success Metrics

### Immediate (Post-Merge)

- ✅ All 922+ tests passing
- ✅ No security vulnerabilities
- ✅ Documentation complete
- ✅ Clean command structure (no legacy cruft)

### Short-Term (1-2 Weeks)

- 📊 Personal usage validates new command structure
- 📊 Bug reports (target: 0 critical, <3 minor)
- 📊 Documentation clarity

### Long-Term (As Tool Gains Users)

- 📊 Consistent command patterns across all features
- 📊 Reduced support burden (better error messages)
- 📊 Clean codebase with no deprecated code

## Timeline

**Day 1 (8 hours):**
- Morning: Phase 1 (Docs fixes) - 3 hours
- Afternoon: Phase 2 (Command refactor) - 2 hours
- Evening: Phase 3 (Validation) - 3 hours

**Day 2 (4-6 hours):**
- Morning: Phase 4 (Testing) - 2 hours
- Late Morning: Phase 5 (Docs updates) - 2 hours
- Afternoon: Phase 6 (Security & QA) - 1 hour
- Final review and commit - 1 hour

**Total:** 1-1.5 days (faster with no backward compat)

## Deliverables

### Code Changes
- `/packages/cli/src/cli/maproom.ts` - Refactored command structure
- `/packages/cli/src/cli/maproom-validation.ts` - New validation module
- `/packages/cli/tests/unit/maproom-validation.test.ts` - Unit tests
- `/packages/cli/tests/integration/maproom-commands.int.test.ts` - Integration tests

### Documentation
- `/packages/cli/README.md` - Comprehensive updates
- Changelog entry - New command structure and features

### Quality Artifacts
- Test coverage report
- Performance benchmark results
- Security audit checklist (completed)

## Definition of Done

### Code Complete
- [x] All phases implemented
- [x] All tests written and passing
- [x] Code reviewed
- [x] Security audited

### Documentation Complete
- [x] README updated
- [x] Migration guide created
- [x] Examples tested
- [x] Changelog written

### Quality Gates
- [x] 922+ tests passing (existing + new)
- [x] No security vulnerabilities
- [x] Performance acceptable (<10ms regression)
- [x] All acceptance criteria met
- [x] Manual testing checklist complete

### Ready to Merge
- [x] All tasks complete
- [x] All acceptance criteria met
- [x] Ticket verified
- [x] Conventional commit created

## Post-Merge Tasks

**Immediate:**
- Validate command structure in personal usage
- Be ready for quick fixes if needed
- Document any edge cases discovered

**1 Week:**
- Verify all features working as expected
- Update docs based on any confusion
- Fix any discovered bugs

**Ongoing:**
- Keep CLI in sync with maproom updates
- Document new features as they're added
- Maintain clean, consistent command structure
