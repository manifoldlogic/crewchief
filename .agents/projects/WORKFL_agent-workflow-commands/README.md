# Project: WORKFL_agent-workflow-commands

## Project Summary
Migrate the agentic workflow logic currently residing in `.claude/commands` (prompt files) into executable CLI commands within `packages/cli`. This project creates deterministic code paths for `crewchief project create`, `crewchief tickets generate`, and `crewchief tickets verify`. Instead of relying on the LLM to "read the prompt and do the right thing," the CLI will implement the standard operating procedures (SOPs) directly, generating the standard markdown structures for plans and tickets programmatically. This reduces hallucination risk and standardizes the output format across all agents.

## Relevant Agents
- **Typescript Engineer**: To implement the CLI commands.
- **Prompt Engineer**: To verify the logic matches the original intent of the prompts.

## Planning Documents
- [Analysis](./planning/analysis.md)
- [Architecture](./planning/architecture.md)
- [Quality Strategy](./planning/quality-strategy.md)
- [Security Review](./planning/security-review.md)
- [Implementation Plan](./planning/plan.md)

