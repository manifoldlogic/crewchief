# Architecture: Marketplace Registration

## Overview

This ticket creates the marketplace registration layer for the maproom and worktree plugins. The architecture creates a `.claude-plugin/` directory at the marketplace root with a marketplace.json registry, plus a catalog README that documents available plugins for users.

**Important Context**: This marketplace is configured as a directory-based marketplace (per `.claude/settings.json` with `"source": "directory"`), which means Claude Code may auto-discover plugins from the `plugins/` directory without needing marketplace.json. This ticket creates the registry structure as a best practice, but verification will determine if marketplace.json is actually necessary for plugin discovery.

```
.crewchief/claude-code-plugins/
├── .claude-plugin/             # TO CREATE: Directory for marketplace metadata
│   └── marketplace.json        # TO CREATE: Plugin registry (may be optional)
└── plugins/
    ├── README.md               # TO CREATE: Plugin catalog documentation
    ├── maproom/                # EXISTS
    │   ├── .claude-plugin/
    │   │   └── plugin.json
    │   ├── README.md
    │   └── skills/
    └── worktree/               # EXISTS
        ├── .claude-plugin/
        │   └── plugin.json
        ├── README.md
        └── skills/
```

## Design Decisions

### Decision 1: Minimal marketplace.json Schema

**Context:** marketplace.json needs to reference plugins for installation. Could include full plugin metadata or just references.

**Decision:** Include only `name`, `source`, and `description` in marketplace.json entries.

**Rationale:**
- Full plugin metadata already exists in each plugin's plugin.json
- Avoids duplication and potential drift
- Marketplace.json serves as a directory, not a database
- Simpler maintenance when plugins update

### Decision 2: Relative Paths for Source References

**Context:** Plugin source paths could be absolute or relative.

**Decision:** Use relative paths from marketplace.json location (e.g., `./plugins/maproom`).

**Rationale:**
- Works regardless of repository checkout location
- Follows filesystem conventions for relative references
- Simpler than managing absolute paths

### Decision 3: Separate plugins/README.md

**Context:** Documentation could go in various places - root README, marketplace README, or plugins/README.

**Decision:** Create plugins/README.md at the plugins directory level.

**Rationale:**
- Natural discovery location for users browsing the plugins directory
- Keeps plugin catalog documentation close to plugins
- Allows root-level documentation to focus on marketplace itself
- Follows the pattern of having documentation at the level it describes

### Decision 4: Include Version and Skill Information in README

**Context:** README could be minimal (just links) or comprehensive (duplicated info).

**Decision:** Include version, features, skill name, and link to detailed README.

**Rationale:**
- Quick scanning without opening each plugin README
- Version helps users understand currency
- Skill name helps with post-installation verification
- Link provides path to full documentation

### Decision 5: Create marketplace.json for Directory-Based Marketplace

**Context:** The marketplace is configured as `"source": "directory"` in `.claude/settings.json`, which may enable auto-discovery of plugins without marketplace.json.

**Decision:** Create marketplace.json and `.claude-plugin/` directory as specified, but verify during testing if it's actually necessary.

**Rationale:**
- Follows expected marketplace structure from epic planning
- Provides explicit registry even if auto-discovery works
- Serves as documentation of available plugins
- Easy to remove if verification shows it's unnecessary
- Low cost to create (simple JSON file)
- Verification phase will determine actual necessity

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Registry Format | JSON | Standard for plugin registries, easy to parse |
| Documentation | Markdown | Standard for GitHub/Claude Code, familiar format |
| Paths | Relative | Portable across environments |

## Component Design

### marketplace.json

**Responsibilities:**
- Register available plugins for discovery
- Provide source paths for installation
- Include brief descriptions for catalog display

**Interface:**
```json
{
  "plugins": [
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
  ]
}
```

**Key Fields:**
- `name`: Plugin identifier used in install commands
- `source`: Relative path to plugin directory
- `description`: One-line summary for catalog display

### plugins/README.md

**Responsibilities:**
- Document all available plugins in one place
- Provide quick reference for features and installation
- Link to detailed plugin documentation

**Structure:**
```markdown
# CrewChief Plugins

Overview of available plugins in the crewchief marketplace.

## Installation

Install plugins individually:
```bash
/plugin install maproom@crewchief
/plugin install worktree@crewchief
```

## Available Plugins

### Maproom
**Version:** 0.1.0
Description and features...
**Skill:** maproom-search
**[Read More](maproom/README.md)**

### Worktree
**Version:** 0.1.0
Description and features...
**Skill:** worktree-management
**[Read More](worktree/README.md)**
```

## Data Flow

```
User: /plugin install maproom@crewchief
          |
          v
+-------------------+
| Claude Code       |
| Plugin System     |
+-------------------+
          |
          v
+-------------------+
| Marketplace       |
| .claude-plugin/   |
| marketplace.json  |
+-------------------+
          |
          | Look up "maproom" in plugins array
          v
+-------------------+
| Plugin Directory  |
| ./plugins/maproom |
+-------------------+
          |
          v
+-------------------+
| Plugin Metadata   |
| plugin.json       |
| README.md         |
| skills/           |
+-------------------+
          |
          v
+-------------------+
| Plugin Installed  |
| Skills Available  |
+-------------------+
```

### Installation Flow

1. User runs `/plugin install maproom@crewchief`
2. Claude Code reads marketplace.json
3. Finds "maproom" entry with source path
4. Loads plugin from `./plugins/maproom/`
5. Reads plugin.json for metadata
6. Registers skills from `skills/` directory
7. Plugin available for use

### Uninstall Flow

1. User runs `/plugin uninstall maproom@crewchief`
2. Claude Code removes plugin registration
3. Skills no longer available
4. Plugin files remain (not deleted)

## Integration Points

### With Plugin System

The marketplace.json integrates with Claude Code's plugin discovery:
- Plugin names become installation identifiers
- Source paths point to plugin directories
- Descriptions appear in plugin catalogs

### With Existing Plugins

References existing plugin structure:
- Each plugin has `.claude-plugin/plugin.json`
- Each plugin has README.md and skills/
- Paths are relative to marketplace.json

### With User Discovery

plugins/README.md provides:
- Browsable documentation for available plugins
- Installation commands ready to copy
- Links to detailed documentation

## File Specifications

### marketplace.json

**Location:** `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`

**Contents:**
```json
{
  "plugins": [
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
  ]
}
```

**Notes:**
- JSON must be valid (trailing commas not allowed)
- Array order does not matter
- Source paths are relative to parent directory (marketplace root)

### plugins/README.md

**Location:** `.crewchief/claude-code-plugins/plugins/README.md`

**Sections:**
1. Title and overview
2. Installation instructions
3. Maproom plugin section
   - Version
   - Description
   - Features
   - Skill name
   - Link to detailed README
4. Worktree plugin section
   - Version
   - Description
   - Features
   - Skill name
   - Link to detailed README

## Maintainability

### Adding New Plugins

When new plugins are added:
1. Create plugin in plugins/ directory
2. Add entry to marketplace.json
3. Add section to plugins/README.md

### Updating Plugins

When plugins are updated:
1. Update plugin files (version, features, etc.)
2. Update README.md if features changed
3. marketplace.json rarely needs updates (names don't change)

### Versioning

- Plugin versions tracked in individual plugin.json files
- marketplace.json does not track versions
- README.md shows versions for quick reference (sync manually)
