# Ticket: Register Plugins in Marketplace

**Ticket ID:** PLUGIN-003
**Priority:** 3 (Medium)
**Effort:** S (1 day)

## Summary

Register both the `maproom` and `worktree` plugins in the crewchief marketplace. Update `marketplace.json` with plugin entries and update the plugins README with documentation for the new plugins. Verify installation works via Claude Code.

## Deliverables

1. **Updated `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`:**
   - Add maproom plugin entry
   - Add worktree plugin entry

2. **Updated `.crewchief/claude-code-plugins/plugins/README.md`:**
   - Add maproom plugin section
   - Add worktree plugin section
   - Update installation examples

3. **Verification:**
   - Test `/plugin install maproom@crewchief` works
   - Test `/plugin install worktree@crewchief` works
   - Test `/plugin uninstall` works for both
   - Verify skills are discoverable after installation

## Dependencies

- PLUGIN-001 (Maproom Plugin must exist)
- PLUGIN-002 (Worktree Plugin must exist)

## Value Proposition

Makes plugins discoverable and installable through the standard Claude Code plugin system. Users can find these plugins in the marketplace and install them with a single command.

## Acceptance Criteria

- [ ] marketplace.json updated with both plugin entries
- [ ] plugins/README.md documents both new plugins
- [ ] `/plugin install maproom@crewchief` succeeds
- [ ] `/plugin install worktree@crewchief` succeeds
- [ ] Skills are listed after installation
- [ ] No errors during install/uninstall cycle

## Technical Notes

### marketplace.json Changes

Add these entries to the `plugins` array:

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

### plugins/README.md Section

Add after existing plugins:

```markdown
### Maproom
**Version:** 0.1.0

Semantic code search using the crewchief-maproom CLI.

**Features:**
- Find code by concept (e.g., "authentication", "error handling")
- Understand codebase architecture
- Explore relationships between code elements
- Context-aware code exploration (callers, callees, tests)

**Skill:** `maproom-search` - Semantic search and context assembly

**[Read More](maproom/README.md)**

### Worktree
**Version:** 0.1.0

Git worktree management using the crewchief CLI.

**Features:**
- Create parallel development environments
- Safe worktree lifecycle management
- Merge strategies (ff, squash, cherry-pick)
- Copy ignored files to worktrees

**Skill:** `worktree-management` - Create, manage, and merge worktrees

**[Read More](worktree/README.md)**
```

### Installation Commands Section

Update to include:

```markdown
Install plugins individually:
```bash
/plugin install workstream@crewchief
/plugin install github-actions@crewchief
/plugin install maproom@crewchief
/plugin install worktree@crewchief
```

### Verification Steps

1. Start Claude Code in the repository
2. Run `/plugin install maproom@crewchief`
3. Verify no errors
4. Check that maproom-search skill appears in skill list
5. Run `/plugin uninstall maproom@crewchief`
6. Repeat for worktree plugin

## Reference Documentation

- `/workspace/.crewchief/claude-code-plugins/.claude-plugin/marketplace.json` - Current marketplace config
- `/workspace/.crewchief/claude-code-plugins/plugins/README.md` - Plugin documentation
- Claude Code plugin system documentation
