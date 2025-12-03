# Ticket: [MXBAI-1004]: Update Rust Test Assertions

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - Ollama tests: 32/32 passed, Factory tests: 21/21 passed
- [x] **Verified** - by the verify-ticket agent

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
Update all Rust test assertions to reflect new default model (mxbai-embed-large) and dimension (1024), and add backward compatibility test to verify explicit nomic-embed-text configuration still works.

## Background
This ticket addresses Phase 1, Deliverable 4 from plan.md. The grep audit identified 15+ DEFAULT_MODEL assertions, 37+ dimension assertions, and 50+ test fixtures that need updating. After code changes in MXBAI-1001, all existing tests will fail because they assert the old defaults.

Reference: plan.md Phase 1, Deliverable 4 "Update Test Assertions (Rust tests)"

## Acceptance Criteria
- [ ] test_ollama_provider_default_config() updated to assert "mxbai-embed-large" and 1024
- [ ] All DEFAULT_MODEL assertions updated (15+ occurrences)
- [ ] All dimension assertions updated from 768 to 1024 (37+ occurrences)
- [ ] Test fixtures with model references updated (50+ occurrences)
- [ ] New backward compatibility test added for nomic-embed-text
- [ ] All Rust tests pass: `cargo test -p crewchief-maproom` exits with code 0

## Technical Requirements
**Primary test to update**:
- `crates/maproom/src/embedding/ollama.rs` - test_ollama_provider_default_config():
  - Update assertion: `assert_eq!(provider.model, "mxbai-embed-large");`
  - Update assertion: `assert_eq!(provider.dimension(), 1024);`

**Search and replace**:
Use grep to find all test assertions that need updating:
```bash
# Find DEFAULT_MODEL assertions
grep -rn 'assert.*DEFAULT_MODEL.*"nomic-embed-text"' crates/maproom/

# Find dimension assertions
grep -rn 'assert.*768' crates/maproom/

# Find test fixtures
grep -rn 'nomic-embed-text' crates/maproom/tests/
```

**New backward compatibility test** (add to factory.rs tests):
```rust
#[tokio::test]
async fn test_backward_compat_nomic_embed_text() {
    env::set_var("MAPROOM_EMBEDDING_MODEL", "nomic-embed-text");
    env::set_var("MAPROOM_EMBEDDING_DIMENSION", "768");

    let provider = create_provider_from_env().await.unwrap();
    assert_eq!(provider.provider_name(), "ollama");
    assert_eq!(provider.dimension(), 768);

    env::remove_var("MAPROOM_EMBEDDING_MODEL");
    env::remove_var("MAPROOM_EMBEDDING_DIMENSION");
}
```

## Implementation Notes
**Strategy**:
1. Run `cargo test -p crewchief-maproom` to see which tests fail
2. For each failing test, update assertions to match new defaults
3. Be careful with dimension assertions - some might be testing OpenAI (1536) or other providers
4. Focus on Ollama-related tests only

**What to update**:
- Assertions checking DEFAULT_MODEL == "nomic-embed-text" → "mxbai-embed-large"
- Assertions checking dimension == 768 (in Ollama context) → 1024
- Mock data/fixtures using nomic-embed-text → mxbai-embed-large

**What NOT to update**:
- Sanitization tests (these should still reference nomic-embed-text specifically)
- Tests for explicit configuration (these test env var overrides)
- OpenAI provider tests (dimension 1536 is correct for OpenAI)

**Pattern from quality-strategy.md**:
- Focus on Ollama default path tests
- Preserve tests for explicit configuration
- Add new test for backward compatibility

## Dependencies
- **Critical dependency**: MXBAI-1001 (Rust constants must be updated first, otherwise tests will fail for wrong reason)
- **External dependency**: None

## Risk Assessment
- **Risk**: Updating wrong assertions (e.g., OpenAI dimension tests)
  - **Mitigation**: Review context of each assertion, focus on Ollama provider tests

- **Risk**: Missing some test locations
  - **Mitigation**: Use grep to find all occurrences, verification scan in MXBAI-1006

- **Risk**: Breaking backward compatibility
  - **Mitigation**: New test verifies explicit nomic-embed-text still works

## Files/Packages Affected
- `/workspace/crates/maproom/src/embedding/ollama.rs` (test section)
- `/workspace/crates/maproom/src/embedding/factory.rs` (test section)
- `/workspace/crates/maproom/tests/**/*.rs` (integration tests if applicable)
- Other test files identified by grep search

## Verification Notes
verify-ticket agent should check:
- [ ] All tests pass: `cargo test -p crewchief-maproom` exits 0
- [ ] Backward compatibility test exists and passes
- [ ] Test output shows no failures or warnings
- [ ] grep for "nomic-embed-text" in tests shows only expected references (sanitization tests, backward compat)
