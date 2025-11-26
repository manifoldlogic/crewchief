# .agents/archive - Completed Work

Historical records of completed projects, sessions, and reports.

## Purpose

Preserves completed work for historical reference. Projects are archived after all tickets are verified and key learnings have been synthesized to `/docs/`.

## Structure

```
archive/
├── projects/    # Completed project folders (full planning + tickets)
├── reports/     # Archived point-in-time reports
└── sessions/    # Historical session logs
```

## Archive Criteria

Projects are archived when:
1. **All tickets verified**: Every ticket has `- [x] **Verified**` checkbox
2. **No future work**: Project scope is complete
3. **Knowledge synthesized**: Reusable knowledge moved to `/docs/`

## Using Archived Content

Archived projects remain valuable for:
- Understanding past design decisions
- Referencing implementation patterns
- Learning from completed work
- Historical context for related projects

## Archive Process

Use `/archive-projects` to:
1. Audit active projects for completion
2. Update README status to "Complete (Archived YYYY-MM-DD)"
3. Move project folder to `archive/projects/`
4. Update `archive/README.md` with project summary

## What Does NOT Belong Here

- Active projects (keep in `../projects/`)
- Incomplete projects (finish first or mark abandoned)
- Temporary files (use `../scratchpad/`)
