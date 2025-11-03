# Implementation Plan: Provider Configuration Fixes

## Overview

Fix critical bugs in Maproom embedding provider configuration discovered during CLI implementation. Remove workarounds, fix Rust endpoint resolution, and address database schema issues.

## Phases and Deliverables

### Phase 1: Rust Core Fixes (Critical Path)

**Objective**: Fix endpoint resolution bug in Rust codebase

**Deliverables**:
1. **Fix `EmbeddingConfig::from_env()`**
   - Add provider-aware endpoint loading
   - Validate endpoint matches provider domain
   - Prevent cross-provider endpoint pollution

2. **Update `api_endpoint_url()` method**
   - Use validated endpoint from config
   - Remove buggy conditional logic
   - Add clear provider-specific defaults

3. **Add unit tests**
   - Test endpoint resolution for each provider
   - Test cross-provider endpoint rejection (the bug)
   - Test custom endpoint overrides
   - Test default endpoint fallbacks

**Files Changed**:
- `/workspace/crates/maproom/src/embedding/config.rs` (main fix)
- `/workspace/crates/maproom/src/embedding/config.rs` (tests)

**Agent**: General-purpose or Rust-focused agent

**Success Criteria**:
- ✅ All unit tests pass
- ✅ `cargo build --release` succeeds
- ✅ No warnings introduced

**Estimated Effort**: 2-3 hours

---

### Phase 2: Database Schema Fix

**Objective**: Add missing `updated_at` column to chunks table

**Deliverables**:
1. **Create migration file**
   - Number sequentially in `migrations/` folder
   - Add `updated_at TIMESTAMPTZ` column
   - Create trigger for auto-update
   - Add IF NOT EXISTS for safety

2. **Test migration**
   - Fresh database: column created
   - Existing database: column added without data loss
   - Verify embedding updates succeed

**Files Changed**:
- `/workspace/crates/maproom/migrations/00XX_add_updated_at_to_chunks.sql` (new file)

**Agent**: General-purpose agent

**Success Criteria**:
- ✅ Migration runs on fresh database
- ✅ Migration runs on existing database
- ✅ No "column does not exist" errors
- ✅ Embedding updates persist to database

**Estimated Effort**: 1 hour

---

### Phase 3: Remove CLI Workarounds

**Objective**: Clean up CLI code now that Rust fixes are in place

**Deliverables**:
1. **Remove explicit endpoint setting**
   - Function: `runScan()` (line ~1495)
   - Function: `runSetup()` (line ~1716)
   - Function: `upsertFiles()` (line ~1647)
   - Remove: `providerEnv.EMBEDDING_API_ENDPOINT = 'https://...'`

2. **Simplify environment handling**
   - Remove now-unnecessary deletion logic
   - Remove workaround comments
   - Clean up debug logging (optional)

3. **Test without workarounds**
   - Setup with OpenAI
   - Scan and verify embeddings generate
   - Verify no connection errors to localhost:11434

**Files Changed**:
- `/workspace/packages/maproom-mcp/bin/cli.cjs` (3 functions)

**Agent**: General-purpose agent

**Success Criteria**:
- ✅ OpenAI embeddings work without explicit endpoint
- ✅ No duplicate endpoint logic
- ✅ Code is cleaner and simpler

**Estimated Effort**: 30 minutes

---

### Phase 4: Docker Compose Cleanup

**Objective**: Remove default endpoint that caused bug

**Deliverables**:
1. **Update docker-compose.yml**
   - Remove or empty default for `EMBEDDING_API_ENDPOINT`
   - Document that Rust handles defaults
   - Keep other provider variables as-is

2. **Test with clean Docker environment**
   - Down and up containers
   - Verify OpenAI still works
   - Verify Ollama still works

**Files Changed**:
- `/workspace/packages/maproom-mcp/config/docker-compose.yml`

**Agent**: General-purpose agent

**Success Criteria**:
- ✅ No default endpoint set
- ✅ All providers still work
- ✅ Environment is cleaner

**Estimated Effort**: 15 minutes

---

### Phase 5: Integration Testing

