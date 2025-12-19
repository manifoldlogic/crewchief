# Plan: Marketplace Registration

## Overview

This document outlines the execution plan for registering the maproom and worktree plugins in the crewchief marketplace. The work is organized into two phases: Marketplace Registration (create marketplace.json) and Documentation Update (create plugins/README.md). A final verification step ensures everything works correctly.

## Phases

### Phase 1: Marketplace Registration

**Objective:** Create the marketplace.json file that registers both plugins for discovery and installation.

**Deliverables:**
- marketplace.json file at `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`
- Both plugins (maproom, worktree) registered with name, source, and description

**Agent Assignments:**
- General implementation agent: Create marketplace.json file

**Acceptance Criteria:**
- [ ] marketplace.json file exists at correct location
- [ ] File contains valid JSON
- [ ] plugins array contains maproom entry
- [ ] plugins array contains worktree entry
- [ ] Each entry has name, source, and description fields
- [ ] Source paths are relative and correct

**Estimated Effort:** 30 minutes

### Phase 2: Documentation Update

**Objective:** Create the plugins/README.md that documents all available plugins.

**Deliverables:**
- README.md file at `.crewchief/claude-code-plugins/plugins/README.md`
- Documentation for both maproom and worktree plugins
- Installation examples

**Agent Assignments:**
- General implementation agent: Create plugins/README.md file

**Acceptance Criteria:**
- [ ] README.md file exists at correct location
- [ ] Overview section explains purpose
- [ ] Installation section with commands for both plugins
- [ ] Maproom plugin section with version, features, skill name, link
- [ ] Worktree plugin section with version, features, skill name, link
- [ ] All links are valid
- [ ] No placeholder content

**Estimated Effort:** 45 minutes

### Phase 3: Verification (Manual)

**Objective:** Verify that plugin installation works correctly and determine if marketplace.json is necessary for directory-based marketplace.

**Deliverables:**
- Verification report confirming successful installation
- Analysis of whether marketplace.json is required for directory-based marketplace
- Recommendation on whether to keep or remove marketplace.json

**Agent Assignments:**
- Verification agent (or manual): Execute test commands and analyze marketplace.json necessity

**Test Cases:**
1. Install maproom plugin: `/plugin install maproom@crewchief`
2. Verify maproom-search skill is available
3. Uninstall maproom plugin: `/plugin uninstall maproom@crewchief`
4. Install worktree plugin: `/plugin install worktree@crewchief`
5. Verify worktree-management skill is available
6. Uninstall worktree plugin: `/plugin uninstall worktree@crewchief`
7. Analyze whether marketplace.json is required for plugin discovery (important for directory-based marketplaces)

**Acceptance Criteria:**
- [ ] Maproom plugin installs without errors
- [ ] Maproom-search skill is discoverable after installation
- [ ] Maproom plugin uninstalls without errors
- [ ] Worktree plugin installs without errors
- [ ] Worktree-management skill is discoverable after installation
- [ ] Worktree plugin uninstalls without errors
- [ ] Verification report documents whether marketplace.json is necessary
- [ ] Clear recommendation provided on keeping or removing marketplace.json

**Important Context:** This marketplace is configured as directory-based (`"source": "directory"` in `.claude/settings.json`), which may enable auto-discovery without marketplace.json. Verification must determine if the file is actually needed.

**Estimated Effort:** 45 minutes (increased to include marketplace.json analysis)

## Dependencies

### External Dependencies

| Dependency | Description | Mitigation |
|------------|-------------|------------|
| Claude Code | Plugin system must be available | Document as requirement |
| Existing plugins | PLUGIN-001 and PLUGIN-002 must be complete | Both are complete |

### Internal Dependencies

| Phase | Depends On |
|-------|------------|
| Phase 2 | Phase 1 (marketplace.json should exist first) |
| Phase 3 | Phase 1 and Phase 2 (both files needed) |

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| JSON validation errors | Low | Low | Use JSON validator before committing |
| Path resolution issues | Low | Medium | Test with actual plugin install |
| Plugin not found errors | Low | Medium | Verify source paths match actual directories |
| README links broken | Low | Low | Test links before completion |

## Success Metrics

### Completion Criteria

- [ ] marketplace.json exists and is valid JSON
- [ ] plugins/README.md exists and is complete
- [ ] Both plugins can be installed
- [ ] Both plugins can be uninstalled
- [ ] Skills are discoverable after installation

### Quality Criteria

- [ ] JSON is properly formatted
- [ ] Markdown follows consistent style
- [ ] No placeholder content
- [ ] All links work

### Testing Checklist

- [ ] marketplace.json validates as JSON
- [ ] Source paths point to existing directories
- [ ] `/plugin install maproom@crewchief` succeeds
- [ ] `/plugin install worktree@crewchief` succeeds
- [ ] Skills listed after installation
- [ ] Uninstall works for both plugins

## File Manifest

Files to be created:

```
.crewchief/claude-code-plugins/
├── .claude-plugin/
│   └── marketplace.json         # ~15 lines
└── plugins/
    └── README.md                # ~80 lines
```

Total new files: 2
Total lines: ~95

## Implementation Notes

### marketplace.json Content

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

### plugins/README.md Structure

```markdown
# CrewChief Plugins

Overview of available plugins in the crewchief marketplace.

## Installation

Install plugins individually:

```
/plugin install maproom@crewchief
/plugin install worktree@crewchief
```

## Available Plugins

### Maproom

**Version:** 0.1.0

Semantic code search using the crewchief-maproom CLI.

**Features:**
- Full-Text Search (FTS) for keyword-based search
- Vector search for semantic similarity
- Context expansion (callers, callees, tests)
- Multi-repository support

**Skill:** `maproom-search`

**[Read More](maproom/README.md)**

### Worktree

**Version:** 0.1.0

Git worktree management using the crewchief CLI.

**Features:**
- Parallel development environments
- Safe worktree lifecycle management
- Merge strategies (ff, squash, cherry-pick)
- Copy ignored files to worktrees

**Skill:** `worktree-management`

**[Read More](worktree/README.md)**
```

## Post-Completion Steps

After implementation:

1. Verify marketplace.json is valid JSON
2. Verify README.md links work
3. Test plugin installation
4. Test skill discovery
5. Test plugin uninstallation
6. Commit with appropriate message
7. Update epic status (all tickets complete)
