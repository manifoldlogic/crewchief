# Project: Maproom Binary Configuration

**Slug:** MRBIN_maproom-binary-configuration
**Priority:** High
**Effort:** S (1 day)

## Summary

Add `repository.maproomBinaryPath` config option to explicitly specify the maproom binary location, and update binary resolution order to prioritize config over local build detection.

## Deliverables

1. **Config schema field** - Add `maproomBinaryPath` to `RepositorySchema` (optional string)
2. **Binary resolution update** - Check config after env var, before local paths
3. **Resolution order change** - Global install before packaged binary
4. **Shared utility extraction** - Extract `findMaproomBinary()` function for reuse
5. **Updated tests** - Verify config-based resolution and precedence order
6. **Documentation** - Update README with development workflow using custom binary

## Dependencies

None - Can be developed in parallel with WTPATH.

## Value Proposition

Developers can explicitly configure the maproom binary path for local development without relying on environment variables. The updated resolution order makes production usage (global install) the default, reducing confusion when local builds exist from development work.

## Technical Approach

1. Add `maproomBinaryPath: z.string().optional()` to `RepositorySchema`
2. Update binary resolution in `worktrees.ts` and `maproom.ts`:
   - Check `CREWCHIEF_MAPROOM_BIN` env var
   - Check `config.repository.maproomBinaryPath`
   - Check global install (`command -v crewchief-maproom`)
   - Check packaged binary as fallback
3. Extract resolution logic to shared utility function
4. Document development workflow with custom binary path

## Acceptance Criteria

- [ ] Config accepts `maproomBinaryPath` setting
- [ ] Config path takes precedence over packaged binary
- [ ] Env var still takes highest precedence
- [ ] Global install checked before local packaged binary
- [ ] Binary resolution is consistent across all commands
- [ ] Development workflow documented

## Breaking Changes

**Non-breaking:** This is an additive change. Existing binary resolution continues to work.

**Behavior change:** Global installation now preferred over packaged binary (more predictable for production use).