**Objective**: Verify complete fix across all scenarios

**Deliverables**:
1. **Test OpenAI provider**
   - Setup from scratch
   - Scan repository
   - Verify embeddings generated
   - Check cost/token metrics

2. **Test Ollama provider**
   - Setup with default endpoint
   - Scan repository
   - Verify still works

3. **Test environment precedence**
   - Wrong endpoint ignored
   - Custom endpoint accepted
   - Defaults work

4. **Test database updates**
   - Verify `updated_at` column exists
   - Verify timestamps update correctly
   - No errors during embedding updates

**Files Changed**: None (testing only)

**Agent**: General-purpose agent or manual testing

**Success Criteria**:
- ✅ All test scenarios pass
- ✅ No regressions in existing functionality
- ✅ Clear error messages for misconfigurations

**Estimated Effort**: 1 hour

---

### Phase 6: Documentation Updates

**Objective**: Document the fixes and new behavior

**Deliverables**:
1. **Update README.md**
   - Remove workaround mentions
   - Document environment variable precedence
   - Add troubleshooting for common issues

2. **Add comments to code**
   - Document why endpoint validation exists
   - Explain provider-specific logic
   - Reference this project for context

3. **Update changelog** (if exists)
   - Document bug fix
   - Note breaking change (if any)
   - Mention improved validation

**Files Changed**:
- `/workspace/packages/maproom-mcp/README.md`
- `/workspace/crates/maproom/src/embedding/config.rs` (comments)

**Agent**: General-purpose agent

**Success Criteria**:
- ✅ Documentation accurate
- ✅ Users understand environment variables
- ✅ Troubleshooting section helpful

**Estimated Effort**: 30 minutes

---

## Total Estimated Effort

**Total**: ~5-6 hours of focused work

**Critical Path**: Phase 1 (Rust fixes) → Phase 3 (CLI cleanup) → Phase 5 (testing)

**Can Parallelize**: Phase 2 (database) can be done independently

## Dependencies

**Phase 1** must complete before:
- Phase 3 (removing workarounds depends on Rust fix)
- Phase 5 (integration testing needs fixed code)

**Phase 2** can be done anytime:
- Independent of Phase 1
- Can be tested separately

**Phase 4** should follow Phase 3:
- Verify CLI works before changing Docker defaults

## Risk Mitigation

**If Phase 1 takes longer than expected**:
- Keep workarounds in place temporarily
- Phase 2 can proceed independently
- Delay Phase 3 until Phase 1 complete

**If tests fail in Phase 5**:
- Roll back specific phase
- Workarounds provide safety net
- Debug with unit tests

## Success Metrics

**Before Fixes**:
- ❌ OpenAI: Connection refused errors
- ❌ Database: Column does not exist errors
- ⚠️ CLI: Workaround in 3 places

**After Fixes**:
- ✅ OpenAI: Embeddings generate successfully
- ✅ Database: Updates persist without errors
- ✅ CLI: Clean code, no workarounds

## Agents Involved

**Primary Agent**: General-purpose agent
- Can handle all phases
- Rust, JavaScript, SQL all within scope

**Alternative Agents**:
- **Rust specialist**: If Phase 1 is complex
- **Database specialist**: If Phase 2 has migration issues
- **Testing specialist**: For Phase 5 if integration issues arise

**Recommended Approach**: Single general-purpose agent for consistency

## Deployment Strategy

**For Development**:
1. Create feature branch
2. Implement all phases
3. Test thoroughly
4. Merge to main

**For Production**:
1. Deploy Rust changes first (Phase 1)
2. Deploy CLI changes (Phase 3)
3. Update Docker Compose (Phase 4)
4. Run database migration (Phase 2)

**Rollback Plan**:
- Keep workaround code in git history
- Document how to revert if needed
- Test rollback procedure before deployment

## Future Enhancements

**Not in this project**:
- Google Vertex AI testing (nice to have)
- Endpoint allowlist (enterprise feature)
- Audit logging improvements (not critical)
- Configuration validation tool (overkill)

**Why not now**: Focus on fixing critical bugs, avoid scope creep
