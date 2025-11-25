# Project Review: Test Environment Infrastructure (TESTENV)

**Review Date**: 2025-11-25
**Reviewer**: Claude Code
**Project Status**: Planning Complete, Pre-Ticket Creation

---

## Executive Summary

**Recommendation: PROCEED WITH MODIFICATIONS**

The TESTENV project is well-designed and addresses a real problem (5 failing tests due to daemon unavailability). However, the review identified significant opportunities to **reduce scope by 50%** by leveraging existing infrastructure that wasn't fully considered in the planning phase.

### Key Findings

| Category | Finding | Impact |
|----------|---------|--------|
| **Duplication** | Dockerfile already exists at `/workspace/Dockerfile.maproom` | Phase 2 scope reduced |
| **Duplication** | Fixture script pattern exists at `crates/maproom/scripts/create_fixture.sh` | Phase 1 effort reduced |
| **Gap** | CI workflow already uses Rust migrations, not SQL init | Plan needs alignment |
| **Risk** | Test corpus design underdefined | May cause rework |

---

## Detailed Analysis

### 1. Reinvention/Duplication Check

#### CRITICAL: Dockerfile Already Exists

**Finding**: The plan proposes creating `crates/maproom/Dockerfile` in TESTENV-2001, but a production-ready Dockerfile already exists at `/workspace/Dockerfile.maproom`.

**Existing Dockerfile Features**:
- Multi-stage build (rust:1.82-slim → debian:bookworm-slim)
- Non-root user (`maproom`, uid 1000)
- Health check endpoint (port 3000)
- Stripped binary for size optimization
- Already follows all security recommendations in `security-review.md`

**Impact**: TESTENV-2001 should be **modified** to add the daemon service to docker-compose using the existing Dockerfile, not create a new one.

**Recommendation**: Update plan to reference existing `Dockerfile.maproom` instead of creating new one.

#### MODERATE: Fixture Script Pattern Exists

**Finding**: The plan correctly identifies `crates/maproom/scripts/create_fixture.sh` as a pattern to follow. However, it proposes creating a new script rather than adapting the existing one.

**Existing Script Features**:
- Stratified sampling by language/kind
- COPY format for proper escaping
- Preserves FK relationships
- Sequence reset handling
- Verification queries

**Recommendation**: TESTENV-1002 should adapt the existing script, not reinvent it.

### 2. Architecture Alignment

#### CI Workflow Alignment

**Finding**: The current CI workflow (`test.yml`) initializes the database using **Rust migrations** (`crewchief-maproom db migrate`), not SQL dumps. The plan proposes SQL fixtures loaded via `psql`.

**Current CI Flow**:
```yaml
- name: Initialize test database schema
  run: |
    cargo build --release --bin crewchief-maproom
    MAPROOM_DATABASE_URL=... ./target/release/crewchief-maproom db migrate
```

**Plan Proposes**:
```typescript
// ensure-test-db.ts
function loadTestFixtures(): void {
  execSync(`docker exec -i ${CONTAINER} psql -U maproom -d maproom_test`, {
    input: fixtureSQL
  })
}
```

**Compatibility**: These approaches are compatible - schema via migrations, fixtures via SQL. However, the plan should clarify that:
1. Schema initialization happens via Rust migrations (already working in CI)
2. Fixtures are *data only*, not schema

**Impact**: Low risk, but documentation should be clearer.

### 3. Gap Analysis

#### Test Corpus Design Under-Specified

**Finding**: TESTENV-1001 "Design test corpus with known query results" is critical but underspecified. The quality-strategy.md mentions expected queries but doesn't define:

1. **What files should be in the corpus?** The plan says `/tmp/semrank-test-corpus` but this directory doesn't exist
2. **What exact chunks should produce what rankings?** Only 5 example queries shown
3. **How to regenerate when schema changes?** Process unclear

**Risk**: Without clear corpus design, fixture generation (TESTENV-1002, 1003) may require multiple iterations.

**Recommendation**: Add acceptance criteria to TESTENV-1001:
- [ ] Corpus source files versioned in repository
- [ ] At least 10 documented query→result pairs
- [ ] Regeneration script with clear instructions

#### E2E Test Skip Logic Not Defined

**Finding**: The plan mentions `describe.skipIf(!isDaemonAvailable())` but doesn't specify:
- Where the environment variable comes from
- How CI sets it for E2E jobs
- How developers enable/disable daemon locally

**Recommendation**: Add to TESTENV-2003/2004 acceptance criteria.

