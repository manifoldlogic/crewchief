import { spawnSync } from 'node:child_process'
import chalk from 'chalk'
import { Command } from 'commander'
import { runCommand } from '../utils/exec.js'
import { findMaproomBinary } from '../utils/maproom-binary.js'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** A capability that is ready to use right now. */
export type ReadyItem = {
  name: string
  note?: string // e.g. "[tmux detected]"
}

/** A capability that can be added with a concrete action. */
export type AddableItem = {
  name: string
  reason: string // e.g. "add embedding provider"
  actions: { label: string; command: string }[] // at least one
}

/** A capability that is not applicable in this environment. */
export type NotApplicableItem = {
  name: string
  reason: string // e.g. "deprecated -- use MCP server instead"
}

export type CapabilityTiers = {
  ready: ReadyItem[]
  addable: AddableItem[]
  notApplicable: NotApplicableItem[]
  /** True when a required prerequisite (Node, git) is missing. */
  hasCriticalErrors: boolean
  /** Critical error messages to display. */
  criticalErrors: string[]
}

// ---------------------------------------------------------------------------
// Individual checks (exported for testing)
// ---------------------------------------------------------------------------

export function compareNodeVersion(minMajor: number): boolean {
  const match = /^v(\d+)\.(\d+)\.(\d+)/.exec(process.version)
  if (!match) return false
  const major = Number(match[1])
  return major >= minMajor
}

export async function checkBinaryAvailable(
  cmd: string,
  args: string[] = ['--version'],
): Promise<{ ok: boolean; version?: string }> {
  try {
    const { exitCode, stdout, stderr } = await runCommand(cmd, args, { timeoutMs: 5_000 })
    const out = (stdout || stderr || '').trim()
    return { ok: exitCode === 0, version: out.split('\n')[0] }
  } catch {
    return { ok: false }
  }
}

export function checkMaproomBinary(): boolean {
  const result = findMaproomBinary()
  return result.path !== null
}

export async function checkTmuxAvailable(): Promise<{ ok: boolean; version?: string }> {
  return checkBinaryAvailable('tmux', ['-V'])
}

export function checkITermAvailable(): { ok: boolean; notMacOS: boolean } {
  if (process.platform !== 'darwin') {
    return { ok: false, notMacOS: true }
  }
  try {
    const res = spawnSync('osascript', ['-e', 'tell application "System Events" to name of every application process'])
    const apps = res.stdout.toString()
    return { ok: apps.includes('iTerm'), notMacOS: false }
  } catch {
    return { ok: false, notMacOS: false }
  }
}

export async function checkEmbeddingProvider(): Promise<{ provider: string } | null> {
  // Check OpenAI
  if (process.env.OPENAI_API_KEY) {
    return { provider: 'openai' }
  }
  // Check Google Vertex
  if (process.env.GOOGLE_APPLICATION_CREDENTIALS) {
    return { provider: 'google-vertex' }
  }
  // Check Ollama
  try {
    const { exitCode } = await runCommand(
      'curl',
      ['-s', '-o', '/dev/null', '-w', '%{http_code}', 'http://localhost:11434/api/tags'],
      { timeoutMs: 3_000 },
    )
    if (exitCode === 0) {
      return { provider: 'ollama' }
    }
  } catch {
    // Ollama not available
  }
  return null
}

// ---------------------------------------------------------------------------
// Tier builder
// ---------------------------------------------------------------------------

