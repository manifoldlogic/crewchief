---
description: Scaffold a new initiative for higher-order discovery and research
argument-hint: [initiative idea, name, or description]
---

# Create Initiative

## Purpose

Scaffold a new initiative folder in `.agents/initiatives/` based on user input. Initiatives are precursors to projects — the bridge between research and execution. They represent higher-order discovery work that may decompose into multiple projects.

## Input

User input: "$ARGUMENTS"

This input is freeform and may be:
- A high-level idea name (e.g., "Remixa POC")
- A detailed description of a problem space
- Research notes or context
- A mix of goals, constraints, and exploration areas

Interpret the input flexibly. Extract the core intent and scope.

## Preparation

1. **Read boundary evaluation criteria:**
   - Load `.agents/reference/initiative-boundary-evaluation.md`
   - Apply the three core criteria to validate scope:
     - **Conceptual Stability**: Does this define a stable problem space, not a moving target?
     - **Domain Coherence**: Do all aspects live in a single conceptual domain?
     - **Directional Clarity**: Is there a clear desired end state?

2. **Assess initiative validity:**
   - If input describes a single concrete deliverable, suggest using `/create-project` instead
   - If input spans multiple unrelated domains, suggest splitting into separate initiatives
   - If input is too vague, create the scaffold but note areas needing clarification

3. **Generate identifiers:**
   - DATE: Today's date in `YYYY-MM-DD` format
   - NAME: Concise, descriptive name derived from input (use kebab-case)
   - Folder: `.agents/initiatives/{DATE}_{NAME}/`

## Folder Structure

Create the following structure:

```
.agents/initiatives/{DATE}_{NAME}/
├── overview.md           # Vision, scope, and context
├── reference/            # Source materials (empty initially)
├── analysis/             # Discovery work
│   ├── opportunity-map.md
│   ├── domain-model.md
│   └── research-synthesis.md
├── decomposition/        # Project breakdown (populated later)
│   ├── multi-project-overview.md
│   └── project-summaries/
├── decisions.md          # Running decision log
└── backlog.md            # Ideas not yet ready for projects
```

## Document Templates

### overview.md

```markdown
# Initiative: {NAME}

Created: {DATE}

## Vision Statement

[2-3 sentences describing the purpose and long-term goal]

## Conceptual Frame

[Define the problem space, context, and why this initiative exists]

## Domain Coherence

**Core Domain Concepts:**
- [Concept 1]
- [Concept 2]
- ...

## Directional Clarity

**Desired End State:**
"When this initiative succeeds, [X] will be true."

**Success Signals:**
- [ ] Signal 1
- [ ] Signal 2
- [ ] Signal 3

## Scope Boundaries

**In Scope:**
- [Area 1]
- [Area 2]

**Out of Scope:**
- [Area 1]
- [Area 2]

## Derived Projects

(To be generated during decomposition phase)

## Status

- [ ] Research complete
- [ ] Analysis complete
- [ ] Decomposition complete
- [ ] Projects created

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| [Risk 1] | [Impact] | [Mitigation] |
```

### analysis/opportunity-map.md

```markdown
# Opportunity Map: {NAME}

## Problem Spaces

[What problems does this initiative address?]

## Goals

[What outcomes are we seeking?]

## Constraints

[What limitations must we work within?]

## Opportunities

[What possibilities exist?]
```

### analysis/domain-model.md

```markdown
# Domain Model: {NAME}

## Core Entities

[Key concepts and their relationships]

## Boundaries

[Where does this domain end and others begin?]

## Interactions

[How do entities relate to each other?]
```

### analysis/research-synthesis.md

```markdown
# Research Synthesis: {NAME}

## Key Findings

[Distilled insights from reference materials]

## Open Questions

[Areas requiring further exploration]

## Assumptions

[What are we assuming to be true?]
```

### decomposition/multi-project-overview.md

```markdown
# Multi-Project Overview: {NAME}

## Context

Initiative created: {DATE}
Reference: .agents/initiatives/{DATE}_{NAME}/

## Projects (in execution order)

(To be populated during decomposition)

## Dependencies

[Cross-project dependencies and ordering rationale]
```

### decisions.md

```markdown
# Decisions: {NAME}

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

(Entries added as decisions are made)
```

### backlog.md

```markdown
# Backlog: {NAME}

Ideas identified during research but not yet ready for project creation.

## Ideas

| Idea | Source | Notes | Status |
|------|--------|-------|--------|
| [Idea] | [Where it came from] | [Context] | Captured |
```

## Execution

1. **Interpret input:** Extract initiative name, scope, and key concepts from user input
2. **Validate boundaries:** Check against initiative boundary evaluation criteria
3. **Create folder structure:** Generate all directories and files
4. **Populate overview:** Fill in as much as possible from user input
5. **Note gaps:** Identify areas needing further research or clarification

## Output

Provide summary when complete:

```
📁 INITIATIVE CREATED: {DATE}_{NAME}

📂 Structure:
.agents/initiatives/{DATE}_{NAME}/
├── overview.md
├── reference/
├── analysis/
│   ├── opportunity-map.md
│   ├── domain-model.md
│   └── research-synthesis.md
├── decomposition/
│   ├── multi-project-overview.md
│   └── project-summaries/
├── decisions.md
└── backlog.md

📋 Boundary Evaluation:
- Conceptual Stability: {assessment}
- Domain Coherence: {assessment}
- Directional Clarity: {assessment}

🔍 Areas Needing Clarification:
- {item 1}
- {item 2}

🎯 Next Steps:
1. Add reference materials to `reference/`
2. Complete analysis documents
3. Run decomposition to identify projects
4. Use `/create-project` for each identified project
```

## Notes

- Initiatives are flexible containers for discovery work
- Not all sections need to be complete immediately
- The goal is to scaffold a structure that guides thinking
- Reference `.agents/reference/initiative-boundary-evaluation.md` for detailed criteria
