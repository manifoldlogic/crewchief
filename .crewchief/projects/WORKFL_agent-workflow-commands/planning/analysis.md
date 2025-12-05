# Analysis: Agent Workflow Commands

## 1. Problem Definition

The project workflow involves deterministic operations (scaffolding, status checking) mixed with creative operations (content generation). Currently, all operations go through prompt files in `.claude/commands/`, which has several issues:

**Current Issues:**
- **Non-deterministic scaffolding**: LLM might format output differently, miss sections, or place files in wrong directories
- **Token inefficiency**: Agents must read and interpret lengthy prompt files for simple operations
- **Inconsistent status reporting**: Different agents parse ticket status differently
- **No programmatic access**: Human users and agents both lack structured output for automation

**Goal**: Create CLI commands that handle deterministic operations (scaffolding, status) while slash commands continue to orchestrate the full workflow including LLM-driven content generation.

## 2. Audience and Usage Patterns

These CLI commands serve **two audiences**:

### Human Users (Terminal)
- Direct CLI invocation: `crewchief project init MYPROJ my-project`
- Human-readable formatted output by default
- Interactive prompts for missing information

### AI Agents (Tool Calls)
- Invoked from within slash commands via tool calls
- JSON output with `--json` flag for structured parsing
- Reduces tokens vs reading/interpreting prompt files
- Enables consistent operations across different agents

### Slash Command Integration
- Slash commands (e.g., `/workstream:project-create`) orchestrate the full workflow
- CLI commands are **primitives** that slash commands can invoke
- Example flow: `/workstream:project-create` → validates inputs → calls `crewchief project init` → continues with LLM content generation

## 3. Requirements

### Scaffolding Commands
1. `crewchief project init <slug> <name>` - Create project folder structure with template files

### Project Management Commands
2. `crewchief project list` - List active projects
3. `crewchief project status <slug>` - Show project completion status

### Ticket Commands
4. `crewchief project tickets list <slug>` - List all tickets with detailed status:
   - Individual checkbox states (Task completed, Tests pass, Verified)
   - Human-readable table format by default
   - JSON output for agent consumption

5. `crewchief project tickets show <slug> <ticket-id>` - Full ticket summary:
   - All status checkboxes
   - Acceptance criteria with individual checkbox states
   - Dependencies list
   - Assigned agents

### Output Format Requirements
- **Default**: Human-readable formatted output
- **`--json` flag**: Machine-readable JSON for agents
- **Exit codes**: 0 on success, non-zero on error

## 4. Agent-CLI Interaction

**Current flow:**
```
User -> /workstream:project-create -> LLM reads prompt -> LLM generates files
```

**Target flow:**
```
User -> /workstream:project-create -> LLM validates inputs -> CLI scaffolds structure -> LLM generates content
```

**Benefits:**
- CLI ensures folder structure is 100% correct
- LLM focuses on creative content generation
- Status commands provide consistent ticket state across agents
- JSON output enables programmatic decisions

## 5. Future: Project Workflow Skills

These CLI commands are designed to enable future **project workflow skills**:
- Skills can compose CLI primitives for complex operations
- Consistent output formats enable reliable skill logic
- Reduced token usage improves skill efficiency

## 6. Risks

- **Over-automation**: CLI should NOT generate creative content (analysis, architecture, etc.)
- **Parsing fragility**: Ticket status parsing depends on consistent markdown format
  - Mitigation: Focus on Status section checkboxes only, defensive parsing
- **Scope creep**: Commands should remain focused on scaffolding and status

