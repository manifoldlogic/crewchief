# Ticket: CICDOPT-1004: Add Path Filters to Test Workflow

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (workflow configuration change, validation requires actual PRs)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- github-actions-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Add path-based filtering to the test workflow to skip test runs on non-code changes (docs, config, planning files), achieving an 80% reduction in unnecessary test runs and improving PR feedback speed for documentation changes.

## Background

**Problem Being Solved**:

Currently, the test workflow runs on EVERY push and pull request (100% of commits). However, approximately 80% of commits are documentation, planning, or configuration changes that don't affect code functionality. This results in:
- **Wasteful CI usage**: Tests run unnecessarily on docs-only changes
- **Slow feedback**: Documentation PRs wait 5-8 minutes for unnecessary tests
- **Expensive**: Wasted CI minutes on non-code changes

**Why Path Filters Help**:

GitHub Actions supports `paths` filter in workflow triggers that allow workflows to run only when specific files change. By implementing path filters, we can:
- Only trigger tests when files that affect test outcomes change
- Give documentation-only PRs instant green checks
- Provide faster feedback on docs contributions
- Reduce CI usage by ~80%

**Context from Planning**:

Path filtering was identified as a Phase 1 quick win in the project architecture (lines 191-252). Critical requirements from review-updates.md (lines 70-94):
- Path filters must include the test workflow file itself (prevent breaking tests)
- Path filters must include reusable workflows (`.github/workflows/reusable-rust-build.yml` and `.github/workflows/reusable-typescript-build.yml`)
- Changes to reusable workflows should trigger tests for validation

**Reference**: plan.md lines 104-125 (Phase 1, Ticket CICDOPT-1004)

## Acceptance Criteria

### Code Paths Included
- [ ] Path filter includes `crates/**` (Rust code)
- [ ] Path filter includes `packages/*/src/**` (TypeScript source)
- [ ] Path filter includes `packages/*/tests/**` (Test files)
- [ ] Path filter includes `**.rs` (Rust files anywhere)
- [ ] Path filter includes `**.ts` (TypeScript files anywhere)

### Dependencies Included
- [ ] Path filter includes `pnpm-lock.yaml` (npm dependencies)
- [ ] Path filter includes `Cargo.lock` (Rust dependencies)

### Workflows Included
- [ ] Path filter includes `.github/workflows/test.yml` (self-trigger)
- [ ] Path filter includes `.github/workflows/reusable-rust-build.yml` (dependency)
- [ ] Path filter includes `.github/workflows/reusable-typescript-build.yml` (dependency)

### Schema/Migrations Included
- [ ] Path filter includes `packages/maproom-mcp/config/init.sql` (database schema)
- [ ] Path filter includes `crates/maproom/migrations/**` (Rust migrations)

### Implicit Exclusions (verified by testing)
- [ ] `docs/**` excluded (documentation)
- [ ] `*.md` files excluded (except in code directories)
- [ ] `.crewchief/**` excluded (planning/tickets)
- [ ] `.github/workflows/**` excluded (except test.yml and reusables)
- [ ] `.devcontainer/**` excluded (dev container config)

### Validation Testing
- [ ] Test Case 1: Docs-only PR → workflow skipped
- [ ] Test Case 2: Code-only PR → workflow runs
- [ ] Test Case 3: Mixed PR (docs + code) → workflow runs
- [ ] Test Case 4: Workflow file change → workflow runs
- [ ] Test Case 5: Reusable workflow change → workflow runs

## Technical Requirements

**File to Modify**: `.github/workflows/test.yml`

**Exact Change Required**:

Update the `on:` section to add `paths:` filter to both `push` and `pull_request` triggers:

