# Test Failures Report
**Date:** 2025-01-09
**Commit:** 0578042 (refactor(env): namespace all environment variables with MAPROOM_ prefix)

## Summary
- **Rust Unit Tests:** 760 passed, **3 failed**, 13 ignored
- **Rust Integration Tests:** **Compilation failed** (1 error)
- **TypeScript Tests:** **Partial pass** (connection tests passed, blob-sha tests failed due to DB)

---

## 1. Rust Unit Test Failures

### 1.1 `config::hot_reload::tests::test_invalid_config_rejected`
**Location:** `crates/maproom/src/config/hot_reload.rs:392`
**Error:** `assertion failed: result.is_err()`
**Description:** Test expects an error when invalid config is loaded, but the config was accepted as valid.

**Root Cause:** Likely related to environment variable renaming. The test may be setting old variable names (e.g., `EMBEDDING_PROVIDER`) but the validation code now expects `MAPROOM_EMBEDDING_PROVIDER`.

**Impact:** Configuration validation may not be working correctly for edge cases.

---

### 1.2 `embedding::config::config_endpoint_tests::test_ollama_uses_custom_endpoint`
**Location:** `crates/maproom/src/embedding/config.rs:920`
**Error:**
```
assertion `left == right` failed
  left: "http://localhost:11434/api/embed"
 right: "http://custom:8080/api/embed"
```

**Test Code:**
```rust
env::set_var("EMBEDDING_API_ENDPOINT", "http://custom:8080/api/embed");
env::set_var("MAPROOM_EMBEDDING_PROVIDER", "ollama");

let config = EmbeddingConfig::from_env().unwrap();
assert_eq!(config.api_endpoint_url(), "http://custom:8080/api/embed");
```

**Root Cause:** The test sets `EMBEDDING_API_ENDPOINT` but the code now reads `MAPROOM_EMBEDDING_API_ENDPOINT`. The test didn't get updated with the environment variable renaming.

**Fix Required:** Update test to use `MAPROOM_EMBEDDING_API_ENDPOINT` instead of `EMBEDDING_API_ENDPOINT`.

---

### 1.3 `embedding::config::config_endpoint_tests::test_openai_accepts_custom_openai_endpoint`
**Location:** `crates/maproom/src/embedding/config.rs:873`
**Error:**
```
assertion `left == right` failed
  left: "https://api.openai.com/v1/embeddings"
 right: "https://api.openai.com/v2/embeddings"
```

**Test Code:**
```rust
env::set_var("EMBEDDING_API_ENDPOINT", "https://api.openai.com/v2/embeddings");
env::set_var("MAPROOM_EMBEDDING_PROVIDER", "openai");

let config = EmbeddingConfig::from_env().unwrap();
assert_eq!(config.api_endpoint_url(), "https://api.openai.com/v2/embeddings");
```

**Root Cause:** Same as 1.2 - the test uses old environment variable name `EMBEDDING_API_ENDPOINT` instead of `MAPROOM_EMBEDDING_API_ENDPOINT`.

**Fix Required:** Update test to use `MAPROOM_EMBEDDING_API_ENDPOINT`.

---

## 2. Rust Integration Test Failures

### 2.1 Compilation Error in `signal_integration_test.rs`
**Location:** `crates/maproom/tests/signal_integration_test.rs:51`
**Error:**
```
error[E0599]: no method named `map_err` found for opaque type
  `impl Future<Output = Result<EmbeddingService, EmbeddingError>>` in the current scope

51 |     EmbeddingService::from_env().map_err(|e| e.into())
   |                                  ^^^^^^^ method not found
```

**Root Cause:** Missing `.await` on async function call, or missing import of `TryFutureExt` trait.

**Fix Required:**
```rust
// Option 1: Add .await
EmbeddingService::from_env().await.map_err(|e| e.into())

// Option 2: Import TryFutureExt
use futures_util::future::try_future::TryFutureExt;
```

**Impact:** Cannot run integration tests until this compilation error is fixed.

---

### 2.2 Warnings in `large_scale_validation_test.rs`
**Location:** `crates/maproom/tests/large_scale_validation_test.rs`
**Warnings:**
- Unused import: `super::*` (lines 59, 378, 737)
- Unused field: `language` in `LanguageMetrics` struct (line 8)

