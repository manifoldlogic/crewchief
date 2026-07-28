import { spawnSync } from 'node:child_process'
import { Command } from 'commander'
import { validateMaproomEnvironment, displayValidationResult } from './maproom-validation.js'
import { loadConfig } from '../config/loader.js'
import { findMaproomBinary } from '../utils/maproom-binary.js'

/**
 * Resolve the maproom binary path. Shared between runMaproomForward and
 * runMaproomSearch so the resolution logic is not duplicated.
 */
async function resolveMaproomBinaryPath(): Promise<string | null> {
  let configPath: string | undefined
  try {
    const config = await loadConfig()
    configPath = config.repository.maproomBinaryPath
  } catch {
    // Config file missing or invalid - continue with defaults
  }

  const result = findMaproomBinary({ configPath })

  if (!result.path) {
    console.error(
      'maproom binary not found. Options:\n' +
        '1. Install globally: npm install -g @crewchief/cli\n' +
        '2. Set MAPROOM_BIN environment variable\n' +
        '3. Add maproomBinaryPath to crewchief.config.js\n\n' +
        'Resolution attempts:\n' +
        '- Environment: ' +
        (process.env.MAPROOM_BIN || process.env.CREWCHIEF_MAPROOM_BIN || 'not set') +
        '\n' +
        '- Config: ' +
        (configPath || 'not configured') +
        '\n' +
        '- Global: not found\n' +
        '- Packaged: not found',
    )
    return null
  }

  return result.path
}

export async function runMaproomForward(args: string[]) {
  const subcommand = args[0]

  // Skip validation for help commands and non-database commands
  const skipValidation = args.includes('--help') || args.includes('-h') || subcommand === 'cache'

  if (!skipValidation) {
    // Commands that require database and may need embeddings
    const needsValidation = ['scan', 'upsert', 'search', 'generate-embeddings']

    if (needsValidation.includes(subcommand)) {
      const validation = validateMaproomEnvironment()
      displayValidationResult(validation)

      if (!validation.valid) {
        process.exitCode = 1
        return
      }
    }
  }

  const binaryPath = await resolveMaproomBinaryPath()
  if (!binaryPath) {
    process.exitCode = 1
    return
  }

  const res = spawnSync(binaryPath, args, { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}

/**
 * Run a maproom search, forwarding args directly to the binary.
 *
 * The previous auto-index logic (SQLite filesystem check + spurious scan) has
 * been removed. Under the shared-Postgres backend the database file never
 * exists locally, so every search was triggering a wasted scan. maproom's own
 * error messages guide users who have not yet indexed their repository.
 */
export async function runMaproomSearch(args: string[]) {
  const validation = validateMaproomEnvironment()
  displayValidationResult(validation)
  if (!validation.valid) {
    process.exitCode = 1
    return
  }

  const binaryPath = await resolveMaproomBinaryPath()
  if (!binaryPath) {
    process.exitCode = 1
    return
  }

  const res = spawnSync(binaryPath, ['search', ...args], { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}

export function registerMaproomCommands(program: Command) {
  const maproom = program
    .command('maproom')
    .description('Semantic code indexing and search')
    .addHelpText(
      'after',
      '\nExamples:\n  $ crewchief maproom scan              # Index current repository\n  $ crewchief maproom search "auth"     # Search for authentication code\n  $ crewchief maproom watch             # Auto-index on file changes\n  $ crewchief maproom db migrate        # Initialize database',
    )

  maproom
    .command('scan')
    .description('Scan and index repository files into the configured database backend (SQLite or PostgreSQL)')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText(
      'after',
      '\nAuto-detects: repo name, worktree, file path, and commit from git context\nSupports: TypeScript, JavaScript, Rust, Markdown, JSON, YAML, TOML',
    )
    .action(async (args) => await runMaproomForward(['scan', ...(args || [])]))

  maproom
    .command('search')
    .description('Semantic search across indexed code, docs, and configs')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText(
      'after',
      '\nExamples:\n  $ crewchief maproom search "authentication flow"\n  $ crewchief maproom search "database queries" --limit 10',
    )
    .action(async (args) => await runMaproomSearch(args || []))

  maproom
    .command('upsert')
    .description('Update specific files in the index at a given commit')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText('after', '\nExample: crewchief maproom upsert src/index.ts src/utils.ts')
    .action(async (args) => await runMaproomForward(['upsert', ...(args || [])]))

  maproom
    .command('watch')
    .description('Watch repository for changes and auto-index modified files')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText('after', '\nAuto-detects git context and watches for file changes\nPress Ctrl-C to stop watching')
    .action(async (args) => await runMaproomForward(['watch', ...(args || [])]))

  // Nested subcommand for database operations
  const db = maproom.command('db').description('Database operations')

  db.command('migrate')
    .description('Initialize/migrate the configured database backend (SQLite or PostgreSQL) for code indexing')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action(async (args) => await runMaproomForward(['db', 'migrate', ...(args || [])]))

  // New commands
  maproom
    .command('branch-watch')
    .description('Auto-index worktrees on branch switch')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action(async (args) => await runMaproomForward(['branch-watch', ...(args || [])]))

  maproom
    .command('cache')
    .description('Manage maproom caches')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action(async (args) => await runMaproomForward(['cache', ...(args || [])]))

  maproom
    .command('generate-embeddings')
    .description('Generate embeddings for indexed chunks')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action(async (args) => await runMaproomForward(['generate-embeddings', ...(args || [])]))

  maproom
    .command('clean-ignored')
    .description('Delete indexed chunks matching patterns in .maproomignore')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText(
      'after',
      '\nExamples:\n  $ crewchief maproom clean-ignored --repo myrepo --worktree main --dry-run\n  $ crewchief maproom clean-ignored --repo myrepo --worktree main',
    )
    .action(async (args) => await runMaproomForward(['clean-ignored', ...(args || [])]))
}
