# Quality Strategy: Workflow Commands

## 1. Testing

### Unit Tests
- Verify scaffolding logic creates correct files and content
- Verify ticket status parsing extracts correct checkbox states
- Verify acceptance criteria parsing handles various formats
- Test JSON output structure matches documented schema

### Integration Tests
- Run `crewchief project init` and verify file system state
- Run `crewchief project tickets list` and verify output format
- Run `crewchief project tickets show` and verify full details
- Test with real project directories in `.crewchief/projects/`

## 2. Validation

- SLUG format: uppercase, 2-8 alphanumeric chars, starts with letter
- Name format: lowercase kebab-case
- Prevent overwriting existing projects without `--force` flag
- Graceful handling of malformed ticket files (warn and continue)

## 3. Acceptance Criteria

### Scaffolding Commands
- [ ] `crewchief project init` creates all planning docs (analysis, architecture, plan, quality-strategy, security-review, README)
- [ ] `crewchief project list` detects new and existing projects
- [ ] Directories follow naming convention `{SLUG}_{name}/`

### Ticket Commands
- [ ] `crewchief project tickets list <slug>` outputs detailed status table
- [ ] `crewchief project tickets show <slug> <id>` outputs full ticket summary
- [ ] Both commands support `--json` flag for machine-readable output
- [ ] Status parsing correctly identifies Task completed, Tests pass, Verified states
- [ ] Acceptance criteria parsing shows individual checkbox states

### Dual Audience Support
- [ ] Human-readable output is well-formatted and readable in terminal
- [ ] JSON output is valid JSON parseable by agents
- [ ] Exit codes: 0 for success, non-zero for errors
- [ ] Error messages are clear for both human and agent consumption

## 4. Test Matrix

| Command | Human Output | JSON Output | Error Handling |
|---------|--------------|-------------|----------------|
| `project init` | Created files listed | N/A | Invalid slug/name |
| `project list` | Table format | Array of projects | Empty directory |
| `project status` | Progress summary | Status object | Project not found |
| `project tickets list` | Status table | Array of tickets | Project not found |
| `project tickets show` | Full details | Ticket object | Ticket not found |

