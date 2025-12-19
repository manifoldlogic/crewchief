# Task: [PLUGIN-003.2001]: Create plugins/README.md

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation task)
- [x] **Verified** - by the verify-task agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-implementation
- verify-task
- commit-task

## Summary
Create the plugins/README.md documentation file that catalogs all available plugins with installation instructions, features, and links to detailed documentation.

## Background
This ticket creates the user-facing catalog documentation for the crewchief marketplace. The README provides quick reference for plugin discovery, installation commands, and feature summaries, helping users understand and install available plugins.

This implements Phase 2 of the PLUGIN-003 plan, building on the marketplace.json registry created in Phase 1.

## Acceptance Criteria
- [x] README.md file exists at `.crewchief/claude-code-plugins/plugins/README.md`
- [x] Overview section explains the purpose of crewchief plugins
- [x] Installation section includes commands for both maproom and worktree plugins
- [x] Maproom plugin section includes version, description, features list, skill name, and link
- [x] Worktree plugin section includes version, description, features list, skill name, and link
- [x] All links use relative paths and point to existing files
- [x] No placeholder content (e.g., "TODO", "TBD", "{placeholder}")
- [x] Markdown renders correctly without formatting errors

## Technical Requirements
- **File location**: `.crewchief/claude-code-plugins/plugins/README.md`
- **Format**: Valid Markdown
- **Version accuracy**: Plugin versions must match those in respective plugin.json files (0.1.0)
- **Link format**: Relative paths from plugins/ directory (e.g., `maproom/README.md`)
- **Skill names**: Must match actual skill directory names
- **Encoding**: UTF-8

## Implementation Notes

### File Structure
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

### Content Sources
- **Version numbers**: Read from `.crewchief/claude-code-plugins/plugins/{name}/.claude-plugin/plugin.json`
- **Features**: Extract from individual plugin README files
- **Skill names**: Match directory names in `plugins/{name}/skills/`

### Design Decisions
1. **Quick reference format**: Version, features, skill name visible without clicking links
2. **Installation commands**: Ready to copy/paste for user convenience
3. **Relative links**: Allow documentation to work from any checkout location
4. **Consistent structure**: Each plugin follows same format for scannability

## Dependencies
- **PLUGIN-003.1001**: marketplace.json should exist first (establishes registration pattern)
- **PLUGIN-001**: Maproom plugin README must exist at `plugins/maproom/README.md`
- **PLUGIN-002**: Worktree plugin README must exist at `plugins/worktree/README.md`

## Risk Assessment
- **Risk**: Plugin versions drift out of sync with plugin.json
  - **Mitigation**: Document version as of creation (0.1.0), update when plugins update
- **Risk**: Feature lists become stale
  - **Mitigation**: Keep high-level features only, link to detailed README for specifics
- **Risk**: Links break if plugin directories move
  - **Mitigation**: Use relative paths, verify links exist during task
- **Risk**: Skill names are incorrect
  - **Mitigation**: Verify against actual skill directory names

## Files/Packages Affected
- `.crewchief/claude-code-plugins/plugins/README.md` (new file)

## Deliverables Produced

Documents created in `deliverables/` directory:

- None

## Verification Notes

The verify-task agent should check:

1. **File existence**: README.md exists at correct location
2. **Structure completeness**: All required sections present (overview, installation, both plugins)
3. **Content completeness**: Each plugin has version, description, features, skill, link
4. **Version accuracy**: Versions match plugin.json files (both should be 0.1.0)
5. **Link validity**: All relative links point to existing files
6. **Skill name accuracy**: Skill names match actual skill directories
7. **No placeholders**: No TODO, TBD, or placeholder text
8. **Markdown validity**: Renders correctly, no broken formatting
9. **Installation commands**: Both commands present and correctly formatted

### Test Commands
```bash
# Verify file exists
ls -la .crewchief/claude-code-plugins/plugins/README.md

# Check for placeholder content
grep -i "todo\|tbd\|placeholder\|fixme" .crewchief/claude-code-plugins/plugins/README.md

# Verify section headers
grep -E "^#" .crewchief/claude-code-plugins/plugins/README.md

# Verify links exist
cd .crewchief/claude-code-plugins/plugins
grep -o '\[.*\]([^)]*\.md)' README.md | sed 's/.*(\(.*\))/\1/' | \
  while read link; do
    ls -la "$link" || echo "Missing link: $link"
  done

# Verify skill directories exist
ls -d .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search
ls -d .crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management

# Check version in maproom plugin.json
jq -r '.version' .crewchief/claude-code-plugins/plugins/maproom/.claude-plugin/plugin.json

# Check version in worktree plugin.json
jq -r '.version' .crewchief/claude-code-plugins/plugins/worktree/.claude-plugin/plugin.json
```

### Validation Checklist
- [ ] Title: "# CrewChief Plugins"
- [ ] Overview paragraph explaining purpose
- [ ] Installation section with both commands
- [ ] Maproom section with: Version 0.1.0, 4 features, skill name, link
- [ ] Worktree section with: Version 0.1.0, 4 features, skill name, link
- [ ] Links: `maproom/README.md` and `worktree/README.md` exist
- [ ] Skills: `maproom-search` and `worktree-management` directories exist

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-17 | verify-task | PASS | All 8 acceptance criteria met, versions match plugin.json (0.1.0), all links verified |
<!-- Entries added automatically during verification -->
