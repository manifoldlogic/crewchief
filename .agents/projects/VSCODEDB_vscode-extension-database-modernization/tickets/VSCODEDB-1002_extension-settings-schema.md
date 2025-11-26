# Ticket: VSCODEDB-1002: Update Extension Settings Schema

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (schema validation via VSIX packaging)
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist (primary)
- verify-ticket
- commit-ticket

## Summary

Add the `maproom.database.sqlitePath` setting to the extension's `package.json` configuration schema, allowing users to specify custom SQLite database paths.

## Background

The `maproom.database.provider` setting already exists with `sqlite`/`postgres` enum (defaulting to `sqlite`), but there's no setting for users to customize the SQLite file path. This ticket adds that setting while maintaining all existing PostgreSQL settings.

**Reference:** plan.md Phase 1 - "VSCODEDB-1002: Update Extension Settings Schema"

**Note:** This is a schema-only change. No TypeScript code changes required - the setting will be consumed by `database-checker.ts` from VSCODEDB-1001.

## Acceptance Criteria

- [x] New setting `maproom.database.sqlitePath` appears in VSCode Settings UI under Maproom > Database
- [x] Setting has correct type (`string`) and default value (`""` - empty string)
- [x] Setting description clearly explains:
  - Default behavior when empty (`~/.maproom/maproom.db`)
  - Only applies when provider is `sqlite`
  - Supports tilde expansion
- [x] VSIX packages successfully: `pnpm vsce:package` completes without schema errors
- [x] Existing PostgreSQL settings unchanged and functional

## Technical Requirements

### New Setting Schema

Add to `package.json` under `contributes.configuration.properties`:

```json
"maproom.database.sqlitePath": {
  "type": "string",
  "default": "",
  "markdownDescription": "Path to SQLite database file. Leave empty for default (`~/.maproom/maproom.db`). Supports tilde (`~`) for home directory. Only used when provider is `sqlite`.",
  "scope": "machine-overridable",
  "order": 2
}
```

### Setting Order

Update `order` values to ensure logical grouping:
1. `provider` (order: 1) - already exists
2. `sqlitePath` (order: 2) - new
3. PostgreSQL settings (order: 10+) - existing

### Description Updates

Update existing `provider` setting description for clarity:

```json
"maproom.database.provider": {
  "type": "string",
  "enum": ["sqlite", "postgres"],
  "default": "sqlite",
  "markdownDescription": "Database backend for code search index.\n\n**SQLite (recommended)**: Zero-config local file. Works immediately with existing `~/.maproom/maproom.db`.\n\n**PostgreSQL**: Full-featured with Docker. Required for team sharing and advanced features.",
  "enumDescriptions": [
    "Local SQLite file (zero-config, recommended for most users)",
    "PostgreSQL via Docker (advanced, enables team sharing)"
  ],
  "scope": "machine-overridable",
  "order": 1
}
```

## Implementation Notes

### Schema Location

Edit `packages/vscode-maproom/package.json` in the `contributes.configuration.properties` section.

### Scope Selection

Use `"scope": "machine-overridable"` because:
- Database paths are machine-specific (not shareable across machines)
- But allow workspace override for project-specific databases

### Markdown in Descriptions

Use `markdownDescription` instead of `description` for rich formatting:
- Backticks for paths: `` `~/.maproom/maproom.db` ``
- Bold for emphasis: `**SQLite (recommended)**`
- Newlines for readability

### Verification

After making changes, verify schema with:
```bash
cd packages/vscode-maproom
pnpm compile && pnpm vsce:package --no-dependencies
```

If packaging succeeds, schema is valid.

## Dependencies

- **VSCODEDB-1001**: Must be complete so `database-checker.ts` exists to consume this setting
  - Note: The implementation in 1001 should read `maproom.database.sqlitePath` - ensure the setting key matches

## Risk Assessment

- **Risk**: Invalid schema breaks VSIX packaging
  - **Mitigation**: Run `pnpm vsce:package` as validation step

- **Risk**: Setting name mismatch with code
  - **Mitigation**: Verify `database-checker.ts` uses exact key `maproom.database.sqlitePath`

## Files/Packages Affected

### Modified Files
- `packages/vscode-maproom/package.json` (contributes.configuration section)

## Verification Steps

1. Run `pnpm compile` - should succeed
2. Run `pnpm vsce:package --no-dependencies` - should create VSIX without errors
3. Install VSIX in VSCode and verify:
   - Settings UI shows "SQLite Path" under Maproom > Database
   - Default value is empty string
   - Description shows markdown formatting correctly
   - Provider setting shows updated description with enum descriptions
