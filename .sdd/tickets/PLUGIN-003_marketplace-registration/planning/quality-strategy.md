# Quality Strategy: Marketplace Registration

## Testing Philosophy

This ticket creates two documentation/configuration files (marketplace.json and plugins/README.md). Testing focuses on:

1. **Structural validation** - Files exist in correct locations with correct formats
2. **Content validation** - All required fields and sections present
3. **Functional validation** - Plugin installation works correctly

Unlike code-centric tickets, there is no unit test coverage to measure. Quality is assessed through validation checklists and manual testing.

## Coverage Requirements

**Minimum Thresholds:**
- File completeness: 100% (all 2 files created)
- Field completeness: 100% (all required fields present)
- Functional completeness: 100% (install/uninstall works)

## Test Types

### Structural Validation

**Scope:** File existence and format correctness

**Tools:** File system checks, JSON validation

**What to Validate:**
- marketplace.json exists at correct location
- marketplace.json is valid JSON
- plugins/README.md exists at correct location
- Markdown is properly formatted

**Validation Commands:**
```bash
# Check file existence
ls -la .crewchief/claude-code-plugins/.claude-plugin/marketplace.json
ls -la .crewchief/claude-code-plugins/plugins/README.md

# Validate JSON
cat .crewchief/claude-code-plugins/.claude-plugin/marketplace.json | jq .

# Check plugin directories exist
ls -la .crewchief/claude-code-plugins/plugins/maproom/
ls -la .crewchief/claude-code-plugins/plugins/worktree/
```

### Content Validation

**Scope:** Required content present and correct

**Tools:** Manual review, jq for JSON

**What to Validate:**
- marketplace.json has plugins array
- Each plugin has name, source, description
- Source paths point to existing directories
- README.md has all sections
- Links in README are valid

**Validation Checks:**
```bash
# Check marketplace.json structure
jq '.plugins[] | .name, .source, .description' \
  .crewchief/claude-code-plugins/.claude-plugin/marketplace.json

# Check source paths exist
jq -r '.plugins[].source' .crewchief/claude-code-plugins/.claude-plugin/marketplace.json | \
  while read path; do
    ls -la ".crewchief/claude-code-plugins/$path"
  done

# Check README sections
grep -E "^#" .crewchief/claude-code-plugins/plugins/README.md
```

### Functional Validation

**Scope:** Plugin installation and discovery

**Tools:** Claude Code CLI

**What to Validate:**
- Plugin installation succeeds
- Skills are discoverable after installation
- Plugin uninstallation succeeds
- No errors in any operation

**Test Workflow:**
1. Start Claude Code in the repository
2. Install maproom plugin: `/plugin install maproom@crewchief`
3. Verify maproom-search skill is available
4. Uninstall maproom plugin
5. Install worktree plugin: `/plugin install worktree@crewchief`
6. Verify worktree-management skill is available
7. Uninstall worktree plugin

## Critical Paths

The following paths MUST be validated:

### 1. Plugin Discovery

- **Happy path:** marketplace.json is read, plugins are listed
- **Error case:** Invalid JSON, missing file
- **Edge case:** Empty plugins array

### 2. Plugin Installation

- **Happy path:** Plugin installs without errors
- **Error case:** Source path doesn't exist, plugin.json missing
- **Edge case:** Plugin already installed

### 3. Skill Discovery

- **Happy path:** Skills listed after installation
- **Error case:** SKILL.md missing or malformed
- **Edge case:** Multiple skills in same plugin

### 4. Plugin Uninstallation

- **Happy path:** Plugin uninstalls without errors
- **Error case:** Plugin not installed
- **Edge case:** Skill in use during uninstall

## Negative Testing Requirements

### Invalid/Malformed Content

- [ ] What happens with invalid JSON in marketplace.json
- [ ] What happens with missing source path
- [ ] What happens with missing plugin name

### Error Scenarios

- [ ] Plugin not found in marketplace.json
- [ ] Source directory doesn't exist
- [ ] plugin.json missing in source directory

### Edge Cases

- [ ] Plugin already installed (should handle gracefully)
- [ ] Plugin not installed during uninstall (should handle gracefully)
- [ ] Empty plugins array in marketplace.json

## Test Data Strategy

No external test data required. Validation uses:
- The created files themselves
- The existing plugin directories
- Claude Code plugin commands

## Quality Gates

Before verification:

### Structural Gates

- [ ] marketplace.json exists at `.crewchief/claude-code-plugins/.claude-plugin/marketplace.json`
- [ ] plugins/README.md exists at `.crewchief/claude-code-plugins/plugins/README.md`
- [ ] marketplace.json is valid JSON
- [ ] Markdown renders correctly

### Content Gates

- [ ] marketplace.json has plugins array
- [ ] Maproom entry has name: "maproom"
- [ ] Maproom entry has source: "./plugins/maproom"
- [ ] Maproom entry has description
- [ ] Worktree entry has name: "worktree"
- [ ] Worktree entry has source: "./plugins/worktree"
- [ ] Worktree entry has description
- [ ] README.md has overview section
- [ ] README.md has installation section
- [ ] README.md has maproom section with version, features, skill, link
- [ ] README.md has worktree section with version, features, skill, link

### Style Gates

- [ ] JSON is properly formatted
- [ ] Markdown follows consistent style
- [ ] No placeholder content
- [ ] Links use relative paths

### Functional Gates

- [ ] `/plugin install maproom@crewchief` succeeds
- [ ] Maproom-search skill is discoverable
- [ ] `/plugin install worktree@crewchief` succeeds
- [ ] Worktree-management skill is discoverable
- [ ] Uninstall works for both plugins

## Verification Checklist

Final verification before marking complete:

```markdown
## File Structure
- [ ] .crewchief/claude-code-plugins/.claude-plugin/marketplace.json exists
- [ ] .crewchief/claude-code-plugins/plugins/README.md exists

## marketplace.json
- [ ] Valid JSON structure
- [ ] Has plugins array
- [ ] Maproom entry: name, source, description
- [ ] Worktree entry: name, source, description
- [ ] Source paths point to existing directories
- [ ] Source paths use correct relative format

## plugins/README.md
- [ ] Title and overview
- [ ] Installation section with both commands
- [ ] Maproom section
  - [ ] Version: 0.1.0
  - [ ] Features list
  - [ ] Skill: maproom-search
  - [ ] Link to maproom/README.md
- [ ] Worktree section
  - [ ] Version: 0.1.0
  - [ ] Features list
  - [ ] Skill: worktree-management
  - [ ] Link to worktree/README.md
- [ ] All links are valid

## Functional Testing
- [ ] Maproom plugin installs successfully
- [ ] Maproom-search skill appears after installation
- [ ] Maproom plugin uninstalls successfully
- [ ] Worktree plugin installs successfully
- [ ] Worktree-management skill appears after installation
- [ ] Worktree plugin uninstalls successfully
- [ ] No errors during install/uninstall cycle
```
