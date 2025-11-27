# Project: WORKFL_agent-workflow-commands

## Project Summary

Create CLI commands that support the project workflow by providing deterministic scaffolding and status operations. These commands can be used by **both human users and AI agents** to ensure consistency and reduce the tokens required to perform workflow tasks.

**Key Benefits:**
- **Dual Audience**: Commands work for humans at the terminal AND agents via tool calls
- **Token Efficiency**: Agents using CLI commands require fewer tokens than reading/interpreting prompt files
- **Consistency**: Deterministic CLI output vs variable LLM interpretation of prompts
- **Slash Command Integration**: CLI commands support (not replace) existing slash commands
- **Future Skills**: Enables creation of project workflow skills that compose CLI primitives

**Important Clarification**: CLI commands handle *scaffolding* (folder/file creation) and *status reporting* (parsing ticket states). The slash commands (e.g., `/create-project`, `/work-on-project`) continue to orchestrate the full workflow, including content generation that requires LLM creativity.

## Commands Overview

```
crewchief project
  ├── init <slug> <name>        # Scaffold new project structure
  ├── list                       # List active projects
  ├── status <slug>              # Show project completion status
  └── tickets
      ├── list <slug>            # List tickets with detailed status
      └── show <slug> <id>       # Show full ticket details
```

## Relevant Agents

- **TypeScript Engineer**: Implement the CLI commands
- **Prompt Engineer**: Verify logic matches original intent of prompts

## Planning Documents
- [Analysis](./planning/analysis.md)
- [Architecture](./planning/architecture.md)
- [Quality Strategy](./planning/quality-strategy.md)
- [Security Review](./planning/security-review.md)
- [Implementation Plan](./planning/plan.md)

