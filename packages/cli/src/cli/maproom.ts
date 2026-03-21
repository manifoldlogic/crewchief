import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { Command } from 'commander'
import { validateMaproomEnvironment, displayValidationResult } from './maproom-validation.js'
import { loadConfig } from '../config/loader.js'
import { findMaproomBinary } from '../utils/maproom-binary.js'

/**
 * Resolve the Maproom SQLite database path.
 *
 * Mirrors the Rust convention in `crates/maproom/src/db/connection.rs`:
 * 1. MAPROOM_DATABASE_URL env var (strip `sqlite://` prefix, expand `~`)
 * 2. ~/.maproom/maproom.db (default)
 */
export function resolveMaproomDbPath(): string {
  const envUrl = process.env.MAPROOM_DATABASE_URL
  if (envUrl) {
    let dbPath = envUrl
    if (dbPath.startsWith('sqlite://')) {
      dbPath = dbPath.slice('sqlite://'.length)
    }
    // Expand leading tilde
    if (dbPath.startsWith('~/')) {
      dbPath = path.join(os.homedir(), dbPath.slice(2))
    } else if (dbPath === '~') {
      dbPath = os.homedir()
    }
    return dbPath
  }
  return path.join(os.homedir(), '.maproom', 'maproom.db')
}

/**
 * Check whether a Maproom index exists by testing for the database file.
 *
 * This is a fast synchronous stat call (sub-millisecond) so it adds
 * negligible overhead to every search invocation.
 */
export function maproomIndexExists(): boolean {
  return fs.existsSync(resolveMaproomDbPath())
}

/**
 * Run a FTS-only scan to bootstrap the index for the current repository.
 *
 * @param binaryPath - Resolved path to the maproom binary
 * @returns The exit code from the scan (0 = success)
 */
export function runAutoIndexScan(binaryPath: string): number {
  console.log('No index found. Building FTS index for this repo (no embedding provider required).')
  const scanResult = spawnSync(binaryPath, ['scan'], { stdio: 'inherit' })
  return scanResult.status ?? 1
}

/**
 * Resolve the maproom binary path. Shared between runMaproomForward and
 * runMaproomSearchWithAutoIndex so the resolution logic is not duplicated.
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
 * Run a maproom search with auto-index support.
 *
 * Detection strategy (two-pronged):
 *   1. **Pre-flight**: Fast filesystem check for the SQLite database file.
 *      If the file does not exist, run a FTS-only scan before the search.
 *   2. **Post-flight**: If the search exits with code 2 (config_error,
 *      typically "database not found" or "repository not indexed"),
 *      auto-scan and retry once.
 *
 * The pre-flight check handles the common case (fresh install, no index)
 * with zero extra process spawns. The post-flight fallback catches edge
 * cases where the database file exists but the repo isn't indexed yet.
 */
export async function runMaproomSearchWithAutoIndex(args: string[]) {
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

  // Pre-flight: fast filesystem check for the database file
  if (!maproomIndexExists()) {
    const scanExit = runAutoIndexScan(binaryPath)
    if (scanExit !== 0) {
      console.error('Auto-index failed. Run "crewchief maproom scan" manually for details.')
      process.exitCode = scanExit
      return
    }
  }

  // Run the search
  const searchArgs = ['search', ...args]
  const res = spawnSync(binaryPath, searchArgs, { stdio: 'inherit' })

  // Post-flight fallback: if exit code 2 (config error / repo not indexed),
  // try auto-indexing and retry once
  if (res.status === 2) {
    const scanExit = runAutoIndexScan(binaryPath)
    if (scanExit !== 0) {
      console.error('Auto-index failed. Run "crewchief maproom scan" manually for details.')
      process.exitCode = scanExit
      return
    }
    // Retry search after scan
    const retryRes = spawnSync(binaryPath, searchArgs, { stdio: 'inherit' })
    if (retryRes.status !== 0) process.exitCode = retryRes.status ?? 1
    return
  }

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
    .description('Scan and index repository files into SQLite (auto-detects git context)')
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
    .action(async (args) => await runMaproomSearchWithAutoIndex(args || []))

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
    .description('Initialize/migrate SQLite database for code indexing')
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
