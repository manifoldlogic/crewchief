# Post-MVP Roadmap: VSCode Maproom Extension

## Purpose

This document contains all features and enhancements that are explicitly **OUT OF SCOPE** for the MVP release. These features may be implemented in future versions based on user feedback and demand.

**MVP Completion Definition:** Extension provides automatic code indexing with Ollama/OpenAI providers, file/branch watching, Docker orchestration, and VSIX distribution.

---

## Phase 5: Marketplace & Refinement (Post-MVP)

**Timeline:** 2-3 weeks after MVP release

### 5.1: VSCode Marketplace Publishing

**Why:** Wider distribution, automated updates, better trust

**Features:**
- Publish to VSCode Marketplace
- Publish to Open VSX (for Cursor compatibility)
- Marketplace listing with screenshots
- Release notes automation
- Automated publishing via GitHub Actions

**Requirements:**
- Verified publisher account
- Extension icon and banner
- Screenshots of setup wizard, status bar, notifications
- Detailed README for marketplace
- Privacy policy published

---

### 5.2: Google Vertex AI Full Support

**Why:** Deferred from MVP to reduce scope

**Features:**
- Full Google Vertex AI provider integration
- Service account JSON credential support
- Google Cloud project ID validation
- Vertex AI service health checks
- Documentation for Google setup

**MVP Note:** Google provider scaffolding exists, but not fully tested.

---

### 5.3: Windows Platform Support

**Why:** Docker Desktop on Windows has edge cases

**Features:**
- Full Windows x64 support
- Windows-specific binary testing
- Docker Desktop for Windows integration
- PowerShell installation scripts
- Windows firewall configuration guidance

**MVP Note:** Windows x64 binary bundled, but not extensively tested. Documented as experimental.

---

## Phase 6: Advanced Features (3-6 months post-MVP)

### 6.1: Multi-Workspace Support

**Current Limitation:** Extension indexes one workspace at a time

**Proposed:**
- Support multiple open workspaces simultaneously
- Separate index for each workspace
- Workspace switcher in status bar
- Shared embeddings across workspaces (same provider)
- Database: Multiple repos in same database, OR separate databases per workspace

**Design Questions:**
- Shared PostgreSQL with multiple repos/worktrees?
- Separate Docker Compose stack per workspace?
- Performance impact of multiple concurrent scans?

**Estimated Effort:** 2-3 weeks

---

### 6.2: Index Statistics Dashboard

**Current Limitation:** No visibility into index health, size, or performance

**Proposed:**
- WebView panel showing:
  - Total chunks indexed
  - Chunks per file type (TypeScript, Python, etc.)
  - Index size (MB)
  - Last scan time
  - Embedding provider used
  - Query performance metrics (future)
- Chart.js visualizations
- Export stats to JSON/CSV

**Technical Approach:**
- Query PostgreSQL for stats (extension reads, not writes)
- WebView with React or plain HTML
- Refresh stats on demand or periodic (every 5 min)

**Estimated Effort:** 1-2 weeks

---

### 6.3: Custom Embedding Models

**Current Limitation:** Fixed models per provider (Ollama: nomic-embed-text, OpenAI: text-embedding-3-small)

**Proposed:**
- Allow user to select custom Ollama models
- Download model via extension command
- Validate model dimension (must match database schema)
- Support sentence-transformers models
- Allow switching models (requires re-index)

**Design Questions:**
- How to handle dimension changes? (New table? Migration?)
- Which models to support? (Only tested, or any?)
- Performance impact of larger models?

**Estimated Effort:** 2-3 weeks

---

### 6.4: Search UI in Extension (Maybe)

**Current Approach:** Search via MCP (Claude/Cursor chat)

**Proposed (if user demand exists):**
- Sidebar search panel
- Enter search query → show results
- Result preview (code snippet + context)
- Click result → open file at location
- Search history

**Design Questions:**
- Is this needed? (MCP search works well)
- Would users prefer in-editor search or MCP?
- Adds significant UI complexity

**Decision:** Gather user feedback before implementing. MCP-first approach may be sufficient.

**Estimated Effort:** 3-4 weeks (if implemented)

---

### 6.5: Advanced Configuration UI

**Current:** Settings in VSCode settings JSON

**Proposed:**
- WebView settings panel
- Visual provider selection
- API key input with validation
- Docker service status indicators
- One-click setup wizard (re-run)
- Configuration import/export

**Estimated Effort:** 2 weeks

---

## Phase 7: Enterprise Features (6-12 months post-MVP)

### 7.1: Audit Logging

**Why:** Enterprise visibility and compliance

**Features:**
- Structured audit log (JSON lines)
- Log security events:
  - Credential stored/deleted
  - Binary spawned
  - Docker started/stopped
  - Index scanned
- Append-only log file
- Optional SIEM integration (syslog, Splunk)
- Log rotation and retention

**Estimated Effort:** 1-2 weeks

---

### 7.2: Policy Enforcement

**Why:** Enterprise security requirements

**Features:**
- Block indexing of specific file types (`.env`, `.pem`)
- Require approval for new repositories
- Enforce specific embedding providers
- Disable cloud providers (Ollama-only mode)
- Credential rotation policies

**Estimated Effort:** 2-3 weeks

---

### 7.3: Self-Hosted Embedding Service

**Why:** Enterprise air-gapped environments

