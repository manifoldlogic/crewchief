# CrewChief Project Analysis

## Overview

CrewChief is a multi-agent orchestration tool designed to enable AI agents to collaborate on software development tasks within a single repository. It uses isolated git worktrees for each agent, visual coordination through tmux, and a message bus for communication. The system aims to simplify complex workflows, support competitive evaluations, and provide observability, all accessible via a single CLI entrypoint: `crewchief`.

The deeper intention is to create an ergonomic, scalable way to leverage multiple AI agents (like Claude or Gemini) for tasks such as coding, planning, and review, while minimizing manual intervention, ensuring isolation to prevent conflicts, and optimizing for quality through competition and benchmarking.

## Core Objectives and Intentions

Based on the specifications in `context/cli/specification.md` and other codebase elements, the project's intentions include:

1. **Simplify Git Worktree Management**: Abstract away the complexity of git worktrees, automatically creating and managing isolated environments for each agent to work without interfering with the main branch or each other. Intention: Enable parallel work without merge conflicts or manual setup.

2. **Enable Parallel AI Agent Collaboration**: Allow multiple agents to run simultaneously, each in its own tmux pane, with mechanisms for delegation, handoffs, and input injection between agents. Intention: Facilitate hierarchical agent structures (e.g., project manager delegating to developers) to handle complex, multi-step tasks efficiently.

3. **Provide Visual Orchestration and Monitoring**: Integrate with tmux for real-time visibility of agent activities. The OpsDeck component (Rust-based) offers a dashboard for monitoring agent status, health, and metrics. Intention: Give users an "ops deck" to oversee operations without needing to switch contexts constantly, reducing cognitive load.

4. **Support Competitive Agent Evaluation**: Implement competition mode where multiple agents tackle the same task, with automated evaluation and selection of the best result. Intention: Improve output quality by pitting agents against each other, benchmarking performance, and evolving configurations over time.

5. **Minimize Command Complexity**: Use a single CLI (`crewchief`) that auto-detects state, runs setups, and launches sessions. Intention: Make the tool accessible to non-experts, streamlining workflows into intuitive commands while hiding underlying complexity.

6. **Ensure Observability and Debuggability**: A logged message bus captures all inter-agent communications and events, with tools to inspect runs, logs, and evaluations. Intention: Provide transparency to debug issues, trace decisions, and audit agent behavior.

7. **Smart Context Management**: Shared vector database and semantic retrieval for consistent knowledge access across agents. Intention: Prevent information silos, enabling agents to build on each other's work effectively.

8. **Quality Assurance and Integration**: Automated merging with quality checks (tests, linting), rollback capabilities, and evaluation frameworks. Intention: Safely integrate agent outputs into the main codebase, maintaining repository integrity.

9. **Extensibility and Configuration**: Pluggable agent types, customizable configs, and support for custom platforms. Intention: Adapt to various AI models and workflows, future-proofing the tool.

## Key Components

- **CLI (TypeScript)**: The main interface in `packages/cli/`, handling commands for setup, agents, tasks, competitions, merges, etc. It orchestrates tmux, git, and agents.
- **OpsDeck (Rust Crate)**: In `crates/opsdeck/`, a binary for aggregating heartbeats and painting dashboards (Ops Deck grid or Roster list) for monitoring.
- **Specifications**: Centralized in `context/` (e.g., `cli/specification.md`, `opsdeck/specification.md`), ensuring all docs and requirements are in one place.
- **Message Bus and Logging**: JSONL-based event streaming for communication and persistence.
- **Git Integration**: Worktree management for isolation.
- **Tmux Service**: For pane creation, input injection, and session management.
- **Agent Registry**: Defines agent types (e.g., Claude, Gemini) with execution commands and capabilities.
- **Orchestrator**: Manages task distribution, scheduling, evaluations, and auto-merging.
- **Fixtures and Tests**: Sample data and integration tests for validation.

## Deeper Intentions and Aspects to Consider for Pivot

When pivoting to a new implementation, consider retaining these core aspects to meet the original goals:

- **Isolation via Worktrees**: Critical for parallel, conflict-free work. Keep if multi-agent parallelism is desired.
- **Visual Dashboard (OpsDeck)**: Provides at-a-glance monitoring; essential for scaling to many agents.
- **Competition and Benchmarking**: Drives quality improvements; retain for any system aiming to optimize AI outputs.
- **Message Bus**: Enables inspectable communication; key for debuggability and coordination.
- **Auto-Setup and Ergonomics**: Single-entrypoint CLI with wizards; improves user experience.
- **Hierarchical Delegation**: Allows complex workflows; useful for breaking down large tasks.
- **Quality Gates and Merging**: Automated checks and integration; prevents bad changes.
- **Extensibility**: Pluggable agents and configs; allows adaptation to new AI tools.
- **Observability**: Logging and event tracing; vital for trust in autonomous systems.
- **Performance Constraints**: Low overhead for monitoring; ensures usability on dev machines.

Potential pivots could simplify to single-agent focus, enhance web-based UIs over tmux, or integrate with specific IDEs, but these aspects capture the "true deeper intentions" of scalable, observable AI orchestration.

This report is based on a comprehensive review of the codebase, including specifications, CLI implementation, and Rust components.
