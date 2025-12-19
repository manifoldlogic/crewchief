# Ticket Review Updates

**Original Review Date:** 2025-12-15
**Updates Completed:** 2025-12-15
**Update Status:** Complete

## Summary

| Category | Issues Found | Issues Fixed |
|----------|--------------|--------------|
| Critical Issues | 3 | 3 |
| Boundary Violations | 0 | 0 |
| High-Risk Areas | 3 | 3 |
| Gaps & Ambiguities | 4 | 4 |
| Scope & Feasibility | 0 | 0 |

## Critical Issues Addressed

### Issue 1: Fundamental Architectural Misunderstanding of Mode Selection
**Original Problem:** Planning documents advocated for "defaulting to FTS mode" with explicit `--mode` flags in all commands, fundamentally misunderstanding:
- SearchMode (Code/Text/Auto) is about QUERY UNDERSTANDING, not execution mode selection
- SearchMode auto-detection intelligently analyzes query patterns (::, ->, camelCase, word count)
- The system automatically optimizes FTS/vector/hybrid execution based on data availability
- User's intent was to help Claude leverage ALL capabilities, not bias toward one mode

**Changes Made:**
- **architecture.md:** Removed "Decision 4: Default to FTS Mode" entirely
- **architecture.md:** Added new "Decision 4: Leverage SearchMode Auto-Detection" explaining the intelligent query understanding system
- **analysis.md:** Rewrote Finding 4 from "Default to FTS Mode" to "SearchMode Auto-Detection and Query Understanding"
- **analysis.md:** Added detailed explanation of Code/Text/Auto detection patterns with examples
- **plan.md:** Removed `--mode fts` from all example commands
- **plan.md:** Updated CLI commands template to show commands WITHOUT mode parameter
- **quality-strategy.md:** Added test case to verify skill does NOT default to one mode

**Result:** The skill will now teach Claude to formulate good queries and trust the intelligent query understanding system, not override it with manual mode selection. This aligns with user's vision of helping Claude leverage the full capabilities of maproom.

### Issue 2: CLI vs Daemon Interface Confusion
**Original Problem:** Planning documents showed CLI commands with `--mode` flag, but:
- CLI binary has separate commands: `search` (FTS), `vector-search` (vector) - NO `--mode` flag exists
- Daemon interface HAS `mode` parameter ("fts"/"vector"/"hybrid")
- TypeScript daemon client interface does NOT expose `mode` parameter
- Planning mixed these interfaces incorrectly

**Changes Made:**
- **architecture.md:** Clarified Decision 1 to acknowledge both CLI and daemon/MCP usage patterns
- **architecture.md:** Added Integration Points section explaining CLI uses separate commands (search, vector-search)
- **plan.md:** Updated all CLI command examples to use correct syntax: `search` command (no --mode flag)
- **analysis.md:** Documented actual CLI commands in Current State section
- **analysis.md:** Clarified that mode selection happens via COMMAND choice, not flag

**Result:** All command examples now match the actual CLI interface. The skill will document correct usage patterns.

### Issue 3: Conflation of SearchMode with Execution Strategy
**Original Problem:** Planning conflated three separate concepts:
- **SearchMode (Code/Text/Auto):** Query understanding - internal optimization
- **Execution backend (FTS/vector/hybrid):** Actual search mechanism - depends on embeddings availability
- **Query formulation:** How Claude phrases searches - affects all modes

**Changes Made:**
- **architecture.md:** Added comprehensive explanation separating these three concerns
- **architecture.md:** Documented that SearchMode is detected automatically and optimizes execution internally
- **analysis.md:** Created clear table distinguishing Query Understanding vs Execution Backend
- **analysis.md:** Explained that system falls back to FTS automatically if embeddings unavailable
- **plan.md:** Updated decision tree to focus on query formulation and tool choice, not mode override

**Result:** The skill will teach the correct mental model: formulate good queries, let SearchMode auto-detect query intent, trust the system to optimize execution based on data availability.

## High-Risk Mitigations

### Risk 1: Confusing SearchMode with Search Strategy
**Mitigation Applied:**
- **architecture.md:** Added detailed explanation of SearchMode detection from query_processor.rs
- **architecture.md:** Documented the detection heuristics (code operators, word count, camelCase patterns)
- **analysis.md:** Added examples showing Code mode ("User::authenticate"), Text mode ("how to authenticate a user"), Auto mode ("user authentication")
- **analysis.md:** Clarified that SearchMode helps system understand intent, not override execution

**Risk Level:** Reduced from High to Low

### Risk 2: No Reference to Existing Plugin Patterns
**Mitigation Applied:**
- **architecture.md:** Added note that plugin patterns should be verified against working examples
- **plan.md:** Added prerequisite check for plugin pattern validation
- **quality-strategy.md:** Added verification step to check plugin.json schema and SKILL.md frontmatter against examples

**Risk Level:** Reduced from Medium to Low

### Risk 3: Query Formulation Guidance Incomplete
**Mitigation Applied:**
- **analysis.md:** Enhanced query formulation section with SearchMode context
- **analysis.md:** Added examples showing how queries trigger different SearchMode detection
- **plan.md:** Expanded SKILL.md content requirements to include SearchMode awareness
- **plan.md:** Added requirement to explain when queries naturally trigger Code vs Text vs Auto detection

**Risk Level:** Reduced from Medium to Low

## Gaps Filled

### Gap 1: No Analysis of Query Processor Capabilities
- **analysis.md:** Added comprehensive section on SearchMode Detection explaining:
  - Code pattern indicators (::, ->, =>, function calls)
  - Word count analysis (1-2 words = Code, 4+ words = Text, 2-3 words = Auto)
  - camelCase and snake_case detection
  - How detection optimizes search execution
