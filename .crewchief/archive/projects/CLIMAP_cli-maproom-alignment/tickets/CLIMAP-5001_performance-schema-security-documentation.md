# Ticket: CLIMAP-5001: Add performance, schema, and security documentation to CLI README

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- technical-writer
- typescript-engineer (example verification)
- verify-ticket
- commit-ticket

## Summary

Add three new major sections to packages/cli/README.md: (1) Performance Optimization documenting incremental scanning, parallel processing, and batch tuning, (2) Schema & Features explaining the three major migrations and their benefits, (3) Security Best Practices for credential management. These updates complete the documentation alignment with maproom capabilities.

## Background

After implementing all code changes (Phases 1-3) and tests (Phase 4), this ticket adds the final documentation updates to the CLI README. These sections document advanced features that users might not discover on their own:

- **Performance features exist but are undocumented**: Users don't know about `--parallel`, `--force`, or batch size tuning
- **Schema has evolved significantly**: Three major migrations (0018, 0019, 0020) added content addressing, deduplication, and branch-aware search
- **Security best practices are missing**: Credential management guidance is important but not documented

This builds on the foundation from CLIMAP-1001 (basic environment variable docs) and completes the documentation vision from the CLIMAP planning phase.

**References:**
- Analysis: `.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/analysis.md` (section on new features)
- Security Review: `.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/security-review.md` (security best practices)
- Architecture: `docs/architecture/DATABASE_ARCHITECTURE.md` (schema details)
- Plan: Phase 5, tasks 5.1-5.4

## Acceptance Criteria

### Performance Optimization Section
- [ ] "Performance Optimization" section added after Embedding Provider Setup
- [ ] Documents incremental scanning behavior (default, `--force` to disable)
- [ ] Documents parallel processing (`--parallel`, `--parallel-workers N`)
- [ ] Documents batch size tuning (`--batch-size`, `--embedding-batch-size`)
- [ ] Includes practical examples with flags

### Schema & Features Section
- [ ] "Schema & Features" section added after Performance Optimization
- [ ] Explains Migration 0018 (blob_sha for content addressing)
- [ ] Explains Migration 0019 (code_embeddings deduplication, 70-90% storage reduction)
- [ ] Explains Migration 0020 (worktree_ids for branch-aware search)
- [ ] Describes benefits of each migration clearly

### Security Best Practices Section
- [ ] "Security Best Practices" section added near end of README
- [ ] Documents .env file usage with proper permissions
- [ ] Documents secret manager integration (AWS Secrets Manager, HashiCorp Vault)
- [ ] Warns about credential exposure in environment variables
- [ ] Includes practical code examples for each approach

### Quality Standards
- [ ] All code examples are tested and work correctly
- [ ] Flag names match Rust binary implementation
- [ ] Formatting is consistent with existing README sections
- [ ] Examples are concise and actionable
- [ ] Security recommendations are sound

## Technical Requirements

### Section 1: Performance Optimization

**Location**: Add after "Embedding Provider Setup" section

**Content to Include**:

1. **Incremental Scanning** (default behavior)
   - Uses git tree SHA comparison
   - Only indexes changed files
   - `--force` flag for full re-index
   - Dramatically faster for second scan
   - Example comparing default vs --force

2. **Parallel Processing**
   - `--parallel` flag enables parallel processing
   - `--parallel-workers N` sets worker count (default: 4)
   - 4x+ faster on large codebases (>10k files)
   - Best for multi-core systems
   - Example with --parallel flag

3. **Batch Size Tuning**
   - `--batch-size N` (database inserts, default: 50)
   - `--embedding-batch-size N` (embedding generation, default: 50)
   - Larger batches = faster, more memory
   - Smaller batches = slower, less memory
   - Example with custom batch sizes

**Example Code Block**:
```bash
# Incremental scan (default)
crewchief maproom scan

# Force full re-index
crewchief maproom scan --force

# Parallel processing for large repos
crewchief maproom scan --parallel --parallel-workers 8

# Tune batch sizes for performance
crewchief maproom scan --batch-size 100 --embedding-batch-size 100
```

### Section 2: Schema & Features

**Location**: Add after "Performance Optimization" section

**Content to Include**:

1. **Migration 0018: blob_sha**
   - Added content-addressed storage
   - Each chunk has unique blob_sha based on content hash
   - Foundation for deduplication
   - Enables efficient content tracking

2. **Migration 0019: code_embeddings**
   - Dedicated embeddings table
   - Deduplicated from per-chunk storage
   - HNSW vector index for fast similarity search
   - 70-90% storage reduction for typical codebases
   - Significantly faster search queries

3. **Migration 0020: worktree_ids**
   - Tracks which worktrees contain each chunk
   - Enables branch-aware search
   - JSONB column with GIN index for fast lookups
   - Supports incremental indexing per worktree
   - Core feature for multi-worktree workflows

**Formatting**: Use a clear subsection structure with h3 headings for each migration

### Section 3: Security Best Practices

**Location**: Add near end of README (before Contributing/License sections)

**Content to Include**:

