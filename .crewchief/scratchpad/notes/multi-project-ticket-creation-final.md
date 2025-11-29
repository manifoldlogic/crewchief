# Project Ticket Generation Workflow

## Objective
Generate development tickets for all projects in `crewchief_context/maproom/`, delegating ticket creation to the **ticket-creator subagent**.

## Project Structure

Each project has three core documents:
- **{SLUG}_ANALYSIS.md** - Problem space, domain understanding, user needs
- **{SLUG}_ARCHITECTURE.md** - Technical design, system components, patterns
- **{SLUG}_PLAN.md** - **Primary source for tickets** - implementation roadmap, phases, features

The **MAPROOM_PROJECT_OVERVIEW.md** provides cross-project context and relationships.

## Workflow

### 1. Project Discovery
- Read `MAPROOM_PROJECT_OVERVIEW.md` to identify all projects and their relationships
- For each project, locate its three documents: `{SLUG}_ANALYSIS.md`, `{SLUG}_ARCHITECTURE.md`, `{SLUG}_PLAN.md`

### 2. Ticket Generation per Project

For each project:
1. Read the **{SLUG}_PLAN.md** - this defines the work to be ticketed
2. Reference **{SLUG}_ANALYSIS.md** for domain context
3. Reference **{SLUG}_ARCHITECTURE.md** for technical context
4. Break the plan into discrete tickets following the conventions below

**Ticket Numbering:**
- Format: `{PROJECT_SLUG}-{PHASE}{SEQUENTIAL}_{descriptive-name}.md`
- Examples:
  - `HYBRID_SEARCH-1001_setup-vector-store.md` (Phase 1, ticket 1)
  - `LANG_PARSE-2003_implement-ast-transformer.md` (Phase 2, ticket 3)
  - `HYBRID_SEARCH-1901_test-vector-search.md` (Phase 1, test 901)

**Test Tickets (MVP Strategy):**
- Number in 900s range per phase (1901, 1902, 2901, etc.)
- Only for critical paths: core logic, integrations, complex transformations
- Skip exhaustive coverage; focus on confidence and velocity

### 3. Delegate to ticket-creator Agent

For each ticket, provide ticket-creator with:

```
Create ticket: {PROJECT_SLUG}-{PHASE}{NUMBER}_{name}

REFERENCE DOCUMENTS:
- Analysis: crewchief_context/maproom/{SLUG}_ANALYSIS.md
- Architecture: crewchief_context/maproom/{SLUG}_ARCHITECTURE.md  
- Plan: crewchief_context/maproom/{SLUG}_PLAN.md

PLAN SECTION:
[Reference the specific section/feature from {SLUG}_PLAN.md this ticket implements]

TICKET CONTEXT:

Title: [Action-oriented title]

Summary: [1-2 sentences describing the work]

Background:
- Domain context from ANALYSIS doc
- Why this work matters (design thinking angle)

Acceptance Criteria:
[3-5 specific, measurable outcomes from PLAN]

Technical Requirements:
[Requirements from ARCHITECTURE and PLAN docs]

Implementation Notes:
- Approach from ARCHITECTURE doc
- DDD patterns to apply
- Key technical considerations

Dependencies:
[Prerequisite tickets or external dependencies]

Risks:
[Technical risks and mitigations from PLAN/ARCHITECTURE]

Files/Packages:
[Expected files/packages from ARCHITECTURE]

Primary Agent: [e.g., backend-engineer, frontend-engineer]
Supporting Agents: [e.g., ddd-expert, test-engineer]

Output Location: .crewchief/work-tickets/{PROJECT_SLUG}/
```

### 4. Agent Assignment

- **Backend/API**: backend-engineer, ddd-expert
- **Frontend/UI**: frontend-engineer, ux-specialist  
- **Data/ETL**: data-engineer, ddd-expert
- **Integration**: integration-specialist, backend-engineer
- **Testing**: test-engineer + domain expert
- **Architecture**: solution-architect, ddd-expert

All tickets include: `test-runner`, `verify-ticket`, `commit-ticket`

### 5. Execution Pattern

```
For each project in MAPROOM_PROJECT_OVERVIEW.md:
  1. Read {SLUG}_PLAN.md to understand work breakdown
  2. For each feature/phase in the plan:
     - Extract ticket context
     - Reference ANALYSIS and ARCHITECTURE docs
     - Delegate to ticket-creator subagent
  3. Create test tickets for critical paths
  4. Move to next project
```

### 6. Output Organization

```
.crewchief/work-tickets/
├── {PROJECT_SLUG}/
│   ├── {SLUG}-1001_first-ticket.md
│   ├── {SLUG}-1002_second-ticket.md
│   ├── {SLUG}-1901_test-feature.md
│   └── {SLUG}-INDEX.md
└── README.md
```

## Key Principles

1. **PLAN is primary source** - tickets implement what's in {SLUG}_PLAN.md
2. **Reference, don't duplicate** - point ticket-creator to docs, don't paste content
3. **MVP focus** - defer nice-to-haves, test only critical paths
4. **Actionable specificity** - concrete requirements, not vague directives
5. **Right-sized scope** - 2-8 hours per ticket

---

**Begin**: Read MAPROOM_PROJECT_OVERVIEW.md, then process each project's PLAN document, delegating to ticket-creator with document references and extracted context.