# Ticket: Maproom Skill Progressive Disclosure

**Ticket ID:** MPRSKL
**Status:** Planning Complete
**Created:** 2025-12-19

## Summary

This ticket addresses three interconnected issues affecting maproom usability for AI agents:

1. **Factory/Config Dimension Mismatch Bug (CRITICAL)**: When Ollama is auto-detected via network, the factory doesn't propagate the detected provider to config, causing dimension mismatch errors (expects 1536, gets 1024).

2. **Progressive Disclosure Skill Architecture**: Current skill documentation is comprehensive (196 lines) but not optimized for AI agent consumption. Needs restructuring with brief SKILL.md and layered reference docs.

3. **CLI Error Messages**: Dimension mismatch errors lack actionable guidance for diagnosis and recovery.

## Problem Statement

The dimension mismatch bug occurs because:
- Factory (`factory.rs`) auto-detects Ollama via network probe
- Factory calls `EmbeddingConfig::from_env()` without passing detected provider
- Config defaults to `Provider::OpenAI` (no env var set)
- Dimension inference only triggers when `provider == Provider::Ollama`
- Result: dimension stays at 1536 (OpenAI default) but Ollama returns 1024-dim embeddings

## Proposed Solution

1. **Bug Fix**: Add `EmbeddingConfig::from_env_with_provider(Option<Provider>)` to accept runtime-detected provider. Factory passes `Some(Provider::Ollama)` when auto-detecting.

2. **Skill Restructure**: Brief SKILL.md (<50 lines) with layered references:
   - `search-best-practices.md` (existing)
   - `cli-reference.md` (new)
   - `troubleshooting.md` (new)

3. **Error Enhancement**: Include configuration context and fix suggestions in error messages.

## Relevant Agents

- ticket-planner (planning phase) - COMPLETE
- rust-engineer (Phase 1: bug fix, Phase 3: error messages)
- docs-writer (Phase 2: skill restructure)
- verify-task (verification)
- commit-task (commit)

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis with root cause identification
- [architecture.md](planning/architecture.md) - Solution design with decision rationale
- [plan.md](planning/plan.md) - Phased execution plan with 7 tasks
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach and coverage requirements
- [security-review.md](planning/security-review.md) - Security assessment (low risk)

## Key Files to Modify

**Rust Code:**
- `crates/maproom/src/embedding/config.rs` - Add `from_env_with_provider()`
- `crates/maproom/src/embedding/factory.rs` - Use new constructor for Ollama
- `crates/maproom/src/embedding/error.rs` - Enhanced error messages
- `crates/maproom/src/main.rs` - CLI help text improvements

**Documentation:**
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md` - Brief version
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/cli-reference.md` - NEW
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/troubleshooting.md` - NEW

## Task Summary

| ID | Phase | Title | Priority |
|----|-------|-------|----------|
| MPRSKL-1001 | 1 | Add from_env_with_provider() to EmbeddingConfig | Critical |
| MPRSKL-1002 | 1 | Update factory.rs to propagate provider | Critical |
| MPRSKL-2001 | 2 | Create brief SKILL.md | High |
| MPRSKL-2002 | 2 | Create cli-reference.md | High |
| MPRSKL-2003 | 2 | Create troubleshooting.md | High |
| MPRSKL-3001 | 3 | Enhance dimension mismatch error | Medium |
| MPRSKL-3002 | 3 | Improve --generate-embeddings documentation | Medium |

## Success Criteria

- [ ] `crewchief-maproom scan` works without env vars when Ollama is running locally
- [ ] Dimension is correctly detected as 1024 for mxbai-embed-large
- [ ] SKILL.md is under 50 lines
- [ ] All skill references/ files exist and are linked
- [ ] Error messages include actionable guidance
- [ ] All existing tests pass
- [ ] New integration test for auto-detection flow passes

## Next Step

Run `/sdd:review MPRSKL` to validate planning documents before task creation.

## Tasks

See [tasks/](tasks/) for all ticket tasks (created after review).