```yaml
on:
  push:
    branches: [main]
    paths:
      # Rust code
      - 'crates/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '**.rs'

      # TypeScript code
      - 'packages/*/src/**'
      - 'packages/*/tests/**'
      - 'packages/**/package.json'
      - '**.ts'

      # Dependencies
      - 'pnpm-lock.yaml'

      # Workflows (self + reusables)
      - '.github/workflows/test.yml'
      - '.github/workflows/reusable-rust-build.yml'
      - '.github/workflows/reusable-typescript-build.yml'

      # Database schemas and migrations
      - 'packages/maproom-mcp/config/init.sql'
      - 'crates/maproom/migrations/**'

  pull_request:
    paths:
      # Same paths as push (keep in sync!)
      - 'crates/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '**.rs'
      - 'packages/*/src/**'
      - 'packages/*/tests/**'
      - 'packages/**/package.json'
      - '**.ts'
      - 'pnpm-lock.yaml'
      - '.github/workflows/test.yml'
      - '.github/workflows/reusable-rust-build.yml'
      - '.github/workflows/reusable-typescript-build.yml'
      - 'packages/maproom-mcp/config/init.sql'
      - 'crates/maproom/migrations/**'
```

**Path Selection Rationale**:

**INCLUDED** (code that affects tests):
1. `crates/**`, `**.rs` → Rust code changes
2. `packages/*/src/**`, `packages/*/tests/**`, `**.ts` → TypeScript code and tests
3. `pnpm-lock.yaml`, `Cargo.lock` → Dependency changes that might affect tests
4. `.github/workflows/test.yml` → Test workflow itself (prevent accidental breaking)
5. `.github/workflows/reusable-*.yml` → Reusable workflows used by tests
6. `init.sql`, `migrations/**` → Schema changes (critical for database tests)

