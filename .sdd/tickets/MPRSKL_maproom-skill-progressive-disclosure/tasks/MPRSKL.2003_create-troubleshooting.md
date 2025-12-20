# Task: [MPRSKL.2003]: Create troubleshooting.md

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only task)
- [x] **Verified** - by the verify-task agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general
- verify-task
- commit-task

## Summary
Create troubleshooting guide documenting common maproom errors with root causes, solutions, and configuration verification steps, enabling agents and users to diagnose and fix issues independently.

## Background
Part of the progressive disclosure restructure, troubleshooting.md provides error recovery information separate from the main SKILL.md. This is especially important given the dimension mismatch bug being fixed in Phase 1.

The troubleshooting guide should document the dimension mismatch error comprehensively, including the fix from MPRSKL.1001-1002, and provide actionable solutions for other common errors.

**References:** plan.md Phase 2, Task 5; architecture.md Decision 3 (Error Messages)

## Acceptance Criteria
- [x] File created at `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/troubleshooting.md`
- [x] Documents dimension mismatch error with cause and solution
- [x] Documents "repository not found" error
- [x] Documents "embeddings unavailable" or "vector search not available" error
- [x] Each error includes: error message, cause, solution steps, prevention tips
- [x] Configuration verification section with commands to check current setup
- [x] Links back to cli-reference.md where relevant
- [x] References the bug fix from Phase 1 for dimension mismatch
- [x] All solutions are actionable and testable

## Technical Requirements
- Create new file at `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/troubleshooting.md`
- Use consistent error documentation format for each issue
- Include example error messages (exact text from CLI)
- Provide command examples for verification and fixes
- Use warning/note blocks where appropriate
- Link to relevant CLI reference sections
- Test solutions where possible to ensure accuracy

## Implementation Notes
**Document structure (from architecture.md):**

```markdown
# Maproom Troubleshooting

Common errors and solutions for crewchief-maproom.

## Quick Diagnostics

Check your current configuration:
```bash
# View current status
crewchief-maproom status

# Check database location
echo $MAPROOM_DATABASE_URL

# Check embedding configuration
env | grep MAPROOM_EMBEDDING
```

## Common Errors

### Dimension Mismatch

**Error Message:**
```
Error: Dimension mismatch: expected 1536 but got 1024
```

**Cause:**
Embedding provider configuration doesn't match the actual provider being used. This commonly occurs when:
- Ollama is auto-detected but config assumes OpenAI defaults
- Environment variables specify one provider but another is detected
- Model was changed but dimension wasn't updated

**Solution:**

1. **Set provider explicitly** (recommended):
   ```bash
   export MAPROOM_EMBEDDING_PROVIDER=ollama
   crewchief-maproom scan --path /your/repo
   ```

2. **Match dimension to your model**:
   ```bash
   export MAPROOM_EMBEDDING_DIMENSION=1024  # For Ollama mxbai-embed-large
   crewchief-maproom scan --path /your/repo
   ```

3. **Skip embeddings if not needed**:
   ```bash
   crewchief-maproom scan --path /your/repo --generate-embeddings=false
   ```

**Note:** As of MPRSKL.1001-1002, auto-detected Ollama should correctly infer dimension without manual configuration.

**Prevention:**
- Use explicit `MAPROOM_EMBEDDING_PROVIDER` env var
- Let dimension be inferred from provider/model combination
- Only set `MAPROOM_EMBEDDING_DIMENSION` for custom models

---

### Repository Not Found

**Error Message:**
```
Error: Repository 'myproject' not found in index
```

**Cause:**
The repository hasn't been scanned yet, or was scanned with a different name.

**Solution:**

1. **List indexed repositories**:
   ```bash
   crewchief-maproom status
   ```

2. **Scan the repository**:
   ```bash
   crewchief-maproom scan --path /path/to/myproject --repo myproject
   ```

3. **Check exact repository name** - names are case-sensitive.

---

### Vector Search Unavailable

**Error Message:**
```
Vector search not available for repository 'myproject'
```

**Cause:**
- Repository was scanned without embeddings (`--generate-embeddings=false`)
- Embedding generation failed during scan
- sqlite-vec extension not available

**Solution:**

1. **Rescan with embeddings**:
   ```bash
   crewchief-maproom scan --path /your/repo --generate-embeddings=true
   ```

2. **Check embedding configuration**:
   ```bash
   env | grep MAPROOM_EMBEDDING
   ```

3. **Use full-text search instead**:
   ```bash
   crewchief-maproom search --repo myproject --query "your query"
   ```

**Fallback:** Full-text search works without embeddings and covers many use cases.

---

## Configuration Verification

### Check Current Setup

```bash
# Database location
ls -lh ~/.maproom/maproom.db

# Embedding provider status
env | grep MAPROOM_EMBEDDING

# Indexed repositories
crewchief-maproom status
```

### Validate Embedding Provider

For Ollama:
```bash
curl http://localhost:11434/api/tags
# Should return list of models including mxbai-embed-large
```

For OpenAI:
```bash
echo $OPENAI_API_KEY
# Should be set with valid API key
```

---

## Getting Help

If issues persist:
1. Check [CLI Reference](./cli-reference.md) for command syntax
2. Review [Search Best Practices](./search-best-practices.md) for usage patterns
3. Check logs for detailed error messages
4. Verify embedding provider is accessible
```

**Critical content requirements:**
- **Dimension mismatch must reference Phase 1 fix** - note that bug was fixed in MPRSKL.1001-1002
- **Solutions must be actionable** - specific commands that solve the problem
- **Include verification steps** - how to confirm the fix worked

## Dependencies
- **MPRSKL.1001, MPRSKL.1002** (Phase 1 bug fix) - Should reference that dimension mismatch is fixed for auto-detected Ollama

## Risk Assessment
- **Risk**: Solutions become outdated as CLI evolves
  - **Mitigation**: Document current behavior; recommend periodic review
- **Risk**: Missing common errors users actually encounter
  - **Mitigation**: Start with known issues (dimension mismatch, repo not found, embeddings unavailable); can expand based on feedback
- **Risk**: Solutions don't work
  - **Mitigation**: Test each solution command where possible

## Files/Packages Affected
- .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/troubleshooting.md (new file)

## Deliverables Produced

Documents created in `deliverables/` directory:

- None

## Verification Notes
The verify-task agent should specifically check:

- [ ] File created in correct location (references/ subdirectory)
- [ ] At least 3 common errors documented (dimension mismatch, repo not found, vector search unavailable)
- [ ] Each error has: message example, cause, solution, prevention
- [ ] Dimension mismatch section references Phase 1 bug fix (MPRSKL.1001-1002)
- [ ] All solution commands are syntactically correct
- [ ] Configuration verification section is complete
- [ ] Links to other reference docs are valid
- [ ] Error messages match actual CLI output (verify with current binary if possible)
- [ ] Solutions are actionable and specific
- [ ] Markdown formatting is correct (code blocks, headings, lists)

**Content verification:**
```bash
# Verify error message formats by triggering errors (if safe):
# 1. Dimension mismatch - set wrong dimension
MAPROOM_EMBEDDING_DIMENSION=999 crewchief-maproom scan --path .

# 2. Repository not found
crewchief-maproom search --repo nonexistent --query "test"

# Compare actual error messages with documented messages
```

**Solution testing:**
- Test at least one solution per error category
- Verify environment variable commands work
- Check that fallback solutions are practical

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-20 | verify-task | PASS | All 9 acceptance criteria met, comprehensive troubleshooting guide with 3+ errors documented |
