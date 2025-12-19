# Quality Strategy: Maproom Plugin

## Testing Philosophy

This ticket creates a Claude Code plugin consisting of documentation files (markdown and JSON). Testing focuses on:

1. **Structural validation** - Files exist in correct locations with correct formats
2. **Content validation** - All required sections present, no placeholders
3. **Functional validation** - Plugin installs correctly, skill activates appropriately

Unlike code-centric tickets, there is no unit test coverage to measure. Quality is assessed through validation checklists and manual testing.

## Coverage Requirements

**Minimum Thresholds:**
- File completeness: 100% (all 4 files created)
- Section completeness: 100% (all required sections present)
- Example count: 10+ query transformation examples in references

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
ls -la .crewchief/claude-code-plugins/plugins/maproom/
ls -la .crewchief/claude-code-plugins/plugins/maproom/.claude-plugin/
ls -la .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/
ls -la .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/

# Validate JSON
cat .crewchief/claude-code-plugins/plugins/maproom/.claude-plugin/plugin.json | jq .

# Check YAML frontmatter (should not error)
head -20 .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md
```

### Content Validation

**Scope:** Required content present and correct

**Tools:** Manual review, grep

**What to Validate:**
- plugin.json has name, version, description, author, repository, keywords
- README.md has all sections (introduction, features, prerequisites, installation, usage, troubleshooting)
- SKILL.md frontmatter has name (lowercase, hyphens) and description (<1024 chars)
- SKILL.md body has decision tree, query patterns, CLI commands, error handling
- search-best-practices.md has 10+ examples

**Validation Checks:**
```bash
# Check plugin.json fields
jq '.name, .version, .description, .author, .repository, .keywords' \
  .crewchief/claude-code-plugins/plugins/maproom/.claude-plugin/plugin.json

# Check README sections
grep -E "^#" .crewchief/claude-code-plugins/plugins/maproom/README.md

# Count examples in references
grep -c "^|" .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/search-best-practices.md

# Check description length
head -10 .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md
```

### Functional Validation

**Scope:** Plugin works in Claude Code

**Tools:** Claude Code CLI

**What to Validate:**
- Plugin can be installed
- Skill appears in skill list
- Conceptual queries trigger skill activation
- CLI commands execute successfully

**Test Workflow:**
1. Install plugin: `/plugin install maproom@crewchief`
2. Verify installation: `/plugin list`
3. Test activation: Ask "How does authentication work in this codebase?"
4. Verify CLI: Run `crewchief-maproom status` command

## Critical Paths

The following paths MUST be validated:

### 1. Plugin Discovery and Installation
- **Happy path:** Plugin installs without errors
- **Error case:** Plugin directory missing or malformed plugin.json
- **Edge case:** Plugin already installed (should update or skip)

### 2. Skill Activation
- **Happy path:** Conceptual query triggers maproom-search skill
- **Error case:** Skill description doesn't match query patterns
- **Edge case:** Multiple skills could match (maproom should win for conceptual queries)

### 3. CLI Invocation
- **Happy path:** Commands execute and return JSON results
- **Error case:** CLI not installed, database not indexed
- **Edge case:** Repository name unknown (status command helps)

## Negative Testing Requirements

### Invalid/Malformed Content
- [ ] plugin.json with missing required fields (should fail validation)
- [ ] SKILL.md with invalid YAML (should fail parsing)
- [ ] Description longer than 1024 characters (should warn)

### Error Scenarios
- [ ] CLI not installed - skill should document this prerequisite
- [ ] Database not indexed - error handling section should address this
- [ ] Repository not found - status command workflow should help

### Missing Prerequisite Handling
- [ ] README documents prerequisites clearly
- [ ] SKILL.md includes status check workflow
- [ ] Error messages are actionable

## Test Data Strategy

No external test data required. Validation uses:
- The created files themselves
- An indexed repository (any repo with maproom database)
- Sample conceptual queries

**Sample Test Queries:**
- "How does authentication work?" (should trigger skill, natural language query)
- "Find the error handling logic" (should trigger skill, conceptual search)
- "User::authenticate" (should trigger skill, code identifier search)
- "Search for TODO comments" (should NOT trigger skill - use Grep)
- "Find all .ts files" (should NOT trigger skill - use Glob)

**SearchMode Detection Examples:**
- "authentication" → Code mode (single word)
- "user authentication" → Auto mode (2 words)
- "how to authenticate users" → Text mode (natural language)
- "UserAuth::login()" → Code mode (code patterns)

## Quality Gates

Before verification:

### Structural Gates
- [ ] All 4 files exist in correct locations
- [ ] plugin.json is valid JSON
- [ ] SKILL.md has valid YAML frontmatter
- [ ] Directory structure matches specification

### Content Gates
- [ ] plugin.json has all required fields
- [ ] README.md has all required sections
- [ ] SKILL.md name is lowercase with hyphens
- [ ] SKILL.md description is under 1024 characters
- [ ] SKILL.md includes decision tree (maproom vs grep vs glob)
- [ ] SKILL.md includes command selection guidance (search vs vector-search)
- [ ] SKILL.md includes CLI command reference (search, vector-search, status, context)
- [ ] SKILL.md includes SearchMode awareness section
- [ ] SKILL.md does NOT default to one mode or add --mode flags
- [ ] SKILL.md includes error handling
- [ ] search-best-practices.md has 10+ examples with SearchMode patterns

### Style Gates
- [ ] Instructions use imperative form (verb-first)
- [ ] No placeholder content remains
- [ ] CLI examples are copy-paste ready
- [ ] Consistent markdown formatting

### Functional Gates
- [ ] Plugin installs without errors
- [ ] Skill appears in skill list
- [ ] Skill description matches conceptual query patterns

## Verification Checklist

Final verification before marking complete:

```markdown
## File Structure
- [ ] .crewchief/claude-code-plugins/plugins/maproom/.claude-plugin/plugin.json exists
- [ ] .crewchief/claude-code-plugins/plugins/maproom/README.md exists
- [ ] .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md exists
- [ ] .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/search-best-practices.md exists

## plugin.json
- [ ] name: "maproom"
- [ ] version: "0.1.0"
- [ ] description present
- [ ] author object with name, email, url
- [ ] repository URL
- [ ] keywords array with relevant terms

## README.md
- [ ] Introduction section
- [ ] Features section
- [ ] Prerequisites section (CLI installed, database indexed)
- [ ] Installation section (with /plugin install command)
- [ ] Usage examples section
- [ ] Troubleshooting section

## SKILL.md
- [ ] Frontmatter: name is "maproom-search"
- [ ] Frontmatter: description < 1024 chars, mentions semantic search
- [ ] Decision tree: when maproom vs grep vs glob
- [ ] Query formulation patterns with examples (2-3 words, concepts)
- [ ] Command selection guidance: search vs vector-search
- [ ] SearchMode awareness: Code/Text/Auto auto-detection explained
- [ ] CLI commands: search, vector-search, status, context (NO --mode flags)
- [ ] Error handling: no results, database not indexed, embeddings missing
- [ ] Reference to search-best-practices.md

## search-best-practices.md
- [ ] 10+ query transformation examples
- [ ] Strategy patterns by task type
- [ ] Anti-patterns to avoid
- [ ] Examples are concrete and actionable
```
