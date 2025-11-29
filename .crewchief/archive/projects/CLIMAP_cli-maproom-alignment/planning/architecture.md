# Architecture: CLI-Maproom Alignment

**Project:** CLIMAP - CLI-Maproom Alignment
**Date:** 2025-01-10

## Architecture Decisions

### 1. Command Structure Refactoring

**Decision:** Convert from colon-separated to subcommand pattern

**Before:**
```bash
crewchief maproom:scan
crewchief maproom:search "query"
crewchief maproom:db migrate
```

**After:**
```bash
crewchief maproom scan
crewchief maproom search "query"
crewchief maproom db migrate
```

**Implementation:**
```typescript
// packages/cli/src/cli/maproom.ts

export function registerMaproomCommands(program: Command) {
  // Parent command
  const maproom = program
    .command('maproom')
    .description('Semantic code indexing and search')

  // Subcommands
  maproom
    .command('scan')
    .description('Scan and index repository files')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['scan', ...(args || [])]))

  maproom
    .command('search')
    .description('Semantic search across indexed code')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['search', ...(args || [])]))

  maproom
    .command('upsert')
    .description('Update specific files in the index')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['upsert', ...(args || [])]))

  maproom
    .command('watch')
    .description('Watch repository for changes')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['watch', ...(args || [])]))

  // Nested subcommand for database operations
  const db = maproom
    .command('db')
    .description('Database operations')

  db
    .command('migrate')
    .description('Run database migrations')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['db', 'migrate', ...(args || [])]))

  // New commands
  maproom
    .command('branch-watch')
    .description('Auto-index worktrees on branch switch')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['branch-watch', ...(args || [])]))

  maproom
    .command('cache')
    .description('Manage maproom caches')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['cache', ...(args || [])]))

  maproom
    .command('generate-embeddings')
    .description('Generate embeddings for indexed chunks')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args) => runMaproomForward(['generate-embeddings', ...(args || [])]))
}
```

**Rationale:**
- Aligns with `worktree`, `agent`, `spawn` command patterns
- Industry standard (git, docker, kubectl)
- Better help organization (`crewchief maproom --help` shows all subcommands)
- Clearer mental model (maproom is a feature, not a separate tool)
- **Clean break:** No legacy cruft, tool has no existing users

### 2. Environment Validation Layer

**Decision:** Add pre-flight validation before spawning Rust binary

**Implementation:**
```typescript
// packages/cli/src/cli/maproom-validation.ts

export interface ValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}

export function validateMaproomEnvironment(): ValidationResult {
  const errors: string[] = []
  const warnings: string[] = []

  // Check database connection
  const dbUrl = process.env.MAPROOM_DATABASE_URL
    || process.env.MAPROOM_DB_HOST
    || process.env.PG_DATABASE_URL
    || process.env.DATABASE_URL

  if (!dbUrl) {
    errors.push('No database connection configured.')
    errors.push('Set MAPROOM_DATABASE_URL environment variable.')
    errors.push('See: https://github.com/your-org/crewchief#database-setup')
  }

  // Check embedding provider (only for scan/upsert commands)
  const provider = process.env.MAPROOM_EMBEDDING_PROVIDER

  if (!provider) {
    warnings.push('MAPROOM_EMBEDDING_PROVIDER not set.')
    warnings.push('Embeddings will not be generated during indexing.')
    warnings.push('See: https://github.com/your-org/crewchief#embedding-setup')
  } else {
    // Provider-specific validation
    if (provider === 'openai') {
      if (!process.env.OPENAI_API_KEY && !process.env.MAPROOM_OPENAI_API_KEY) {
        errors.push('OpenAI provider requires OPENAI_API_KEY or MAPROOM_OPENAI_API_KEY')
      }
    } else if (provider === 'google') {
      if (!process.env.GOOGLE_PROJECT_ID && !process.env.MAPROOM_GOOGLE_PROJECT_ID) {
        errors.push('Google provider requires GOOGLE_PROJECT_ID or MAPROOM_GOOGLE_PROJECT_ID')
      }
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  }
}

export function displayValidationResult(result: ValidationResult): void {
  if (result.errors.length > 0) {
    logger.error('❌ Environment validation failed:')
    for (const err of result.errors) {
      logger.error(`   ${err}`)
    }
  }

  if (result.warnings.length > 0) {
    logger.warn('⚠️  Warnings:')
    for (const warn of result.warnings) {
      logger.warn(`   ${warn}`)
    }
  }
}
```

**Usage in Commands:**
```typescript
// Only validate for commands that need database/embeddings
const commandsRequiringValidation = ['scan', 'upsert', 'search', 'generate-embeddings']

function runMaproomForward(args: string[]) {
  const subcommand = args[0]

  // Pre-flight validation
  if (commandsRequiringValidation.includes(subcommand)) {
    const validation = validateMaproomEnvironment()
    displayValidationResult(validation)

    if (!validation.valid) {
      logger.error('Fix the errors above before continuing.')
      process.exitCode = 1
      return
    }
  }

  // Forward to Rust binary
  const bin = resolvePackagedMaproomBin()
  if (!bin) {
    logger.error('crewchief-maproom binary not found.')
    logger.error('Run: pnpm build:rust')
    process.exitCode = 1
    return
  }

  const res = spawnSync(bin, args, { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}
```

**Rationale:**
- Catch configuration errors early
- Provide actionable error messages
- Link to documentation
- Better DX than cryptic Rust errors
- Warnings don't block (e.g., missing embedding provider)

### 3. Documentation Structure

**Decision:** Reorganize README with clear sections

