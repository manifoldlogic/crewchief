# Research Synthesis: CrewChief CLI Plugins

## Key Findings

### Finding 1: Plugin Structure is Well-Documented

The crewchief marketplace at `.crewchief/claude-code-plugins/` contains several existing plugins that provide clear patterns to follow:

**Existing Plugins:**
- `workstream` - Project workflow management (0.3.0)
- `github-actions` - CI/CD workflow management (0.1.0)
- `claude-code-dev` - Development tools for Claude Code (0.1.0)
- `sdd` - Spec Driven Development (0.1.0)

**Common Structure:**
```
plugin-name/
├── .claude-plugin/
│   └── plugin.json     # Required metadata
├── README.md           # Required documentation
├── agents/             # Optional agent definitions
├── commands/           # Optional slash commands
├── skills/             # Optional skill directories
│   └── skill-name/
│       ├── SKILL.md    # Skill definition
│       ├── scripts/    # Executable code
│       └── references/ # On-demand docs
└── hooks/              # Optional event handlers
```

**Implication:** New plugins should follow this exact structure. The marketplace registration in `marketplace.json` is straightforward.

### Finding 2: MCP and CLI Have Full Parity

The maproom-mcp server and crewchief-maproom CLI provide equivalent functionality. All MCP tools (search, open, context, status) have CLI counterparts. The CLI additionally provides database management commands (migrate, cleanup-stale, clean-ignored) that MCP does not expose.

**Implication:** Plugins can use CLI directly without needing MCP. This simplifies the skill design and avoids daemon management complexity.

### Finding 3: Query Formulation is Critical

The MCP search tool description contains extensive query formulation guidance:

**Transformation Patterns:**
1. Extract 2-3 core technical terms
2. Remove: how, what, where, when, why, does, is, are, the, a, an
3. Prefer code-like terminology

**Examples:**
- "How does authentication work?" -> "authentication"
- "What handles errors?" -> "error handler"
- "Where is WebSocket disconnect?" -> "WebSocket disconnect"

**Best Practices:**
- 2-3 words work best
- Use concepts, not sentences
- Avoid special characters
- Try variations if <3 results

**Implication:** The maproom skill must include this guidance to help Claude formulate effective queries.

### Finding 4: Search Mode Selection Matters

Three search modes exist with different strengths:

| Mode | Best For | Requires |
|------|----------|----------|
| fts | Exact keyword matches, identifiers | Nothing extra |
| vector | Conceptual queries, similar code | Embeddings generated |
| hybrid | Combined ranking, comprehensive | Embeddings generated |

**Implication:** Skill should default to `fts` for reliability, recommend `hybrid` when embeddings are available.

### Finding 5: Worktree Commands Have Rich Options

The crewchief worktree CLI provides:

| Command | Purpose | Key Options |
|---------|---------|-------------|
| create | New worktree | --branch, --shell, --no-copy-ignored |
| list | Show worktrees | (none) |
| use | Switch to worktree | --shell |
| clean | Remove worktree | --stale, --all, --keep-branch |
| merge | Merge and cleanup | --strategy, --no-delete |
| copy-ignored | Copy ignored files | --dry-run |

**Implication:** Skill should document common workflows, not just commands.

### Finding 6: Claude Code Skills Use SKILL.md Format

Skills consist of:
1. **SKILL.md** (required) - YAML frontmatter + markdown instructions
2. **scripts/** (optional) - Executable code
3. **references/** (optional) - On-demand documentation
4. **assets/** (optional) - Output resources

Key requirements:
- `name`: lowercase, hyphens, max 64 chars
- `description`: what it does + when to use it, max 1024 chars
- Model-invoked (Claude decides when to use based on description)

**Implication:** Description quality is critical for skill discovery.

### Finding 7: Tool Selection Decision Tree

Based on codebase analysis, here's when to use each tool:

**Use Maproom Search when:**
- Finding implementations by concept ("authentication logic", "error handling")
- Understanding architecture ("main entry point", "database connections")
- Discovering related code (callers, callees, tests)
- Exploring unfamiliar codebases
- Query is 1-4 conceptual words

**Use Grep when:**
- Exact text matching ("TODO", "FIXME", specific error messages)
- Finding all usages of a known identifier
- Searching for regex patterns
- Query contains special characters or exact strings

**Use Glob when:**
- Finding files by name pattern ("*.test.ts", "*.config.js")
- Locating specific file types
- Path-based file discovery

**Implication:** Skill should include this decision tree for Claude to reference.

### Finding 8: Plugin Installation Pattern

From the marketplace README:

```bash
# Add marketplace (if not already added)
/plugin marketplace add manifoldlogic/claude-code-plugins

# Install individual plugins
/plugin install maproom@crewchief
/plugin install worktree@crewchief
```

The marketplace is already configured in `.claude/settings.json` for this repository via submodule.

**Implication:** Only marketplace.json registration and plugin creation are needed.

### Finding 9: Reference Document Pattern

The `github-actions` plugin's `gh-cli` skill provides a good pattern:
- Concise SKILL.md with essential guidance
- Authentication check documented prominently
- Commands organized by category
- Error handling table with solutions

**Implication:** Follow this pattern for CLI command documentation.

## Open Questions

### Question 1: Skill Location Within Plugin (Resolved)

Where should skills be located?
- Pattern from existing plugins: `plugins/{plugin-name}/skills/{skill-name}/SKILL.md`

**Decision:** Use `plugins/maproom/skills/maproom-search/SKILL.md` and `plugins/worktree/skills/worktree-management/SKILL.md`.

### Question 2: Reference File Strategy (Resolved)

Should CLI documentation be:
- Embedded in SKILL.md (always in context)
- In references/ (loaded on demand)
- Linked to existing CLAUDE.md files

**Decision:** Embed decision tree and query formulation in SKILL.md, put detailed examples in references/, link to CLAUDE.md for CLI docs (single source of truth).

### Question 3: Error Handling Patterns (Resolved)

What should Claude do when:
- Search returns no results -> Try broader query, check status
- Database not indexed -> Run scan command
- Worktree already exists -> Use instead of create

**Decision:** Include error handling workflows in skills.

## Assumptions

1. **Database is pre-indexed** - Plugins assume maproom database exists with indexed content. Users must run `crewchief-maproom scan` first.

2. **CLI is installed** - Both `crewchief-maproom` and `crewchief` CLIs are available in PATH.

3. **Git repository context** - Worktree commands require being in a git repository.

4. **FTS mode is always available** - Vector/hybrid modes require embeddings; FTS does not.

5. **Users understand worktree concept** - Skill provides workflow guidance, not worktree education.

6. **Plugins complement, not replace** - Native tools (Grep, Glob, Read) remain the right choice for many tasks.

7. **Marketplace is accessible** - Users have the crewchief marketplace configured or can add it.
