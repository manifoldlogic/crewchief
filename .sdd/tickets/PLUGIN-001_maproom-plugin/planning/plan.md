# Plan: Maproom Plugin

## Overview

This document outlines the execution plan for creating the maproom plugin. The work is organized into two phases: Foundation (plugin structure and metadata) and Content (skill and reference documentation). Each phase produces testable deliverables.

## Phases

### Phase 1: Plugin Foundation

**Objective:** Create the plugin directory structure with required metadata files.

**Deliverables:**
- Plugin directory at `.crewchief/claude-code-plugins/plugins/maproom/`
- `.claude-plugin/plugin.json` with name, version, description, author, repository, keywords
- `README.md` with installation instructions, features, prerequisites, usage examples

**Agent Assignments:**
- General implementation agent: Create directory structure and write metadata files

**Acceptance Criteria:**
- [ ] Directory structure matches specification
- [ ] plugin.json validates against schema (name, version, description required)
- [ ] README.md includes all sections: introduction, features, prerequisites, installation, usage, troubleshooting
- [ ] No placeholder content remains

**Estimated Effort:** 1-2 hours

### Phase 2: Skill Implementation

**Objective:** Create the maproom-search skill with comprehensive documentation.

**Deliverables:**
- `skills/maproom-search/SKILL.md` with:
  - Valid YAML frontmatter (name: maproom-search, description <1024 chars)
  - Decision tree: when to use maproom vs grep/glob
  - Query formulation patterns with examples (2-3 words, concepts)
  - Command selection guidance (search vs vector-search)
  - CLI command reference (search, vector-search, status, context)
  - SearchMode awareness (Code/Text/Auto auto-detection)
  - Error handling guidance
- `skills/maproom-search/references/search-best-practices.md` with:
  - 10+ query transformation examples
  - Search strategy patterns by task type
  - Examples showing SearchMode detection patterns
  - Anti-patterns to avoid

**Agent Assignments:**
- General implementation agent: Write skill documentation and references

**Acceptance Criteria:**
- [ ] SKILL.md frontmatter has valid name (lowercase, hyphens, max 64 chars)
- [ ] SKILL.md description clearly states when to trigger skill (<1024 chars)
- [ ] Decision tree covers maproom, grep, and glob selection criteria
- [ ] Query formulation section includes transformation examples (2-3 words, concepts)
- [ ] Command selection guidance explains when to use search vs vector-search
- [ ] CLI commands include search, vector-search, status, and context with correct syntax
- [ ] SearchMode awareness section explains Code/Text/Auto detection
- [ ] NO --mode flags in command examples (use separate commands instead)
- [ ] Error handling covers: no results, database not indexed, embeddings missing
- [ ] References file has 10+ query examples with SearchMode detection patterns
- [ ] All instructions use imperative form (verb-first)

**Estimated Effort:** 2-3 hours

## Dependencies

### External Dependencies

| Dependency | Description | Mitigation |
|------------|-------------|------------|
| crewchief-maproom CLI | Must be installed for testing | Document as prerequisite |
| Indexed database | Required for search to work | Include status check workflow |
| Git submodule | Marketplace is a submodule | Ensure submodule is initialized |

### Internal Dependencies

| Phase | Depends On |
|-------|------------|
| Phase 2 | Phase 1 (directory structure must exist) |

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| CLI commands change | Low | Medium | Link to CLAUDE.md as authoritative source |
| Plugin schema changes | Low | High | Follow existing plugin patterns exactly |
| Description too long | Low | Low | Keep under 1024 chars, focus on trigger conditions |
| Skill doesn't activate | Medium | High | Test description with various conceptual queries |

## Success Metrics

### Completion Criteria

- [ ] All files created in correct locations
- [ ] plugin.json validates
- [ ] README.md is complete
- [ ] SKILL.md frontmatter is valid
- [ ] Decision tree is actionable
- [ ] CLI commands are correct
- [ ] 10+ query examples in references

### Quality Criteria

- [ ] Instructions use imperative form throughout
- [ ] Examples are copy-paste ready
- [ ] No placeholder content
- [ ] Consistent formatting

### Testing Checklist

- [ ] Plugin can be installed: `/plugin install maproom@crewchief`
- [ ] Skill appears in installed skills
- [ ] Conceptual query triggers skill activation
- [ ] CLI commands work when executed
- [ ] Search returns results for indexed repo

## File Manifest

Files to be created:

```
.crewchief/claude-code-plugins/plugins/maproom/
├── .claude-plugin/
│   └── plugin.json                              # 20-30 lines
├── README.md                                    # 80-120 lines
└── skills/
    └── maproom-search/
        ├── SKILL.md                             # 150-250 lines
        └── references/
            └── search-best-practices.md         # 100-150 lines
```

Total new files: 4
Total lines: ~350-550

## Implementation Notes

### plugin.json Template

```json
{
  "name": "maproom",
  "version": "0.1.0",
  "description": "Semantic code search using the crewchief-maproom CLI. Find code by concept, understand architecture, and explore relationships between code elements.",
  "author": {
    "name": "Daniel Bushman",
    "email": "dbushman@manifoldlogic.com",
    "url": "https://github.com/manifoldlogic/claude-code-plugins"
  },
  "repository": "https://github.com/manifoldlogic/claude-code-plugins",
  "keywords": [
    "maproom",
    "semantic-search",
    "code-search",
    "fts",
    "vector-search"
  ]
}
```

### SKILL.md Frontmatter Template

```yaml
---
name: maproom-search
description: This skill should be used for semantic code search when exploring unfamiliar codebases, finding implementations by concept (e.g., "authentication", "error handling"), or understanding code architecture. Uses the crewchief-maproom CLI for FTS and vector search. Prefer native Grep for exact text matches and Glob for file patterns.
---
```

### Key CLI Commands to Document

```bash
# Check status (discover repo name, embedding availability)
crewchief-maproom status --repo <repo>

# FTS search (keyword matching, always works)
crewchief-maproom search --query "authentication" --repo <repo>

# Vector search (semantic similarity, requires embeddings)
crewchief-maproom vector-search --query "authentication" --repo <repo>

# Context expansion (after finding chunk_id)
crewchief-maproom context --chunk-id <id> --callers --callees --json
```

**Note:** CLI uses separate commands, NOT `--mode` flags. The daemon interface has a `mode` parameter, but CLI does not.

## Post-Completion Steps

After implementation:

1. Run verification: Check all files exist and validate
2. Test installation: `/plugin install maproom@crewchief`
3. Test activation: Try conceptual queries to verify skill triggers
4. Update marketplace.json (handled by PLUGIN-003 ticket)
