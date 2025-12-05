# Ticket: [DIM1024-2002]: Conditional Sanitization for Model Compatibility

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-developer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement conditional sanitization based on model name, applying character sanitization only for nomic-embed-text while preserving raw text for mxbai-embed-large and other models.

## Background
This ticket implements Phase 3 of the DIM1024 project. Currently, character sanitization (lines 344-386 in ollama.rs) is applied to all text regardless of model, working around tokenization bugs in nomic-embed-text. This sanitization replaces characters like |, [], (), and Unicode symbols.

The mxbai-embed-large model does not have these tokenization issues and should receive raw text for higher quality embeddings. This ticket adds conditional logic to apply sanitization only when model == "nomic-embed-text", while letting mxbai-embed-large and future models use raw text.

Dependencies: This ticket requires DIM1024-2001 (Provider Configuration) to be completed first, as we need the dimension-aware provider infrastructure before adding model-specific behavior.

References: `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/plan.md` (Phase 3), `/workspace/.crewchief/projects/DIM1024_embedding-dimension-1024/planning/architecture.md` (Decision 3, Component 5).

## Acceptance Criteria
- [ ] Sanitization logic extracted into helper function (e.g., sanitize_for_nomic)
- [ ] Model name check before applying sanitization
- [ ] Sanitization applies when model == "nomic-embed-text"
- [ ] Sanitization skipped for mxbai-embed-large and other models
- [ ] Unit test: Conditional sanitization logic (model-based branching)
- [ ] Integration test: Problematic characters (|, [], (), Unicode) handled correctly
- [ ] Integration test: nomic-embed-text still uses sanitization (backward compat)
- [ ] Integration test: mxbai-embed-large receives raw text
- [ ] All existing unit tests still pass

## Technical Requirements
- Extract sanitization code (lines 344-386 in ollama.rs) into named function
- Function signature: `fn sanitize_for_nomic(text: &str) -> String`
- Add conditional check in embed_batch_raw() before processing texts
- Condition: `if self.model == "nomic-embed-text"`
- Apply sanitization to texts when condition is true
- Use raw texts when condition is false (mxbai-embed-large and others)
- Preserve all existing sanitization logic for nomic-embed-text backward compatibility

## Implementation Notes
**Current Code Pattern** (lines 344-386 in ollama.rs):
The sanitization code replaces specific characters that cause nomic-embed-text tokenization crashes. This workaround is necessary for nomic-embed-text but degrades embedding quality by mangling content.

**Refactoring Pattern**:
```rust
fn sanitize_for_nomic(text: &str) -> String {
    // Move existing sanitization logic here (lines 344-386)
    // Keep all existing character replacements
}

async fn embed_batch_raw(&self, texts: Vec<String>) -> Result<Vec<Vector>, EmbeddingError> {
    let processed_texts = if self.model == "nomic-embed-text" {
        // Apply sanitization workaround for nomic-embed-text
        texts.into_iter().map(|t| sanitize_for_nomic(&t)).collect()
    } else {
        // Use raw text for mxbai-embed-large and other models
        texts
    };
    // ... rest of embedding logic uses processed_texts
}
```

**Quality Improvement**: mxbai-embed-large receives unmodified text, preserving:
- Pipe characters (|) in tables
- Square brackets ([]) in links and checkboxes
- Parentheses () in function calls
- Unicode symbols (→, ←, ↔, ├, etc.)

**Backward Compatibility**: Users with nomic-embed-text continue to have sanitization applied automatically. No configuration changes required.

**Test Data**: Use problematic text containing |, [], (), Unicode symbols to verify sanitization is applied/skipped correctly.

## Dependencies
- **DIM1024-2001**: Provider Configuration (MUST be completed first)
  - Reason: Requires dimension-aware provider infrastructure and model field access
- **No external dependencies**: Only modifying existing ollama.rs code

## Risk Assessment
- **Risk**: Removing sanitization for nomic-embed-text breaks existing users
  - **Mitigation**: Keep conditional sanitization for nomic-embed-text, no removal
- **Risk**: Model name string comparison is fragile (typos, case sensitivity)
  - **Mitigation**: Use exact string match "nomic-embed-text", add test coverage
- **Risk**: mxbai-embed-large actually needs sanitization for some edge cases
  - **Mitigation**: Integration test with problematic characters, monitor production logs
- **Risk**: Future Ollama models need different sanitization logic
  - **Mitigation**: Document pattern for adding model-specific handling, keep extensible

## Files/Packages Affected
- `/workspace/crates/maproom/src/embedding/ollama.rs`

## Verification Notes
The verify-ticket agent should specifically check:

1. **Function extraction**: sanitize_for_nomic() function exists with sanitization logic
2. **Conditional logic**: Model check before applying sanitization
3. **Backward compatibility**: nomic-embed-text path still uses sanitization
4. **Raw text path**: mxbai-embed-large path skips sanitization
5. **Test execution**: Unit and integration tests EXECUTED with passing output
6. **Test coverage**: Tests include problematic characters (|, [], (), Unicode)
7. **Code preservation**: Existing sanitization logic unchanged, just moved to function
8. **No regressions**: All existing ollama.rs tests still pass
9. **Model field access**: Code correctly reads self.model for comparison
