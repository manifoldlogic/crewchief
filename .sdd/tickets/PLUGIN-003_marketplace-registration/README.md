# Ticket: Register Plugins in Marketplace

**Ticket ID:** PLUGIN-003
**Status:** Planning Complete
**Created:** 2025-12-17

## Summary

Register both the `maproom` and `worktree` plugins in the crewchief marketplace. Create the marketplace.json file with plugin entries, create a plugins README.md with documentation for both plugins, and verify installation works via Claude Code.

## Problem Statement

The maproom and worktree plugins have been created (PLUGIN-001, PLUGIN-002) but are not yet discoverable through the Claude Code plugin system. Without marketplace registration, users cannot find or install these plugins using `/plugin install`. This ticket completes the plugin distribution pipeline by adding marketplace metadata and documentation.

## Proposed Solution

Create two files in the marketplace root:

```
.crewchief/claude-code-plugins/
├── .claude-plugin/
│   └── marketplace.json        # NEW: Plugin registry
└── plugins/
    ├── README.md               # NEW: Plugin catalog documentation
    ├── maproom/                # EXISTS: From PLUGIN-001
    └── worktree/               # EXISTS: From PLUGIN-002
```

**Key Design Decisions:**
- marketplace.json uses relative paths to plugin directories
- README.md provides overview and installation examples for all plugins
- Both plugins registered with consistent metadata format
- Verification via actual Claude Code installation commands

## Relevant Agents

- ticket-planner (planning phase) - Complete
- task-creator (ticket generation) - Next
- General implementation agent (file creation)
- verify-task (verification)
- commit-task (commit)

## Deliverables

### Files to Create

| File | Purpose |
|------|---------|
| `.claude-plugin/marketplace.json` | Plugin registry with maproom and worktree entries |
| `plugins/README.md` | Plugin catalog with installation instructions |

### Acceptance Criteria

- [ ] marketplace.json created with valid JSON structure
- [ ] marketplace.json contains maproom plugin entry with name, source, description
- [ ] marketplace.json contains worktree plugin entry with name, source, description
- [ ] plugins/README.md created with plugin catalog
- [ ] plugins/README.md documents installation for both plugins
- [ ] `/plugin install maproom@crewchief` succeeds
- [ ] `/plugin install worktree@crewchief` succeeds
- [ ] Skills are discoverable after installation
- [ ] `/plugin uninstall` works for both plugins

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis and research findings
- [architecture.md](planning/architecture.md) - Solution design and file structure
- [plan.md](planning/plan.md) - Two-phase execution plan
- [quality-strategy.md](planning/quality-strategy.md) - Validation approach and checklists
- [security-review.md](planning/security-review.md) - Security assessment (minimal risk)

## Key Technical References

- **Epic Overview:** `/.sdd/epics/2025-12-15_crewchief-cli-skills/overview.md`
- **Maproom Plugin:** `.crewchief/claude-code-plugins/plugins/maproom/`
- **Worktree Plugin:** `.crewchief/claude-code-plugins/plugins/worktree/`
- **Plugin Metadata:** `plugins/maproom/.claude-plugin/plugin.json`, `plugins/worktree/.claude-plugin/plugin.json`

## Dependencies

- **PLUGIN-001 (Maproom Plugin):** COMPLETED
- **PLUGIN-002 (Worktree Plugin):** COMPLETED

## Tasks

See [tasks/](tasks/) for all ticket tasks (to be created by task-creator).

## Next Steps

**Recommended:** Run `/sdd:review PLUGIN-003` before creating tasks to validate planning documents.

Then: Run `/sdd:create-tasks PLUGIN-003` to generate implementation tasks.

## Notes

- This is the final ticket in the CrewChief CLI Skills epic (PLUGIN-001, PLUGIN-002, PLUGIN-003)
- Completing this ticket makes both plugins discoverable and installable
- Low complexity - primarily file creation and verification
- No custom agents needed - straightforward documentation task
