# Task Index: PLUGIN-002 Worktree Plugin

## Overview
This index tracks all tasks for the PLUGIN-002 (Worktree Plugin) ticket. Tasks are organized by phase and follow the execution plan defined in `planning/plan.md`.

## Task Summary

**Total Tasks**: 2
- **Phase 1 (Foundation)**: 1 task
- **Phase 2 (Skill Documentation)**: 1 task

**Estimated Total Effort**: 3-5 hours

## Phase 1: Plugin Foundation

Objective: Create the plugin directory structure with required metadata files.

| Task ID | Title | Effort | Status |
|---------|-------|--------|--------|
| PLUGIN-002.1001 | Create Plugin Directory Structure | 1-2 hours | Not Started |

**Phase 1 Deliverables**:
- plugin.json - Plugin metadata
- README.md - User documentation
- Directory structure for skills

**Phase 1 Dependencies**: None (foundation phase)

## Phase 2: Skill Documentation

Objective: Create the worktree-management skill with comprehensive lifecycle documentation.

| Task ID | Title | Effort | Status |
|---------|-------|--------|--------|
| PLUGIN-002.2001 | Create Worktree Management Skill Documentation | 2-3 hours | Not Started |

**Phase 2 Deliverables**:
- SKILL.md - Complete skill documentation with lifecycle, safety, commands, workflows

**Phase 2 Dependencies**:
- PLUGIN-002.1001 (directory structure required)
- External: crewchief CLI documentation

## Task Details

### PLUGIN-002.1001: Create Plugin Directory Structure
**File**: `PLUGIN-002.1001_create-plugin-structure.md`

**Summary**: Create the worktree plugin directory structure with required metadata files (plugin.json and README.md).

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

### PLUGIN-002.2001: Create Worktree Management Skill Documentation
**File**: `PLUGIN-002.2001_create-skill-documentation.md`

**Summary**: Create the worktree-management skill documentation (SKILL.md) with YAML frontmatter, worktree lifecycle phases, safety considerations, CLI command reference, and common workflow examples.

**Key Deliverables**:
- SKILL.md with valid YAML frontmatter
- Worktree lifecycle (5 phases)
- Safety considerations section
- CLI command reference (all 6 commands)
- Common workflows (3+ examples)
- Error handling guidance

**Acceptance Criteria Highlights**:
- Frontmatter description <1024 characters
- All 5 lifecycle phases documented (create -> use -> work -> merge -> clean)
- Safety section covers 4 key considerations
- All 6 commands documented with correct syntax
- Instructions use imperative form

**Agent**: general-implementation

**Dependencies**: PLUGIN-002.1001

---

## Execution Order

### Sequential Execution (Recommended)
1. **PLUGIN-002.1001** (Foundation - creates structure)
2. **PLUGIN-002.2001** (Skill content - uses structure)

### Parallel Opportunities
None - Phase 2 depends on Phase 1 directory structure.

## Quality Gates

### Phase 1 Completion Criteria
- [ ] All directories exist at specified paths
- [ ] plugin.json is valid JSON with all required fields
- [ ] README.md has all 6 required sections
- [ ] No placeholder content in any Phase 1 file

### Phase 2 Completion Criteria
- [ ] SKILL.md has valid YAML frontmatter
- [ ] SKILL.md description is under 1024 characters
- [ ] All 5 worktree lifecycle phases documented
- [ ] Safety section covers 4 key considerations
- [ ] All 6 CLI commands documented with correct syntax
- [ ] At least 3 common workflows documented
- [ ] Instructions use imperative form
- [ ] Error handling guidance included
- [ ] No placeholder content in any Phase 2 file

### Final Validation
- [ ] Plugin installs: `/plugin install worktree@crewchief`
- [ ] Skill appears in skill list
- [ ] Worktree-related queries trigger skill activation
- [ ] CLI commands execute successfully

## Risk Tracking

| Risk | Impact | Mitigation | Status |
|------|--------|------------|--------|
| CLI commands change | Medium | Link to CLI source as authoritative reference | Monitoring |
| Plugin schema changes | High | Follow existing plugin patterns (maproom) | Monitoring |
| Skill doesn't activate | High | Test description with various queries | Monitoring |
| Description too long | Low | Keep under 1024 chars | Monitoring |

## References

- **Planning**: `planning/plan.md`
- **Architecture**: `planning/architecture.md`
- **Quality Strategy**: `planning/quality-strategy.md`
- **Task Template**: `/home/vscode/.crewchief/worktrees/CC-PLUGIN/.sdd/reference/work-task-template.md`
- **CLI Source**: `/packages/cli/src/cli/worktree.ts` (authoritative command reference)

## Notes

**Worktree Lifecycle Focus**: The skill emphasizes the complete lifecycle (create -> use -> work -> merge -> clean) rather than isolated commands, matching the natural workflow pattern.

**Safety First**: Safety considerations are documented prominently before command reference, as worktree operations can cause data loss if sequenced incorrectly.

**No References Subdirectory**: Unlike the maproom plugin, worktree plugin does not need a separate references file. All content fits well in a single SKILL.md due to the linear nature of worktree workflows.

**Testing**: All tasks are documentation-focused. "Tests pass" is N/A, validation uses structural checks and manual review per quality-strategy.md.
