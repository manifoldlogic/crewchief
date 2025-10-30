# Maproom Project Overview

## Executive Summary

Maproom is a code-aware indexing and retrieval system that requires completion across seven distinct projects. Based on the project boundary evaluation framework, each project has been designed with stable interfaces, coherent context, and testable completion criteria.

This overview provides the recommended implementation order, dependencies, and critical path for completing Maproom to production readiness.

## Project Definitions

### 1. HYBRID_SEARCH - Hybrid Retrieval System
**Purpose:** Implement state-of-the-art search combining FTS, vectors, and graph signals
**Complexity:** High
**Duration:** 6 weeks
**Critical:** Yes - Core functionality

### 2. MCP_CORE - MCP Server Implementation
**Purpose:** Complete the MCP server with all tools for AI agent integration
**Complexity:** Medium
**Duration:** 3 weeks
**Critical:** Yes - User interface

### 3. CONTEXT_ASM - Context Assembly Engine
**Purpose:** Build budget-aware context bundling for LLMs
**Complexity:** High
**Duration:** 6 weeks
**Critical:** Yes - Core value proposition

### 4. INC_INDEX - Incremental Indexing Pipeline
**Purpose:** Enable real-time updates with file watching
**Complexity:** Medium
**Duration:** 4 weeks
**Critical:** Yes - User experience

### 5. LANG_PARSE - Multi-Language Parser Support
**Purpose:** Add Python, Rust, and Go language support
**Complexity:** Medium
**Duration:** 7 weeks
**Critical:** No - Enhancement

### 6. PERF_OPT - Performance Optimization
**Purpose:** Achieve performance targets through systematic optimization
**Complexity:** Medium
**Duration:** 5 weeks
**Critical:** No - Quality

### 7. MD_ENHANCE - Enhanced Markdown Support
**Purpose:** Upgrade to tree-sitter markdown parsing
**Complexity:** Low
**Duration:** 4 weeks
**Critical:** No - Enhancement

## Recommended Implementation Order

### Phase 1: Core Foundation (Weeks 1-8)
**Goal:** Establish the essential search and retrieval capabilities

#### Parallel Track A: HYBRID_SEARCH (Weeks 1-6)
- **Why First:** Foundation for all search operations
- **Dependencies:** None
- **Deliverables:** Functioning hybrid search with >80% recall
- **Agents:** database-engineer, embeddings-engineer, search-quality-engineer

#### Parallel Track B: MCP_CORE (Weeks 1-3)
- **Why Early:** Enables testing and user interaction
- **Dependencies:** Basic search (from HYBRID_SEARCH Week 2)
- **Deliverables:** All 5 MCP tools operational
- **Agents:** mcp-tools-engineer

#### Sequential: CONTEXT_ASM (Weeks 3-8)
- **Why After Search:** Depends on search results for context
- **Dependencies:** HYBRID_SEARCH query capabilities
- **Deliverables:** Budget-aware context assembly
- **Agents:** mcp-context-engineer, database-engineer

### Phase 2: Production Features (Weeks 9-14)
**Goal:** Add essential production capabilities

#### INC_INDEX (Weeks 9-12)
- **Why Now:** Critical for developer experience
- **Dependencies:** Core indexing pipeline
- **Deliverables:** Real-time incremental updates
- **Agents:** rust-indexer-engineer

#### PERF_OPT - First Pass (Weeks 13-14)
- **Why Now:** Baseline optimization before adding features
- **Dependencies:** All core components
- **Deliverables:** Meet basic performance targets
- **Agents:** performance-engineer

### Phase 3: Enhancements (Weeks 15-22)
**Goal:** Expand capabilities and quality

#### LANG_PARSE (Weeks 15-21)
- **Why Now:** Core system stable, can add languages
- **Dependencies:** Stable indexing pipeline
- **Deliverables:** Python, Rust, Go support
- **Agents:** parser-engineer

#### MD_ENHANCE (Weeks 18-21) - Parallel
- **Why Parallel:** Independent of language parsing
- **Dependencies:** Basic markdown already works
- **Deliverables:** Tree-sitter markdown parsing
- **Agents:** parser-engineer

### Phase 4: Final Optimization (Weeks 22-24)
**Goal:** Production-ready performance

#### PERF_OPT - Final Pass (Weeks 22-24)
- **Why Last:** Optimize the complete system
- **Dependencies:** All features implemented
- **Deliverables:** All performance targets exceeded
- **Agents:** performance-engineer, database-engineer

## Critical Path

```
HYBRID_SEARCH → CONTEXT_ASM → MCP_CORE(complete) → INC_INDEX
                     ↓
                Production Ready (Week 12)
                     ↓
         LANG_PARSE + MD_ENHANCE + PERF_OPT
                     ↓
              Feature Complete (Week 24)
```

