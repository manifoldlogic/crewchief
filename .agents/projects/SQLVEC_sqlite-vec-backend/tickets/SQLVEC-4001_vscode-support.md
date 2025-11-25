# Ticket: Implement SQLite Support in VSCode Extension

**ID:** SQLVEC-4001
**Phase:** 4 (Extension Updates)
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Update the `vscode-maproom` extension to support the zero-dependency SQLite backend, making Docker optional.

## Background
Currently, the extension hard-enforces Docker usage and PostgreSQL connection strings. With the new SQLite backend available in the daemon, we need to expose this capability to users.

## Acceptance Criteria
- [ ] Configuration setting `maproom.database.provider` added (default: `sqlite` for new users, or `postgres` to maintain compatibility? Let's default to `postgres` for existing, or auto-detect. Ideally default `sqlite` for simplicity).
- [ ] `ensureDockerRunning` and `ensurePostgresAvailable` are SKIPPED when provider is `sqlite`.
- [ ] `ProcessOrchestrator` passes `MAPROOM_DB_URL=sqlite://...` (or file path) when provider is `sqlite`.
- [ ] Setup wizard allows choosing between "Local (Zero Config)" and "Docker (Postgres)".

## Technical Requirements
- **Settings**: Add enum setting to `package.json`.
- **Logic**: Refactor `initializeServices` in `extension.ts`.
- **Env Vars**: Update `buildEnvironment` in `orchestrator.ts`.

## Implementation Notes
- For `sqlite`, the database path should probably be in the extension's global storage path or the workspace folder (hidden). Global storage is better for cache reuse. `context.globalStorageUri.fsPath`.

## Dependencies
- SQLVEC-3001 (Backend switching in daemon)

## Risks
- Breaking existing users if we default to SQLite without migrating their data (they would lose their index). Default should probably be `sqlite` for NEW installs, but we can't easily detect that.
- **Mitigation**: If `maproom.database.host` is configured (non-default), assume Postgres? Or just add the setting defaulting to `postgres` for now to be safe, or `sqlite` and warn?
- **Decision**: Add explicit setting. Default to `sqlite` to achieve the "Zero Config" goal, but check if a valid Postgres config exists and prompt/switch?
- **MVP**: Just add the setting, default to `sqlite`. Users upgrading might need to switch back or re-index. Re-indexing is acceptable for a 0.x release.

