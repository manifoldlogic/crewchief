# Task Index: PLUGIN-001 Maproom Plugin

## Overview
This index tracks all tasks for the PLUGIN-001 (Maproom Plugin) ticket. Tasks are organized by phase and follow the execution plan defined in `planning/plan.md`.

## Task Summary

**Total Tasks**: 3
- **Phase 1 (Foundation)**: 1 task
- **Phase 2 (Skill Implementation)**: 2 tasks

**Estimated Total Effort**: 3-5 hours

## Phase 1: Plugin Foundation

Objective: Create the plugin directory structure with required metadata files.

| Task ID | Title | Effort | Status |
|---------|-------|--------|--------|
| PLUGIN-001.1001 | Create Plugin Directory Structure | 1-2 hours | Not Started |

**Phase 1 Deliverables**:
- plugin.json - Plugin metadata
- README.md - User documentation
- Directory structure for skills and references

**Phase 1 Dependencies**: None (foundation phase)

## Phase 2: Skill Implementation

Objective: Create the maproom-search skill with comprehensive documentation.

| Task ID | Title | Effort | Status |
|---------|-------|--------|--------|
| PLUGIN-001.2001 | Create maproom-search Skill Documentation | 1-2 hours | Not Started |
| PLUGIN-001.2002 | Create Search Best Practices Reference | 1-2 hours | Not Started |

**Phase 2 Deliverables**:
- SKILL.md - Core skill documentation
- search-best-practices.md - Detailed examples and patterns

**Phase 2 Dependencies**:
- PLUGIN-001.1001 (directory structure required)
- External: crewchief-maproom CLI documentation

## Task Details

### PLUGIN-001.1001: Create Plugin Directory Structure
**File**: `PLUGIN-001.1001_create-plugin-structure.md`

**Summary**: Create the maproom plugin directory structure with required metadata files (plugin.json and README.md).

**Key Deliverables**:
- Directory structure matching specification
- Valid plugin.json with required fields
- Complete README.md with all sections

**Acceptance Criteria Highlights**:
- plugin.json validates with `jq .`
- README.md has 6 sections (Introduction, Features, Prerequisites, Installation, Usage, Troubleshooting)
- No placeholder content

**Agent**: general-implementation

---

### PLUGIN-001.2001: Create maproom-search Skill Documentation
**File**: `PLUGIN-001.2001_create-skill-documentation.md`

**Summary**: Create the maproom-search skill documentation (SKILL.md) with YAML frontmatter, decision tree, query formulation patterns, and CLI command reference.

**Key Deliverables**:
- SKILL.md with valid YAML frontmatter
- Decision tree for tool selection
- Query formulation guidance
- CLI command reference
- SearchMode awareness section

**Acceptance Criteria Highlights**:
- Frontmatter description <1024 characters
- Decision tree covers maproom/grep/glob selection
- Command reference includes search, vector-search, status, context
- NO `--mode` flags in examples
- SearchMode auto-detection explained

**Agent**: general-implementation

**Dependencies**: PLUGIN-001.1001

---

### PLUGIN-001.2002: Create Search Best Practices Reference
**File**: `PLUGIN-001.2002_create-search-best-practices.md`

**Summary**: Create the search-best-practices.md reference document with 10+ query transformation examples, search strategy patterns, and anti-patterns.

**Key Deliverables**:
- Query transformation examples table (10+ entries)
- Search strategy patterns by task type
- Anti-patterns section
- SearchMode detection patterns

**Acceptance Criteria Highlights**:
- 10+ query transformation examples
- Examples show SearchMode detection (Code/Text/Auto)
- Strategy patterns for 3+ task types
- 5+ anti-patterns documented

**Agent**: general-implementation

**Dependencies**: PLUGIN-001.1001, PLUGIN-001.2001

## Execution Order

### Sequential Execution (Recommended)
1. **PLUGIN-001.1001** (Foundation - creates structure)
2. **PLUGIN-001.2001** (Core skill - references structure)
3. **PLUGIN-001.2002** (References - referenced by skill)

### Parallel Opportunities
After PLUGIN-001.1001 completes, tasks 2001 and 2002 can be executed in parallel as they create independent files. However, 2001 references 2002, so completing 2001 first ensures accurate references.

## Quality Gates

### Phase 1 Completion Criteria
- [ ] All directories exist at specified paths
- [ ] plugin.json is valid JSON with all required fields
- [ ] README.md has all 6 required sections
- [ ] No placeholder content in any Phase 1 file

### Phase 2 Completion Criteria
- [ ] SKILL.md has valid YAML frontmatter
- [ ] SKILL.md description is under 1024 characters
- [ ] Decision tree clearly differentiates maproom/grep/glob
- [ ] CLI commands match verified syntax (no `--mode` flags)
- [ ] SearchMode auto-detection explained (not manual override)
- [ ] search-best-practices.md has 10+ examples
- [ ] All query examples are 2-3 words
- [ ] No placeholder content in any Phase 2 file

### Final Validation
- [ ] Plugin installs: `/plugin install maproom@crewchief`
- [ ] Skill appears in skill list
- [ ] Conceptual query triggers skill activation
- [ ] CLI commands execute successfully

## Risk Tracking

| Risk | Impact | Mitigation | Status |
|------|--------|------------|--------|
| CLI commands change | Medium | Reference CLAUDE.md as source of truth | Monitoring |
| Plugin schema changes | High | Follow existing plugin patterns | Monitoring |
| Skill doesn't activate | High | Test description with various queries | Monitoring |
| Description too long | Low | Keep under 1024 chars | Monitoring |

## References

- **Planning**: `planning/plan.md`
- **Architecture**: `planning/architecture.md`
- **Quality Strategy**: `planning/quality-strategy.md`
- **Task Template**: `/home/vscode/.crewchief/worktrees/CC-PLUGIN/.sdd/reference/work-task-template.md`
- **CLI Documentation**: `crates/maproom/CLAUDE.md` (source of truth for commands)

## Notes

**SearchMode Philosophy**: The skill teaches query formulation and command selection (search vs vector-search), NOT mode overrides. SearchMode auto-detection (Code/Text/Auto) is an intelligent system feature that should be explained but not overridden.

**Command Syntax**: CLI uses separate commands (`search` and `vector-search`), not `--mode` flags. The daemon interface has a mode parameter, but CLI does not.

**Progressive Disclosure**: SKILL.md contains essential guidance (~2-3k tokens), search-best-practices.md contains detailed examples (loaded on demand).

**Testing**: All tasks are documentation-focused. "Tests pass" is N/A, validation uses structural checks and manual review per quality-strategy.md.
