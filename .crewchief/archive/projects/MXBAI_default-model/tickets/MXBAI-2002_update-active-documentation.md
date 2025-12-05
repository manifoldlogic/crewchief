# Ticket: [MXBAI-2002]: Update Active Documentation Files

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation updates, no executable code)
- [x] **Verified** - by the verify-ticket agent

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
Update all 7 active documentation files identified in MXBAI-2001 audit to reflect mxbai-embed-large as the default model, replacing nomic-embed-text examples and references.

## Background
This ticket implements Phase 2, Deliverable 2 from plan.md. After code changes in Phase 1, user-facing documentation must be updated to match the new defaults. This ensures users following documentation examples get correct, working configurations.

Reference: plan.md Phase 2, Deliverable 2 "Update Example Code"

## Acceptance Criteria
- [x] ollama-setup.md updated with mxbai-embed-large examples (nomic examples replaced)
- [x] crates/maproom/CLAUDE.md updated with new default model references
- [x] README.md updated (quickstart, examples, default mentions)
- [x] packages/vscode-maproom/README.md updated with correct setup instructions
- [x] packages/maproom-mcp/README.md updated if model mentioned
- [x] crates/maproom/.env.example verified (already updated in MXBAI-1003)
- [x] All 7 files show mxbai-embed-large as default, with backward compat notes where appropriate

## Completion Notes

### Files Updated
1. **docs/providers/ollama-setup.md** - Comprehensive update:
   - Changed default model from nomic-embed-text to mxbai-embed-large
   - Updated dimensions from 768 to 1024
   - Updated model pull commands
   - Updated expected outputs
   - Added "legacy" section for nomic-embed-text
   - Updated model comparison tables
   - Updated troubleshooting sections
   - Updated quick reference

2. **crates/maproom/CLAUDE.md** - Updated:
   - Environment variable example to use mxbai-embed-large
   - Dimension table to show mxbai-embed-large as default
   - Configuration examples

3. **README.md** - Updated:
   - Ollama provider example to use mxbai-embed-large with 1024 dimensions

4. **packages/vscode-maproom/README.md** - Updated:
   - Ollama setup instructions to use mxbai-embed-large
   - Model download size to 669MB

5. **docs/providers/comparison.md** - Updated:
   - Quick comparison table (Ollama: 1024 dim, mxbai-embed-large)
   - Quality comparison table
   - Setup steps
   - Handling different dimensions section
   - FAQ section

6. **docs/providers/README.md** - Updated:
   - Verify setup output to show 1024 dimensions for Ollama
   - Ollama features section

7. **docs/architecture/MAPROOM_ARCHITECTURE.md** - Updated:
   - Multi-provider table to show mxbai-embed-large as default
   - Model list in embedding section

8. **crates/maproom/.env.example** - Verified already updated in MXBAI-1003

### Backward Compatibility
All documentation now includes backward compatibility notes for users who prefer nomic-embed-text:
```bash
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
```

## Technical Requirements
**File 1: `/workspace/docs/providers/ollama-setup.md`** (20 min):
- Replace example commands showing nomic-embed-text with mxbai-embed-large
- Update `ollama pull` commands to reference mxbai-embed-large
- Update dimension references from 768 to 1024
- Add section: "Using nomic-embed-text (legacy)" with backward compat instructions

**File 2: `/workspace/crates/maproom/CLAUDE.md`** (15 min):
- Find all references to "default model"
- Update to show mxbai-embed-large as default
- Update dimension references where applicable
- Ensure examples use correct model name

**File 3: `/workspace/README.md`** (15 min):
- Check quickstart section for model references
- Update any hardcoded examples using nomic-embed-text
- Update "Getting Started" or "Quick Start" sections
- Verify code examples show correct defaults

**File 4: `/workspace/packages/vscode-maproom/README.md`** (10 min):
- Update setup instructions if they reference model name
- Update any screenshots or examples showing old model
- Add note about automatic model download (mxbai-embed-large)
- Ensure configuration examples are current

**File 5: `/workspace/packages/maproom-mcp/README.md`** (10 min):
- Check if documentation mentions specific model
- Update any model references to mxbai-embed-large
- Update configuration examples if present
- Add backward compatibility note if applicable

**File 6: `/workspace/crates/maproom/.env.example`** (verification only):
- Verify already updated in MXBAI-1003
- No changes needed, just confirm

**Pattern for Updates**:
- Replace "nomic-embed-text" → "mxbai-embed-large"
- Replace "768" → "1024" (in embedding dimension context)
- Replace "274MB" → "670MB" (model download size if mentioned)
- Add backward compatibility notes where helpful

**Pattern for Backward Compatibility Notes**:
```markdown
**Using nomic-embed-text**: If you prefer the legacy model:
```bash
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
```
```

## Implementation Notes
**Update Strategy**:
1. Read each file to understand context
2. Find all nomic-embed-text references
3. Determine if each reference should be updated or preserved (e.g., troubleshooting sections might mention both)
4. Update examples and default references
5. Add backward compatibility notes where users might want old model

**What to Update**:
- Default model examples
- Quick start commands
- Configuration examples
- Setup instructions
- Model download commands

**What to Preserve**:
- Historical context (e.g., "we previously used nomic-embed-text")
- Troubleshooting sections that mention both models
- Comparison tables
- Migration guidance (covered in MXBAI-2003)

**Quality Standards**:
- Examples must be copy-pasteable and work correctly
- Dimension values must match model (1024 for mxbai)
- Backward compatibility documented where helpful
- No conflicting information across files

## Dependencies
- **Critical dependency**: MXBAI-2001 (audit must identify which files to update)
- **External dependency**: None

## Risk Assessment
- **Risk**: Breaking example commands
  - **Mitigation**: Test example commands in terminal before committing

- **Risk**: Creating inconsistent documentation
  - **Mitigation**: MXBAI-2004 will perform consistency check

- **Risk**: Confusing users with too many backward compat notes
  - **Mitigation**: Add notes only where users are likely to need them (setup guides, not every mention)

## Files/Packages Affected
- `/workspace/docs/providers/ollama-setup.md`
- `/workspace/crates/maproom/CLAUDE.md`
- `/workspace/README.md`
- `/workspace/packages/vscode-maproom/README.md`
- `/workspace/packages/maproom-mcp/README.md`
- `/workspace/crates/maproom/.env.example` (verification only)

## Verification Notes
Tests pass: N/A (documentation updates, no executable code)

verify-ticket agent should check:
- [ ] All 7 files reviewed and updated where needed
- [ ] No remaining nomic-embed-text references in updated files (except backward compat notes)
- [ ] Examples are correct and would work if copy-pasted
- [ ] Dimension values match model (1024 for mxbai)
- [ ] Backward compatibility documented appropriately
- [ ] No conflicting information across files
- [ ] .env.example verified as already updated (MXBAI-1003)
