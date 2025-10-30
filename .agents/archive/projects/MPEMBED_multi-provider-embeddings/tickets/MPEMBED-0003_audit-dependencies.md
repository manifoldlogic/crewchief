# Ticket: MPEMBED-0003: Audit and update project dependencies for multi-provider support

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Run security audit, update outdated dependencies, and document new dependencies required for Google Vertex AI and trait-based providers.

## Background
The multi-provider embedding migration requires adding new dependencies:
- `google-cloud-auth` for Google Vertex AI service account authentication
- `async-trait` for trait-based provider abstraction

Before adding new dependencies, we must ensure the existing dependency tree is secure and up-to-date. This is also a good opportunity to document dependency choices for future maintainers.

**Reference**: `crewchief_context/maproom/MPEMBED-multi-provider-embeddings/` - Phase 0, Day 0

## Acceptance Criteria
- [ ] `cargo audit` runs without critical vulnerabilities
- [ ] Outdated dependencies updated (non-breaking only): run `cargo outdated`
- [ ] New dependencies documented in Cargo.toml with justification comments
- [ ] `google-cloud-auth` added with version pinned
- [ ] `async-trait` added with version pinned
- [ ] Dependency versions recorded in `docs/dependencies.md`

## Technical Requirements
- Install cargo-audit: `cargo install cargo-audit`
- Install cargo-outdated: `cargo install cargo-outdated`
- Update dependencies conservatively (only patch/minor versions)
- Pin Google SDK versions (breaking changes common in Google crates)
- Document rationale for each new dependency
- Verify build succeeds on all platforms after dependency changes

## Implementation Notes

**Security Audit**:
```bash
cargo audit
# Fix any vulnerabilities found (update affected dependencies)
cargo audit fix --dry-run  # Preview fixes
cargo audit fix            # Apply fixes
```

**Update Outdated Dependencies**:
```bash
cargo outdated
# Update conservatively (patch/minor only)
# Example: tokio 1.35 → 1.40 (minor update OK)
# Avoid: tokio 1.x → 2.x (major breaking change)
```

**Add New Dependencies**:
```toml
# Add to Cargo.toml:
[dependencies]
# Multi-provider trait abstraction
# Used for EmbeddingProvider trait to support OpenAI, Ollama, Google
async-trait = "0.1.80"

# Google Vertex AI embeddings
# Service account authentication for google-cloud-aiplatform
google-cloud-auth = "0.13"  # Pin to minor version (breaking changes common)

# Note: google-cloud-aiplatform will be added in Phase 1 (MPEMBED-1xxx tickets)
# Deferring until provider trait is implemented
```

**Dependency Documentation** (`docs/dependencies.md`):
```markdown
# Maproom Dependencies

## Core Dependencies
- tokio: Async runtime
- sqlx: PostgreSQL driver with compile-time query checking
- pgvector: Vector similarity search (IVFFlat indexes)

## New: Multi-Provider Embeddings (MPEMBED)
- async-trait: Trait abstraction for embedding providers
  - Version: 0.1.80
  - Reason: Enable runtime provider selection (OpenAI, Ollama, Google)
- google-cloud-auth: Service account authentication
  - Version: 0.13.x (pinned to minor)
  - Reason: Authenticate with Google Vertex AI
  - Note: Has heavy transitive deps (gRPC, protobuf ~15MB)
```

**Verification**:
```bash
# Ensure build succeeds
cargo build --release

# Ensure tests still pass
cargo test

# Check dependency tree size
cargo tree | wc -l  # Document size before/after
```

## Dependencies
None

## Risk Assessment
- **Risk**: `google-cloud-auth` has complex transitive dependencies (gRPC, protobuf, tonic)
  - **Mitigation**: Pin all versions, test build on CI before merge; document expected build time increase (~10-15%)

- **Risk**: Dependency updates introduce breaking changes
  - **Mitigation**: Only update patch/minor versions; run full test suite before committing

- **Risk**: Google SDK breaking changes in future
  - **Mitigation**: Pin to minor version (0.13.x); add comment to review quarterly

- **Risk**: Build time increases due to gRPC compilation
  - **Mitigation**: Document expected increase; consider feature flag for Google provider in future

## Files/Packages Affected
- Cargo.toml (modify - add async-trait, google-cloud-auth)
- Cargo.lock (modify - transitive dependency updates)
- docs/dependencies.md (create - dependency documentation)
