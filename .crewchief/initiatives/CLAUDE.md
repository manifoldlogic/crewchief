# Initiatives

Initiatives bridge research and execution. They hold discovery work before it decomposes into concrete projects.

**Flow:** Initiative → Projects → Tickets → Code

## When to Create an Initiative

Create an initiative when:
- Work spans multiple potential projects
- Scope needs exploration before implementation
- Research should be preserved for traceability

Skip to `/create-project` when:
- Scope is clear and bounded
- Single deliverable with defined completion

Reference `.crewchief/reference/initiative-boundary-evaluation.md` for boundary criteria.

## Commands

| Command | Purpose |
|---------|---------|
| `/create-initiative [input]` | Scaffold new initiative from idea or description |

## Folder Structure

```
{YYYY-MM-DD}_{initiative-name}/
├── overview.md              # Vision, scope, success criteria
├── reference/               # Source materials, research notes
├── analysis/
│   ├── opportunity-map.md   # Problems, goals, constraints
│   ├── domain-model.md      # Concepts and boundaries
│   └── research-synthesis.md
├── decomposition/
│   ├── multi-project-overview.md
│   └── project-summaries/   # Stubs for /create-project
├── decisions.md             # Decision log with rationale
└── backlog.md               # Ideas not ready for projects
```

## Lifecycle

1. **Scaffold** — `/create-initiative` creates structure
2. **Research** — Populate `reference/` and `analysis/`
3. **Decompose** — Identify projects in `decomposition/`
4. **Execute** — `/create-project` for each project

## Key Files

| File | Purpose |
|------|---------|
| `overview.md` | Source of truth for scope and goals |
| `decisions.md` | Rationale for choices made |
| `backlog.md` | Parked ideas |
| `multi-project-overview.md` | Project ordering and dependencies |

## Agent Guidelines

- Do not create initiatives for single, well-scoped projects
- Preserve research materials in `reference/` for traceability
- Log significant decisions in `decisions.md` with timestamps
- Keep `overview.md` current as understanding evolves
