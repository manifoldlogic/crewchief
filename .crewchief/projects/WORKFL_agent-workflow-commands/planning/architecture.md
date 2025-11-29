# Architecture: Workflow Commands

## 1. CLI Command Structure

Add a `project` namespace to `packages/cli`:

```
crewchief project
  ├── init <slug> <name>        # Scaffolds .crewchief/projects/{SLUG}_{name}/
  ├── list                       # Lists active projects
  ├── status <slug>              # Shows project completion status
  └── tickets
      ├── list <slug>            # Lists tickets with detailed status
      └── show <slug> <ticket-id> # Shows full ticket details
```

## 2. Output Formats

All commands support dual output formats for both human users and agents:

### Human-Readable (Default)
```
crewchief project tickets list WORKFL

Tickets for WORKFL (agent-workflow-commands):

ID          Title                              Task  Tests  Verified
-----------------------------------------------------------------------
WORKFL-1001 Create Project Command Structure    [ ]   [ ]     [ ]
WORKFL-1002 Create Planning Document Templates  [ ]   [ ]     [ ]
WORKFL-2001 Implement Project List Command      [ ]   [ ]     [ ]

Summary: 0/3 tickets complete
```

### JSON Output (`--json` flag)
```json
{
  "project": { "slug": "WORKFL", "name": "agent-workflow-commands" },
  "tickets": [
    {
      "id": "WORKFL-1001",
      "title": "Create Project Command Structure",
      "status": {
        "taskCompleted": false,
        "testsPassed": false,
        "verified": false
      }
    }
  ],
  "summary": { "total": 3, "complete": 0, "percentage": 0 }
}
```

## 3. Scaffolding Templates

Templates for standard markdown files located in `packages/cli/src/templates/project/`:
- `analysis.md` - Problem definition and requirements template
- `architecture.md` - Technical design template
- `plan.md` - Execution plan template
- `quality-strategy.md` - Testing approach template
- `security-review.md` - Security assessment template
- `README.md` - Project overview template

Templates are exported as TypeScript string functions accepting `{ slug, name, date }`.

## 4. Data Model

### Project Structure
- Projects are folders matching pattern `{SLUG}_{name}/`
- Location: `.crewchief/projects/`
- Ticket files match regex `{SLUG}-\d{4}_.*.md`

### Ticket Status Parsing

```typescript
// packages/cli/src/project/types.ts

interface TicketStatusCheckboxes {
  taskCompleted: boolean;   // - [x] **Task completed**
  testsPassed: boolean;     // - [x] **Tests pass**
  verified: boolean;        // - [x] **Verified**
}

interface TicketSummary extends TicketStatusCheckboxes {
  id: string;               // e.g., "WORKFL-1001"
  title: string;            // From ticket heading
  filename: string;         // e.g., "WORKFL-1001_structure.md"
}

interface AcceptanceCriterion {
  text: string;
  checked: boolean;
}

interface TicketDetails extends TicketSummary {
  summary: string;
  acceptanceCriteria: AcceptanceCriterion[];
  dependencies: string[];
  agents: string[];
  filesAffected: string[];
}
```

### Status Parsing Strategy
- Parse Status section with regex: `/^- \[(x| )\] \*\*(Task completed|Tests pass|Verified)\*\*/gm`
- Parse Acceptance Criteria with regex: `/^- \[(x| )\] (.+)$/gm`
- Defensive parsing: log warnings for unparseable files, continue with others

## 5. Agent-CLI Integration

Agents invoke CLI commands via tool calls (e.g., `run_terminal_cmd`):

```
Agent -> run_terminal_cmd("crewchief project tickets list WORKFL --json")
      -> Parse JSON response
      -> Make decisions based on ticket states
```

Benefits:
- Fewer tokens than reading/parsing markdown directly
- Consistent status interpretation across agents
- Structured output enables programmatic decisions

## 6. File Structure

```
packages/cli/src/
├── cli/
│   └── project.ts           # Command definitions
├── project/
│   ├── manager.ts           # Core logic (list, status, parsing)
│   └── types.ts             # TypeScript interfaces
└── templates/
    └── project/
        ├── index.ts         # Template barrel export
        ├── analysis.ts
        ├── architecture.ts
        ├── plan.ts
        ├── quality-strategy.ts
        ├── security-review.ts
        └── readme.ts
```

