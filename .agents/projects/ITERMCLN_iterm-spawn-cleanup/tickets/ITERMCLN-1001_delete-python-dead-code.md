# Ticket: ITERMCLN-1001: Delete Dead Python Bridge Code

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (deletion only, no tests required)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-development
- verify-ticket
- commit-ticket

## Summary
Delete all Python files related to the abandoned JSON-RPC bridge approach. These files total approximately 1,650 lines of dead code that has never worked and should be removed to improve codebase clarity.

## Background
The codebase contains an abandoned JSON-RPC bridge implementation in `scripts/iterm_scripts/`. The `iterm_bridge.py` server was intended to provide HTTP JSON-RPC for terminal management but was never completed or put into production use. All related files (controller, manager, tests, demos) are dead code that should be removed.

The working approach uses direct script calls via `spawn_agent.py`, `send_to_pane.py`, and related scripts, which remain in active use.

Reference: ITERMCLN plan.md Phase 1 - Dead Code Removal

## Acceptance Criteria
- [ ] All bridge-related Python files deleted (~10 files totaling ~1,650 lines)
- [ ] No broken imports in remaining Python scripts (verified via grep)
- [ ] `requirements.txt` remains valid and contains necessary dependencies
- [ ] README.md in `scripts/iterm_scripts/` updated to remove all bridge references

## Technical Requirements
Delete these Python files from `scripts/iterm_scripts/`:
- `iterm_bridge.py` (309 lines) - JSON-RPC server
- `iterm_controller.py` (255 lines) - Bridge controller
- `iterm_agent_manager.py` (351 lines) - Bridge manager
- `test_bridge.py` (~250 lines) - Bridge tests
- `test_badge.py` (~60 lines) - Badge tests
- `test_enter.py` (~145 lines) - Enter key tests
- `test_agent_detection.py` (~50 lines) - Agent detection tests
- `demo_smart_spawning.py` (~140 lines) - Demo script
- `debug_send.py` (~90 lines) - Debug utility
- `start_bridge.sh` (~27 lines) - Bridge startup script

Keep active scripts:
- `spawn_agent.py`
- `send_to_pane.py`
- `list_panes.py`
- `agent_config.py`

Keep manual tools:
- `spawn_multi_agents.py`
- `kill_agent.py`
- `list_agents.py`

## Implementation Notes
1. Before deletion, verify each file is not imported by active scripts:
   ```bash
   grep -r "import.*bridge\|from.*bridge" scripts/iterm_scripts/
   ```

2. Delete all bridge-related files listed in Technical Requirements

3. Update `scripts/iterm_scripts/README.md` to remove documentation about:
   - JSON-RPC bridge architecture
   - Bridge startup instructions
   - Bridge testing procedures
   - Any other bridge-specific content

4. Verify `requirements.txt` is still valid. May only need `iterm2` dependency after cleanup.

5. Final verification: ensure no broken imports remain in active scripts

## Dependencies
- None (can be done first in Phase 1)

## Risk Assessment
- **Risk**: Accidentally delete active script used by current workflow
  - **Mitigation**: Verify each file is not imported by active scripts before deletion. Cross-reference with list of scripts to keep.

- **Risk**: Break remaining functionality by removing shared utility code
  - **Mitigation**: Grep for all imports of files to be deleted. Ensure only bridge files import bridge code.

## Files/Packages Affected
**Files to DELETE**:
- `scripts/iterm_scripts/iterm_bridge.py`
- `scripts/iterm_scripts/iterm_controller.py`
- `scripts/iterm_scripts/iterm_agent_manager.py`
- `scripts/iterm_scripts/test_bridge.py`
- `scripts/iterm_scripts/test_badge.py`
- `scripts/iterm_scripts/test_enter.py`
- `scripts/iterm_scripts/test_agent_detection.py`
- `scripts/iterm_scripts/demo_smart_spawning.py`
- `scripts/iterm_scripts/debug_send.py`
- `scripts/iterm_scripts/start_bridge.sh`

**Files to UPDATE**:
- `scripts/iterm_scripts/README.md` - Remove bridge documentation
- `scripts/iterm_scripts/requirements.txt` - Verify still valid (may only need iterm2)
