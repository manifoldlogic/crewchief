# Quality Strategy: Make mxbai-embed-large the Default Model

## Testing Philosophy

This is a configuration change, not a feature addition. Focus testing on:
1. **Defaults work correctly**: Zero-config uses mxbai-embed-large
2. **Backward compatibility**: Explicit configuration still works
3. **No regressions**: Existing functionality unaffected

**Not testing**: Embedding quality or model behavior (already validated in DIM1024 project).

## Critical Paths

### MUST Test (High Priority)

1. **Default model selection**:
   - Without env vars, provider uses mxbai-embed-large
   - Without env vars, dimension is 1024
   - Factory fallback uses mxbai-embed-large

2. **Backward compatibility**:
   - Explicit `MAPROOM_EMBEDDING_MODEL=nomic-embed-text` still works
   - Explicit `MAPROOM_EMBEDDING_DIMENSION=768` still works
   - Sanitization still applies to nomic-embed-text

3. **Test assertions updated**:
   - `test_ollama_provider_default_config()` passes
   - All DEFAULT_MODEL assertions updated
   - All default dimension assertions updated

### SHOULD Test (Medium Priority)

4. **Integration scenarios**:
   - VSCode extension zero-config generates 1024-dim embeddings
   - CLI without env vars uses mxbai-embed-large
   - Mixed dimension search (768 and 1024) works

5. **Documentation accuracy**:
   - Code examples in docs are correct
   - Migration guide commands work
   - No conflicting default references

### NICE TO Test (Low Priority)

6. **Edge cases**:
   - Model download when mxbai not present
   - Error messages when model unavailable
   - Performance with larger model

## Testing Approach

### Unit Tests

**Scope**: Rust constant values and default config methods

**Tools**: `cargo test`

**Rust tests to update**:
```rust
// crates/maproom/src/embedding/ollama.rs
#[test]
fn test_ollama_provider_default_config() {
    let provider = OllamaProvider::default_config().unwrap();
    assert_eq!(provider.model, "mxbai-embed-large");  // ← UPDATE
    assert_eq!(provider.dimension(), 1024);           // ← UPDATE
}

#[test]
fn test_default_model_constant() {
    assert_eq!(OllamaProvider::DEFAULT_MODEL, "mxbai-embed-large");  // ← UPDATE
}

// Update 15+ DEFAULT_MODEL assertions
// Update 37+ dimension assertions
// Update 50+ test fixtures with model references
```

**TypeScript tests to update**:
```typescript
// packages/vscode-maproom/src/ollama/model-manager.test.ts
test('DEFAULT_EMBEDDING_MODEL constant', () => {
  expect(DEFAULT_EMBEDDING_MODEL).toBe('mxbai-embed-large')  // ← UPDATE
})

// Update 8+ assertions in model-manager.test.ts
```

```typescript
// packages/maproom-mcp/tests/provider-detection.test.ts
// Update 10+ test cases with mxbai-embed-large in mocks
// Update warning message expectations
```

**New tests to add**:
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

**Test Audit Checklist**:
- [ ] Grep for DEFAULT_MODEL assertions: 15+ identified
- [ ] Grep for dimension assertions: 37+ identified
- [ ] Grep for test fixtures with model: 50+ identified
- [ ] Grep for TypeScript test files: 10+ identified
- [ ] Total test updates required: 90+ (based on audit)

**Success Criteria**:
- All Rust tests pass (`cargo test -p crewchief-maproom` exit code 0)
- All TypeScript tests pass (`pnpm test` in vscode-maproom and maproom-mcp, exit code 0)
- No new test failures introduced
- Backward compat test verifies explicit nomic-embed-text in all layers

### Integration Tests

**Scope**: End-to-end default behavior across all layers (Rust daemon, MCP server, VSCode extension)

**Method**: Manual CLI and extension tests

**Test 1: Zero-config Rust daemon uses mxbai**
```bash
unset MAPROOM_EMBEDDING_MODEL
unset MAPROOM_EMBEDDING_DIMENSION
cargo run --bin crewchief-maproom -- status --repo test

# Expected log output:
# "Using provider: ollama (model: mxbai-embed-large, dimension: 1024)"
```

**Test 2: Explicit nomic still works**
```bash
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
cargo run --bin crewchief-maproom -- status --repo test

# Expected log output:
# "Using provider: ollama (model: nomic-embed-text, dimension: 768)"
```

