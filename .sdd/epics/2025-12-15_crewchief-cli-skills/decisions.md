# Decisions: CrewChief CLI Plugins

Running log of key decisions made during this epic.

---

## Decisions

### [2025-12-15] Use Plugin Architecture Instead of Standalone Skills

**Context:** Initially planned to create skills in `.agent/skills/`. After review, determined that the crewchief marketplace at `.crewchief/claude-code-plugins/` provides a better architecture.

**Decision:** Create two separate plugins (`maproom`, `worktree`) in the crewchief marketplace instead of standalone skills.

**Rationale:**
- Plugin architecture allows users to enable/disable capabilities per project
- Each plugin is independent with its own version
- Follows established patterns from existing plugins (workstream, github-actions, sdd)
- Plugins are discoverable via `/plugin install`
- Skills within plugins still use the same SKILL.md format

**Alternatives Considered:**
- Standalone skills in `.agent/skills/`: Rejected because it doesn't integrate with the plugin marketplace and lacks versioning/discoverability
- Single combined plugin: Rejected because maproom and worktree serve different use cases and should be independently installable

---

### [2025-12-15] Use CLI Instead of MCP for Skills

**Context:** Both maproom-mcp and crewchief-maproom CLI provide search/indexing capabilities. Need to decide which interface skills should use.

**Decision:** Skills will use CLI commands directly, not MCP protocol.

**Rationale:**
- CLI has full parity with MCP (all features available)
- CLI adds database management commands MCP doesn't expose (migrate, cleanup-stale, clean-ignored)
- CLI can be invoked via Bash tool without daemon management
- Simpler skill design without subprocess/daemon lifecycle handling

**Alternatives Considered:**
- MCP protocol: Rejected because it requires spawning and managing a daemon process, adding complexity without additional capability
- Direct database access: Rejected because it would bypass the optimized Rust implementation

---

### [2025-12-15] Two Separate Plugins (Not Combined)

**Context:** Could create one combined "crewchief" plugin or two separate plugins for maproom and worktree functionality.

**Decision:** Create two separate plugins: `maproom` and `worktree`.

**Rationale:**
- Different trigger patterns (search queries vs worktree management requests)
- Modular design allows independent updates and installation
- Clearer skill descriptions for model-invoked discovery
- Worktree plugin could be useful in projects without maproom
- Maproom plugin could be useful in projects without worktree needs

**Alternatives Considered:**
- Combined plugin: Rejected because it would force users to get both capabilities even if they only need one
- Three+ plugins: Rejected as unnecessary granularity; context and search are tightly coupled within maproom

---

### [2025-12-15] Link to CLAUDE.md for CLI Documentation

**Context:** CLI commands have detailed documentation in CLAUDE.md files. Skills need to provide command reference.

**Decision:** Link to existing CLAUDE.md files for authoritative CLI documentation; embed only decision trees and query formulation in SKILL.md.

**Rationale:**
- Single source of truth for CLI commands (no documentation drift)
- Keeps SKILL.md lean for frequent context loading
- CLAUDE.md files already maintained by development team

**Alternatives Considered:**
- Duplicate CLI docs in skill: Rejected due to maintenance burden and potential drift
- Full embedding in SKILL.md: Rejected as too verbose for skill discovery phase

---

### [2025-12-15] Default to FTS Search Mode

**Context:** Three search modes exist (fts, vector, hybrid) with different requirements and performance characteristics.

**Decision:** Skills should default to `fts` mode, recommend `hybrid` only when embeddings are confirmed available.

**Rationale:**
- FTS always works (no embedding requirement)
- FTS is fastest
- Hybrid/vector modes require embeddings to be generated first
- Better to get results than fail due to missing embeddings

**Alternatives Considered:**
- Default to hybrid: Rejected because it may fail silently if embeddings don't exist
- Auto-detect embeddings: Rejected as it adds complexity; status command can check this

---

### [2025-12-15] Plugin Location in Marketplace

**Context:** Need to determine where plugins should be created within the repository structure.

**Decision:** Create plugins at `.crewchief/claude-code-plugins/plugins/{plugin-name}/`.

**Rationale:**
- Follows existing marketplace structure
- Consistent with other plugins (workstream, github-actions, sdd)
- Enables installation via `/plugin install {name}@crewchief`
- Automatically discovered by Claude Code when marketplace is configured

**Alternatives Considered:**
- Separate repository: Rejected as unnecessary for two simple plugins
- Different marketplace: Rejected because crewchief marketplace already exists and is configured

---

### [2025-12-15] Include Reference Document in Maproom Plugin

**Context:** Maproom search has extensive query formulation patterns and tool selection guidance. Need to decide how to structure this documentation.

**Decision:** Create `search-best-practices.md` in maproom skill's `references/` directory.

**Rationale:**
- Progressive disclosure: SKILL.md contains essentials, reference has details
- Follows Claude Code skill architecture pattern
- Keeps SKILL.md under recommended size limits
- Reference loaded on-demand when Claude needs more examples

**Alternatives Considered:**
- All in SKILL.md: Rejected as it would make the skill too large
- External link only: Rejected because reference should be bundled with plugin

---

## Decision Template

### [DATE] Decision Title

**Context:** [Why this decision was needed]

**Decision:** [What was decided]

**Rationale:** [Why this choice]

**Alternatives Considered:**
- [Option A]: [Why rejected]
- [Option B]: [Why rejected]
