# Implementation Project Breakdown

**Date**: November 25, 2025
**Based on**: `comprehensive-analysis-2025-11-25` & `project-boundry-evaluation.md`

Based on the architectural analysis, the recommended work has been decomposed into three distinct projects. Each project satisfies the **Stable Context Triangle** criteria: Interface Stability, Context Coherence, and Testable Completion.

---

## Project 1: Headless CLI Core (`HEADLS`)

**Slug**: `HEADLS`
**Name**: `headless-cli-core`

### Project Description (for `/create-project`)
Refactor the `packages/cli` architecture to decouple the core orchestration logic from specific terminal emulators (currently hardcoded to iTerm2). This project will introduce a `TerminalProvider` interface pattern, implementing three providers: `ITermProvider` (legacy support), `HeadlessProvider` (for CI/CD and background execution), and a `MockProvider` (for testing). The goal is to allow CrewChief to run in environments without a UI (Linux servers, DevContainers) while maintaining the "dashboard" experience for macOS users. This includes abstracting window management, pane splitting, and command injection into a provider-agnostic API.

### Boundary Evaluation
*   **Interface Stability 🔒**: Input (CLI args) and Output (Provider Interface) are clearly defined.
*   **Context Coherence 📦**: Focused entirely on `packages/cli/src/terminal` and `packages/cli/src/orchestrator`. No cross-domain pollution.
*   **Testable Completion 🎯**: Success is defined by running the full agent lifecycle in a headless environment (e.g., a GitHub Action or plain shell) without crashing or requiring iTerm.

---

## Project 2: SQLite-Vec Backend (`SQLVEC`)

**Slug**: `SQLVEC`
**Name**: `sqlite-vec-backend`

### Project Description (for `/create-project`)
Implement a zero-dependency storage backend for the Maproom Rust daemon using `sqlite-vec`. This project involves refactoring `crates/maproom` to introduce a `VectorStore` trait, abstracting away the direct dependency on `tokio-postgres`. A new `SqliteStore` implementation will be created using `rusqlite` and statically linking the `sqlite-vec` C extension. This enables a "single binary" distribution model where the database is a local file (`maproom.db`) rather than a Docker container. The project includes build system updates (`build.rs`), schema migration (SQL -> SQLite), and a configuration switch to toggle between Postgres (server) and SQLite (local) modes.

### Boundary Evaluation
*   **Interface Stability 🔒**: The internal `VectorStore` trait will be the stable contract. The JSON-RPC API exposed to clients remains unchanged.
*   **Context Coherence 📦**: tightly bounded to `crates/maproom` and its database logic.
*   **Testable Completion 🎯**: Success is verified by running the `maproom search` command against a local SQLite file and receiving accurate vector search results equivalent to the Postgres backend.

---

## Project 3: Workflow Automation Commands (`WORKFL`)

**Slug**: `WORKFL`
**Name**: `agent-workflow-commands`

### Project Description (for `/create-project`)
Migrate the agentic workflow logic currently residing in `.claude/commands` (prompt files) into executable CLI commands within `packages/cli`. This project creates deterministic code paths for `crewchief project create`, `crewchief tickets generate`, and `crewchief tickets verify`. Instead of relying on the LLM to "read the prompt and do the right thing," the CLI will implement the standard operating procedures (SOPs) directly, generating the standard markdown structures for plans and tickets programmatically. This reduces hallucination risk and standardizes the output format across all agents.

### Boundary Evaluation
*   **Interface Stability 🔒**: The file formats for Plans (`.agents/projects/.../planning/*.md`) and Tickets are the stable data contract.
*   **Context Coherence 📦**: Focuses on the "Management" domain of the CLI (`src/cli/commands/project`).
*   **Testable Completion 🎯**: Success is defined by generating a valid project structure and ticket set using the CLI command alone, matching the existing manual/prompt-driven templates.

---

## Usage Guide

To initialize these projects, run the following commands:

1.  **Headless Core**:
    `/create-project Refactor packages/cli to abstract terminal interactions behind a TerminalProvider interface, enabling headless execution and removing the strict iTerm2 dependency.`

2.  **SQLite Backend**:
    `/create-project Implement a zero-dependency SQLite+sqlite-vec backend for the Rust daemon, abstracted behind a VectorStore trait in crates/maproom to replace the Dockerized Postgres requirement.`

3.  **Workflow Commands**:
    `/create-project Port the .claude/commands workflow logic into deterministic CLI commands (crewchief project/tickets) to standardize project scaffolding and ticket generation.`

