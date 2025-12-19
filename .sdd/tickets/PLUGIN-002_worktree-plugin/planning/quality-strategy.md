# Quality Strategy: Worktree Plugin

## Testing Philosophy

This ticket creates a Claude Code plugin consisting of documentation files (markdown and JSON). Testing focuses on:

1. **Structural validation** - Files exist in correct locations with correct formats
2. **Content validation** - All required sections present, no placeholders
3. **Functional validation** - Plugin installs correctly, skill activates appropriately

Unlike code-centric tickets, there is no unit test coverage to measure. Quality is assessed through validation checklists and manual testing.

## Coverage Requirements

**Minimum Thresholds:**
- File completeness: 100% (all 3 files created)
- Section completeness: 100% (all required sections present)
- Command documentation: 100% (all 6 CLI commands documented)
- Workflow examples: 3+ common workflows with step-by-step instructions

## Test Types

### Structural Validation

**Scope:** File existence and format correctness

**Tools:** File system checks, JSON validation

**What to Validate:**
- Directory structure matches specification
- plugin.json is valid JSON with required fields
- SKILL.md has valid YAML frontmatter
- All files use UTF-8 encoding

**Validation Commands:**
```bash
# Check directory structure
ls -la .crewchief/claude-code-plugins/plugins/worktree/
ls -la .crewchief/claude-code-plugins/plugins/worktree/.claude-plugin/
ls -la .crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/

# Validate JSON
cat .crewchief/claude-code-plugins/plugins/worktree/.claude-plugin/plugin.json | jq .

# Check YAML frontmatter (should not error)
head -20 .crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/SKILL.md
```

### Content Validation

**Scope:** Required content present and correct

**Tools:** Manual review, grep

**What to Validate:**
- plugin.json has name, version, description, author, repository, keywords
- README.md has all sections (Introduction, Features, Prerequisites, Installation, Usage Examples, Troubleshooting)
- SKILL.md frontmatter has name (lowercase, hyphens) and description (<1024 chars)
- SKILL.md body has lifecycle phases, safety section, CLI commands, workflows, error handling

**Validation Checks:**
```bash
# Check plugin.json fields
jq '.name, .version, .description, .author, .repository, .keywords' \
  .crewchief/claude-code-plugins/plugins/worktree/.claude-plugin/plugin.json

# Check README sections
grep -E "^#" .crewchief/claude-code-plugins/plugins/worktree/README.md

# Check SKILL.md sections
grep -E "^#" .crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/SKILL.md

# Check all 6 commands are documented
grep -c "crewchief worktree" .crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/SKILL.md
```

### Functional Validation

**Scope:** Plugin works in Claude Code

**Tools:** Claude Code CLI

**What to Validate:**
- Plugin can be installed
- Skill appears in skill list
- Worktree-related queries trigger skill activation
- CLI commands execute successfully

**Test Workflow:**
1. Install plugin: `/plugin install worktree@crewchief`
2. Verify installation: `/plugin list`
3. Test activation: Ask "How do I create a worktree?"
4. Verify CLI: Run `crewchief worktree list` command

## Critical Paths

The following paths MUST be validated:

### 1. Plugin Discovery and Installation
- **Happy path:** Plugin installs without errors
- **Error case:** Plugin directory missing or malformed plugin.json
- **Edge case:** Plugin already installed (should update or skip)

### 2. Skill Activation
- **Happy path:** Worktree-related query triggers worktree-management skill
- **Error case:** Skill description doesn't match query patterns
- **Edge case:** Multiple skills could match (worktree should win for worktree queries)

### 3. CLI Invocation
- **Happy path:** Commands execute and return expected output
- **Error case:** CLI not installed
- **Edge case:** Not in a git repository (should fail gracefully)

### 4. Worktree Lifecycle Documentation
- **Happy path:** User follows documented lifecycle, operations succeed
- **Error case:** User skips steps (e.g., tries to merge unmerged branch)
- **Edge case:** User tries to delete current worktree (CLI prevents this)

## Negative Testing Requirements

### Invalid/Malformed Content
- [ ] plugin.json with missing required fields (should fail validation)
- [ ] SKILL.md with invalid YAML (should fail parsing)
- [ ] Description longer than 1024 characters (should warn)

