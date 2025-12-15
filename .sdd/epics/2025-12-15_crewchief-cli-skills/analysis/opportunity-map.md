# Opportunity Map: CrewChief CLI Plugins

## Problem Spaces

### Problem 1: Undiscoverable CLI Capabilities

**Description:** Claude Code has no way to know that maproom and crewchief CLIs exist or when to use them. Users must manually instruct Claude to use these tools.

**Impact:** Semantic search capabilities go unused; users default to grep/glob even when maproom would be more effective.

**Current State:** Claude uses built-in Read/Grep/Glob tools exclusively. No automatic discovery of project-specific CLI tools.

### Problem 2: Tool Selection Confusion

**Description:** Users and Claude struggle to know when maproom search is better than grep/glob and vice versa.

**Impact:** Inefficient code exploration. Users may spend time with grep finding exact strings when they need conceptual search, or use maproom for exact text matching where grep excels.

**Current State:** No guidance exists for tool selection decision-making.

### Problem 3: Complex CLI Syntax

**Description:** Maproom CLI has many options (mode, filters, debug, context expansion) that are hard to remember and use correctly.

**Impact:** Users underutilize powerful features or get poor results from misconfigured searches.

**Current State:** Documentation exists in CLAUDE.md but is not accessible to Claude Code automatically.

### Problem 4: Worktree Workflow Friction

**Description:** Git worktrees are powerful for parallel development but require multi-step CLI interactions that are error-prone.

**Impact:** Users avoid worktrees or make mistakes (deleting current worktree, forgetting to merge, etc.).

**Current State:** CLI commands exist but Claude Code doesn't know about them or their proper sequencing.

### Problem 5: All-or-Nothing Capability Bundles

**Description:** If capabilities were bundled in a single skill file, users would get everything or nothing.

**Impact:** Projects that only need worktree management would also get maproom search context, and vice versa.

**Current State:** No modular way to provide CLI capabilities to Claude.

## Goals

### Goal 1: Plugin-Based Capability Discovery

**Outcome:** Users can selectively install `maproom` and/or `worktree` plugins based on their project needs.

**Measurement:** Plugins install successfully via `/plugin install maproom@crewchief` and `/plugin install worktree@crewchief`.

### Goal 2: Automatic Skill Activation

**Outcome:** Claude Code automatically considers maproom search when users ask conceptual code questions (if plugin installed).

**Measurement:** Skill triggers when users ask "How does X work?" or "Find the authentication logic" type questions.

### Goal 3: Optimal Tool Selection

**Outcome:** Clear decision tree for when to use maproom vs grep/glob, followed by Claude.

**Measurement:** Claude can explain why it chose a particular search tool for a given query.

### Goal 4: Correct CLI Usage

**Outcome:** Claude uses CLI commands with appropriate options and handles errors gracefully.

**Measurement:** No syntax errors in CLI invocations; helpful suggestions when searches return no results.

### Goal 5: Safe Worktree Operations

**Outcome:** Claude guides users through worktree lifecycle without data loss or confusion.

**Measurement:** Proper sequencing (create -> use -> merge -> clean) with safety checks.

## Constraints

- Plugins must follow crewchief marketplace conventions (see existing plugins)
- Skills must use CLI commands, not MCP protocol (MCP requires daemon spawning)
- Database must be pre-indexed; plugins cannot automatically index on first use
- Skills should complement native tools, not replace them
- Two separate plugins for modularity (not one combined plugin)
- Exclude agent spawning, competitions, and optimization features
- Initial version includes skills only (no agents, commands, or hooks)

## Opportunities

### Opportunity 1: Semantic Code Understanding

**Value:** Enable "understand the codebase" workflows that grep cannot support. Find related code, architectural patterns, and conceptual relationships.

**Feasibility:** High - CLI already supports this via search modes and context command.

### Opportunity 2: Parallel Development Enablement

**Value:** Make worktrees accessible to users who avoid them due to complexity. Enable safe experimentation in isolated branches.

**Feasibility:** High - CLI already handles all worktree operations.

### Opportunity 3: Query Formulation Guidance

**Value:** Teach users (via Claude) how to formulate effective semantic search queries. Transform natural language questions into optimal search terms.

**Feasibility:** High - MCP tool description already has query transformation patterns that can be documented.

### Opportunity 4: Context-Aware Code Exploration

**Value:** Use maproom's context command to gather callers, callees, tests, and related code automatically. Build understanding of code relationships.

**Feasibility:** Medium - Requires chunk_id from search results, multi-step workflow.

### Opportunity 5: Marketplace Distribution

**Value:** Plugins registered in crewchief marketplace can be installed by any Claude Code user with access to the marketplace, enabling broader adoption.

**Feasibility:** High - Marketplace infrastructure already exists and is documented.

### Opportunity 6: Independent Versioning

**Value:** Each plugin can be updated independently, allowing rapid iteration without affecting the other plugin.

**Feasibility:** High - Plugin architecture inherently supports this.
