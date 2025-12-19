# Architecture: Worktree Plugin

## Overview

The worktree plugin is a Claude Code plugin that teaches Claude how to manage git worktrees using the crewchief CLI. It follows the established plugin pattern from the maproom plugin, consisting of:

1. **Plugin metadata** (`plugin.json`) - Discovery and identification
2. **User documentation** (`README.md`) - Installation and usage guide
3. **Skill definition** (`SKILL.md`) - Detailed instructions for Claude

The plugin contains no executable code - it is purely instructional content that Claude consumes to understand worktree management capabilities.

## Design Decisions

### Decision 1: Single Skill for Complete Lifecycle

**Context:** Worktree operations form a coherent lifecycle (create -> use -> merge -> clean). Should this be one skill or multiple skills per operation?

**Decision:** Create a single `worktree-management` skill covering the complete lifecycle.

**Rationale:**
- Worktree operations are interdependent (merge requires create, clean follows merge)
- Safety guidance requires understanding the full lifecycle
- Users typically need the complete workflow, not isolated commands
- Matches the natural mental model of "worktree management" as one concept

### Decision 2: No References Subdirectory

**Context:** The maproom plugin includes `references/search-best-practices.md` for detailed examples. Should worktree plugin have similar references?

**Decision:** No separate references file. All content in SKILL.md.

**Rationale:**
- Worktree workflows are more linear than search patterns
- The 6 commands with options fit well in a single file
- Common workflows are finite (feature dev, experiment, cleanup)
- Keeps plugin simpler and easier to maintain
- Can add references later if needed

### Decision 3: Safety-First Documentation Structure

**Context:** How should the skill document safety considerations?

**Decision:** Dedicate a prominent section to safety checks and dangerous operations, placed before command reference.

**Rationale:**
- Worktree operations can cause data loss if sequenced incorrectly
- CLI has built-in protections but users need to understand why
- Placing safety early ensures it's not overlooked
- Matches the importance given to error handling in maproom skill

### Decision 4: Workflow-Centric Organization

**Context:** Should the skill be organized by command or by workflow?

**Decision:** Organize by workflow first (lifecycle phases), then provide command reference.

**Rationale:**
- Users think in terms of "I want to do X" not "I need command Y"
- Workflows provide context for when/why to use each command
- Command reference serves as quick lookup after learning workflows
- Consistent with how maproom skill presents decision trees first

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Plugin Format | Claude Code Plugin Spec | Required for marketplace compatibility |
| Documentation Format | Markdown with YAML frontmatter | Standard for skills, supports rich content |
| Example Format | Bash code blocks | Copy-paste ready CLI commands |
| Structure | Single skill, no references | Simpler structure fits worktree's linear nature |

## Component Design

### Plugin Metadata (`plugin.json`)

**Responsibilities:**
- Identify plugin for discovery (`name: "worktree"`)
- Version tracking (`version: "0.1.0"`)
- Describe capabilities for search/discovery
- Attribute authorship and repository

**Interface:**
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

### User Documentation (`README.md`)

**Responsibilities:**
- Introduce plugin capabilities
- List prerequisites (crewchief CLI, git repository)
- Provide installation instructions
- Show usage examples
- Troubleshoot common issues

**Sections:**
1. Introduction - What the plugin does
2. Features - Capability summary
3. Prerequisites - What users need
4. Installation - Plugin install command
5. Usage Examples - Quick start scenarios
6. Troubleshooting - Common problems and solutions

### Skill Definition (`skills/worktree-management/SKILL.md`)

**Responsibilities:**
- Define when this skill applies (frontmatter description)
- Explain worktree lifecycle phases
- Document all CLI commands with examples
- Provide safety guidance and warnings
- Show complete workflow examples

**Structure:**
1. **YAML Frontmatter** - name, description (triggers skill activation)
2. **Overview** - Worktree concept introduction
3. **Decision Tree** - When to use worktree-management vs other git workflows
4. **Worktree Lifecycle** - The 5 phases
5. **Safety Considerations** - Dangers and protections
6. **CLI Command Reference** - All 6 commands with options
7. **Common Workflows** - Step-by-step examples
8. **Error Handling** - What to do when things go wrong

## Data Flow

```
User Request -> Claude Code -> Skill Matching (description)
                                      |
                                      v
                              Load SKILL.md content
                                      |
                                      v
                              Claude understands workflow
                                      |
                                      v
                              Generate CLI commands
                                      |
                                      v
                              Execute via Bash tool
```

The plugin does not execute commands - it provides instructions that Claude uses to generate appropriate CLI invocations through the Bash tool.

## Integration Points

### With Claude Code Plugin System

- **Installation**: `/plugin install worktree@crewchief`
- **Discovery**: Plugin metadata enables marketplace listing
- **Activation**: Skill description triggers when user asks about worktrees

### With Crewchief CLI

- **Runtime Dependency**: CLI must be installed for commands to work
- **Command Interface**: All operations go through `crewchief worktree <subcommand>`
- **Output Parsing**: CLI outputs paths and messages that Claude can interpret

### With Git

- **Repository Context**: Commands only work in git repositories
- **Worktree Storage**: Default at `.crewchief/worktrees/` (configurable)
- **Branch Management**: Worktree commands create/delete branches

### With Maproom

- **Database Cleanup**: `worktree clean` invokes maproom stale record cleanup
- **Best-Effort Integration**: Maproom cleanup failures don't block worktree operations

## Performance Considerations

Not applicable - plugin is documentation only. Performance depends on:
- Claude Code skill loading (handled by Claude Code)
- CLI execution speed (handled by crewchief CLI)
- Git operations (depends on repository size)

## Maintainability

### Documentation Updates

When CLI commands change:
1. Update SKILL.md command reference
2. Update README.md examples if affected
3. Bump version in plugin.json

### Version Strategy

- **Patch** (0.1.x): Documentation fixes, clarifications
- **Minor** (0.x.0): New commands documented, workflow changes
- **Major** (x.0.0): Breaking changes to documented behavior

### Authoritative Source

The crewchief CLI source (`/packages/cli/src/cli/worktree.ts`) is the authoritative reference. Plugin documentation should link to it for edge cases.

## File Manifest

```
.crewchief/claude-code-plugins/plugins/worktree/
├── .claude-plugin/
│   └── plugin.json                    # ~20 lines
├── README.md                          # ~100 lines
└── skills/
    └── worktree-management/
        └── SKILL.md                   # ~200-250 lines

Total files: 3
Total lines: ~320-370
```
