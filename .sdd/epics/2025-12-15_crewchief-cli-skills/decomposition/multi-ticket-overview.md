# Multi-Ticket Overview: CrewChief CLI Plugins

## Context

Epic created: 2025-12-15
Reference: /workspace/.sdd/epics/2025-12-15_crewchief-cli-skills/

## Plugin Architecture

Both plugins will be created in the crewchief marketplace:

```
.crewchief/claude-code-plugins/
├── .claude-plugin/
│   └── marketplace.json        # Add plugin registrations
└── plugins/
    ├── maproom/                 # Ticket PLUGIN-001
    │   ├── .claude-plugin/
    │   │   └── plugin.json
    │   ├── README.md
    │   └── skills/
    │       └── maproom-search/
    │           ├── SKILL.md
    │           └── references/
    │               └── search-best-practices.md
    └── worktree/                # Ticket PLUGIN-002
        ├── .claude-plugin/
        │   └── plugin.json
        ├── README.md
        └── skills/
            └── worktree-management/
                └── SKILL.md
```

## Tickets (in execution order)

### Ticket 1: PLUGIN-001 - Create Maproom Plugin

**Priority:** 1 (High)
**Effort:** M (2-3 days)

**Summary:** Create the `maproom` plugin for the crewchief marketplace. The plugin provides semantic code search capabilities via the crewchief-maproom CLI, teaching Claude when and how to use maproom search instead of grep/glob.

**Deliverables:**
- `.crewchief/claude-code-plugins/plugins/maproom/.claude-plugin/plugin.json`
- `.crewchief/claude-code-plugins/plugins/maproom/README.md`
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md`
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/search-best-practices.md`

**Dependencies:** None

**Value Proposition:** Enables Claude to leverage semantic search for conceptual code exploration, improving code understanding tasks beyond what grep can provide. Plugin architecture allows users to install only if they use maproom.

---

### Ticket 2: PLUGIN-002 - Create Worktree Plugin

**Priority:** 2 (High)
**Effort:** M (2-3 days)

**Summary:** Create the `worktree` plugin for the crewchief marketplace. The plugin provides git worktree management capabilities via the crewchief CLI, documenting the worktree lifecycle with safety guidance.

**Deliverables:**
- `.crewchief/claude-code-plugins/plugins/worktree/.claude-plugin/plugin.json`
- `.crewchief/claude-code-plugins/plugins/worktree/README.md`
- `.crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/SKILL.md`

**Dependencies:** None (can be done in parallel with PLUGIN-001)

**Value Proposition:** Makes git worktrees accessible through Claude, enabling parallel development workflows with proper safety guardrails. Plugin architecture allows users to install only if they use worktrees.

---

### Ticket 3: PLUGIN-003 - Register Plugins in Marketplace

**Priority:** 3 (Medium)
**Effort:** S (1 day)

**Summary:** Register both plugins in the crewchief marketplace.json and test installation via Claude Code.

**Deliverables:**
- Updated `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`
- Updated `.crewchief/claude-code-plugins/plugins/README.md`
- Test that `/plugin install maproom@crewchief` works
- Test that `/plugin install worktree@crewchief` works

**Dependencies:** PLUGIN-001, PLUGIN-002

**Value Proposition:** Makes plugins discoverable and installable through the standard Claude Code plugin system.

## Dependencies

```
PLUGIN-001 (Maproom Plugin) ─────┐
                                 ├──> PLUGIN-003 (Marketplace Registration)
PLUGIN-002 (Worktree Plugin) ────┘
```

Tickets 1 and 2 can be executed in parallel. Ticket 3 depends on both being complete.

## Execution Order Rationale

1. **PLUGIN-001 and PLUGIN-002 in parallel** - Both plugins are independent and can be developed simultaneously. Each creates a complete plugin structure.

2. **PLUGIN-003 last** - Marketplace registration requires both plugins to exist. This ticket also validates the plugins install correctly.

## Acceptance Criteria Summary

All tickets complete when:
- [ ] Both plugins have valid plugin.json with name, version, description
- [ ] Both plugins have README.md with installation and usage docs
- [ ] Both plugins have at least one skill with SKILL.md
- [ ] Skills have proper YAML frontmatter (name, description)
- [ ] Skills include CLI command examples
- [ ] Skills document when to use plugin vs native tools
- [ ] Skills handle common error scenarios
- [ ] Maproom skill includes search-best-practices.md reference
- [ ] Both plugins registered in marketplace.json
- [ ] Plugin installation verified via `/plugin install`

## plugin.json Requirements

### Maproom Plugin

```json
{
  "name": "maproom",
  "version": "0.1.0",
  "description": "Semantic code search using the crewchief-maproom CLI. Find code by concept, understand architecture, and explore relationships between code elements.",
  "author": {
    "name": "Daniel Bushman",
    "email": "dbushman@manifoldlogic.com"
  },
  "repository": "https://github.com/manifoldlogic/claude-code-plugins",
  "keywords": [
    "maproom",
    "semantic-search",
    "code-search",
    "fts",
    "vector-search"
  ]
}
```

### Worktree Plugin

```json
{
  "name": "worktree",
  "version": "0.1.0",
  "description": "Git worktree management using the crewchief CLI. Create, manage, and merge parallel development branches safely.",
  "author": {
    "name": "Daniel Bushman",
    "email": "dbushman@manifoldlogic.com"
  },
  "repository": "https://github.com/manifoldlogic/claude-code-plugins",
  "keywords": [
    "git",
    "worktree",
    "branches",
    "parallel-development"
  ]
}
```

## marketplace.json Additions

```json
{
  "name": "maproom",
  "source": "./plugins/maproom",
  "description": "Semantic code search using crewchief-maproom CLI"
},
{
  "name": "worktree",
  "source": "./plugins/worktree",
  "description": "Git worktree management using crewchief CLI"
}
```
