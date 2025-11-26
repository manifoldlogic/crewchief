# SQLINFRA Quality Strategy

## Overview

This project modifies CI/CD workflows and documentation - no application code changes. Quality assurance focuses on:

1. **CI Workflow Integrity** - All existing tests continue to pass
2. **Documentation Accuracy** - Commands and paths work as documented
3. **Link Validity** - No broken internal/external links
4. **User Journey Validation** - New user can follow Quick Start successfully

## Testing Approach

### Test Philosophy

This project is **infrastructure and documentation only**. Traditional unit/integration tests don't apply. Instead, we use:

1. **CI Verification** - Workflow changes validated by running workflows
2. **Manual Smoke Tests** - Documentation commands tested locally
3. **Link Checking** - Automated link validation
4. **Peer Review** - Human review of documentation clarity

### Test Layers

```
┌─────────────────────────────────────────────────┐
│         Manual Validation (Highest Priority)     │
│  • Quick Start walkthrough                       │
│  • PostgreSQL path still works                   │
│  • Documentation flows logically                 │
├─────────────────────────────────────────────────┤
│         CI Workflow Verification                 │
│  • All jobs pass on PR                           │
│  • SQLite jobs run without PostgreSQL            │
│  • PostgreSQL jobs still available               │
├─────────────────────────────────────────────────┤
│         Automated Checks (if applicable)         │
│  • Link validation                               │
│  • Markdown linting                              │
│  • YAML syntax checking                          │
└─────────────────────────────────────────────────┘
```

## Quality Criteria

### CI/CD Workflow Changes

| Criterion | Verification Method | Pass Condition |
|-----------|---------------------|----------------|
| SQLite tests pass | PR CI run | All SQLite jobs green |
| PostgreSQL tests pass | PR CI run | PostgreSQL job green |
| No new failures | Compare to main | Same or better pass rate |
| Workflow syntax valid | GitHub Actions validation | No syntax errors |
| Job dependencies correct | Review workflow graph | Correct job order |

### Documentation Changes

| Criterion | Verification Method | Pass Condition |
|-----------|---------------------|----------------|
| Commands work | Manual execution | All Quick Start commands succeed |
| Paths exist | File system check | Referenced files exist |
| Links valid | Manual or tooling | No 404s or broken anchors |
| Formatting correct | Visual inspection | Renders correctly on GitHub |
| Grammar/clarity | Peer review | Clear and concise |

### Smoke Tests

#### Test 1: SQLite Quick Start (New User Path)

**Steps**:
```bash
# 1. Start with clean environment (no existing ~/.maproom/)
rm -rf ~/.maproom/

# 2. Run Quick Start commands from README
crewchief maproom:scan /path/to/small/repo

# 3. Verify database created
ls ~/.maproom/maproom.db

# 4. Search for something
crewchief maproom:search "function"
```

**Expected**: Commands succeed without Docker/PostgreSQL.

#### Test 2: PostgreSQL Path Still Works

**Steps**:
```bash
# 1. Start PostgreSQL
cd config && docker compose up -d

# 2. Set environment variable
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5433/maproom"

# 3. Run scan
crewchief maproom:scan /path/to/repo

# 4. Search
crewchief maproom:search "function"
```

**Expected**: PostgreSQL path unchanged, commands succeed.

#### Test 3: MCP Server with SQLite

**Steps**:
```bash
# 1. Set SQLite URL
export MAPROOM_DATABASE_URL="sqlite://~/.maproom/maproom.db"

# 2. Start MCP server
npx @crewchief/maproom-mcp

# 3. Send test request (in another terminal)
echo '{"jsonrpc":"2.0","method":"ping","id":1}' | npx @crewchief/maproom-mcp
```

**Expected**: MCP server starts with SQLite, responds to ping.

#### Test 4: VSCode Extension (SQLite Mode)

**Steps**:
1. Open VSCode with extension installed
2. Verify no Docker error messages
3. Ensure extension activates with existing `~/.maproom/maproom.db`
4. Test search command

**Expected**: Extension works without Docker when SQLite database exists.

## Verification Checklist

### Pre-Merge Checklist

- [ ] PR CI passes all jobs
- [ ] README Quick Start tested manually
- [ ] PostgreSQL documentation path tested
- [ ] All internal links verified
- [ ] Markdown renders correctly on GitHub
- [ ] No duplicate or conflicting information
- [ ] Job names are clear and descriptive

### Post-Merge Verification

- [ ] Main branch CI passes
- [ ] Documentation accessible via GitHub UI
- [ ] No user complaints about broken workflows

## Risk Mitigation

### CI Workflow Risks

| Risk | Mitigation |
|------|------------|
| Workflow syntax errors | GitHub validates on PR push |
| Breaking existing tests | Run full matrix before merge |
| Missing job dependencies | Review workflow graph |

### Documentation Risks

| Risk | Mitigation |
|------|------------|
| Outdated commands | Test all commands before merge |
| Broken links | Manual review + optional link checker |
| Confusing flow | Peer review for clarity |

## Acceptance Criteria

### Must Have

1. **CI Passes**: All GitHub Actions jobs pass on PR
2. **Quick Start Works**: SQLite Quick Start succeeds on clean machine
3. **PostgreSQL Works**: Existing PostgreSQL path unchanged
4. **Links Valid**: No broken internal links

### Should Have

1. **Clear Documentation**: Reviewers confirm documentation is clear
2. **Consistent Formatting**: Markdown follows existing patterns
3. **Appropriate Detail Level**: Not too verbose, not too sparse

### Nice to Have

1. **Automated Link Checking**: Tool validates links in CI
2. **Documentation Screenshots**: Visual guides for complex steps
3. **User Feedback**: Early user testing of new Quick Start

## Test Coverage Summary

| Area | Coverage Approach |
|------|-------------------|
| CI Workflows | GitHub Actions validation + PR runs |
| README | Manual smoke test |
| Architecture Docs | Manual review + link check |
| Docker Compose Comments | Visual inspection |
| Internal Links | Manual or automated check |

## Monitoring Post-Release

### Metrics to Watch

1. **CI Pass Rate** - Should remain stable or improve
2. **Documentation Issues** - Track GitHub issues mentioning docs
3. **User Onboarding** - Feedback on Quick Start experience

### Rollback Plan

If issues arise:
1. Revert documentation changes via PR
2. CI workflow changes can be reverted independently
3. No application code to rollback
