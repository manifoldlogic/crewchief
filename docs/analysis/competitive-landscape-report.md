# Competitive Landscape Report: CrewChief/Maproom vs. Industry Alternatives

**Date:** 2026-03-21
**Scope:** Six competitors evaluated across six dimensions, grounded in codebase metrics from `/workspace/repos/crewchief/CRITIQUE/`

---

## Validated Codebase Metrics

All metrics were gathered by running commands against the repository. Where planning estimates differ from actual counts, both are noted.

| Metric | Planning Estimate | Actual | Command |
|--------|------------------|--------|---------|
| Rust `#[test]` annotations | ~2,780 | **2,148** | `grep -r '#\[test\]' crates/ \| wc -l` |
| Rust files with tests | ~240 | **~200** | `grep -c '#[test]' crates/**/*.rs \| grep -v ':0$' \| wc -l` |
| TypeScript test files | 56 | **147** | `find packages/ -name '*.test.ts' \| wc -l` |
| Benchmark suites | 21 | **20** | `ls crates/maproom/benches/ \| wc -l` |
| Cargo.toml dependencies | -- | **57** | `sed -n '/\[dependencies\]/,/^\[/p' Cargo.toml \| grep -c '='` |
| package.json dependencies | -- | **11** | parsed via Python json module |
| Total LOC (Rust + TypeScript) | -- | **~256,800** | `find crates/ packages/ -name '*.rs' -o -name '*.ts' \| xargs wc -l` |

Planning overestimated Rust tests by ~630 and underestimated TS tests by ~91 (the 147 includes 30+ search-optimization test files). Benchmark count is 20, not 21. These discrepancies are modest: Rust test coverage is strong, TypeScript coverage is broader than assessed but lacks E2E depth.

---

## Comparison Matrix

Rating scale: **Strong** (clear advantage), **Adequate** (competitive parity), **Weak** (meaningful disadvantage), **N/A** (not applicable).

| Dimension | CrewChief/Maproom | Claude Code | CocoIndex (ccc) | Sourcegraph/Cody | Aider | Cursor/Windsurf | GitHub Copilot Workspace |
|-----------|------------------|-------------|-----------------|------------------|-------|-----------------|--------------------------|
| **Platform reach** | Weak | Strong | Strong | Strong | Strong | Adequate | Adequate |
| **Setup complexity** | Weak | Strong | Strong | Adequate | Strong | Adequate | Strong |
| **Search quality** | Adequate | Adequate | Adequate | Strong | Weak | Adequate | Adequate |
| **Agent orchestration** | Strong | Adequate | N/A | N/A | Weak | Adequate | Strong |
| **Distribution model** | Weak | Strong | Strong | Adequate | Strong | Adequate | Strong |
| **Maintenance burden** | Adequate | Strong | Adequate | Strong | Strong | Strong | Strong |

### Cell Rationale

**Platform reach.** CrewChief defaults to iTerm2 on macOS (`iterm.ts`). tmux backend exists and is tested (`tmux.ts`, `tmux.test.ts`, `tmux.integration.test.ts`) but not surfaced as default Linux/Windows path. All competitors work cross-platform without backend selection. GitHub Copilot Workspace is cloud-only (browser-based); any OS but no offline mode.

**Setup complexity.** CrewChief requires Node.js + bundled Rust binary + optional embedding provider (Ollama ~4GB or API key). Current default requires embedding config before first search. CocoIndex: single binary, zero config. Claude Code: built-in. Aider: `pip install`. Sourcegraph: server deployment. Cursor: IDE download. Copilot Workspace: zero local setup, cloud-hosted via GitHub account.

**Search quality.** FTS via SQLite FTS5, optional vector search, tree-sitter AST chunking (10+ languages). Competition benchmark: 162/180 vs. 152/180 grep baseline, 37.9 vs. 54.8 tool calls (`packages/cli/src/search-optimization/`). Sourcegraph's SCIP is more mature at scale. CocoIndex uses AST without embeddings. Copilot Workspace leverages GitHub's code graph within cloud sessions.

**Agent orchestration.** Only tool offering worktree-per-agent isolation with multi-platform spawning (Claude, Gemini, Aider, Codex), inter-agent messaging (`packages/cli/src/bus/message.types.ts`), and token-budgeted context assembly (`crates/maproom/src/context/assembler.rs`). Claude Code subagents share filesystem. Cursor agents are IDE-bound. Copilot Workspace runs parallel cloud agents in isolated sessions, enterprise-backed.

