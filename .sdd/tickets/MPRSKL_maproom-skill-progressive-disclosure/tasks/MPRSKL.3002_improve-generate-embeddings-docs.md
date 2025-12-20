# Task: [MPRSKL.3002]: Improve --generate-embeddings documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass - N/A** - documentation-only change (no code logic modified)
- [x] **Verified** - by the verify-task agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- verify-task
- commit-task

## Summary
Improve the help text for the `--generate-embeddings` flag in the `scan` command to clearly explain its purpose, when to use it, and how to skip embeddings when needed.

## Background
The `--generate-embeddings` flag already exists (defaults to true) but its help text may not be clear enough about its purpose and usage. Users need to understand:
1. What embeddings are used for (vector search)
2. When to skip them (config issues, FTS-only use case)
3. How to use the flag (--generate-embeddings=false or --no-generate-embeddings)

Better documentation helps users self-serve when encountering configuration issues and understand the performance implications.

**References:** plan.md Phase 3, Task 7; architecture.md Decision 4

## Acceptance Criteria
- [x] Help text for `--generate-embeddings` flag is clear and informative
- [x] Help text explains what embeddings enable (vector search)
- [x] Help text indicates when to skip embeddings (config issues, FTS-only)
- [x] Help text shows both ways to disable: --generate-embeddings=false and --no-generate-embeddings
- [x] Example usage included in scan --help output
- [x] Changes verified with `crewchief-maproom scan --help`
- [x] No functional changes to flag behavior

## Technical Requirements
- Modify `crates/maproom/src/main.rs` - locate the scan command's `generate_embeddings` flag definition
- Update the help text/documentation comment for the flag
- Use clap's help attributes to improve clarity
- Keep flag behavior unchanged (defaults to true)
- Ensure help text fits well in terminal output (not too verbose)
- No changes to argument parsing logic

## Implementation Notes
**Current implementation (main.rs around line 316-317):**
```rust
/// Automatically generate embeddings after scanning (default: true)
#[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
generate_embeddings: bool,
```

**Recommended implementation approach:** Use the enhanced version with `long_help` for better user guidance:

**Enhanced version:**
```rust
/// Generate embeddings for vector search (default: true).
/// Embeddings enable semantic search but require embedding provider configuration.
/// Use --generate-embeddings=false (or --no-generate-embeddings) to skip if:
/// - Embedding provider is not configured
/// - Only using full-text search
/// - Troubleshooting configuration issues
#[arg(
    long,
    default_value_t = true,
    action = clap::ArgAction::Set,
    help = "Generate embeddings for vector search (default: true)",
    long_help = "Generate embeddings for vector search.\n\
                 Embeddings enable semantic search via vector-search command.\n\
                 Full-text search works without embeddings.\n\n\
                 Skip embeddings with --generate-embeddings=false or --no-generate-embeddings when:\n\
                 - Embedding provider is not configured\n\
                 - Only using full-text search\n\
                 - Troubleshooting configuration issues"
)]
generate_embeddings: bool,
```

**Alternative simpler version:**
```rust
/// Generate embeddings for vector search (default: true).
/// Skip with --no-generate-embeddings to use only full-text search or troubleshoot config.
#[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
generate_embeddings: bool,
```

**Design considerations:**
- **Brevity vs clarity**: Balance helpful information without overwhelming help output
- **Common use cases**: Focus on most common reasons to skip (config issues, FTS-only)
- **Discoverability**: Make it clear that FTS works without embeddings
- **Flag syntax**: Mention both --generate-embeddings=false and --no-generate-embeddings

**Testing approach:**
- No code tests needed (documentation only)
- Manual verification: `crewchief-maproom scan --help` and `crewchief-maproom help scan`
- Verify help text appears correctly formatted
- Confirm information is accurate

## Dependencies
- **MPRSKL.2003** (troubleshooting.md) - Help text can reference troubleshooting scenarios

## Risk Assessment
- **Risk**: Help text too verbose, clutters output
  - **Mitigation**: Use `long_help` for detailed text, keep `help` brief
- **Risk**: Terminology confusion (embeddings, vector search, FTS)
  - **Mitigation**: Use clear, consistent terminology; explain what each enables
- **Risk**: Help text becomes outdated
  - **Mitigation**: Keep content general; focus on concepts not implementation details

## Files/Packages Affected
- crates/maproom/src/main.rs

## Deliverables Produced

Documents created in `deliverables/` directory:

- None

## Verification Notes
The verify-task agent should specifically check:

- [ ] Help text updated for --generate-embeddings flag
- [ ] Short help (--help) is concise and clear
- [ ] Long help (if implemented) provides detailed information
- [ ] Help text explains purpose: enable vector search
- [ ] Help text indicates when to skip: config issues, FTS-only
- [ ] Both flag syntaxes mentioned: --generate-embeddings=false and --no-generate-embeddings
- [ ] No functional changes to flag behavior
- [ ] Flag still defaults to true
- [ ] Code compiles without warnings (`cargo build -p crewchief-maproom`)
- [ ] Code formatted (`cargo fmt -- --check`)

**Manual verification:**
```bash
# Check short help
crewchief-maproom scan --help | grep -A 5 "generate-embeddings"

# Check long help (if --help shows abbreviated version)
crewchief-maproom help scan | grep -A 10 "generate-embeddings"

# Verify help text includes:
# - "vector search" or "semantic search"
# - "full-text search" works without embeddings
# - When to skip embeddings
# - Both flag syntaxes
```

**Content checklist:**
- [ ] Mentions vector search capability
- [ ] Notes full-text search works without embeddings
- [ ] Lists common reasons to skip
- [ ] Shows flag usage syntax
- [ ] Fits well in help output (not too long)

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-20 | verify-task | PASS | All 7 acceptance criteria met, documentation-only change verified via help output |
