import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { Command } from 'commander'

function resolvePackagedMaproomBin(): string | null {
  const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'

  // Map architecture names to match our build script convention
  const arch = process.arch === 'x64' ? 'x64' : process.arch === 'arm64' ? 'arm64' : process.arch
  const platform = `${process.platform}-${arch}`

  // 1) Explicit env override
  const envBin = process.env.CREWCHIEF_MAPROOM_BIN
  if (envBin && fs.existsSync(envBin)) return envBin

  // 2) Packaged inside this CLI package with platform subdirectory
  try {
    const here = __dirname
    const out = path.join(here, '..', 'bin', platform, execName)
    if (fs.existsSync(out)) return out
  } catch {
    // ignore errors
  }

  // 3) Fallback to symlink in bin root (for backwards compatibility)
  try {
    const here = __dirname
    const out = path.join(here, '..', 'bin', execName)
    if (fs.existsSync(out)) return out
  } catch {
    // ignore errors
  }

  // 4) Packaged in sibling maproom-mcp package (monorepo dev convenience)
  try {
    const here = __dirname
    const mcp = path.join(here, '..', '..', 'maproom-mcp', 'bin', platform, execName)
    if (fs.existsSync(mcp)) return mcp
  } catch {
    // ignore errors
  }

  // 5) Global on PATH
  const which = spawnSync('bash', ['-lc', 'command -v crewchief-maproom'])
  if (which.status === 0) return 'crewchief-maproom'

  return null
}

function runMaproomForward(args: string[]) {
  const bin = resolvePackagedMaproomBin()
  if (!bin) {
    console.error(
      'crewchief-maproom not found. Ensure it is installed or built. You can set CREWCHIEF_MAPROOM_BIN to an absolute path.',
    )
    process.exitCode = 1
    return
  }
  const res = spawnSync(bin, args, { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}

export function registerMaproomCommands(program: Command) {
  program
    .command('maproom')
    .description('Semantic code indexing and search (forwards to Rust binary)')
    .allowUnknownOption(true)
    .argument('[args...]', 'Arguments forwarded to crewchief-maproom')
    .addHelpText('after', '\nExamples:\n  $ crewchief maproom --help        # Show all maproom commands\n  $ crewchief maproom:scan           # Index current repository\n  $ crewchief maproom:search "auth"  # Search for authentication code')
    .action((args: string[]) => runMaproomForward(args || []))

  // Convenience shims for common subcommands
  const sub = program.command('maproom:db').description('Initialize/migrate PostgreSQL database for code indexing').allowUnknownOption(true)
  sub.argument('[args...]').action((args: string[]) => runMaproomForward(['db', ...(args || [])]))

  program
    .command('maproom:scan')
    .description('Scan and index repository files into PostgreSQL (auto-detects git context)')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText('after', '\nAuto-detects: repo name, worktree, file path, and commit from git context\nSupports: TypeScript, JavaScript, Rust, Markdown, JSON, YAML, TOML')
    .action((args: string[]) => runMaproomForward(['scan', ...(args || [])]))

  program
    .command('maproom:upsert')
    .description('Update specific files in the index at a given commit')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText('after', '\nExample: crewchief maproom:upsert src/index.ts src/utils.ts')
    .action((args: string[]) => runMaproomForward(['upsert', ...(args || [])]))

  program
    .command('maproom:watch')
    .description('Watch repository for changes and auto-index modified files')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText('after', '\nAuto-detects git context and watches for file changes\nPress Ctrl-C to stop watching')
    .action((args: string[]) => runMaproomForward(['watch', ...(args || [])]))

  program
    .command('maproom:search')
    .description('Semantic search across indexed code, docs, and configs')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText('after', '\nExamples:\n  $ crewchief maproom:search "authentication flow"\n  $ crewchief maproom:search "database queries" --limit 10')
    .action((args: string[]) => runMaproomForward(['search', ...(args || [])]))
}
