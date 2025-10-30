# Project Naming Guidelines

This document defines the naming conventions for projects in the `.agents/` directory structure.

## Format

```
{CODE}_{descriptive-name}
```

**Components:**
- **CODE**: UPPERCASE project identifier (4-8 characters)
- **Separator**: Single underscore `_`
- **descriptive-name**: lowercase-with-dashes description

## Requirements

### CODE Component
- **Length**: 4-8 characters
- **Case**: UPPERCASE only
- **Purpose**: Matches ticket prefix for easy association
- **Uniqueness**: Must be unique across all projects (active and archived)
- **Clarity**: Should hint at project area when possible

**Good Examples:**
- `DKRHUB` - Docker Hub
- `LOCAL` - Local deployment
- `MCPSTART` - MCP startup
- `MPEMBED` - Multi-provider embeddings

**Bad Examples:**
- `DHP` - Too cryptic
- `DOCKERHUBPUBLISHING` - Too long
- `docker` - Wrong case
- `DKR_HUB` - No underscores in code

### descriptive-name Component
- **Case**: lowercase only
- **Separator**: Hyphens `-` between words
- **Length**: 2-5 words typically
- **Clarity**: Should be immediately understandable
- **Specificity**: Specific enough to distinguish from similar projects

**Good Examples:**
- `docker-hub-publishing`
- `local-deployment`
- `mcp-provider-startup-fix`
- `hybrid-retrieval-system`

**Bad Examples:**
- `dockerhubpublishing` - No separators
- `Docker-Hub-Publishing` - Wrong case
- `docker_hub_publishing` - Wrong separator (underscores)
- `stuff` - Too vague
- `a-really-long-description-with-many-unnecessary-words` - Too long

## Benefits of This Format

### 1. Self-Documenting Paths
```bash
# Clear what this is without opening it
.agents/projects/DKRHUB_docker-hub-publishing/

# vs unclear short code
.agents/projects/DKRHUB/
```

### 2. Better Searchability
```bash
# Find all Docker-related projects
ls -d .agents/projects/*docker* .agents/archive/projects/*docker*

# Find embedding-related projects
ls -d .agents/**/projects/*embed*
```

### 3. AI Agent Comprehension
AI agents can immediately understand project purpose from the path alone, without needing to read README files.

### 4. Ticket Association
Tickets still use the short code:
- Folder: `DKRHUB_docker-hub-publishing/`
- Tickets: `DKRHUB-001_setup.md`, `DKRHUB-002_build.md`

## Examples by Category

### Infrastructure
- `DKRHUB_docker-hub-publishing`
- `LOCAL_local-deployment`
- `CICD_continuous-integration`

### Features
- `HYBRID_SEARCH_hybrid-retrieval-system`
- `MPEMBED_multi-provider-embeddings`
- `CONTEXT_ASM_context-assembly-engine`

### Improvements
- `PERF_OPT_performance-optimization`
- `CODE_QUALITY_code-quality-improvements`
- `SEC_AUDIT_security-audit`

### Bug Fixes
- `MCPSTART_mcp-provider-startup-fix`
- `MAPROOM_misc-fixes`
- `AUTH_FIX_authentication-bug-fix`

### Language Support
- `LANG_PARSE_multi-language-support`
- `MD_ENHANCE_markdown-enhancement`
- `PY_PARSER_python-parser-integration`

## Creating a New Project

### Step 1: Choose a CODE

1. Review existing codes in `.agents/projects/` and `.agents/archive/projects/`
2. Choose a unique, memorable code (4-8 chars)
3. Use abbreviations that make sense in your domain

### Step 2: Write descriptive-name

1. Think: "What is this project doing?"
2. Use 2-5 words
3. Use lowercase and hyphens
4. Be specific and clear

### Step 3: Verify Format

Check your name against these rules:
```bash
# Pattern to match
^[A-Z]{4,8}_[a-z][a-z0-9-]*[a-z0-9]$

# Valid examples
DKRHUB_docker-hub-publishing  ✓
LOCAL_local-deployment        ✓
MPEMBED_multi-provider-embed  ✓

# Invalid examples
dkrhub_docker-hub             ✗ (lowercase code)
DKRHUB-docker-hub             ✗ (wrong separator)
DKRHUB_Docker-Hub             ✗ (uppercase in description)
DKR_docker-hub-publishing     ✗ (code too short)
DKRHUBPUB_docker-hub          ✗ (code too long)
```

### Step 4: Create Structure

```bash
mkdir -p .agents/projects/{CODE}_{descriptive-name}/{planning,tickets}
```

### Step 5: Update Documentation

Add your project to:
- `.agents/projects/README.md` (if active)
- `.agents/archive/README.md` (when archived)

## Renaming Existing Projects

If you need to rename an existing project:

1. **Rename folder:**
   ```bash
   mv .agents/projects/{OLD} .agents/projects/{CODE}_{descriptive-name}
   ```

2. **Update references in:**
   - `.agents/README.md`
   - `.agents/projects/README.md` or `.agents/archive/README.md`
   - Any documentation that links to the project

3. **Do NOT rename tickets** - they keep their original `{CODE}-NNN` format

## Anti-Patterns to Avoid

### ❌ Generic Names
- `PROJECT_stuff`
- `WORK_things`
- `MISC_various`

Use specific, descriptive names instead.

### ❌ Redundant Words
- `PROJECT_project-name` (redundant "project")
- `FEATURE_feature-name` (redundant "feature")

The structure already indicates it's a project.

### ❌ Implementation Details
- `DOCKER_using-compose-and-swarm` (too specific)
- `EMBED_openai-and-ollama-providers` (might change)

Focus on the goal, not the implementation.

### ❌ Version Numbers
- `SEARCH_v2-hybrid-search` (versions should be in git)
- `API_new-api-design` (avoid "new", "old")

Projects should describe their purpose, not their iteration.

## FAQ

**Q: What if my CODE is already taken?**
A: Choose a different code. Add numbers if needed: `MCP2`, `AUTH2`, or use a more specific abbreviation.

**Q: Can I use numbers in the CODE?**
A: Yes, but prefer letters. Numbers are useful for disambiguation: `LOCAL2`, `MCP3`.

**Q: Can I use numbers in descriptive-name?**
A: Yes, if meaningful: `http2-support`, `oauth2-integration`.

**Q: Should I include the parent project name?**
A: Only if it adds clarity: `MAPROOM_search-optimization` is better than `SEARCH_optimization` if there are multiple search systems.

**Q: How specific should descriptive-name be?**
A: Specific enough to distinguish from similar projects, but general enough to encompass the full scope.

**Q: What if the project scope changes?**
A: Rename the folder and update documentation. Better to have accurate names than historical ones.

## Enforcement

These guidelines should be followed for:
- ✅ All new projects
- ✅ Projects being moved to archive
- ⚠️ Existing projects (rename during maintenance)

## Related Documents

- [Work Ticket Template](./work-ticket-template.md) - Ticket naming follows the CODE
- [Spec-Driven Development](./spec-driven-development.md) - Process from vision to tickets
- [.agents README](../README.md) - Overall directory structure
