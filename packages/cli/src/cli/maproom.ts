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
    .description('Forward maproom subcommands to the bundled Rust binary')
    .allowUnknownOption(true)
    .argument('[args...]', 'Arguments forwarded to crewchief-maproom')
    .action((args: string[]) => runMaproomForward(args || []))

  // Convenience shims for common subcommands
  const sub = program.command('maproom:db').description('Run database migrations').allowUnknownOption(true)
  sub.argument('[args...]').action((args: string[]) => runMaproomForward(['db', ...(args || [])]))

  program
    .command('maproom:scan')
    .description('Scan and index files into Postgres')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args: string[]) => runMaproomForward(['scan', ...(args || [])]))

  program
    .command('maproom:upsert')
    .description('Upsert files at a given commit')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args: string[]) => runMaproomForward(['upsert', ...(args || [])]))

  program
    .command('maproom:watch')
    .description('Watch a worktree for changes and incrementally upsert')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args: string[]) => runMaproomForward(['watch', ...(args || [])]))

  program
    .command('maproom:search')
    .description('Full-text search against indexed chunks')
    .allowUnknownOption(true)
    .argument('[args...]')
    .action((args: string[]) => runMaproomForward(['search', ...(args || [])]))
}
