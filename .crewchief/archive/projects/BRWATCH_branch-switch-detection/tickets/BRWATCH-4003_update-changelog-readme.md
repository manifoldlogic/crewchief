# Ticket: BRWATCH-4003: Update CHANGELOG and README

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update project CHANGELOG and README files to document the new automatic branch switch detection feature.

## Background
This ticket implements Step 4.3 from the implementation plan (plan.md - Phase 4). The CHANGELOG and README are the first places users look for new features and breaking changes. We need to document BRWATCH's addition to the project.

**Planning Reference**: `/workspace/.crewchief/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 4.3

## Acceptance Criteria
- [x] CHANGELOG.md updated with new entry for BRWATCH feature
- [x] Entry includes version number (or "Unreleased")
- [x] Entry describes feature, benefits, and usage
- [x] README.md updated to mention automatic indexing
- [x] Quick start or usage section updated
- [x] Links to detailed documentation added
- [x] Changelog follows Keep a Changelog format
- [x] No broken links

## Technical Requirements
- Update `/workspace/CHANGELOG.md` (or create if doesn't exist)
- Update `/workspace/packages/maproom-mcp/README.md` or `/workspace/README.md`
- Follow semantic versioning conventions
- Use clear, concise language
- Include command examples
- Link to new documentation in docs/features/

## Implementation Notes

### CHANGELOG.md Update

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Automatic Branch Switch Detection**: New `maproom watch` command automatically detects branch switches and triggers incremental indexing
  - Watches `.git/HEAD` for changes using OS-level file events
  - Auto-indexes branches within 1 minute of switching
  - Resource efficient: <5% CPU while idle, <20MB memory
  - Graceful shutdown with Ctrl+C
  - See [docs/features/automatic-indexing.md](docs/features/automatic-indexing.md) for usage guide

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A

## [0.2.0] - 2024-11-XX

### Added
- **BRANCHX: Branch-Aware Indexing**: Content-addressed chunk storage with multi-worktree support
  - Incremental updates using git tree SHA optimization
  - Chunk deduplication across branches
  - 8.7ms tree SHA skip performance (91% faster than target)

## [0.1.0] - 2024-10-XX

### Added
- Initial release with semantic code search
- PostgreSQL-backed vector search
- Tree-sitter parsing for TypeScript, Rust
```

### README.md Update

Add to Features section:
```markdown
## Features

- **Semantic Code Search**: Vector-based search using PostgreSQL and embeddings
- **Branch-Aware Indexing (BRANCHX)**: Multi-worktree support with content-addressed chunks
- **Automatic Branch Detection**: Auto-index branches on switch (no manual scan needed) ✨ NEW
- **Incremental Updates**: Fast re-indexing using git tree SHA optimization
- **Tree-sitter Parsing**: Extract symbols from TypeScript, Rust, and more
```

Add to Quick Start or Usage section:
```markdown
## Quick Start

### Automatic Indexing (Recommended)

Start the branch watcher to automatically index as you switch branches:

```bash
# Terminal 1: Start watcher
export DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"
maproom watch --repo /path/to/your/project

# Terminal 2: Work normally, branches auto-index
git checkout feature-auth  # Automatically indexed in <1 minute
```

### Manual Indexing

Alternatively, manually trigger indexing:

```bash
maproom scan --worktree main --repo /path/to/your/project
```

For more details, see [Automatic Indexing Guide](docs/features/automatic-indexing.md).
```

### Documentation Links

Add to README "Documentation" section:
```markdown
## Documentation

- [Automatic Branch Switch Detection](docs/features/automatic-indexing.md) - Auto-indexing setup
- [BRANCHX Architecture](docs/architecture/BRANCHX.md) - Branch-aware indexing design
- [Incremental Updates](docs/architecture/incremental-updates.md) - Fast re-indexing
```

## Dependencies
- BRWATCH-4001 complete (user documentation exists to link to)

## Risk Assessment
- **Risk**: Version number incorrect
  - **Mitigation**: Verify with maintainer, use "Unreleased" if unsure
- **Risk**: README becomes too long
  - **Mitigation**: Keep main README concise, link to detailed docs

## Files/Packages Affected
- `/workspace/CHANGELOG.md` (update or create)
- `/workspace/packages/maproom-mcp/README.md` (update)
- `/workspace/README.md` (update if separate root README)