export async function buildCapabilityTiers(): Promise<CapabilityTiers> {
  const ready: ReadyItem[] = []
  const addable: AddableItem[] = []
  const notApplicable: NotApplicableItem[] = []
  const criticalErrors: string[] = []

  // --- Required: Node.js ---
  const nodeOk = compareNodeVersion(18)
  if (!nodeOk) {
    criticalErrors.push(`Node.js ${process.version} detected; require >= v18. Install: https://nodejs.org/`)
  }

  // --- Required: git ---
  const git = await checkBinaryAvailable('git', ['--version'])
  if (git.ok) {
    ready.push({ name: 'Worktree management', note: git.version })
  } else {
    criticalErrors.push('git not found. Install: brew install git / sudo apt install git')
  }

  // --- Maproom / FTS ---
  const hasMaproom = checkMaproomBinary()
  if (hasMaproom) {
    ready.push({ name: 'Code search (FTS)' })
  } else {
    addable.push({
      name: 'Code search (FTS)',
      reason: 'maproom binary not found',
      actions: [{ label: 'Install', command: 'npm install -g crewchief' }],
    })
  }

  // --- tmux ---
  const tmux = await checkTmuxAvailable()
  if (tmux.ok) {
    ready.push({ name: 'Agent orchestration (tmux)', note: `[${tmux.version || 'tmux detected'}]` })
  } else {
    addable.push({
      name: 'Agent orchestration (tmux)',
      reason: 'install tmux for multi-agent orchestration',
      actions: [
        { label: 'Debian/Ubuntu', command: 'sudo apt install tmux' },
        { label: 'macOS', command: 'brew install tmux' },
      ],
    })
  }

  // --- Embedding provider (semantic vector search) ---
  const embedding = await checkEmbeddingProvider()
  if (embedding) {
    ready.push({ name: 'Semantic vector search', note: `[${embedding.provider}]` })
  } else {
    addable.push({
      name: 'Semantic vector search',
      reason: 'add embedding provider',
      actions: [
        { label: 'Fastest:  ollama (local)', command: 'crewchief maproom setup --embeddings ollama' },
        { label: 'Simplest: openai (cloud)', command: 'crewchief maproom setup --embeddings openai' },
      ],
    })
  }

  // --- iTerm2 ---
  const iterm = checkITermAvailable()
  if (iterm.ok) {
    ready.push({ name: 'Agent orchestration (iTerm2)', note: '[iTerm2 detected]' })
  } else if (iterm.notMacOS) {
    addable.push({
      name: 'Agent orchestration (iTerm2)',
      reason: 'macOS + iTerm2 required',
      actions: [{ label: 'Install', command: 'https://iterm2.com' }],
    })
  } else {
    // macOS but iTerm2 not installed
    addable.push({
      name: 'Agent orchestration (iTerm2)',
      reason: 'iTerm2 not detected',
      actions: [{ label: 'Install', command: 'https://iterm2.com' }],
    })
  }

  // --- VSCode extension (deprecated) ---
  notApplicable.push({
    name: 'VSCode extension',
    reason: 'deprecated -- use MCP server instead',
  })

  return {
    ready,
    addable,
    notApplicable,
    hasCriticalErrors: criticalErrors.length > 0,
    criticalErrors,
  }
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/** Column width for aligning status/notes. */
const COL_WIDTH = 34

function padName(name: string): string {
  return name.padEnd(COL_WIDTH)
}

export function formatCapabilityTiers(tiers: CapabilityTiers): string {
  const lines: string[] = []

  // Critical errors first
  if (tiers.hasCriticalErrors) {
    for (const err of tiers.criticalErrors) {
      lines.push(chalk.red(`  REQUIRED  ${err}`))
    }
    lines.push('')
  }

  // WHAT YOU CAN DO RIGHT NOW
  lines.push(chalk.bold.green('WHAT YOU CAN DO RIGHT NOW'))
  if (tiers.ready.length === 0) {
    lines.push(chalk.dim('  (none)'))
  } else {
    for (const item of tiers.ready) {
      const note = item.note ? chalk.dim(`  ${item.note}`) : ''
      lines.push(`  ${padName(item.name)}${chalk.green('ready')}${note}`)
    }
  }

  // WHAT YOU CAN ADD
  lines.push('')
  lines.push(chalk.bold.yellow('WHAT YOU CAN ADD'))
  if (tiers.addable.length === 0) {
    lines.push(chalk.dim('  (nothing to add -- everything is ready)'))
  } else {
    for (const item of tiers.addable) {
      lines.push(`  ${padName(item.name)}${chalk.yellow(item.reason)}`)
      for (const action of item.actions) {
        lines.push(chalk.dim(`    ${action.label.padEnd(28)}${action.command}`))
      }
    }
  }

  // WHAT IS NOT APPLICABLE
  lines.push('')
  lines.push(chalk.bold.dim('WHAT IS NOT APPLICABLE'))
  if (tiers.notApplicable.length === 0) {
    lines.push(chalk.dim('  (none)'))
  } else {
    for (const item of tiers.notApplicable) {
      lines.push(chalk.dim(`  ${padName(item.name)}${item.reason}`))
    }
  }

  return lines.join('\n')
}

// ---------------------------------------------------------------------------
// JSON output (preserves --json flag behavior)
// ---------------------------------------------------------------------------

function buildJsonSummary(tiers: CapabilityTiers): object {
  return {
    ok: !tiers.hasCriticalErrors,
    ready: tiers.ready,
    addable: tiers.addable.map((a) => ({
      name: a.name,
      reason: a.reason,
      actions: a.actions,
    })),
    notApplicable: tiers.notApplicable,
    criticalErrors: tiers.criticalErrors,
  }
}

// ---------------------------------------------------------------------------
// Command registration
// ---------------------------------------------------------------------------

export function registerDoctorCommand(program: Command): void {
  program
    .command('doctor')
    .alias('prereq')
    .description('Show capability tiers: what works now, what you can add, what is not applicable')
    .option('--json', 'Output JSON')
    .action(async (opts: { json?: boolean }) => {
      const tiers = await buildCapabilityTiers()

      if (opts.json) {
        console.log(JSON.stringify(buildJsonSummary(tiers), null, 2))
        process.exit(tiers.hasCriticalErrors ? 1 : 0)
        return
      }

      console.log(formatCapabilityTiers(tiers))
      process.exit(tiers.hasCriticalErrors ? 1 : 0)
    })
}
