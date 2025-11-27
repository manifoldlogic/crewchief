# Ticket: TESTFIX-1003: Fix All Rust Test Compilation Errors

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
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix all 190 Rust test compilation errors in crates/maproom using mechanical API migrations. Tests have fallen out of sync with implementation due to API changes in EmbeddingService, ChangeType enum, SearchOptions, and other structures.

## Background
The `crewchief-maproom` Rust crate underwent significant refactoring. Tests were not updated to match the new APIs. This is Phase 2 of the TESTFIX project - fixing Rust test compilation. The errors are mechanical transformations that follow clear patterns documented in the architecture.md.

This ticket implements the Rust test compilation fixes from the TESTFIX project plan, enabling the test suite to compile successfully before addressing any test logic or assertion issues.

## Acceptance Criteria
- [ ] `cargo check --tests` exits with 0 errors
- [ ] All test files in `crates/maproom/tests/` compile successfully
- [ ] No new warnings introduced (existing warnings acceptable)
- [ ] Tests preserve their original intent (not just made to compile by removing assertions)

## Technical Requirements

### Pattern 1: EmbeddingService API (~42 errors)
Fix tests using old constructor pattern:
```rust
// Before (broken)
let service = EmbeddingService::new(config);

// After (fixed) - Option A: Use from_env()
let service = EmbeddingService::from_env().await?;

// After (fixed) - Option B: Manual construction
use crewchief_maproom::embedding::{
    factory::create_provider_from_env,
    cache::EmbeddingCache,
    config::CacheConfig,
};
let provider = create_provider_from_env().await?;
let cache = Arc::new(EmbeddingCache::new(CacheConfig::default())?);
let service = EmbeddingService::new(provider, cache);
```

### Pattern 2: ChangeType Enum (~35 errors)
Convert from struct-like to tuple variants:
```rust
// Before (broken)
ChangeType::New { hash: content_hash }
ChangeType::Deleted { hash: old_hash }

// After (fixed)
ChangeType::New(content_hash)
ChangeType::Deleted(old_hash)
```

### Pattern 3: SearchOptions (~19 errors)
Remove deleted `include_debug` field:
```rust
// Before (broken)
SearchOptions { include_debug: true, ... }

// After (fixed)
SearchOptions { /* other fields only */ }
```

### Pattern 4: FinalSearchResults (~8 errors)
Update field access:
```rust
// Before (broken)
results.timing
result.fts_score
result.vector_score

// After (fixed)
results.metadata
result.source_scores.get(&SearchSource::Fts)
result.source_scores.get(&SearchSource::Vector)
```

### Pattern 5: BasicWeightedFusion (~5 errors)
Update method calls for fusion API changes.

### Pattern 6: Missing Modules (~6 errors)
- Remove `mod common` references that fail to find module
- Fix private module access (`feature_flags`)
- Fix metrics registry imports

## Implementation Notes
1. Start with `cargo check --tests 2>&1 | head -100` to see first batch of errors
2. Fix errors by pattern, starting with EmbeddingService (highest impact)
3. After each pattern fix, re-run `cargo check --tests` to measure progress
4. Use search/replace for mechanical transformations where safe
5. Review each test to ensure intent is preserved
6. Target files in order of error count:
   - `tests/embedding_service_test.rs` (~37 errors)
   - `tests/embedding_integration.rs` (~17 errors)
   - `tests/incremental_*.rs` (~35 errors)
   - `tests/search_*.rs` (~19 errors)
   - `tests/fusion_*.rs` (~8 errors)

**Important**: Each fix must preserve the original test intent. Do not simply delete assertions or test logic to make compilation succeed. If a test requires architectural changes beyond mechanical API migration, document it separately for follow-up work.

## Dependencies
- TESTFIX-1002 (baseline must be documented first)

## Risk Assessment
- **Risk**: Some tests may require more than mechanical fixes
  - **Mitigation**: Document any tests that need architectural review; focus on compilation first

- **Risk**: Tests may compile but have incorrect assertions
  - **Mitigation**: Verify test intent is preserved; don't just silence errors

- **Risk**: Private API access may indicate tests need restructuring
  - **Mitigation**: Use `#[cfg(test)]` or pub(crate) if appropriate; document concerns

## Files/Packages Affected
- `crates/maproom/tests/embedding_service_test.rs`
- `crates/maproom/tests/embedding_integration.rs`
- `crates/maproom/tests/incremental_*.rs`
- `crates/maproom/tests/cleanup_*.rs`
- `crates/maproom/tests/search_*.rs`
- `crates/maproom/tests/fusion_*.rs`
- `crates/maproom/tests/context_*.rs`
- `crates/maproom/tests/weighted_fusion_test.rs`
- `crates/maproom/tests/context_assembler_test.rs`
- `crates/maproom/tests/metrics_integration_test.rs`
- Other test files in `crates/maproom/tests/` as needed