**Features:**
- Support custom embedding service endpoints
- Compatible with Hugging Face Inference API
- Compatible with vLLM, TGI
- Configuration for custom model endpoints
- Health checks for custom services

**Estimated Effort:** 2-3 weeks

---

### 7.4: Network Proxy Support

**Why:** Corporate proxies

**Features:**
- Respect `HTTP_PROXY`, `HTTPS_PROXY` env vars
- Proxy configuration in settings
- Proxy authentication support
- Proxy bypass for localhost

**Estimated Effort:** 1 week

---

### 7.5: SSO Integration

**Why:** Enterprise authentication

**Features:**
- Authenticate extension via SSO
- SAML/OIDC support
- Token-based activation
- Admin-controlled extension deployment

**Estimated Effort:** 3-4 weeks (requires VSCode API support)

---

## Phase 8: Performance Optimizations (Ongoing)

### 8.1: Lazy Loading

**Current:** All modules loaded on activation

**Proposed:**
- Lazy-load heavy modules (Docker, Indexing)
- Load on-demand when first needed
- Reduce activation time further (<250ms)

**Estimated Effort:** 1 week

---

### 8.2: Incremental Indexing Improvements

**Current:** Incremental via content-addressed deduplication

**Proposed:**
- Track file mtimes for faster skip decisions
- Parallel upserts (spawn multiple binaries)
- Streaming upsert (don't batch, update immediately)

**Estimated Effort:** 2 weeks

---

### 8.3: Database Connection Pooling

**Current:** Binary creates new connection per operation

**Proposed:**
- Long-lived binary process (daemon mode)
- Connection pool reuse
- Faster upserts (no spawn overhead)

**Estimated Effort:** 2-3 weeks (requires Rust binary changes)

---

## Phase 9: User Experience Enhancements (Ongoing)

### 9.1: Progress Improvements

**Current:** Simple percentage notification

**Proposed:**
- File-by-file progress in status bar
- Estimated time remaining
- Pause/resume scanning
- Background scanning (don't block workspace)

**Estimated Effort:** 1 week

---

### 9.2: Better Error Messages

**Current:** Error messages with actions

**Proposed:**
- Contextual help ("Learn More" links to docs)
- In-app troubleshooting wizard
- Automated diagnostics (run checks, suggest fixes)
- Copy error logs button

**Estimated Effort:** 1-2 weeks

---

### 9.3: Onboarding Improvements

**Current:** Setup wizard on first run

**Proposed:**
- Welcome screen with video tutorial
- Interactive tour of features
- Sample workspace for testing
- Guided troubleshooting

**Estimated Effort:** 2 weeks

---

## Decisions Deferred to Post-MVP

### 1. Database Strategy for Multi-Workspace

**Options:**
- A: Single PostgreSQL, multiple repos/worktrees
- B: One PostgreSQL per workspace
- C: Hybrid (shared embeddings, separate metadata)

**Decision:** Defer until multi-workspace implemented

---

### 2. Search UI vs MCP-Only

**Options:**
- A: MCP-only (current approach)
- B: In-extension search UI
- C: Both (MCP + optional UI)

**Decision:** Defer until user feedback collected

---

### 3. Custom Models Dimension Handling

**Options:**
- A: Create new table for different dimensions
- B: Restrict to fixed dimension (1536 for OpenAI)
- C: Dynamic schema migration

**Decision:** Defer until custom models implemented

---

## Feature Prioritization Criteria

**Prioritize features that:**
1. **User-requested:** GitHub issues, feedback, surveys
2. **High-impact:** Significantly improve UX or performance
3. **Low-effort:** Can be implemented in <2 weeks
4. **Complements MCP:** Enhances existing MCP workflows

**Deprioritize features that:**
1. **Niche use cases:** <10% of users
2. **High complexity:** >4 weeks effort
3. **Duplicates MCP:** MCP already does this well
4. **Enterprise-only:** Without enterprise demand

---

## Success Metrics for Post-MVP Features

**Before implementing any post-MVP feature:**
1. **User demand:** >10 GitHub issues OR >50 upvotes
2. **Feasibility:** <4 weeks effort
3. **Compatibility:** Works with MCP, doesn't break MVP
4. **Testing:** Can be tested in CI

**Example:**
- Multi-workspace: High demand, feasible, compatible → Prioritize
- Custom models: Medium demand, feasible, compatible → Consider
- Search UI: Unknown demand, high effort → Defer until feedback

---

## Versioning Strategy

**MVP:** v0.1.0 (VSIX only)
**Phase 5:** v0.2.0 (Marketplace + Windows)
**Phase 6:** v0.3.0-v0.5.0 (Advanced features)
**Phase 7:** v1.0.0 (Enterprise-ready)

**Semantic Versioning:**
- MAJOR: Breaking changes (rare)
- MINOR: New features
- PATCH: Bug fixes

---

## Conclusion

This roadmap contains 25+ post-MVP features spanning:
- **Marketplace & Refinement** (immediate post-MVP)
- **Advanced Features** (3-6 months)
- **Enterprise Features** (6-12 months)
- **Ongoing Optimizations** (continuous)

**MVP Focus:** Ship functional indexing extension, gather feedback, prioritize roadmap based on real user needs.

**Next:** Complete MVP, release VSIX, collect feedback, update roadmap priorities.
