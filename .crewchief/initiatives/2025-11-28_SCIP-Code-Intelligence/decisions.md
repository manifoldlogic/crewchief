# Decisions: SCIP-Based Code Intelligence

Running log of key decisions made during this initiative.

## Template

### [{DATE}] Decision Title

**Context:** [Why this decision was needed]

**Decision:** [What was decided]

**Rationale:** [Why this choice]

**Alternatives Considered:**
- [Option A]: [Why rejected]
- [Option B]: [Why rejected]

---

## Decisions

### [2025-11-28] Use SCIP over Live LSP

**Context:** Need code intelligence for AI agents. Two main approaches: wrap live LSP servers or consume pre-computed indexes.

**Decision:** Build SCIP consumption layer instead of wrapping live LSP.

**Rationale:**
1. Aligns with Maproom's local-first, offline-capable philosophy
2. Lower resource usage (~100MB SQLite vs GB of LSP servers)
3. Instant startup vs seconds of LSP initialization
4. Unique value proposition — no existing tool does this

**Alternatives Considered:**
- Live LSP wrapper: Works today but heavy resources, complex setup
- Tree-sitter queries: Fast but can't do cross-file resolution

---

### [2025-11-28] Limit Initial Scope to TS/Rust/Python

**Context:** SCIP indexers exist for 8+ languages. Could support all, or focus on subset.

**Decision:** Support TypeScript, Rust, and Python initially.

**Rationale:**
1. These languages have the most mature SCIP indexers
2. Covers majority of Maproom user base
3. Validates multi-language architecture without over-extending
4. Go/Java can be added in future initiative

**Alternatives Considered:**
- All languages: Higher risk, longer timeline
- TypeScript only: Misses validation of multi-language support

---

### [2025-11-28] Exclude Call Hierarchy from Scope

**Context:** SCIP data can support call hierarchy queries. Could include in this initiative.

**Decision:** Defer call hierarchy to future initiative.

**Rationale:**
1. Call hierarchy requires recursive graph queries — significant complexity
2. Core navigation (goto_def, find_refs) provides immediate value
3. Clear scope boundary reduces risk
4. Can be added after foundation is proven

**Alternatives Considered:**
- Include call hierarchy: Scope creep risk, longer timeline

---

### [2025-11-28] Store in Existing Maproom SQLite Database

**Context:** Could create separate database for SCIP data or extend existing schema.

**Decision:** Extend existing Maproom database with scip_* tables.

**Rationale:**
1. Single database simplifies deployment and backup
2. Enables future joint queries (semantic search + code intelligence)
3. Leverages existing worktree/repo management
4. Consistent with Maproom architecture

**Alternatives Considered:**
- Separate database: Additional complexity, harder to join data
- In-memory only: Loses persistence, slow startup

---

(Additional decisions added as they occur)
