# Ticket: ITERMCLN-5002: Update Documentation for Agent Commands

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation-only ticket)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-development
- verify-ticket
- commit-ticket

## Summary
Update CLI and Python scripts documentation to reflect the cleaned-up implementation, removed bridge code, and new headless messaging capability.

## Background
After removing dead code and adding new features throughout Phases 1-4, the documentation needs to be updated to accurately reflect the current state. This includes CLI README, Python scripts README, and any agent management documentation. All references to the removed JSON-RPC bridge architecture must be eliminated, and new features like headless messaging need to be properly documented.

Reference: ITERMCLN plan.md Phase 5 - Documentation

## Acceptance Criteria
- [ ] `scripts/iterm_scripts/README.md` updated (remove all bridge references, document current architecture)
- [ ] `scripts/iterm_scripts/AGENT_MANAGEMENT.md` updated (current patterns only, no bridge sections)
- [ ] `scripts/iterm_scripts/PANE_COMMUNICATION.md` updated (remove bridge protocol sections, document direct communication)
- [ ] `packages/cli/README.md` updated (agent spawn/message/list commands documented, headless mode included)
- [ ] No references to deleted files remain (verified via grep)

## Technical Requirements

Documentation updates needed:

1. **`scripts/iterm_scripts/README.md`**:
   - Remove all references to JSON-RPC bridge architecture
   - Remove references to deleted files (iterm_bridge.py, iterm_bridge_server.py, etc.)
   - Update architecture description to "direct script calls" pattern
   - Document only active scripts that still exist
   - Update command examples to match current CLI

2. **`scripts/iterm_scripts/AGENT_MANAGEMENT.md`**:
   - Remove bridge-based management sections
   - Document current spawn/message/list flow using direct script calls
   - Update examples to match current CLI commands
   - Include both iTerm and headless mode examples

3. **`scripts/iterm_scripts/PANE_COMMUNICATION.md`**:
   - Remove bridge protocol sections
   - Document direct script communication pattern (CLI → Python script → iTerm API)
   - Update examples to reflect current implementation
   - Clarify headless mode behavior (no-op for pane operations)

4. **`packages/cli/README.md`** (if exists):
   - Document agent spawn command
   - Document agent message command (iTerm + headless modes)
   - Document agent list command
   - Document multi-agent spawn capability (if Phase 4 complete)
   - Include usage examples for each command

## Implementation Notes

- Use `grep -r "bridge\|iterm_bridge\|JSON-RPC" scripts/` to find stale references across all documentation
- Use `grep -r "iterm_bridge\.py\|iterm_bridge_server\.py" .` to find specific deleted file references
- Keep documentation concise - focus on current behavior, not historical context
- Include practical examples for each command
- Document both iTerm and headless modes where applicable
- Ensure consistency between Python script docs and CLI docs
- Consider adding a simple architecture diagram showing the direct call pattern

## Dependencies
- **ITERMCLN-1001** (Python dead code removal) - Need to know what was deleted
- **ITERMCLN-3002** (Headless messaging) - Document new feature capability
- **ITERMCLN-4001** (Multi-agent spawn) - Document if complete

## Risk Assessment
- **Risk**: Missing stale references to deleted code/architecture
  - **Mitigation**: Use comprehensive grep searches to find all bridge/JSON-RPC references
  - **Mitigation**: Review each documentation file systematically

- **Risk**: Documenting features that were planned but not implemented
  - **Mitigation**: Verify each feature exists in code before documenting
  - **Mitigation**: Check completion status of dependent tickets

## Files/Packages Affected
- `scripts/iterm_scripts/README.md` - UPDATE
- `scripts/iterm_scripts/AGENT_MANAGEMENT.md` - UPDATE
- `scripts/iterm_scripts/PANE_COMMUNICATION.md` - UPDATE
- `packages/cli/README.md` - UPDATE (if exists)
