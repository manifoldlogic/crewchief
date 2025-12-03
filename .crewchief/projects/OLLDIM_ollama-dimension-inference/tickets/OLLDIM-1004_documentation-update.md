# Ticket: [OLLDIM-1004]: Documentation Update

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation-only ticket)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-developer
- verify-ticket
- commit-ticket

## Summary
Update `crates/maproom/CLAUDE.md` to document the new automatic dimension inference feature, explain supported models, show how to override inference, and provide guidance for users upgrading from pre-inference versions.

## Background
After implementing dimension inference (OLLDIM-1001, OLLDIM-1002, OLLDIM-1003), users and future maintainers need clear documentation about:
- How automatic inference works
- Which models are supported
- How to override inference with explicit configuration
- What happens after upgrading (no migration needed)

This documentation will help users understand the zero-config workflow and troubleshoot any dimension-related issues.

Reference: Phase 1 Post-Implementation from plan.md lines 418-448

## Acceptance Criteria
- [ ] New section "Embedding Dimension Configuration" added to CLAUDE.md
- [ ] Documents all supported models with their dimensions
- [ ] Explains explicit override mechanism with example
- [ ] Includes "After Upgrading" section for existing users
- [ ] Shows zero-config example workflow
- [ ] Clear, concise language suitable for both users and developers
- [ ] Markdown formatting is correct and renders properly

## Technical Requirements
- File: `crates/maproom/CLAUDE.md`
- Location: Add new sections at appropriate location (near embedding/configuration content)
- Format: Standard markdown with code blocks
- Style: Match existing documentation tone and format

## Implementation Notes

**Exact content provided in plan.md lines 419-449:**

Add two new sections to `crates/maproom/CLAUDE.md`:

### Section 1: Embedding Dimension Configuration

```markdown
## Embedding Dimension Configuration

Maproom automatically infers embedding dimensions for known Ollama models:
- `mxbai-embed-large*`: 1024 dimensions (default, matches tags like `:latest`)
- `nomic-embed-text*`: 768 dimensions (matches tags like `:latest`)

To override automatic inference or configure custom models:
```bash
export MAPROOM_EMBEDDING_DIMENSION=512
```

Explicit configuration always takes precedence over inference.
```

### Section 2: After Upgrading to Dimension Inference

```markdown
## After Upgrading to Dimension Inference

If you previously experienced dimension mismatch errors:
1. The fix is automatic - no configuration changes needed
2. Existing embeddings are dimension-tagged and remain valid
3. New embeddings will use correct inferred dimensions
4. No regeneration required

Zero-config workflows now work correctly:
```bash
# No environment variables needed for Ollama with standard models
crewchief-maproom generate-embeddings --repo myrepo
# Automatically uses mxbai-embed-large at 1024 dimensions
```
```

**Placement considerations:**
- Look for existing embedding or configuration sections
- If none exist, add after "Quick Start" or "Usage" sections
- Ensure documentation flows logically

## Dependencies
- **Prerequisites**:
  - OLLDIM-1001 (helper function implemented)
  - OLLDIM-1002 (inference logic integrated)
  - OLLDIM-1003 (integration test passing)
- All functionality must be working before documentation is updated

## Risk Assessment
- **Risk**: Documentation becomes outdated if model list changes
  - **Mitigation**: Keep model list synchronized with helper function in config.rs. Add comment in code referencing documentation location.

- **Risk**: Users don't find documentation when needed
  - **Mitigation**: Use clear section headers and add to table of contents if present. Consider mentioning in main project README if embedding setup is commonly documented there.

## Files/Packages Affected
- `crates/maproom/CLAUDE.md` (add 2 new sections)

## Verification Notes
The verify-ticket agent should confirm:
1. Both sections exist in CLAUDE.md
2. Model list matches implementation (nomic-embed-text: 768, mxbai-embed-large: 1024)
3. Code examples are syntactically correct
4. Markdown renders correctly (check with preview if available)
5. Documentation is clear and free of typos
6. Placement makes sense in context of existing documentation
7. Tone and style match existing documentation
8. No broken links or formatting issues
