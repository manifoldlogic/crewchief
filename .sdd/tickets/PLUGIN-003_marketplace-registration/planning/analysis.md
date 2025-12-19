# Analysis: Marketplace Registration

## Problem Definition

The maproom and worktree plugins have been successfully created in the crewchief marketplace directory structure, but they are not yet registered in the marketplace catalog. Without marketplace registration:

1. Users cannot discover available plugins through the plugin system
2. Users cannot install plugins using `/plugin install <name>@crewchief`
3. The plugins exist but are effectively invisible to the Claude Code plugin infrastructure

This ticket completes the plugin distribution pipeline by creating the marketplace.json registry and accompanying documentation.

## Context

### Current Situation

1. **Maproom plugin exists and is complete** - Located at `.crewchief/claude-code-plugins/plugins/maproom/` with:
   - Valid `plugin.json` (name: "maproom", version: "0.1.0")
   - Complete README.md documentation
   - `maproom-search` skill with SKILL.md and references

2. **Worktree plugin exists and is complete** - Located at `.crewchief/claude-code-plugins/plugins/worktree/` with:
   - Valid `plugin.json` (name: "worktree", version: "0.1.0")
   - Complete README.md documentation
   - `worktree-management` skill with SKILL.md

3. **Directory-based marketplace configured** - `.claude/settings.json` shows marketplace configured with `"source": "directory"` and `"path": ".crewchief/claude-code-plugins"`

4. **No .claude-plugin/ directory exists at marketplace root** - The `.crewchief/claude-code-plugins/.claude-plugin/` directory does not exist

5. **No marketplace.json exists** - No marketplace.json file exists anywhere in the repository

6. **No plugins/README.md exists** - No catalog documentation exists at the plugins directory level

**Important Note**: For directory-based marketplaces, Claude Code may auto-discover plugins by scanning the plugins directory, potentially making marketplace.json optional. This ticket will create marketplace.json as a registry best practice, but verification will determine if it's actually necessary.

### Why This Work is Needed

- Completes the epic by making plugins discoverable
- Enables the standard plugin installation workflow
- Provides documentation for users browsing the plugin catalog
- Validates that the entire plugin creation pipeline works end-to-end

## Existing Solutions

### Industry Approaches

1. **npm registry** - Uses package.json in each package plus a central registry
2. **VS Code Marketplace** - Uses package.json manifests plus marketplace metadata
3. **GitHub Actions Marketplace** - Uses action.yml in each action plus README discovery

### Codebase Pattern

The crewchief marketplace follows a simple pattern:
- Each plugin has its own `plugin.json` in `.claude-plugin/`
- A central `marketplace.json` at the marketplace root references plugins
- Plugins are installed via `/plugin install <name>@<marketplace>`

**Reference:** The epic overview specifies this exact structure:
```
.crewchief/claude-code-plugins/
├── .claude-plugin/
│   └── marketplace.json        # Register new plugins here
└── plugins/
    ├── maproom/
    └── worktree/
```

## Current State

### Maproom Plugin Metadata (from plugin.json)

```json
{
  "name": "maproom",
  "version": "0.1.0",
  "description": "Semantic code search using the crewchief-maproom CLI. Find code by concept, understand architecture, and explore relationships between code elements.",
  "author": {
    "name": "Daniel Bushman",
    "email": "dbushman@manifoldlogic.com",
    "url": "https://github.com/manifoldlogic/claude-code-plugins"
  },
  "repository": "https://github.com/manifoldlogic/claude-code-plugins",
  "keywords": ["maproom", "semantic-search", "code-search", "fts", "vector-search"]
}
```

### Worktree Plugin Metadata (from plugin.json)

```json
{
  "name": "worktree",
  "version": "0.1.0",
  "description": "Git worktree management using the crewchief CLI. Create, manage, and merge parallel development branches safely.",
  "author": {
    "name": "Daniel Bushman",
    "email": "dbushman@manifoldlogic.com",
    "url": "https://github.com/manifoldlogic/claude-code-plugins"
  },
  "repository": "https://github.com/manifoldlogic/claude-code-plugins",
  "keywords": ["git", "worktree", "branches", "parallel-development", "parallel", "isolation"]
}
```

### Directory Structure Required

```
.crewchief/claude-code-plugins/
├── .claude-plugin/
│   └── marketplace.json        # TO CREATE
└── plugins/
    ├── README.md               # TO CREATE
    ├── maproom/                # EXISTS
    └── worktree/               # EXISTS
```

## Research Findings

### Finding 1: marketplace.json Format

Based on the epic overview and ticket summary, marketplace.json should contain a `plugins` array with entries for each plugin:

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

Key fields:
- `name`: Plugin identifier for installation commands
- `source`: Relative path to plugin directory
- `description`: Brief description for catalog display

### Finding 2: plugins/README.md Structure

Based on ticket summary guidance, the README should include:

1. Overview of available plugins
2. Plugin sections with:
   - Version information
   - Feature list
   - Skill names
   - Link to detailed README
3. Installation examples

### Finding 3: Verification Steps

Verification requires:
1. Starting Claude Code in the repository
2. Running `/plugin install maproom@crewchief`
3. Confirming no errors
4. Checking skill discovery
5. Running `/plugin uninstall maproom@crewchief`
6. Repeating for worktree plugin

## Constraints

### Technical Constraints

1. **Path format** - Source paths must be relative to marketplace.json location
2. **JSON validity** - marketplace.json must be valid JSON
3. **Plugin existence** - Referenced plugins must exist at specified paths
4. **Description length** - Keep descriptions concise for catalog display

### Business Constraints

1. **Complete both plugins** - Both maproom and worktree must be registered
2. **Consistent format** - Use same structure for both plugin entries
3. **Documentation completeness** - README should document all plugins

### Resource Constraints

1. **Small scope** - Only 2 files to create
2. **Low complexity** - Straightforward JSON and markdown

## Success Criteria

### Functional Criteria

- [ ] marketplace.json created at `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`
- [ ] marketplace.json contains valid JSON with plugins array
- [ ] Maproom plugin entry has name, source, description
- [ ] Worktree plugin entry has name, source, description
- [ ] plugins/README.md created at `.crewchief/claude-code-plugins/plugins/README.md`
- [ ] README.md documents both plugins
- [ ] README.md includes installation commands

### Verification Criteria

- [ ] `/plugin install maproom@crewchief` succeeds
- [ ] `/plugin install worktree@crewchief` succeeds
- [ ] Maproom-search skill is discoverable after installation
- [ ] Worktree-management skill is discoverable after installation
- [ ] `/plugin uninstall` works for both plugins
- [ ] No errors during install/uninstall cycle

### Quality Criteria

- [ ] JSON is properly formatted
- [ ] Markdown follows consistent style
- [ ] All links are valid
- [ ] No placeholder content
