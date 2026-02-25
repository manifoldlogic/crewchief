# .crewchief Directory

SDD (Structured Driven Development) project management workspace.

**SDD root**: `/workspace/repos/crewchief/crewchief/.crewchief/`

## Ticket Workflow

1. Implementation agent completes work
2. `unit-test-runner` executes tests
3. `verify-ticket` checks acceptance criteria
4. `commit-ticket` creates commit

## Commands

Use `/sdd:*` commands for project management:

- `/sdd:plan-ticket` — Create project with planning docs
- `/sdd:review` — Review project readiness
- `/sdd:create-tasks` — Generate tasks from plan
- `/sdd:do-task` — Execute a single task
- `/sdd:do-all-tasks` — Execute all tasks
- `/sdd:status` — Check status
- `/sdd:archive` — Archive completed projects

## Scope Guidance

- **projects/** — Active projects with planning docs and tickets
- **archive/** — Completed work (all tickets verified, knowledge in `/docs/`)
- **reports/** — Dated point-in-time analysis outputs
- **research/** — Exploratory pre-project investigation
- **initiatives/** — Multi-project discovery work
- **scratchpad/** — Temporary notes (cleaned periodically)

## Archive Criteria

Projects archived when: all tickets verified, no future work planned, knowledge synthesized to `/docs/`.