**Test 3: Sanitization conditional**
```bash
# Test with mxbai (no sanitization)
export MAPROOM_EMBEDDING_MODEL=mxbai-embed-large
echo "Test with | and [] characters" | cargo run --bin crewchief-maproom -- ...
# Should NOT sanitize

# Test with nomic (sanitization applied)
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
echo "Test with | and [] characters" | cargo run --bin crewchief-maproom -- ...
# SHOULD sanitize
```

**Test 4: MCP provider detection recognizes mxbai**
```bash
# Test MCP server detects Ollama with mxbai-embed-large
node packages/maproom-mcp/bin/cli.cjs
# Manually trigger provider detection, verify it recognizes mxbai model
```

**Test 5: VSCode extension downloads correct model**
```bash
# Test extension model manager
# Fresh install should check for mxbai-embed-large
# Verify ensureOllamaModel() uses correct DEFAULT_EMBEDDING_MODEL
```

**Success Criteria**:
- Zero-config Rust daemon logs show mxbai-embed-large and 1024
- MCP server provider detection validates mxbai-embed-large availability
- VSCode extension model manager checks for mxbai-embed-large
- Explicit nomic-embed-text config works in all layers
- Sanitization behavior correct for each model

### Documentation Tests

**Scope**: Verify documentation examples actually work

**Method**: Copy-paste commands from docs, verify they execute correctly

**Tests**:
1. Copy example from ollama-setup.md, run it → should work
2. Copy example from migration guide, run it → should work
3. Search for "nomic-embed-text" in docs → should only appear in:
   - Migration guide (explaining how to use old model)
   - Backward compatibility sections
   - Historical context sections

**Success Criteria**:
- All example commands execute successfully
- No conflicting default references found
- Migration guide examples work

### Manual VSCode Test

**Scope**: End-to-end zero-config experience with consistent defaults

**Method**: Fresh extension install, no configuration

**Steps**:
1. Install vscode-maproom extension (or use development host)
2. Open workspace with no .vscode/mcp.json or env vars
3. Extension activates, verify model manager behavior:
   - Check extension logs for "Checking for mxbai-embed-large model"
   - If model missing, should prompt to download mxbai-embed-large (not nomic-embed-text)
4. Trigger "Maproom: Setup" command
5. Index a small repository
6. Run search query
7. Check database: `sqlite3 ~/.maproom/maproom.db "SELECT COUNT(*) FROM vec_code_1024"`
8. Check MCP server logs for provider detection validation

**Expected Results**:
- Setup completes without errors
- Extension model manager checks for mxbai-embed-large
- MCP server detects Ollama with mxbai-embed-large
- Daemon uses mxbai-embed-large (1024-dim)
- Indexing generates 1024-dim embeddings
- Embeddings stored in vec_code_1024 table
- Search returns results

**Success Criteria**:
- Extension works without configuration
- All layers (VSCode, MCP, daemon) use consistent default (mxbai-embed-large)
- Embeddings in correct table (vec_code_1024)
- Search functionality works

## Quality Gates

### Gate 1: Unit Tests Pass
**Requirement**: `cargo test -p crewchief-maproom` exits with code 0

**Blocker**: If tests fail, code changes are incomplete or incorrect

**Resolution**: Fix test assertions or code until all pass

### Gate 2: Manual CLI Validation
**Requirement**: Both zero-config and explicit-config scenarios work

**Blocker**: If either scenario fails, configuration logic is broken

**Resolution**: Debug factory.rs or ollama.rs until both scenarios pass

### Gate 3: Documentation Consistency
**Requirement**: No conflicting default references in documentation

**Blocker**: If conflicts found, users will be confused

**Resolution**: Update all docs to be consistent

### Gate 4: Backward Compatibility Verified
**Requirement**: Explicit nomic-embed-text config still works in all layers

**Blocker**: If broken, this is a breaking change (not acceptable)

**Resolution**: Ensure conditional logic preserves nomic-embed-text path in Rust, VSCode, and MCP

### Gate 5: Location Completeness
**Requirement**: All 6 code locations verified updated via grep audit

**Blocker**: If locations missed, inconsistent defaults across layers

**Resolution**: Run verification scan after changes, fail if unexpected references found

## Test Coverage

### Code Coverage

**Target**: 100% of changed lines

**Changed files (6 total)**:
- `ollama.rs`: 3 lines (2 constants, 1 dimension)
- `factory.rs`: 1 line (fallback string)
- `model-manager.ts`: 1 line (DEFAULT_EMBEDDING_MODEL constant)
- `provider-detection.ts`: 1 line (model validation check)
- `.env.example`: 2 lines (model and dimension examples)

