# Ticket: SQLINFRA-1003: Update README Quick Start

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- This is a documentation ticket; manual smoke testing required
- Commands in Quick Start must be tested to verify they work

## Agents
- general-purpose
- unit-test-runner (N/A - manual smoke test)
- verify-ticket
- commit-ticket

## Summary
Rewrite the README.md Quick Start section to present SQLite as the default, zero-configuration option for Maproom semantic search.

## Background
The README.md currently requires Docker/PostgreSQL setup for the Quick Start path, creating an unnecessarily complex onboarding experience. With SQLite fully implemented, users can achieve semantic code search with zero external dependencies.

This is the highest-visibility documentation in the project and directly impacts new user experience. The goal is to reduce "time to first search" from ~10 minutes (Docker setup) to ~2 minutes (immediate use).

Reference: [SQLINFRA Plan - Phase 2](../planning/plan.md#phase-2-core-documentation)

## Acceptance Criteria
- [ ] Quick Start section works without Docker or PostgreSQL installed
- [ ] All Quick Start commands execute successfully on a clean machine
- [ ] PostgreSQL path is still documented (moved to "Advanced" section)
- [ ] Requirements section clearly shows SQLite as default, PostgreSQL as optional
- [ ] Brief explanation of SQLite benefits included

## Technical Requirements
- **Quick Start Section**:
  - Remove Docker/PostgreSQL prerequisites
  - Show immediate `crewchief maproom:scan` and `crewchief maproom:search` commands
  - Explain that database is auto-created at `~/.maproom/maproom.db`

- **Requirements Section**:
  - Update to show: Node.js >= 18, Git (required)
  - PostgreSQL/Docker marked as optional ("for team sharing")

- **New "Advanced: PostgreSQL Setup" Section**:
  - Move existing PostgreSQL setup instructions here
  - Link to detailed documentation in `docs/`
  - Explain when PostgreSQL is beneficial (team sharing, concurrent access)

- **SQLite Benefits (brief)**:
  - Zero configuration
  - No Docker required
  - Full feature parity with PostgreSQL for individual use

## Implementation Notes
- Current README structure at `/workspace/README.md`
- Target structure:
  ```markdown
  ## Quick Start (SQLite - Recommended)
  [2-3 command sequence that just works]

  ## Requirements
  - Node.js >= 18
  - Git
  - **Optional**: Docker (for PostgreSQL team sharing)

  ## Advanced: PostgreSQL Setup (Team Sharing)
  [Moved from current Quick Start, with link to docs]
  ```
- Preserve all existing content - reorganize, don't delete
- Test commands manually before marking complete:
  ```bash
  rm -rf ~/.maproom/
  crewchief maproom:scan /path/to/small/repo
  crewchief maproom:search "function"
  ```

## Dependencies
- None - can start immediately
- Independent of Phase 1 CI tickets

## Risk Assessment
- **Risk**: Quick Start commands fail on edge cases
  - **Mitigation**: Test on clean environment; document known requirements
- **Risk**: PostgreSQL users can't find their path
  - **Mitigation**: Clear section header; cross-link from Quick Start
- **Risk**: README becomes too long
  - **Mitigation**: Keep Quick Start concise; link to detailed docs

## Files/Packages Affected
- `README.md` - Major restructure of Quick Start and Requirements sections