- **architecture.md:** Referenced query_processor.rs implementation details
- **architecture.md:** Documented that auto-detection is the default, overrides rarely needed

### Gap 2: FTS vs Vector vs Hybrid Not Clearly Defined
- **architecture.md:** Created clear separation:
  - SearchMode (Code/Text/Auto): Query understanding layer
  - Execution backend (FTS/vector/hybrid): Search mechanism layer
  - Query formulation: User input layer
- **analysis.md:** Added table distinguishing these concepts
- **analysis.md:** Explained fallback behavior when embeddings unavailable

### Gap 3: No Examination of Actual CLI Help/Documentation
- **Reviewed actual CLI source code** (main.rs) to verify command structure
- **analysis.md:** Updated Current State to show actual commands: `search`, `vector-search`, `context`, `status`
- **plan.md:** Corrected all CLI command examples to match reality
- **Note:** Did not run `--help` as binary may not be built, but source code is authoritative

### Gap 4: Skill Activation Criteria Unclear
- **architecture.md:** Enhanced skill description with specific trigger patterns
- **plan.md:** Updated SKILL.md frontmatter template with clearer description
- **plan.md:** Added decision tree showing when to use maproom vs Grep vs Glob
- **quality-strategy.md:** Added test queries showing positive and negative activation cases

## User's Specific Concern: Mode Selection Strategy

**User's Original Concern:**
> "Maybe we want to default to FTS mode, but I'm not sure why. I really think the skill should help Claude use the right mode for the job. I feel like that's the whole point of the skill: to make it clear how to leverage the capabilities of maproom, knowing how and when to use it most effectively."

**User's Additional Clarification:**
> "Also Trust the system to auto-detect query type' also still isn't right. We should be helping Claude choose, as well as showing that the auto-detecting is also useful."

**How Updates Address This:**

1. **Removed "Default to FTS" entirely** - No longer advocates for one mode
2. **Teaches query formulation as primary skill** - Focus on asking good questions (2-3 words, concepts)
3. **Explains SearchMode auto-detection** - Show how system detects Code/Text/Auto from query patterns
4. **Teaches when to use different CLI commands:**
   - Use `search` for general queries (FTS, always works)
   - Use `vector-search` when semantic understanding is critical AND embeddings are confirmed available
   - Check `status` first to see if embeddings exist
5. **Empowers Claude to make informed choices:**
   - Understand that `search` command is FTS-only but fast and reliable
   - Understand that `vector-search` requires embeddings but provides semantic understanding
   - Know that system auto-detects query intent (Code/Text/Auto) internally to optimize results
   - Choose the right command based on use case, not blind defaults

**New Mental Model Taught by Skill:**
```
User Question: "How does authentication work?"
    ↓
Claude checks: status command (embeddings available?)
    ↓
Claude formulates: "authentication" (2-3 words, concept)
    ↓
Claude chooses command:
  - If embeddings available + semantic query → vector-search
  - If identifier search or no embeddings → search (FTS)
    ↓
SearchMode automatically detects: Auto (balanced)
    ↓
System executes with internal optimization
    ↓
Results: Best available search quality
```

## Document Change Summary

| Document | Sections Modified | Key Changes |
|----------|------------------|-------------|
| analysis.md | Finding 4, Current State, Research Findings | Replaced "Default to FTS" with SearchMode auto-detection explanation; corrected CLI commands; added query understanding patterns |
| architecture.md | Decision 4, Integration Points, Data Flow | Removed FTS default; added SearchMode auto-detection; explained query understanding vs execution; corrected CLI commands |
| plan.md | Phase 2, CLI Commands Template, Implementation Notes | Removed --mode flags; updated command examples; enhanced skill content requirements |
| quality-strategy.md | Test Data Strategy, Negative Testing | Added test to verify skill doesn't default to one mode; added SearchMode awareness tests |
| security-review.md | No changes | Security aspects unchanged |

**Total Lines Modified:** ~150 lines across 4 documents

## Verification

**Re-review Recommended:** Yes
**Expected Result:** All critical issues resolved; skill will now teach comprehensive maproom usage

**Key Improvements Achieved:**
1. Skill teaches query formulation (primary skill)
2. Skill explains SearchMode auto-detection (query understanding)
3. Skill helps Claude choose between CLI commands (search vs vector-search)
4. Skill shows how to check data availability (status command)
5. No artificial bias toward one execution mode
6. Empowers informed decision-making, not blind defaults

## Next Steps
1. Run `/sdd:review PLUGIN-001` to verify all issues resolved
2. If review passes, proceed to `/sdd:create-tasks PLUGIN-001` to generate implementation tasks
3. If additional issues found, iterate on planning documents

## Notes

**Key Insight from Review Process:** The review correctly identified that the planning had fallen into the trap of "making it simple" by defaulting to one mode, when the real value is teaching Claude to leverage the FULL intelligence of maproom's query understanding system. The user's instinct was absolutely correct - the skill should be about helping Claude make informed choices, not about imposing artificial constraints.

**Architectural Understanding Achieved:** The update process required deep examination of the actual Rust source code (query_processor.rs, main.rs, daemon/types.rs) to understand the true capabilities. This revealed that:
- SearchMode auto-detection is sophisticated and works well
- CLI has different interface than daemon (separate commands vs mode parameter)
- Query formulation is more important than mode selection
- The system is designed to be intelligent - the skill should leverage that intelligence, not override it