**Test coverage**:
- Rust constants: Tested via `test_ollama_provider_default_config()`
- Rust factory fallback: Tested via integration test (zero-config scenario)
- TypeScript constants: Tested via model-manager.test.ts
- MCP validation: Tested via provider-detection.test.ts
- Configuration examples: Verified via manual review (not executable)

**Not tested** (and why):
- Database schema: No changes made (already exists)
- Documentation: Verified via grep audit (not unit tested)

### Edge Cases

**Tested**:
- Zero-config in Rust daemon (default path)
- Zero-config in VSCode extension (model manager path)
- Zero-config in MCP server (provider detection path)
- Explicit config (backward compat path in all layers)
- Mixed dimensions (already tested in DIM1024)

**Not tested** (out of scope):
- Model download failure: Handled by Ollama, not our code
- Network issues: Not related to default change
- Corrupted database: Not related to default change

### Backward Compatibility Test Specification

**Rust test** (crates/maproom/src/embedding/factory.rs):
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

**TypeScript test** (packages/vscode-maproom/src/ollama/model-manager.test.ts):
```typescript
test('respects configured model override', async () => {
  // Test that extension respects explicit model config
  // (implementation depends on how config is passed to model manager)
})
```

**MCP test** (packages/maproom-mcp/tests/provider-detection.test.ts):
```typescript
test('detects Ollama with nomic-embed-text model', async () => {
  // Mock Ollama with nomic-embed-text
  // Verify detection still works (backward compat)
})
```

### Rollback Testing

**Purpose**: Validate that code changes can be cleanly reverted if issues arise.

**Test Plan**:
1. **Pre-change baseline**: Run all tests, record pass count
2. **Apply changes**: Implement all code and test updates
3. **Verify changes**: Run all tests, verify pass count matches baseline
4. **Revert changes**: Revert all code changes (keep test updates)
5. **Verify revert**: Run all tests, verify pass count matches baseline
6. **Document**: Confirm rollback path is clean

**Success Criteria**:
- Tests pass at all three stages (baseline, changed, reverted)
- No test count discrepancies
- Rollback procedure documented and validated

## Risk Assessment

| Risk Area | Severity | Testing Approach |
|-----------|----------|------------------|
| Unit tests fail | High | Run full test suite, fix assertions |
| Backward compat broken | Critical | Explicit config test, manual verification |
| VSCode zero-config broken | High | Manual extension test |
| Documentation inconsistent | Medium | Grep for conflicts, review docs |
| Performance regression | Low | Not tested (no performance changes) |

## Acceptance Criteria

**Phase 1 (Code & Tests):**
- [ ] All unit tests pass
- [ ] Zero-config CLI test shows mxbai-embed-large
- [ ] Explicit nomic-embed-text test works
- [ ] No test failures in CI

**Phase 2 (Documentation):**
- [ ] All docs show mxbai-embed-large as default
- [ ] Migration guide complete
- [ ] No conflicting default references
- [ ] Example commands work

**End-to-End:**
- [ ] VSCode extension works zero-config
- [ ] Embeddings in vec_code_1024 table
- [ ] Search works across mixed dimensions

## Test Automation

### CI Integration

**Existing**: `cargo test` already runs in CI

**No changes needed**: Updated tests will automatically run in CI

**CI validation**:
- All tests pass on main branch
- No new test failures introduced
- Backward compat test passes

### Manual Testing Checklist

```markdown
## Pre-Deployment Testing

- [ ] `cargo test -p crewchief-maproom` passes locally
- [ ] Zero-config CLI test passes
- [ ] Explicit nomic-embed-text test passes
- [ ] Documentation examples work
- [ ] VSCode extension fresh install works
- [ ] Embeddings in vec_code_1024 table
- [ ] Search across mixed dimensions works

## Post-Deployment Monitoring

- [ ] CI tests pass on main branch
- [ ] No user-reported breaking changes
- [ ] Extension telemetry shows successful indexing
- [ ] No spike in error reports
```

## Quality Metrics

**Definition of Success**:
1. Zero test failures
2. Zero breaking changes reported
3. Zero documentation inconsistencies
4. Positive user feedback (or no negative feedback)

**Measurement**:
- Test pass rate: 100%
- CI success rate: 100%
- User issue reports: 0 breaking change reports in first week
- Documentation review: 0 conflicts found

## Tooling

**Test Execution**: `cargo test -p crewchief-maproom`
**Documentation Search**: `grep -r "nomic-embed-text" docs/ *.md`
**Manual CLI Testing**: Shell scripts with env var setup
**Database Inspection**: `sqlite3 ~/.maproom/maproom.db`
**Extension Testing**: VSCode Extension Development Host

**No new tooling needed**: All existing tools sufficient for validation.
