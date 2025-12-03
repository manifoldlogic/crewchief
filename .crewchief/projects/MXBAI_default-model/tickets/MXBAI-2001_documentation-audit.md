# Ticket: [MXBAI-2001]: Documentation Audit and Categorization

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (audit/analysis ticket)
- [x] **Verified** - by the verify-ticket agent

## Audit Results

### Summary
- **Total files with nomic-embed-text references**: 150
- **Active docs (must update)**: 33 files
- **Preserved docs (.crewchief/)**: 117 files
  - Archive: 81 files
  - DIM1024 project: 11 files
  - MXBAI project: 24 files (intentional)

### MUST UPDATE (33 files)
**Core Documentation:**
- /workspace/README.md
- /workspace/crates/maproom/CLAUDE.md
- /workspace/crates/maproom/README.md
- /workspace/docs/providers/ollama-setup.md
- /workspace/docs/providers/README.md
- /workspace/docs/providers/comparison.md
- /workspace/packages/vscode-maproom/README.md
- /workspace/packages/cli/README.md

**Architecture & Guides:**
- /workspace/docs/architecture/MAPROOM_ARCHITECTURE.md
- /workspace/docs/architecture/daemon.md
- /workspace/docs/architecture/overview.md
- /workspace/docs/architecture/sequences.md
- /workspace/docs/guides/performance-tuning.md
- /workspace/docs/guides/provider-migration.md
- /workspace/docs/configuration/embedding-optimization.md
- /workspace/config/QUICKSTART.md

**Troubleshooting:**
- /workspace/docs/troubleshooting/README.md
- /workspace/docs/troubleshooting/common-errors.md
- /workspace/docs/troubleshooting/debugging.md
- /workspace/packages/vscode-maproom/docs/TROUBLESHOOTING.md

**Performance/Historical (review needed):**
- /workspace/docs/performance/*.md (4 files)
- /workspace/docs/profiling/*.md (1 file)
- /workspace/docs/arm64-compatibility-report.md
- /workspace/benchmarks/multi_provider_performance.md

**Agent/Test Files (review needed):**
- /workspace/.agent/reference/agent-bench/search-team/multi-provider-embeddings.md
- /workspace/.claude/agent-bench/search-team/multi-provider-embeddings.md
- /workspace/tests/manual/mpembed_*.md (2 files)
- /workspace/packages/cli/scripts/README-scan-estimation.md
- /workspace/crates/maproom/docs/development/integration-testing.md

### PRESERVE (117 files)
**Rule: Do NOT update these files**
- `.crewchief/archive/**/*.md` (81 files) - Historical project records
- `.crewchief/projects/DIM1024_*/**/*.md` (11 files) - Related project context
- `.crewchief/projects/MXBAI_*/**/*.md` (24 files) - This project's planning docs

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- documentation-writer
- verify-ticket
- commit-ticket

## Summary
Perform comprehensive audit of all documentation files containing "nomic-embed-text" references, categorize them into "must update" vs "preserve for historical context", and create categorized lists for Phase 2 documentation updates.

## Background
This ticket implements Phase 2, Deliverable 1 from plan.md. The grep audit identified 132+ .md files with "nomic-embed-text" references. Most are in archived projects and should be preserved for historical context. We need to identify the 7 active documentation files that require updates.

Reference: plan.md Phase 2, Deliverable 1 "Documentation Audit"

## Acceptance Criteria
- [ ] Complete list of all .md files with "nomic-embed-text" created
- [ ] "Must update" category identified (7 active documentation files)
- [ ] "Preserve" category identified (125+ archived/historical files)
- [ ] Explicit rule documented: Do NOT update `.crewchief/archive/` or `.crewchief/projects/DIM1024_*`
- [ ] Audit results saved to ticket for reference in MXBAI-2002

## Technical Requirements
**Grep Search** for all markdown files with references:
```bash
# Find all .md files with nomic-embed-text
grep -rl "nomic-embed-text" --include="*.md" /workspace/ > /tmp/nomic-references.txt

# Count total files
wc -l /tmp/nomic-references.txt

# Categorize by location
grep "\.crewchief/archive/" /tmp/nomic-references.txt | wc -l  # Should be preserved
grep "\.crewchief/projects/DIM1024_" /tmp/nomic-references.txt | wc -l  # Should be preserved
grep -v "\.crewchief/" /tmp/nomic-references.txt  # Active docs to review
```

**Expected "Must Update" Files** (7 total per architecture.md):
1. `/workspace/docs/providers/ollama-setup.md` - Replace nomic examples with mxbai
2. `/workspace/crates/maproom/CLAUDE.md` - Change default model references
3. `/workspace/README.md` - Update quickstart/examples if model mentioned
4. `/workspace/packages/vscode-maproom/README.md` - Update setup instructions
5. `/workspace/packages/maproom-mcp/README.md` - Update MCP docs if model mentioned
6. `/workspace/crates/maproom/.env.example` - Already updated in MXBAI-1003 (verify)
7. `/workspace/docs/guides/migrating-to-mxbai.md` - New file (create in MXBAI-2003)

**Expected "Preserve" Categories**:
- `.crewchief/archive/projects/**/*.md` - Historical project documentation
- `.crewchief/projects/DIM1024_*/**/*.md` - Related project context
- `.crewchief/projects/MXBAI_*/**/*.md` - This project's planning docs (intentionally reference old defaults)

**Categorization Rules**:
- **Must update**: Active user-facing documentation (READMEs, guides, setup docs)
- **Preserve**: Archived projects, historical context, planning documents
- **Explicit exclusion**: Never update `.crewchief/archive/` or `.crewchief/projects/DIM1024_*`

## Implementation Notes
**Audit Output Format**:
Document findings in this ticket with:
1. Total count of files with references
2. List of "must update" files (expected: 7)
3. Count of "preserve" files (expected: 125+)
4. Any unexpected files requiring decision

**Why Preserve Historical Docs**:
- Archived projects document past decisions and context
- DIM1024 project planning explains why we're making this change
- Changing historical docs would create false history

**Verification**:
After categorization, verify counts match expectations:
- Must update: ~7 files
- Preserve: ~125 files
- Total: ~132 files (per initial grep audit)

## Dependencies
- **Completed dependency**: MXBAI-1006 (Phase 1 verification confirms code changes complete)
- **External dependency**: None

## Risk Assessment
- **Risk**: Accidentally categorizing active docs as "preserve"
  - **Mitigation**: Review each "must update" file to confirm it's user-facing

- **Risk**: Updating archived docs by mistake
  - **Mitigation**: Explicit rule in ticket: Do NOT update .crewchief/archive/

- **Risk**: Missing active documentation files
  - **Mitigation**: Search multiple patterns (nomic-embed-text, 768, default model)

## Files/Packages Affected
- All .md files in repository (audit only, no changes in this ticket)
- Audit results documented in this ticket's completion notes

## Verification Notes
Tests pass: N/A (audit/analysis ticket, no code or executable changes)

verify-ticket agent should check:
- [ ] Audit results documented in ticket completion notes
- [ ] "Must update" list contains ~7 files (all active documentation)
- [ ] "Preserve" list contains ~125 files (all archived/historical)
- [ ] Explicit rule documented about .crewchief/archive/ exclusion
- [ ] No files categorized incorrectly (active docs marked as preserve, or vice versa)
- [ ] Audit is comprehensive (no major docs missed)