**New Structure:**
```markdown
# CrewChief CLI

## Quick Start
- Installation
- Basic usage

## Database Setup
- MAPROOM_DATABASE_URL configuration
- Docker setup
- Connection troubleshooting

## Embedding Provider Setup
- OpenAI configuration
- Google Vertex AI configuration
- Ollama (local) configuration

## Features

### Git Worktree Management
- worktree create
- worktree list
- worktree clean

### Semantic Code Search (Maproom)
- maproom scan
- maproom search
- maproom upsert
- maproom watch
- maproom branch-watch
- maproom cache

### Performance Optimization
- Incremental scanning
- Parallel processing
- Batch size tuning

### Schema & Migrations
- blob_sha (content addressing)
- code_embeddings (deduplication)
- worktree_ids (branch-aware search)

## Troubleshooting
- Database connection errors
- Embedding provider errors
- Binary not found errors

## Environment Variables Reference
- Table of all MAPROOM_* variables
- Fallback hierarchy
```

**Rationale:**
- Progressive disclosure (quick start first)
- Dedicated sections for each major concern
- Reference section for power users
- Troubleshooting for common issues

## Technology Choices

### Commander.js Subcommands

**Choice:** Use Commander.js nested command pattern

**Alternatives Considered:**
- Custom argument parsing (too much work)
- Different CLI framework (unnecessary churn)

**Rationale:**
- Already using Commander.js
- Native subcommand support
- Well-documented pattern

### Validation Approach

**Choice:** Lightweight environment checks in TypeScript

**Alternatives Considered:**
- Full database connection test (too slow for every command)
- Let Rust handle everything (poor error messages)

**Rationale:**
- Fast (just env var checks)
- TypeScript error messages can be user-friendly
- Still forwards to Rust for actual work

## Performance Considerations

### Command Startup Time

**Concern:** Adding validation might slow down commands

**Mitigation:**
- Validation is only environment variable checks (microseconds)
- No database connections in validation
- No network calls
- Negligible impact (<10ms)

### Help Text Generation

**Concern:** More subcommands = longer `--help` output

**Mitigation:**
- Commander.js handles grouping
- `maproom --help` shows only maproom subcommands
- Top-level `crewchief --help` shows command categories

## Long-Term Maintainability

### Adding New Maproom Commands

**Process:**
1. Rust team implements new command in `crates/maproom/src/main.rs`
2. CLI team adds subcommand registration in `maproom.ts`
3. Update README with new command documentation
4. No API coordination needed (pure forwarding)

**Example:**
```typescript
// Adding hypothetical "maproom analyze" command
maproom
  .command('analyze')
  .description('Analyze code complexity')
  .allowUnknownOption(true)
  .argument('[args...]')
  .action((args) => runMaproomForward(['analyze', ...(args || [])]))
```

### Validation Updates

**Process:**
1. New environment variable added to Rust config
2. Update `validateMaproomEnvironment()` with new check
3. Update README environment variable reference

**Low coupling:** Validation is separate from forwarding logic

## Constraints

### Must Not Break

1. **Existing functionality** - All current commands must work
2. **Rust binary compatibility** - Must forward arguments correctly
3. **Environment variable fallbacks** - Rust handles fallback logic

### Can Change

1. **Command names** - Clean break, no existing users
2. **Documentation** - Major restructuring allowed
3. **Validation** - New layer, doesn't affect forwarding

### Should Avoid

1. **Complex validation** - Keep it fast and simple
2. **API dependencies** - Don't couple to Rust internals
3. **Over-engineering** - This is an MVP alignment, not enterprise rewrite

## Architecture Summary

```
┌─────────────────────────────────────────────────────┐
│              User (CLI Interface)                   │
└────────────────────┬────────────────────────────────┘
                     │
                     │ crewchief maproom scan --parallel
                     ▼
┌─────────────────────────────────────────────────────┐
│         Commander.js (Subcommand Routing)           │
│  • maproom (parent)                                 │
│    ├─ scan (subcommand)                            │
│    ├─ search (subcommand)                          │
│    ├─ upsert (subcommand)                          │
│    ├─ watch (subcommand)                           │
│    ├─ branch-watch (subcommand)                    │
│    ├─ cache (subcommand)                           │
│    ├─ generate-embeddings (subcommand)             │
│    └─ db (nested parent)                           │
│        └─ migrate (nested subcommand)              │
└────────────────────┬────────────────────────────────┘
                     │
                     │ Extract args: ['scan', '--parallel']
                     ▼
┌─────────────────────────────────────────────────────┐
│      Validation Layer (TypeScript)                  │
│  • Check MAPROOM_DATABASE_URL                       │
│  • Check MAPROOM_EMBEDDING_PROVIDER (if needed)     │
│  • Provider-specific validation                     │
│  • Display errors/warnings                          │
└────────────────────┬────────────────────────────────┘
                     │
                     │ Validation passed
                     ▼
┌─────────────────────────────────────────────────────┐
│      runMaproomForward (Pure Forwarding)            │
│  • Resolve binary path                              │
│  • spawnSync(bin, args, {stdio: 'inherit'})         │
└────────────────────┬────────────────────────────────┘
                     │
                     │ spawn crewchief-maproom scan --parallel
                     ▼
┌─────────────────────────────────────────────────────┐
│         Rust Binary (crewchief-maproom)             │
│  • Parse arguments                                  │
│  • Read environment (with fallbacks)                │
│  • Execute indexing/search logic                    │
│  • Output to stdout/stderr                          │
└─────────────────────────────────────────────────────┘
```

**Key Properties:**
- **Loose coupling:** CLI doesn't know Rust internals
- **Single responsibility:** CLI handles UX, Rust handles logic
- **Easy to extend:** Add commands without coordination
- **User-friendly:** Validation catches errors early
- **Consistent:** All commands follow same pattern
