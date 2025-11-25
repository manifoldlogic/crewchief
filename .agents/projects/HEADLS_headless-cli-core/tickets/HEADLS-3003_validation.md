# Ticket: Validation and Smoke Testing

**ID:** HEADLS-3003
**Phase:** 3
**Status:** Pending
**Assigned To:** Quality Engineer

## Summary
Perform manual validation of the refactored CLI in both iTerm (macOS) and Headless (Linux/Docker) environments.

## Background
Automated tests cover units, but the integration with the OS shell needs manual verification to ensure no regressions.

## Acceptance Criteria
- [ ] **iTerm Test**: `crewchief spawn` works exactly as before (opens windows/panes).
- [ ] **Headless Test**: `crewchief spawn --headless` works in VSCode terminal (streams logs, no windows).
- [ ] **Linux Test**: Run inside a Docker container (e.g., `node:18`) and verify it runs without crashing.
- [ ] **Cleanup Test**: Ctrl+C in Headless mode kills all spawned agents.

## Technical Requirements
- **Docker**: Use a simple `Dockerfile` to test the Linux scenario.
  ```dockerfile
  FROM node:18
  WORKDIR /app
  COPY . .
  RUN pnpm install && pnpm build
  CMD ["./packages/cli/bin/crewchief", "spawn", "test-agent", "--headless"]
  ```

## Implementation Notes
- Document findings in `.agents/reports/HEADLS_validation.md`.

## Dependencies
- HEADLS-3002

## Risks
- Discovery of OS-specific quirks (e.g., signal propagation in Docker).

