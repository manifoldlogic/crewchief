# Ticket: SRCHREL-0003 - Test Detection Validation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- test-engineer
- verify-ticket
- commit-ticket

## Summary

Measure the accuracy of file path-based test detection heuristics on real codebase data. Validate that precision and recall meet thresholds (≥85% precision, ≥80% recall) before implementation.

## Background

Quality-weighted graph scoring depends on distinguishing production code from test code. The architecture uses file path patterns as the primary signal:
- `/test/`, `/tests/`, `/__tests__/` directories
- `.test.ts`, `.test.js`, `.spec.ts`, `.spec.js` file extensions
- `_test.rs`, `_test.py` file extensions

Before implementing, we must validate that these heuristics accurately identify test code on the actual CrewChief codebase.

## Acceptance Criteria

- [ ] Sample 200 chunks from CrewChief repository database
  - [ ] 100 known test chunks (from `/test/` directories or `.test.` files)
  - [ ] 100 known production chunks (from `/src/` or `/lib/` directories)
- [ ] Apply file path test detection heuristic to all samples
- [ ] Calculate precision: True Positives / (True Positives + False Positives)
- [ ] Calculate recall: True Positives / (True Positives + False Negatives)
- [ ] Document precision ≥85% (few false positives)
- [ ] Document recall ≥80% (few false negatives)
- [ ] Identify false positive patterns (production code misidentified as test)
- [ ] Identify false negative patterns (test code missed)
- [ ] Document findings in architecture.md
- [ ] If thresholds not met, propose pattern refinements and re-test

## Technical Requirements

**Sampling Strategy:**

```sql
-- Sample 100 test chunks
SELECT c.id, c.symbol_name, c.kind, f.relpath
FROM chunks c
JOIN files f ON f.id = c.file_id
WHERE f.relpath LIKE '%/test/%'
   OR f.relpath LIKE '%/tests/%'
   OR f.relpath LIKE '%/__tests__/%'
   OR f.relpath LIKE '%.test.%'
   OR f.relpath LIKE '%.spec.%'
LIMIT 100;

-- Sample 100 production chunks
SELECT c.id, c.symbol_name, c.kind, f.relpath
FROM chunks c
JOIN files f ON f.id = c.file_id
WHERE f.relpath LIKE '%/src/%'
   OR f.relpath LIKE '%/lib/%'
   OR f.relpath LIKE '%/crates/%'
LIMIT 100;
```

**Test Detection Heuristic (Reference Implementation):**

```rust
pub fn is_test_chunk(relpath: &str, kind: &str) -> bool {
    let path_lower = relpath.to_lowercase();
    let is_test_path = path_lower.contains("/test/")
        || path_lower.contains("/tests/")
        || path_lower.contains("/__tests__/")
        || path_lower.ends_with(".test.ts")
        || path_lower.ends_with(".test.js")
        || path_lower.ends_with(".spec.ts")
        || path_lower.ends_with(".spec.js")
        || path_lower.ends_with("_test.rs")
        || path_lower.ends_with("_test.py");

    if is_test_path {
        return true;
    }

    // Secondary: Chunk kind patterns
    let kind_lower = kind.to_lowercase();
    kind_lower.contains("test")
        || kind_lower.contains("describe")
        || kind_lower.contains("it")
}
```

**Manual Validation Process:**

For each sampled chunk:
1. Read file path and chunk symbol name
2. Manually classify as test or production (ground truth)
3. Apply heuristic function
4. Compare heuristic result to ground truth
5. Record: True Positive, False Positive, True Negative, False Negative

**Metrics Calculation:**

```
Precision = TP / (TP + FP)
Recall = TP / (TP + FN)
F1 Score = 2 * (Precision * Recall) / (Precision + Recall)
```

**Expected Confusion Matrix:**

|                     | Predicted Test | Predicted Production |
|---------------------|----------------|---------------------|
| **Actual Test**     | 85+ (TP)       | <15 (FN)           |
| **Actual Production** | <15 (FP)     | 85+ (TN)           |

**Edge Case Testing:**

Specific patterns to validate:
- Files with "test" in name but not tests (e.g., `testUtils.ts` in `/src/`)
- Test utilities in production code (e.g., `/src/testing/helpers.ts`)
- Integration tests vs unit tests (both should be classified as test)
- Benchmark files (e.g., `bench_*.rs`) - should NOT be classified as test
- Example files (e.g., `/examples/`) - should be production, not test

## Implementation Notes

**Tool for Validation:**

Create a simple validation script:
```rust
// In crates/maproom/tests/test_detection_validation.rs
#[test]
fn validate_test_detection_accuracy() {
    let samples = load_sample_chunks(); // From database
    let mut tp = 0;
    let mut fp = 0;
    let mut tn = 0;
    let mut fn = 0;

    for sample in samples {
        let ground_truth = sample.is_test; // Manual classification
        let predicted = is_test_chunk(&sample.relpath, &sample.kind);

        match (ground_truth, predicted) {
            (true, true) => tp += 1,
            (true, false) => fn += 1,
            (false, true) => fp += 1,
            (false, false) => tn += 1,
        }
    }

    let precision = tp as f64 / (tp + fp) as f64;
    let recall = tp as f64 / (tp + fn) as f64;

    println!("Precision: {:.2}%", precision * 100.0);
    println!("Recall: {:.2}%", recall * 100.0);

    assert!(precision >= 0.85, "Precision below 85%: {:.2}%", precision * 100.0);
    assert!(recall >= 0.80, "Recall below 80%: {:.2}%", recall * 100.0);
}
```

**Manual Classification Guidelines:**

A chunk is a "test" if:
- File is in a test directory
- File name includes test/spec patterns
- Chunk contains test assertions (describe, it, test, assert)

A chunk is "production" if:
- File provides application/library functionality
- Chunk is callable by other code (not just test harness)

**Pattern Tuning (if needed):**

If precision <85% (false positives):
- Remove overly broad patterns (e.g., kind contains "test")
- Add negative patterns (exclude /examples/, /benches/)

If recall <80% (false negatives):
- Add missing patterns (e.g., *.test.tsx, test_*.py)
- Check for alternative test directory names

## Dependencies

**Prerequisites:**
- SRCHREL-0001 (schema validation confirms chunk and file data available)

**Blocks:**
- SRCHREL-1001 (cannot implement quality scoring without accurate test detection)

## Risk Assessment

**Risk:** Precision <85% due to false positives
**Examples:** `testUtils.ts` in `/src/`, benchmark files, example files
**Mitigation:** Add explicit exclusion patterns, use negative lookahead in heuristic

**Risk:** Recall <80% due to false negatives
**Examples:** Alternative test patterns (e.g., `test_*.py`), non-standard directories
**Mitigation:** Expand pattern list based on findings, add language-specific patterns

**Risk:** Chunk `kind` unreliable for test detection
**Expected:** Chunk kind is tree-sitter node type (e.g., "function_declaration"), not semantic label
**Mitigation:** Use file path as primary signal, kind as weak secondary signal only

## Files/Packages Affected

**New Files:**
- `crates/maproom/tests/test_detection_validation.rs` (validation script)
- `.crewchief/projects/SRCHREL_relationship-aware-search/planning/test-detection-results.md` (validation results)

**Modified Files:**
- `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (document accuracy metrics and known limitations)

**Data Files:**
- Sample chunks CSV for manual classification (temporary, not committed)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Prerequisite 3, lines 87-112)
- Architecture: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (Test detection design, lines 211-245)
- Quality Strategy: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/quality-strategy.md` (Ranking quality tests, lines 143-188)
