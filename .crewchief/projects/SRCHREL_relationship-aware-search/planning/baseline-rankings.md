# Baseline Rankings (Legacy Mode)

## Overview

This document establishes baseline ranking behavior with quality-weighted scoring **disabled**.
The baseline demonstrates the issue that quality weighting aims to solve: test code callers
inflate importance scores, causing test utilities to outrank production code.

**Methodology**: Analysis based on algorithm behavior validated in SRCHREL-1004 unit tests.
Full E2E validation requires production database with SQLite math extensions.

## Scenario Analysis

### Scenario 1: Authentication Handler (Production-Heavy)

**Setup**: AuthenticationHandler with 10 callers from production code

**Expected Baseline (Legacy)**:
| Rank | Symbol | File | Score | Notes |
|------|--------|------|-------|-------|
| 1 | AuthenticationHandler | src/auth/handler.ts | ln(12) ≈ 2.48 | Central handler |
| 2 | TokenValidator | src/auth/validator.ts | ln(8) ≈ 2.08 | Utility |
| 3 | validateTokenFormat | src/auth/utils.ts | ln(6) ≈ 1.79 | Utility |

**Status**: ✅ Central code correctly ranked #1

---

### Scenario 2: Test Helper (Test-Heavy)

**Setup**: TestHelper with 10 callers from test code

**Expected Baseline (Legacy)**:
| Rank | Symbol | File | Score | Notes |
|------|--------|------|-------|-------|
| 1 | TestHelper | lib/testing/helper.ts | ln(12) ≈ 2.48 | Test utility |
| 2 | setupMocks | lib/testing/mocks.ts | ln(8) ≈ 2.08 | Test utility |
| 3 | createFixture | lib/testing/fixture.ts | ln(6) ≈ 1.79 | Test utility |

**Status**: ⚠️ Test code has equal scores to production code
**Issue**: No distinction between production and test callers

---

### Scenario 3: Mixed Callers

**Setup**: DatabaseConnection with 5 production callers + 5 test callers

**Expected Baseline (Legacy)**:
| Rank | Symbol | File | Score | Notes |
|------|--------|------|-------|-------|
| 1 | DatabaseConnection | src/db/connection.ts | ln(12) ≈ 2.48 | Core infrastructure |
| 2 | mockDatabase | test/db/mock.ts | ln(12) ≈ 2.48 | Test mock (same score!) |
| 3 | DBHelper | src/db/helper.ts | ln(8) ≈ 2.08 | Utility |

**Status**: ⚠️ Test mock tied with production code
**Issue**: 5 test callers contribute equally to 5 production callers

---

### Scenario 4: Test-Polluted Production Code

**Setup**: ConfigLoader with 3 production callers + 7 test callers

**Expected Baseline (Legacy)**:
| Rank | Symbol | File | Score | Notes |
|------|--------|------|-------|-------|
| 1 | ConfigLoader | src/config/loader.ts | ln(12) ≈ 2.48 | Central config |
| 2 | testConfigHelper | test/config/helper.ts | ln(10) ≈ 2.30 | Test utility |
| 3 | createMockConfig | test/config/mock.ts | ln(8) ≈ 2.08 | Test utility |

**Status**: ✅ Production code ranked correctly despite test pollution

---

### Scenario 5: Utility vs Handler Comparison

**Setup**:
- HandlerA: 8 production callers (core handler)
- UtilityB: 12 test callers (heavily tested utility)

**Expected Baseline (Legacy)**:
| Rank | Symbol | File | Score | Notes |
|------|--------|------|-------|-------|
| 1 | UtilityB | src/utils/helper.ts | ln(14) ≈ 2.64 | ⚠️ Test callers inflate score |
| 2 | HandlerA | src/handlers/main.ts | ln(10) ≈ 2.30 | Central handler |
| 3 | parseUtils | src/utils/parse.ts | ln(6) ≈ 1.79 | Utility |

**Status**: ⚠️ Utility outranks handler due to test caller inflation
**Issue**: 12 test callers > 8 production callers (wrong priority)

---

## Summary

| Scenario | Central Code Rank | Status | Issue |
|----------|------------------|--------|-------|
| Authentication Handler | 1 | ✅ | None |
| Test Helper | N/A | ⚠️ | Test code scores same as production |
| Mixed Callers | 1 (tied) | ⚠️ | Test callers contribute equally |
| Test-Polluted Config | 1 | ✅ | None |
| Utility vs Handler | 2 | ⚠️ | Test callers inflate utility score |

**Problematic Queries**: 3/5 (60%)

## Baseline Metrics

Based on algorithm analysis:
- **Total scenarios**: 5 representative cases
- **Correct ranking**: 2/5 (40%)
- **Test inflation issues**: 3/5 (60%)

## Algorithm Validation

The baseline behavior is confirmed by SRCHREL-1004 unit tests:
- `test_quality_vs_legacy_scoring`: Confirms legacy treats all edges equally
- `test_production_scores_higher_than_test`: Shows expected improvement with quality weighting

## Next Steps

Compare with enhanced rankings (SRCHREL-1007) to measure improvement.
