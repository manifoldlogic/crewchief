# Backlog: CrewChief CLI Plugins

Ideas identified during research but not yet ready for ticket creation.

## Tickets (Ready for Implementation)

| Ticket ID | Title | Priority | Effort | Status |
|-----------|-------|----------|--------|--------|
| PLUGIN-001 | Create Maproom Plugin | 1 (High) | M | Ready |
| PLUGIN-002 | Create Worktree Plugin | 2 (High) | M | Ready |
| PLUGIN-003 | Register Plugins in Marketplace | 3 (Medium) | S | Ready (depends on PLUGIN-001, PLUGIN-002) |

## Plugin Architecture Summary

Both plugins will be created at `.crewchief/claude-code-plugins/plugins/`:

```
plugins/
├── maproom/                   # PLUGIN-001
│   ├── .claude-plugin/
│   │   └── plugin.json
│   ├── README.md
│   └── skills/
│       └── maproom-search/
│           ├── SKILL.md
│           └── references/
│               └── search-best-practices.md
└── worktree/                  # PLUGIN-002
    ├── .claude-plugin/
    │   └── plugin.json
    ├── README.md
    └── skills/
        └── worktree-management/
            └── SKILL.md
```

## Future Ideas

| Idea | Source | Notes | Status |
|------|--------|-------|--------|
| Auto-index on first search | Research | Would require detecting "not indexed" error and running scan | Deferred - adds complexity |
| Embedding status check | Research | Check if embeddings exist before recommending hybrid mode | Could add to PLUGIN-001 |
| Multi-repo skill support | Research | Guide for searching across multiple repositories | Future enhancement |
| Worktree templates | Research | Pre-configured worktree setups for common workflows | Future enhancement |
| Search result caching | Research | Cache frequent searches for faster repeated queries | Out of scope - CLI feature |
| Maproom agent | Research | Specialized agent for complex search workflows | v2 enhancement |
| Worktree commands | Research | Slash commands for common worktree operations | v2 enhancement |
| SessionStart hooks | Research | Auto-check maproom status on session start | v2 enhancement |

## Notes

- All three tickets are fully specified in `/workspace/.sdd/epics/2025-12-15_crewchief-cli-skills/decomposition/ticket-summaries/`
- PLUGIN-001 and PLUGIN-002 can be executed in parallel
- PLUGIN-003 depends on both PLUGIN-001 and PLUGIN-002
- Initial version includes skills only (no agents, commands, or hooks)
- Plugins follow patterns from existing marketplace plugins (workstream, github-actions, sdd)
