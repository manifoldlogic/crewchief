# Analysis: Agent Workflow Commands

## 1. Problem Definition
The current "Agentic Workflow" relies on markdown files in `.claude/commands/` which act as system prompts.
- **Issue**: The user types `/create-project`, and the LLM *reads* the prompt file and then *hallucinates* the execution of that prompt (generating files).
- **Risk**: This is non-deterministic. The LLM might format the output differently, miss sections, or place files in the wrong directory.
- **Goal**: Convert these "Soft Commands" (prompts) into "Hard Commands" (CLI code).

## 2. Requirements
We need to replace the following prompts with CLI commands:
1.  `/create-project` -> `crewchief project create <name>`
2.  `/create-project-tickets` -> `crewchief project tickets generate <project>`
3.  `/review-tickets` -> `crewchief project tickets review <project>`
4.  `/review-project` -> `crewchief project review <project>`

## 3. Implementation Approach
- The CLI already has `commander.js`. We will add a `project` command group.
- **Logic**:
  - `create`: Scaffolds the directory structure and creates template markdown files.
  - `tickets generate`: Parses the `plan.md` and creates ticket files programmatically (or perhaps this step *does* use an LLM, but controlled via the CLI which calls the `generate_tickets` agent/tool).
  - **Wait**: The prompt for `create-project-tickets` asks the LLM to use the `ticket-creator` agent. The CLI itself doesn't have an embedded LLM to generate content.
  - **Correction**: The CLI can't generate *content* (creative writing of the plan) without an LLM.
  - **Refined Goal**: The CLI should provide the **Scaffolding** and **Validation**.
    - `crewchief project create`: Creates folders and empty/template files.
    - `crewchief project validate`: Checks if files exist and follow schema.
    - The *content generation* might still require an agent, BUT the CLI can act as the "Tool" the agent calls.

## 4. Agent-CLI Interaction
- **Current**: User -> Prompt -> LLM -> File Writes
- **Target**: User -> Prompt -> LLM -> **CLI Tool (`project_create`)** -> File Writes
- **Benefit**: The CLI ensures the folder structure is 100% correct. The LLM just provides the parameters (name, description).

## 5. Risks
- Over-automating the "creative" part. The CLI shouldn't write the `analysis.md` content, but it SHOULD create the file `analysis.md` with the correct headers.

