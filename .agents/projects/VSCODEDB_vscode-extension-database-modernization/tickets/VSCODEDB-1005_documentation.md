# Ticket: VSCODEDB-1005: Update Extension Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation only)
- [x] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist (primary)
- general-purpose (backup)
- verify-ticket
- commit-ticket

## Summary

Update all extension documentation to reflect SQLite as the default, zero-config database backend while documenting PostgreSQL as an advanced option for teams.

## Background

With all implementation tickets complete, the documentation needs to match the new SQLite-first experience. The README and other docs currently assume PostgreSQL/Docker as the primary path.

**Reference:** plan.md Phase 3 - "VSCODEDB-1005: Update Extension Documentation"

**Goal:** A new user should understand they can use Maproom immediately without Docker by following the README.

## Acceptance Criteria

- [x] README shows SQLite as the default getting-started path
- [x] PostgreSQL setup clearly marked as "Advanced" or "Team Setup"
- [x] Common SQLite error messages documented with solutions
- [x] Settings reference table updated with new `sqlitePath` setting
- [x] Quick start section works without Docker
- [x] CLAUDE.md updated if it exists

## Technical Requirements

### README.md Structure

Update `packages/vscode-maproom/README.md` with this structure:

```markdown
# Maproom for VS Code

Semantic code search for your codebase.

## Quick Start (SQLite - Recommended)

1. Install the extension from marketplace
2. Create an index: `crewchief-maproom scan /path/to/your/repo`
3. Use "Maproom: Search" command (Cmd+Shift+P)

That's it! The extension auto-detects `~/.maproom/maproom.db`.

## Features

- Semantic code search
- Symbol navigation
- ...

## Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `maproom.database.provider` | Database backend: `sqlite` or `postgres` | `sqlite` |
| `maproom.database.sqlitePath` | Custom SQLite path (empty = ~/.maproom/maproom.db) | `""` |
| `maproom.database.host` | PostgreSQL host (only for postgres mode) | `localhost` |
| ... |

## Advanced: PostgreSQL Setup (Team Sharing)

For team environments where you want a shared code index:

1. Change setting: `maproom.database.provider` → `postgres`
2. Start Docker containers: ...
3. Configure connection settings: ...

## Troubleshooting

### "SQLite database not found"

Run `crewchief-maproom scan` to create an index:
\`\`\`bash
crewchief-maproom scan /path/to/your/repo
\`\`\`

### "Cannot connect to PostgreSQL"

If using PostgreSQL mode, ensure Docker containers are running:
\`\`\`bash
docker-compose up -d
\`\`\`

### Switching from PostgreSQL to SQLite

1. Change `maproom.database.provider` to `sqlite`
2. Run `crewchief-maproom scan --sqlite ~/.maproom/maproom.db /path/to/repo`
3. Reload VS Code window
```

### Documentation Sections to Update

1. **Quick Start**: SQLite-first, no Docker required
2. **Installation**: Remove Docker as prerequisite
3. **Settings**: Add `sqlitePath`, mark PostgreSQL settings as "advanced"
4. **Troubleshooting**: Add SQLite-specific issues
5. **Migration Guide**: How to switch between backends

### CLAUDE.md (if exists)

Update `packages/vscode-maproom/CLAUDE.md` with:
- Database mode configuration
- Development setup for both backends
- Testing considerations

### Migration Guide Content

Include section for users migrating from PostgreSQL to SQLite:

```markdown
## Migrating from PostgreSQL to SQLite

If you've been using PostgreSQL and want to switch to SQLite:

1. **Export existing index (optional)**:
   If you want to preserve your index, re-scan with SQLite:
   \`\`\`bash
   crewchief-maproom scan --sqlite ~/.maproom/maproom.db /path/to/your/repo
   \`\`\`

2. **Update settings**:
   - Open VS Code settings
   - Change `maproom.database.provider` to `sqlite`

3. **Reload window**:
   Run "Developer: Reload Window" command

4. **Stop Docker (optional)**:
   If you no longer need PostgreSQL:
   \`\`\`bash
   docker-compose down
   \`\`\`
```

## Implementation Notes

### Preserve PostgreSQL Documentation

Don't delete PostgreSQL documentation - move it to an "Advanced" section. Teams using PostgreSQL for shared indexes need this.

### Use Clear Section Headers

```markdown
## SQLite (Default)        ← Primary section
## PostgreSQL (Advanced)   ← Clearly marked secondary
```

### Error Message Examples

Include actual error messages users might see:

```markdown
### Error: "SQLite database not found at: /Users/you/.maproom/maproom.db"

This means no index exists yet. Create one with:
\`\`\`bash
crewchief-maproom scan /path/to/your/repo
\`\`\`

### Error: "Cannot connect to PostgreSQL at localhost:5433"

If using PostgreSQL mode, ensure:
1. Docker Desktop is running
2. Maproom containers are started
3. Settings match your PostgreSQL configuration
```

### Links to CLI Documentation

Reference the CLI documentation for `crewchief-maproom scan` command:

```markdown
See [Maproom CLI documentation](../cli/README.md) for full `scan` command options.
```

## Dependencies

- **VSCODEDB-1001 through 1004**: All implementation must be complete so documentation matches behavior

## Risk Assessment

- **Risk**: Documentation drift from implementation
  - **Mitigation**: Review implementation before writing docs, include specific settings names

- **Risk**: Confusing existing PostgreSQL users
  - **Mitigation**: Clear "Advanced" sections, migration guide included

## Files/Packages Affected

### Modified Files
- `packages/vscode-maproom/README.md` (primary)
- `packages/vscode-maproom/CLAUDE.md` (if exists)

### Created Files
- None (documentation updates only)

## Verification Checklist

After documentation updates:
1. [ ] New user can follow README to get working without Docker
2. [ ] All mentioned settings exist in package.json
3. [ ] All mentioned commands exist in package.json
4. [ ] Error messages match actual extension output
5. [ ] PostgreSQL setup still documented (in Advanced section)
6. [ ] No broken links
7. [ ] Markdown renders correctly (preview in VS Code)
