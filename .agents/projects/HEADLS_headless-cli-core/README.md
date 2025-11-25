# Project: HEADLS_headless-cli-core

## Project Summary
Refactor the `packages/cli` architecture to decouple the core orchestration logic from specific terminal emulators (currently hardcoded to iTerm2). This project will introduce a `TerminalProvider` interface pattern, implementing three providers: `ITermProvider` (legacy support), `HeadlessProvider` (for CI/CD and background execution), and a `MockProvider` (for testing). The goal is to allow CrewChief to run in environments without a UI (Linux servers, DevContainers) while maintaining the "dashboard" experience for macOS users. This includes abstracting window management, pane splitting, and command injection into a provider-agnostic API.

## Relevant Agents
- **Orchestrator Architect**: To design the `TerminalProvider` interface.
- **TypeScript Engineer**: To implement the refactoring and new providers.
- **CI/CD Specialist**: To verify the headless provider in GitHub Actions.

## Planning Documents
- [Analysis](./planning/analysis.md)
- [Architecture](./planning/architecture.md)
- [Quality Strategy](./planning/quality-strategy.md)
- [Security Review](./planning/security-review.md)
- [Implementation Plan](./planning/plan.md)