### 4. Risk Assessment

| Risk | Plan Assessment | Reviewer Assessment | Notes |
|------|-----------------|---------------------|-------|
| Fixture schema drift | Medium | **High** | Fixtures must match migrations, requires CI validation |
| Daemon build failures | Low | **Very Low** | Dockerfile already proven |
| Flaky E2E tests | Medium | Medium | Health check waits mitigate |
| CI timeout | Low | Low | Parallel jobs help |
| Test corpus undefined | Not assessed | **High** | Critical path item |

### 5. Scope Optimization

#### Tickets That Can Be Simplified

| Ticket | Original Scope | Recommended Change |
|--------|---------------|-------------------|
| TESTENV-2001 | "Create Dockerfile for maproom daemon" | "Add daemon service to docker-compose using existing Dockerfile.maproom" |
| TESTENV-1002 | "Create fixture generation script" | "Adapt existing create_fixture.sh for MCP test corpus" |
| TESTENV-2005 | "Add E2E tests to CI pipeline" | May be optional for MVP - daemon already builds in CI |

#### Suggested Ticket Consolidation

**Option A: Keep all 12 tickets** (current plan)
- Pro: Clear separation of concerns
- Con: More overhead, some tickets are tiny

**Option B: Consolidate to 8 tickets** (recommended)
- Merge TESTENV-2001 + 2002 (both docker-compose changes)
- Merge TESTENV-1002 + 1003 (script + run it)
- Remove TESTENV-2005 from MVP (CI already has good coverage)

---

## Execution Readiness Checklist

### Phase 1: SQL Test Fixtures

| Criterion | Status | Notes |
|-----------|--------|-------|
| Test corpus source files exist | **NO** | Need to create or identify |
| Schema is stable | YES | Migrations 0018-0020 done |
| Fixture loading mechanism clear | YES | psql via docker exec |
| Success criteria measurable | YES | 397 tests passing |
| Dependencies satisfied | YES | PostgreSQL ready |

**Phase 1 Readiness**: **75%** - Blocked on corpus definition

### Phase 2: Dockerized Daemon

| Criterion | Status | Notes |
|-----------|--------|-------|
| Dockerfile exists | **YES** | `/workspace/Dockerfile.maproom` |
| Docker Compose infrastructure exists | YES | `crewchief-dev-env` working |
| Container networking clear | YES | `maproom-network` defined |
| Health check defined | YES | In existing Dockerfile |
| E2E test patterns exist | Partial | Need helper functions |

**Phase 2 Readiness**: **90%** - Mostly infrastructure reuse

---

## Recommendations

### MUST Address Before Ticket Creation

1. **Update TESTENV-2001** to use existing `Dockerfile.maproom` instead of creating new
2. **Define test corpus location** - where will source files live? Suggest `packages/maproom-mcp/tests/corpus/`
3. **Clarify CI fixture loading** - ensure plan aligns with existing Rust migration approach

### SHOULD Address (Non-Blocking)

1. **Consolidate tickets** per Option B above (8 instead of 12)
2. **Add corpus version tracking** - link fixture version to schema version
3. **Document E2E environment variable** - `MAPROOM_DAEMON_URL` or similar

### NICE TO HAVE

1. **Consider testcontainers** - Node.js testcontainers could simplify setup
2. **Add fixture diff check** - CI job to warn when fixtures need regeneration

---

## Final Assessment

| Dimension | Score | Notes |
|-----------|-------|-------|
| Problem Definition | 9/10 | Clear problem, good analysis |
| Solution Design | 7/10 | Good but didn't leverage existing Dockerfile |
| Risk Assessment | 6/10 | Missed corpus definition risk |
| Scope Appropriateness | 7/10 | Could be reduced with consolidation |
| Implementation Readiness | 7/10 | Phase 1 blocked on corpus design |

**Overall Score: 7.2/10**

**Verdict**: PROCEED with the modifications listed above. The project is fundamentally sound and addresses a real need. The identified issues are fixable without major restructuring.

---

## Action Items for Plan Update

- [ ] Update `plan.md` to reference existing `Dockerfile.maproom`
- [ ] Add test corpus file location to `architecture.md`
- [ ] Update TESTENV-1001 acceptance criteria with corpus requirements
- [ ] Consider consolidating tickets per Option B
- [ ] Update `analysis.md` to note existing Dockerfile

Once these updates are made, the project is ready for ticket creation via `/create-project-tickets TESTENV`.
