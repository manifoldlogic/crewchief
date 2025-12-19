# Task: [PLUGIN-003.1001]: Create marketplace.json

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
Create the marketplace.json file that registers both maproom and worktree plugins for discovery and installation via Claude Code plugin system.

## Background
This ticket establishes the marketplace registration layer for the crewchief plugin ecosystem. The marketplace.json serves as the central registry that enables plugin discovery and installation through Claude Code's `/plugin install` command.

This implements Phase 1 of the PLUGIN-003 plan, creating the foundation for plugin distribution before documentation is added in Phase 2.

## Acceptance Criteria
- [x] `.claude-plugin/` directory exists at `.crewchief/claude-code-plugins/.claude-plugin/`
- [x] marketplace.json file exists at `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`
- [x] File contains valid JSON (no syntax errors)
- [x] plugins array contains maproom entry with name, source, and description fields
- [x] plugins array contains worktree entry with name, source, and description fields
- [x] Source paths are relative (format: `./plugins/{name}`)
- [x] Source paths point to existing plugin directories
- [x] JSON validation passes (`jq .` succeeds)

## Technical Requirements
- **File location**: `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`
- **Format**: Valid JSON with no trailing commas
- **Structure**: Root object with `plugins` array
- **Plugin entries**: Each must have `name` (string), `source` (string), `description` (string)
- **Path format**: Relative paths from marketplace.json parent directory
- **Encoding**: UTF-8

## Implementation Notes

### Implementation Steps

1. **Create `.claude-plugin/` directory**:
   ```bash
   mkdir -p .crewchief/claude-code-plugins/.claude-plugin
   ```

2. **Create marketplace.json file** with plugin entries:
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

### Design Decisions
1. **Minimal schema**: Only name, source, description - full metadata lives in plugin.json
2. **Relative paths**: Portable across environments, relative to marketplace root
3. **Array structure**: Allows future plugins to be added easily
4. **Directory creation**: `.claude-plugin/` directory must be created at marketplace root

### Important Note on Directory-Based Marketplaces

The `.claude/settings.json` shows this marketplace is configured as `"source": "directory"`, which means Claude Code may auto-discover plugins by scanning the `plugins/` directory without needing marketplace.json. This task creates marketplace.json as a registry best practice, but verification (Task PLUGIN-003.3001) will determine if it's actually necessary for plugin discovery.

### Validation Steps
After creating the file, validate:
1. Directory creation: `ls -d .crewchief/claude-code-plugins/.claude-plugin`
2. JSON syntax: `cat .crewchief/claude-code-plugins/.claude-plugin/marketplace.json | jq .`
3. Path existence: Verify `./plugins/maproom/` and `./plugins/worktree/` exist
4. Required fields: Each plugin has name, source, description

## Dependencies
- **PLUGIN-001**: Maproom plugin must exist at `.crewchief/claude-code-plugins/plugins/maproom/`
- **PLUGIN-002**: Worktree plugin must exist at `.crewchief/claude-code-plugins/plugins/worktree/`

## Risk Assessment
- **Risk**: marketplace.json may not be needed for directory-based marketplaces
  - **Impact**: Medium - File creation might be unnecessary
  - **Mitigation**: Create file as registry best practice, verify necessity in Task PLUGIN-003.3001
  - **Fallback**: If verification shows file is unnecessary, it can be removed in cleanup task
- **Risk**: JSON syntax errors prevent plugin discovery
  - **Impact**: Low - Would break plugin installation
  - **Mitigation**: Validate with `jq` before committing
- **Risk**: Source paths don't resolve correctly
  - **Impact**: Medium - Plugins wouldn't install
  - **Mitigation**: Test paths exist, use relative format
- **Risk**: `.claude-plugin/` directory at wrong level
  - **Impact**: Medium - Plugin discovery might fail
  - **Mitigation**: Verify directory structure matches Claude Code expectations in verification phase

## Files/Packages Affected
- `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json` (new file)

## Deliverables Produced

Documents created in `deliverables/` directory:

- None

## Verification Notes

The verify-task agent should check:

1. **Directory existence**: `.claude-plugin/` directory exists at `.crewchief/claude-code-plugins/.claude-plugin/`
2. **File existence**: marketplace.json exists at correct location
3. **JSON validity**: File parses without errors (`jq .` succeeds)
4. **Required structure**: Has `plugins` array with 2 entries
5. **Field completeness**: Each plugin has name, source, description
6. **Path correctness**: Source paths use relative format and point to existing directories
7. **Content accuracy**: Plugin names match "maproom" and "worktree" exactly
8. **No placeholders**: All descriptions are meaningful, not placeholder text

### Test Commands
```bash
# Verify directory exists
ls -d .crewchief/claude-code-plugins/.claude-plugin

# Verify file exists
ls -la .crewchief/claude-code-plugins/.claude-plugin/marketplace.json

# Validate JSON
jq . .crewchief/claude-code-plugins/.claude-plugin/marketplace.json

# Check plugin count
jq '.plugins | length' .crewchief/claude-code-plugins/.claude-plugin/marketplace.json

# Extract and verify plugin names
jq -r '.plugins[].name' .crewchief/claude-code-plugins/.claude-plugin/marketplace.json

# Verify source paths exist
jq -r '.plugins[].source' .crewchief/claude-code-plugins/.claude-plugin/marketplace.json | \
  while read path; do
    ls -d ".crewchief/claude-code-plugins/$path" || echo "Missing: $path"
  done
```

### Note on Marketplace.json Necessity

This task creates marketplace.json as specified in the original requirements. However, since this is a directory-based marketplace (per `.claude/settings.json`), Claude Code may auto-discover plugins without needing this file. Task PLUGIN-003.3001 will verify if marketplace.json is actually necessary for plugin discovery. If found to be unnecessary, a cleanup task can be created to remove it.

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-17 | verify-task | PASS | All 8 acceptance criteria met, JSON valid, paths correct |
<!-- Entries added automatically during verification -->