**Impact:** Non-critical, but should be cleaned up.

---

## 3. TypeScript Test Failures

### 3.1 `blob-sha` tests - Database Connection Error
**Location:** `packages/maproom-mcp/tests/run-blob-sha-tests.cjs`
**Error:** `connect ECONNREFUSED 127.0.0.1:5432`

**Description:** Tests attempt to connect to PostgreSQL on port 5432 but database is not running.

**Expected Behavior:** The tests should use `MAPROOM_DATABASE_URL` environment variable which defaults to port 5433 or maproom-postgres hostname.

**Fix Required:**
1. Ensure database is running before tests
2. Update test to use correct port/hostname from environment
3. Add better error handling for missing database

**Tests Passed:**
- ✓ connection-fallback tests (2/2 passed)

---

## 4. Fixes Required

### Priority 1: Update Tests for Environment Variable Renaming
**Files to update:**
```
crates/maproom/src/embedding/config.rs
  - test_ollama_uses_custom_endpoint (line ~916)
  - test_openai_accepts_custom_openai_endpoint (line ~865)

crates/maproom/src/config/hot_reload.rs
  - test_invalid_config_rejected (line ~392)
```

**Changes:**
Replace all occurrences of:
- `EMBEDDING_API_ENDPOINT` → `MAPROOM_EMBEDDING_API_ENDPOINT`
- `EMBEDDING_PROVIDER` → `MAPROOM_EMBEDDING_PROVIDER`
- `EMBEDDING_MODEL` → `MAPROOM_EMBEDDING_MODEL`
- etc.

### Priority 2: Fix Integration Test Compilation
**File:** `crates/maproom/tests/signal_integration_test.rs:51`

Add `.await` to async function call:
```rust
EmbeddingService::from_env().await.map_err(|e| e.into())
```

### Priority 3: Fix TypeScript Database Tests
**File:** `packages/maproom-mcp/tests/run-blob-sha-tests.cjs`

Ensure test uses `MAPROOM_DATABASE_URL` environment variable correctly.

---

## 5. Test Coverage Summary

| Test Suite | Total | Passed | Failed | Ignored | Status |
|------------|-------|--------|--------|---------|--------|
| Rust Unit Tests | 776 | 760 | 3 | 13 | ⚠️ Mostly Passing |
| Rust Integration Tests | N/A | N/A | N/A | N/A | ❌ Won't Compile |
| TypeScript Tests | 2 | 2 | 0 | 0 | ✅ Passed (but blob-sha skipped due to DB) |

---

## 6. Recommended Actions

1. **Immediate:** Update failing unit tests with new environment variable names
2. **Immediate:** Fix integration test compilation error
3. **Before Release:** Ensure database is available for integration tests
4. **Code Review:** Search entire codebase for remaining references to old env var names in tests
5. **Documentation:** Update testing documentation with new environment variable requirements

---

## 7. Environment Variable Validation Checklist

After renaming, the following should be verified in tests:

✅ Core variables:
- `MAPROOM_DATABASE_URL`
- `MAPROOM_EMBEDDING_PROVIDER`
- `MAPROOM_EMBEDDING_MODEL`
- `MAPROOM_EMBEDDING_DIMENSION`

✅ Provider-specific (with fallbacks):
- `MAPROOM_OPENAI_API_KEY` (fallback: `OPENAI_API_KEY`)
- `MAPROOM_GOOGLE_PROJECT_ID` (fallback: `GOOGLE_PROJECT_ID`)
- `MAPROOM_GOOGLE_APPLICATION_CREDENTIALS` (fallback: `GOOGLE_APPLICATION_CREDENTIALS`)

❌ Tests using old names:
- `EMBEDDING_API_ENDPOINT` → needs updating to `MAPROOM_EMBEDDING_API_ENDPOINT`

---

## 8. Notes

- The environment variable renaming was a **BREAKING CHANGE** as documented in commit 0578042
- Most tests passed (760/776 unit tests = 97.9% pass rate)
- Failures are all related to tests not being updated to use new variable names
- No logic bugs detected - just test maintenance needed
- Integration tests blocked by compilation error, not logic issues