## Dependencies Graph

```
Foundation Layer:
├── HYBRID_SEARCH (no dependencies)
└── MCP_CORE (partial dependency on HYBRID_SEARCH)

Core Features Layer:
├── CONTEXT_ASM (depends on HYBRID_SEARCH)
└── INC_INDEX (depends on core indexing)

Enhancement Layer:
├── LANG_PARSE (depends on stable pipeline)
├── MD_ENHANCE (depends on basic parsing)
└── PERF_OPT (depends on all features)
```

## Risk Analysis

### High Risk Items
1. **HYBRID_SEARCH complexity** - Mitigation: Start early, extensive testing
2. **CONTEXT_ASM token accuracy** - Mitigation: Conservative estimates
3. **INC_INDEX race conditions** - Mitigation: Thorough concurrency testing

### Medium Risk Items
1. **LANG_PARSE grammar compatibility** - Mitigation: Incremental language addition
2. **PERF_OPT target achievement** - Mitigation: Two-pass approach
3. **MCP_CORE client compatibility** - Mitigation: Early integration testing

### Low Risk Items
1. **MD_ENHANCE** - Well-understood, clear upgrade path
2. **Basic features** - Most infrastructure exists

## Resource Allocation

### Agent Utilization Timeline
- **Weeks 1-6:** database-engineer (80%), embeddings-engineer (60%), mcp-tools-engineer (40%)
- **Weeks 7-12:** mcp-context-engineer (80%), rust-indexer-engineer (60%)
- **Weeks 13-21:** parser-engineer (80%), performance-engineer (40%)
- **Weeks 22-24:** performance-engineer (80%), integration-tester (60%)

### Critical Agent Gaps
**Must Hire/Develop:**
1. graph-algorithms-engineer (Weeks 3-8)
2. vector-database-engineer (Weeks 1-6)
3. caching-engineer (Weeks 13-24)

## Success Metrics

### Phase 1 Success (Week 8)
- [ ] Search recall >80%
- [ ] Basic MCP tools working
- [ ] Context assembly functional

### Phase 2 Success (Week 14)
- [ ] Incremental indexing <5s
- [ ] Search latency <100ms p95
- [ ] Production viable

### Phase 3 Success (Week 21)
- [ ] 3+ languages supported
- [ ] Enhanced markdown parsing
- [ ] Feature complete

### Phase 4 Success (Week 24)
- [ ] All performance targets met
- [ ] <50ms search p95
- [ ] 150+ files/min indexing
- [ ] Production ready

## Budget Estimates

### Development Effort
- **Total:** ~24 weeks for sequential development
- **Parallel Optimization:** ~18 weeks with 2-3 parallel tracks
- **Agent Hours:** ~2,880 hours (3 agents × 40 hours × 24 weeks)

### Infrastructure Costs
- **Embeddings:** $100-300/month
- **PostgreSQL:** Existing infrastructure
- **Development:** No additional costs

### Ongoing Operational Costs
- **Embeddings:** $100/month per million chunks
- **Storage:** ~10GB per million chunks
- **Compute:** Minimal with proper optimization

## Go/No-Go Decision Points

### Week 6 Checkpoint
**Decision:** Continue with advanced features?
- Hybrid search working with >70% recall → GO
- Hybrid search failing → PIVOT to simpler approach

### Week 12 Checkpoint
**Decision:** Ready for production features?
- Core features stable → GO to Phase 3
- Significant issues → STABILIZE first

### Week 18 Checkpoint
**Decision:** Expand languages or optimize?
- Performance acceptable → EXPAND languages
- Performance poor → FOCUS on optimization

## Conclusion

The Maproom project is well-structured with clear boundaries and dependencies. The recommended implementation order prioritizes core functionality (search and context assembly) before adding enhancements. This approach ensures:

1. **Early Value Delivery:** Basic system functional by Week 8
2. **Risk Mitigation:** Complex projects tackled first
3. **Parallel Efficiency:** Multiple tracks where possible
4. **Clear Milestones:** Measurable success at each phase

Following this plan, Maproom will achieve production readiness in approximately 24 weeks with full feature completion, meeting all performance targets, and supporting multiple languages.

The highest risk is the complexity of HYBRID_SEARCH and CONTEXT_ASM, but starting these early and in parallel mitigates this risk. The missing agents (particularly graph-algorithms-engineer and vector-database-engineer) should be acquired or developed as soon as possible to prevent blocking critical path items.

With proper execution, Maproom will provide best-in-class code indexing and retrieval, enabling powerful AI-assisted development workflows.