### Error Scenarios
- [ ] CLI not installed - README documents prerequisite
- [ ] Not in git repository - SKILL.md documents this requirement
- [ ] Current worktree deletion - safety section explains CLI prevention

### Missing Prerequisite Handling
- [ ] README documents prerequisites clearly
- [ ] SKILL.md includes prerequisite checks
- [ ] Error messages are actionable

## Test Data Strategy

No external test data required. Validation uses:
- The created files themselves
- Any git repository for CLI testing
- Sample worktree-related queries

**Sample Test Queries:**
- "How do I create a worktree?" (should trigger skill)
- "Create a worktree for feature-x" (should trigger skill)
- "Merge my worktree back to main" (should trigger skill)
- "Clean up old worktrees" (should trigger skill)
- "What is a git worktree?" (should trigger skill - conceptual question)
- "How do I work on multiple branches at once?" (should trigger skill - parallel development)
- "Parallel development setup" (should trigger skill - parallel keyword)
- "Isolated branch environment" (should trigger skill - isolation keyword)
- "Search for authentication code" (should NOT trigger skill - use maproom)

## Quality Gates

Before verification:

### Structural Gates
- [ ] All 3 files exist in correct locations
- [ ] plugin.json is valid JSON
- [ ] SKILL.md has valid YAML frontmatter
- [ ] Directory structure matches specification

### Content Gates
- [ ] plugin.json has all required fields (name, version, description, author, repository, keywords)
- [ ] plugin.json name is "worktree"
- [ ] plugin.json version is "0.1.0"
- [ ] README.md has all required sections
- [ ] SKILL.md name is "worktree-management" (lowercase, hyphens)
- [ ] SKILL.md description is under 1024 characters
- [ ] SKILL.md includes worktree lifecycle (5 phases)
- [ ] SKILL.md includes safety considerations section
- [ ] SKILL.md documents all 6 CLI commands:
  - crewchief worktree create
  - crewchief worktree list
  - crewchief worktree use
  - crewchief worktree clean
  - crewchief worktree merge
  - crewchief worktree copy-ignored
- [ ] SKILL.md includes 3+ common workflow examples
- [ ] SKILL.md includes error handling guidance

### Style Gates
- [ ] Instructions use imperative form (verb-first)
- [ ] No placeholder content remains
- [ ] CLI examples are copy-paste ready
- [ ] Consistent markdown formatting
- [ ] Safety warnings are prominent

### Functional Gates
- [ ] Plugin installs without errors
- [ ] Skill appears in skill list
- [ ] Skill description matches worktree query patterns

## Verification Checklist

Final verification before marking complete:

```markdown
## File Structure
- [ ] .crewchief/claude-code-plugins/plugins/worktree/.claude-plugin/plugin.json exists
- [ ] .crewchief/claude-code-plugins/plugins/worktree/README.md exists
- [ ] .crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/SKILL.md exists

## plugin.json
- [ ] name: "worktree"
- [ ] version: "0.1.0"
- [ ] description present
- [ ] author object with name, email, url
- [ ] repository URL
- [ ] keywords array with relevant terms

## README.md
- [ ] Introduction section
- [ ] Features section
- [ ] Prerequisites section (crewchief CLI, git repository)
- [ ] Installation section (with /plugin install command)
- [ ] Usage Examples section
- [ ] Troubleshooting section

## SKILL.md
- [ ] Frontmatter: name is "worktree-management"
- [ ] Frontmatter: description < 1024 chars, mentions worktree lifecycle
- [ ] Overview section
- [ ] Decision Tree section (when to use worktree-management vs other workflows)
- [ ] Worktree Lifecycle section (create -> use -> work -> merge -> clean)
- [ ] Safety Considerations section:
  - Cannot delete current worktree
  - Cannot merge from inside worktree
  - Unmerged branches require git branch -D
  - Check uncommitted changes before merge
- [ ] CLI Command Reference:
  - crewchief worktree create with all options
  - crewchief worktree list
  - crewchief worktree use with options
  - crewchief worktree clean with all options
  - crewchief worktree merge with all options
  - crewchief worktree copy-ignored with options
- [ ] Common Workflows:
  - Feature development workflow
  - Quick experiment workflow
  - At least one additional workflow
- [ ] Error Handling section
```
