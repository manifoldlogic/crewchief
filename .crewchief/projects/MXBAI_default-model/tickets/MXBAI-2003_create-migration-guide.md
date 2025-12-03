# Ticket: [MXBAI-2003]: Create Migration Guide

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation file, no executable code)
- [ ] **Verified** - by the verify-ticket agent

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
Create comprehensive migration guide at docs/guides/migrating-to-mxbai.md with all 7 required sections per architecture.md specification, providing clear guidance for users affected by the default model change.

## Background
This ticket implements Phase 2, Deliverable 3 from plan.md. The migration guide is the primary resource for users who need to understand the change, decide whether to migrate, or configure explicit nomic-embed-text if preferred. It must address CLI users, VSCode users, and MCP server users.

Reference: plan.md Phase 2, Deliverable 3 "Create Migration Guide"

## Acceptance Criteria
- [x] Migration guide created at `/workspace/docs/guides/migrating-to-mxbai.md`
- [x] All 7 required sections present (per architecture.md specification)
- [x] Code examples tested and working
- [x] Storage impact calculator accurate (33% increase)
- [x] Troubleshooting FAQ covers 8+ common issues
- [x] Model comparison table included
- [x] Guide addresses all three user types (CLI, VSCode, MCP)

## Technical Requirements
**Required Sections** (from architecture.md lines 182-192):

**Section 1: Executive Summary** (why/what changed)
- Why: Better quality, no crashes, no sanitization needed
- What: Default changed from nomic-embed-text (768) to mxbai-embed-large (1024)
- Who affected: Zero-config users (automatic upgrade)
- Who unaffected: Explicit config users (no change)

**Section 2: Zero-Config Users** (no action needed)
- Explain automatic upgrade on next indexing
- New embeddings go to vec_code_1024 table
- Old embeddings remain in vec_code_768 (mixed search works)
- No manual action required

**Section 3: Explicit Config Users** (keeping nomic-embed-text)
- How to keep using nomic-embed-text with env vars
- CLI example:
  ```bash
  export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
  export MAPROOM_EMBEDDING_DIMENSION=768
  ```
- VSCode configuration (if applicable)
- MCP server configuration (if applicable)

**Section 4: Re-embedding Guide** (with specific commands)
- When to re-embed (for quality improvement)
- CLI commands:
  ```bash
  # Clear old embeddings
  crewchief-maproom clear --repo /path/to/repo

  # Re-index with new model
  crewchief-maproom index --repo /path/to/repo
  ```
- VSCode: Re-index command
- MCP: Automatic on next index request

**Section 5: Storage Impact Calculator** (33% increase explanation)
- Math: 768 floats × 4 bytes = 3,072 bytes vs 1024 floats × 4 bytes = 4,096 bytes
- Per-embedding: ~33% increase (1,024 bytes larger)
- Example: 10,000 embeddings = ~10MB increase
- Model download: 670MB vs 274MB (one-time)

**Section 6: Troubleshooting FAQ** (8+ common issues from quality-strategy.md):
1. "Model not found" error → Run `ollama pull mxbai-embed-large`
2. Existing embeddings not searchable → Mixed search works automatically
3. Storage concerns → Storage is cheap, quality worth it
4. Performance concerns → Minimal impact, tested in EMBPERF
5. Want to switch back → Use explicit env vars
6. Re-embed all content? → Optional, only if want quality improvement
7. Breaking changes? → No, backward compatible
8. Mixed dimensions supported? → Yes, tested in DIM1024

**Section 7: Model Comparison Table**
| Feature | nomic-embed-text | mxbai-embed-large |
|---------|------------------|-------------------|
| Dimension | 768 | 1024 |
| Quality | Good | Better |
| Special chars | Crashes (needs sanitization) | Works (no sanitization) |
| Model size | 274 MB | 670 MB |
| Storage per embedding | 3,072 bytes | 4,096 bytes |
| Default | Legacy | Current |

**Target Audiences**:
- CLI users: Direct crewchief-maproom usage
- VSCode users: Extension users
- MCP server users: MCP integration users

**Tone and Style**:
- Clear, concise, helpful
- Assume user wants to understand impact
- Provide both "do nothing" and "explicit config" paths
- Emphasize backward compatibility

## Implementation Notes
**Structure**:
```markdown
# Migrating to mxbai-embed-large

## Executive Summary
[Section 1]

## For Zero-Config Users
[Section 2]

## For Explicit Config Users
[Section 3]

## Re-embedding Existing Content
[Section 4]

## Storage Impact
[Section 5]

## Troubleshooting
[Section 6]

## Model Comparison
[Section 7]
```

**Example Commands Must Work**:
- Test all CLI commands before including
- Verify env var examples are correct
- Check paths and syntax

**Quality Standards**:
- Every question answered clearly
- Every command tested
- No jargon without explanation
- Links to other docs where helpful

## Dependencies
- **Completed dependency**: MXBAI-1006 (Phase 1 verification confirms defaults changed)
- **External dependency**: None

## Risk Assessment
- **Risk**: Incomplete FAQ
  - **Mitigation**: Review quality-strategy.md for all identified issues

- **Risk**: Incorrect commands
  - **Mitigation**: Test every command in guide

- **Risk**: Confusing explanations
  - **Mitigation**: Review guide from user perspective, simplify complex sections

## Files/Packages Affected
- `/workspace/docs/guides/migrating-to-mxbai.md` (new file)

## Verification Notes
Tests pass: N/A (documentation file, no executable code)

verify-ticket agent should check:
- [ ] All 7 required sections present
- [ ] Executive summary clear and concise
- [ ] Zero-config path explained (no action needed)
- [ ] Explicit config examples correct (env vars, syntax)
- [ ] Re-embedding commands tested and working
- [ ] Storage math accurate (33% increase calculation)
- [ ] FAQ covers 8+ common issues
- [ ] Model comparison table accurate
- [ ] Guide addresses CLI, VSCode, and MCP users
- [ ] Tone is helpful and reassuring
- [ ] No broken links or syntax errors