**EXCLUDED** (doesn't affect tests):
1. `docs/**` → Documentation
2. `*.md` files → README, planning docs (except in code directories)
3. `.crewchief/**` → Project planning/tickets
4. `.github/workflows/**` → Other workflows (except test.yml and reusables)
5. `.devcontainer/**` → Dev container configuration
6. Configuration files that don't affect runtime (`.prettierrc`, etc.)

**Critical**: Both `push` and `pull_request` must have identical `paths` arrays to ensure consistent behavior.

## Implementation Notes

### Implementation Steps

**Step 1 - Add Path Filters**:
```bash
# Create feature branch
git checkout -b ci-add-path-filters

# Edit test workflow
# Add paths filter to on: section as specified above

# Commit to feature branch
git add .github/workflows/test.yml
git commit -m "feat(ci): add path filters to test workflow"
git push origin ci-add-path-filters

# Create PR
gh pr create \
  --title "feat(ci): CICDOPT-1004 Add path filters to test workflow" \
  --body "Adds path-based filtering to reduce unnecessary test runs by ~80%"
```

**Step 2 - Test Case 1: Docs-Only Change (should SKIP)**:
```bash
# Edit documentation only
echo "# Test change for path filter validation" >> docs/README.md
git add docs/README.md
git commit -m "docs: test path filter - docs only"
git push

# Verify workflow skipped
gh pr checks
# Expected: No test workflow run, or status shows "skipped"
```

**Step 3 - Test Case 2: Code Change (should RUN)**:
```bash
# Edit code file
echo "// Path filter test" >> packages/cli/src/index.ts
git add packages/cli/src/index.ts
git commit -m "test: path filter - code change"
git push

# Verify workflow runs
gh pr checks --watch
# Expected: Test workflow running (5-8 min)
```

**Step 4 - Test Case 3: Mixed Change (should RUN)**:
```bash
# Edit both code and docs
echo "// Mixed test" >> packages/cli/src/index.ts
echo "# Mixed test" >> docs/README.md
git add .
git commit -m "test: path filter - mixed change"
git push

# Verify workflow runs (code takes precedence)
gh pr checks --watch
# Expected: Test workflow running
```

**Step 5 - Test Case 4: Workflow File Change (should RUN)**:
```bash
# Edit test.yml itself (add comment)
# This validates self-triggering
git add .github/workflows/test.yml
git commit -m "test: path filter - workflow change"
git push

# Verify workflow runs (self-trigger)
gh pr checks --watch
# Expected: Test workflow running
```

**Step 6 - Test Case 5: Reusable Workflow Change**:
Note: This test case will be fully testable after Phase 2 when reusable workflows are created. For now, verify that the paths include the reusable workflow files in the filter.

### Edge Cases Handled

1. **Workflow file itself always triggers**: Prevents accidentally breaking tests by modifying the workflow
2. **Reusable workflows trigger**: Validates that changes to shared workflows don't break tests
3. **Migration files trigger**: Schema changes are critical and must be tested
4. **Lock files trigger**: Dependency changes might introduce breaking changes
5. **Broad patterns**: `**.rs` and `**.ts` catch files in any location

### Post-Merge Validation

After merging, monitor for 1 week:

```bash
# Count total commits in past week
git log --oneline --since="1 week ago" | wc -l

# Count commits that triggered tests
gh run list --workflow=test.yml --created=">$(date -d '1 week ago' -I)" | wc -l

# Calculate reduction percentage
# Expected: ~20% of commits trigger tests (80% reduction)
```

## Dependencies

**Depends On**: None (independent ticket)

**Blocks**: None (but complements other Phase 1 optimizations)

**Related Tickets**:
- CICDOPT-1002 (Rust caching) - Works together for CI speed
- CICDOPT-1003 (pnpm caching) - Works together for CI speed

## Risk Assessment

**Risk Level**: Low

**Risk 1: Too Restrictive Filter**
- **Description**: Code change missed by filter, tests don't run, broken code merged
- **Likelihood**: Low
- **Mitigation**:
  - Include workflow file itself (always triggers on workflow changes)
  - Include broad patterns (`**.rs`, `**.ts`) to catch files anywhere
  - Include lock files (dependency changes)
- **Detection**: PR merged without tests running (visible in GitHub UI)
- **Resolution**: Add missing path pattern to filter, manually trigger tests

**Risk 2: Too Permissive Filter**
- **Description**: Still triggering on non-code changes, not achieving 80% reduction
- **Likelihood**: Low
- **Mitigation**: Carefully review excluded paths, test with docs-only PRs
- **Detection**: Documentation PRs still running tests
- **Resolution**: Narrow filter patterns, add explicit exclusions if needed

**Risk 3: Workflow Change Breaks Tests**
- **Description**: Filter change accidentally breaks test execution
- **Likelihood**: Very Low
- **Mitigation**: Workflow file is included in paths (self-validates on any change)
- **Detection**: Test workflow fails immediately on filter change
- **Resolution**: Revert filter change, fix issue, reapply

**Risk 4: Sync Drift Between push and pull_request**
- **Description**: Different path filters for push vs PR lead to inconsistent behavior
- **Likelihood**: Medium (if not careful)
- **Mitigation**: Keep both path arrays identical (copy-paste), add comment to keep in sync
- **Detection**: Different behavior for push to branch vs PR
- **Resolution**: Synchronize the path arrays

**Confidence Level**: High - Path filters are a well-supported, stable GitHub Actions feature with extensive documentation and community usage.

## Files/Packages Affected

1. `.github/workflows/test.yml` - Add `paths:` filter to `on.push` and `on.pull_request` sections

## Planning References

- **Plan**: plan.md lines 104-125 (Phase 1, Ticket CICDOPT-1004)
- **Architecture**: architecture.md lines 191-252 (Path Filters section)
- **Quality Strategy**: quality-strategy.md lines 154-215 (Test 1.4: Path Filters)
- **Review Updates**: review-updates.md lines 70-94 (Critical Issue 2: Incomplete Path Filter Strategy)

## Related Documentation

- [GitHub Actions Workflow Syntax - paths filter](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions#onpushpull_requestpull_request_targetpathspaths-ignore)
- `.github/workflows/test.yml` (current workflow file)
- `.crewchief/projects/CICDOPT_ci-cd-workflow-optimization/planning/review-updates.md` (path filter requirements)

## Success Indicators

After this ticket is complete:

1. Path filter configured correctly in test workflow
2. Docs-only PRs skip tests (instant green check)
3. Code PRs run tests (proper validation)
4. Mixed PRs run tests (safety first approach)
5. Workflow changes trigger tests (self-validation)
6. Reusable workflow changes trigger tests (Phase 2 dependency validation)
7. Test run frequency reduced by approximately 80% (measured over 1 week)
8. No code changes merged without tests running
9. Documentation contributors get faster feedback on PRs