**Distribution model.** Two-language architecture: TypeScript CLI + Rust binaries for 5 platforms, 57 Cargo + 11 npm dependencies. CocoIndex, Aider, Claude Code each ship as single-language packages. Copilot Workspace is fully hosted with no user-managed dependencies.

**Maintenance burden.** 2,148 Rust test annotations, 147 TS test files, 20 benchmarks -- strong coverage. Gaps: no E2E tests, no CI coverage threshold, two stale packages (maproom-mcp mislabeled `[DEPRECATED]`, vscode-maproom genuinely deprecated), PostgreSQL documentation drift. Two ecosystems double dependency maintenance. Copilot Workspace maintenance is handled by GitHub/Microsoft.

---

## Where CrewChief Wins

1. **Worktree-per-agent isolation is unique.** No competitor provides automated worktree creation, agent spawning into isolated branches, and coordinated merge-back in a single CLI. Claude Code subagents share a filesystem; Cursor agents work within a single IDE session. CrewChief's `scheduler.ts` creates the worktree, opens the terminal, and injects environment variables.

2. **Token-budgeted context assembly is differentiated.** The `--budget 4000 --callers --callees --tests` flag produces a context bundle sized to an LLM's window, including graph-traversed related code. No competitor offers this. The benchmark (162/180 vs. 152/180, 37.9 vs. 54.8 tool calls) evidences changed agent behavior (`packages/cli/src/search-optimization/`).

3. **Agent-format output contract is well-designed.** The `--format agent` mode with structured exit codes (0=success, 1=runtime, 2=config) is purpose-built for LLM consumption. Most competitors lack a machine-readable output format for agent tool use.

4. **Multi-agent platform support.** CrewChief can orchestrate Claude, Gemini, Aider, and Codex agents simultaneously across isolated worktrees. Every other tool is either single-agent or locked to one AI platform.

---

## Where Competitors Win

1. **Zero-setup search (Claude Code, CocoIndex).** Both require zero search configuration. CrewChief's default requires embedding provider setup before first search -- the single largest adoption barrier.

2. **Platform reach (all competitors).** Every competitor works cross-platform without backend selection. CrewChief's iTerm2 default limits its primary differentiator (agent orchestration) to macOS/iTerm2 unless users know to pass `--backend tmux`.

3. **Distribution simplicity (CocoIndex, Aider, Claude Code).** Single-language distribution avoids two-ecosystem maintenance. CrewChief's `maproom.ts` passthrough (~150 lines of `spawnSync`) exposes the TypeScript/Rust seam to users.

4. **Enterprise scale (Sourcegraph).** Multi-repo, multi-user search is outside CrewChief's scope (single-repo SQLite at `~/.maproom/maproom.db`).

5. **IDE integration (Cursor/Windsurf).** Inline editor integration. CrewChief's VSCode extension is deprecated (`packages/vscode-maproom/`).

---

## Adoption Barrier Summary

The planning analyses identified three break points in the adoption funnel, validated by codebase evidence:

| Break Point | Layer | Codebase Evidence | Competitive Impact |
|-------------|-------|-------------------|-------------------|
| First contact: no demo, no social proof | Safety | No screencast or demo command exists in the codebase; README is the only documentation | Claude Code, Cursor, and Aider all have prominent demos or integrated first-run experiences |
| Setup: embedding provider blocks basic usage | Competence | `autoScanOnWorktreeUse` defaults `false` (`packages/cli/src/git/worktrees.ts:157`); default scan requires embedding configuration; FTS handles 88% of agent search calls | CocoIndex and Claude Code require zero search configuration |
| Day 3-7: cannot answer "why not Claude Code native?" | Meaning | Three of five system integration points are absent (`scheduler.ts` never calls Maproom; bus events are inert; MCP socket not injected at spawn) -- the integrated workflow that justifies the tool over native alternatives does not activate automatically | Claude Code native subagents are free, built-in, and zero-configuration |

The psychology analysis maps the setup barrier to a "Competence Quit." The constraint-transmutation analysis identifies the fix: default to FTS-only (removing embedding provider from critical path) and surface tmux as the non-macOS default. These require changing defaults, not building new features.

---

## Methodology Notes

- All CrewChief metrics are from the codebase at `/workspace/repos/crewchief/CRITIQUE/` as of 2026-03-21.
- Competitor assessments are based on publicly available information as of the assessment date. AI tooling moves fast; ratings may shift.
- The matrix rates relative positioning, not absolute quality. "Adequate" means parity with the field.
- Source material: architecture.md, prd.md, quality-strategy.md, constraint-transmutation.md, psychology-analysis.md, onboarding-dx-analysis.md, systems-design-critique.md (all in planning/).