1. **.env File Usage**
   - How to create .env file
   - Setting proper permissions (chmod 600)
   - Using with direnv for automatic loading
   - Importance of never committing to git
   - Adding .env to .gitignore

2. **Secret Manager Integration**
   - AWS Secrets Manager example (aws secretsmanager get-secret-value)
   - HashiCorp Vault example (vault kv get)
   - Benefits of external secret management
   - Rotation and access control

3. **Security Warnings**
   - Never commit credentials to git
   - Add `.env` to `.gitignore`
   - Rotate API keys regularly
   - Use read-only database credentials when possible
   - Environment variables visible to all processes
   - Consider IAM roles instead of static credentials

**Example Code Blocks**:

```bash
# Create .env file (never commit!)
cat > .env <<EOF
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5432/maproom
OPENAI_API_KEY=sk-...
EOF

# Restrict permissions
chmod 600 .env

# Load with direnv
direnv allow
```

```bash
# AWS Secrets Manager
export OPENAI_API_KEY=$(aws secretsmanager get-secret-value \
  --secret-id openai-key \
  --query SecretString \
  --output text)

# HashiCorp Vault
export MAPROOM_DATABASE_URL=$(vault kv get \
  -field=url maproom/db)
```

### Formatting Requirements
- Use consistent markdown formatting (##, ###)
- Include practical, copy-paste-ready examples
- Use bash syntax highlighting for code blocks
- Keep examples concise (3-5 lines preferred)
- Use bullet points for feature lists
- Add clear headings for easy scanning
- Match tone and style of existing README

### Verification Requirements
- Test all command examples to ensure they work
- Verify flag names match Rust binary (`crewchief maproom scan --help`)
- Cross-reference migration numbers with database schema
- Ensure security advice aligns with security-review.md
- Proofread for clarity, grammar, and accuracy

## Implementation Notes

### Content Development Approach
1. Start with Performance Optimization section (most actionable)
2. Add Schema & Features section (educational context)
3. Finish with Security Best Practices (critical but not workflow-blocking)
4. Review all three sections together for flow and consistency

### Style Guidelines
- **Tone**: Technical but accessible
- **Audience**: Developers who have completed basic setup
- **Goal**: Help users discover and use advanced features
- **Format**: Practical examples with brief explanations

### Cross-References
- Performance flags should match Rust implementation in `crates/maproom/`
- Migration descriptions should align with `packages/maproom-mcp/migrations/`
- Security advice should reference `.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/security-review.md`

### Testing Strategy
1. Run each command example in a test environment
2. Verify output matches documented behavior
3. Ensure flag names are accurate (check with `--help`)
4. Test .env file examples with real credentials (then delete)
5. Verify external links (AWS docs, Vault docs) are correct

### Length Management
- Keep each section to 15-25 lines of content
- Use tables if comparing multiple options
- Link to external resources rather than duplicating docs
- Focus on "how to use" not "how it works internally"

## Dependencies

**Prerequisites:**
- CLIMAP-1001 (basic README updates must be complete)
- CLIMAP-2001 (command structure documented correctly)
- CLIMAP-3001 (validation behavior to document)

**Blocked by:** None (can proceed independently)

**External Dependencies:**
- Rust binary must be built to verify flag names
- Database must be available to test examples
- Security review document must be finalized

## Risk Assessment

**Risk**: Documentation might become too long and overwhelming
- **Mitigation**: Keep examples concise (3-5 lines), use clear sections with h2/h3 headers, focus on practical usage not implementation details

**Risk**: Command examples might not work or have typos
- **Mitigation**: Test every command example before committing. Run `crewchief maproom scan --help` to verify flag names. Use actual working examples from development environment.

**Risk**: Security advice might be incomplete or incorrect
- **Mitigation**: Reference security-review.md for best practices. Include standard security warnings (never commit credentials, rotate keys). Link to authoritative external sources (AWS, Vault docs).

**Risk**: Migration descriptions might be too technical
- **Mitigation**: Focus on user benefits (70-90% storage reduction, faster search) rather than implementation details. Keep technical jargon minimal.

**Risk**: Examples might not match user environments
- **Mitigation**: Use generic examples (localhost database, standard ports). Provide both Docker and production examples. Include troubleshooting hints.

## Files/Packages Affected

**Primary File:**
- `/workspace/packages/cli/README.md` - Add three new sections

**Reference Files (read-only):**
- `/workspace/.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/analysis.md` - Feature analysis
- `/workspace/.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/security-review.md` - Security guidance
- `/workspace/docs/architecture/DATABASE_ARCHITECTURE.md` - Schema details and migration history
- `/workspace/packages/maproom-mcp/migrations/` - Migration SQL files for verification
- `/workspace/crates/maproom/src/` - Rust code for flag verification

## Estimated Effort

**Total**: 2-3 hours

**Breakdown**:
- 45 min: Performance Optimization section (research flags, write examples, test)
- 45 min: Schema & Features section (review migrations, summarize benefits)
- 45 min: Security Best Practices section (gather best practices, create examples)
- 30 min: Testing all examples and verification
- 15 min: Final review and formatting consistency check
