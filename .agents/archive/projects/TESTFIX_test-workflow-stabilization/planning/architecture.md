# Architecture: Test Workflow Stabilization

## Solution Design

### Core Approach: Iterative Fix-Push-Verify Loop

```
┌─────────────────────────────────────────────────────────────┐
│                    Workflow Iteration Cycle                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │  Check Latest    │
                    │  Workflow Run    │
                    └──────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │  Failure Found?  │
                    └──────────────────┘
                         │        │
                    YES  │        │  NO
                         │        │
                         ▼        ▼
              ┌──────────────┐  ┌──────────────┐
              │ Create       │  │ SUCCESS!     │
              │ Ticket       │  │ Workflow     │
              └──────────────┘  │ Passes       │
                      │         └──────────────┘
                      ▼
              ┌──────────────┐
              │ Implement    │
              │ Fix          │
              └──────────────┘
                      │
                      ▼
              ┌──────────────┐
              │ Verify &     │
              │ Commit       │
              └──────────────┘
                      │
                      ▼
              ┌──────────────┐
              │ Push to      │
              │ Trigger CI   │
              └──────────────┘
                      │
                      └──────────┐
                                 │
                        (Repeat Cycle)
```

## Architecture Decisions

### Decision 1: One Ticket Per Issue
**Rationale**: Each CI failure is independent and needs isolated tracking
- **Traceability**: Each fix documented in its own ticket
- **Rollback Safety**: Can revert individual fixes without affecting others
- **Parallel Learning**: Team learns from each ticket's approach

**Trade-off**: More overhead per fix vs. faster batch fixes
**Chosen**: Systematic approach over speed for quality

### Decision 2: Push After Each Fix
**Rationale**: Discover next failure immediately
- **Fast Feedback**: Know if fix worked within 1-2 minutes
- **Early Detection**: Find new issues before they compound
- **CI Environment Truth**: Local tests can't replicate CI exactly

**Trade-off**: More workflow runs vs. batching multiple fixes
**Chosen**: Frequent pushes for faster iteration

### Decision 3: Use Existing Ticket Workflow
**Rationale**: Leverage `/single-ticket` and verify-ticket agents
- **Consistent Process**: Same workflow as feature development
- **Automated Verification**: Agents check acceptance criteria
- **Proper Commit Messages**: Conventional commits with ticket references

**Implementation**:
```bash
# For each failure:
1. Create ticket in .agents/projects/TESTFIX_*/tickets/
2. Run: /single-ticket TESTFIX-XXXX
3. Agent implements, verifies, commits
4. Push to trigger new workflow run
5. Check latest run for next failure
```

### Decision 4: Schema-First Fix Approach
**Rationale**: Test failures often indicate schema mismatches
- **Root Cause**: Tests expect schema features not in init.sql
- **Fix Strategy**: Add missing schema elements OR update tests
- **Prefer Schema Additions**: Tests are usually correct expectations

**Priorities**:
1. Check if function/table exists in init.sql
2. If missing, search for similar functions in other files
3. Add missing schema element
4. If no clear schema solution, fix/skip test

## Technology Choices

### Tools Used
- **GitHub CLI (`gh`)**: Check workflow status, view logs
- **Git**: Commit and push after each fix
- **Ticket System**: `.agents/projects/TESTFIX_*/tickets/` structure
- **Agents**: ticket-creator, verify-ticket, commit-ticket

### Constraints
- **CI Environment**: Ubuntu latest, PostgreSQL 16.11, Node 20.19.5
- **Test Database**: `maproom_test` database separate from `maproom`
- **pnpm Version**: 10.12.1 (managed by packageManager field)

## Performance Considerations

### Workflow Execution Time
- Average workflow run: ~1 minute
- Per iteration: ~2-3 minutes (fix + push + verify)
- Expected iterations: 3-5 (based on typical CI stabilization)

**Optimization**: No parallelization needed - sequential is acceptable for this use case

### Database Initialization
- Schema size: ~1600 lines SQL
- Init time: <1 second typically
- Performance impact: Negligible

## Long-Term Maintainability

### Preventing Future Failures

**1. Schema Change Protocol**:
```
When adding database features:
1. Update init.sql with new tables/functions
2. Write tests that verify the feature
3. Ensure both are committed together
4. Run local tests before pushing
```

**2. Test Hygiene**:
```
Test files should:
- Check for existence before using (graceful degradation)
- Document expected schema dependencies
- Have clear failure messages
```

**3. CI Monitoring**:
```
After stabilization:
- Monitor workflow success rate
- Alert on new failures immediately
- Investigate root cause, not just symptoms
```

### Schema Version Control
**Current State**: Single init.sql file with all schema
**Recommended Future**: Migration-based system
- **Benefit**: Track schema changes over time
- **Benefit**: Rollback capability
- **MVP**: Not needed now, but consider for Phase 2

## Risk Mitigation

### Identified Risks & Mitigations

**Risk**: Unknown number of failures
- **Mitigation**: Iterative approach discovers them one at a time
- **Acceptance**: This is inherent to the problem

**Risk**: Breaking changes during fixes
- **Mitigation**: Each fix is isolated, verified, and committed separately
- **Mitigation**: Can revert individual commits

**Risk**: Infinite loop (fix creates new failure)
- **Mitigation**: Each fix addresses root cause, not symptoms
- **Mitigation**: Verification step ensures fix works

**Risk**: Time investment too high
- **Mitigation**: Set max iteration limit (e.g., 10 tickets)
- **Mitigation**: Reassess approach if limit hit

## Implementation Strategy

### Phase 1: Current Failure (Missing Function)
**Issue**: `compute_git_blob_sha` function doesn't exist
**Ticket**: TESTFIX-1001
**Approach**:
1. Search codebase for function definition
2. If found elsewhere, add to init.sql
3. If not found, implement based on test expectations
4. Test expects SHA-256 hash of git blob format

### Phase 2: Discover Next Failure
**After**: TESTFIX-1001 is committed and pushed
**Action**: Check latest workflow run
**Create**: TESTFIX-1002 for next issue

### Phase N: Workflow Passes
**Condition**: No failures in Test workflow
**Action**: Mark project complete
**Documentation**: Update CLAUDE.md with lessons learned

## Architecture Principles

1. **Systematic Over Clever**: Don't try to fix everything at once
2. **Verify Early**: Push after each fix to get fast feedback
3. **Document Everything**: Each ticket captures the issue and solution
4. **Root Cause Focus**: Fix the problem, not the symptom
5. **Schema Truth**: init.sql is source of truth for database structure
