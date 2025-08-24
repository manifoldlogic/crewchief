import chalk from 'chalk'
import { Command } from 'commander'
import { spawnSync } from 'node:child_process'
import { runCommand } from '../utils/exec'

type CheckResult = {
  name: string
  ok: boolean
  message: string
  details?: string
  optional?: boolean
}

function compareNodeVersion(minMajor: number): boolean {
  const match = /^v(\d+)\.(\d+)\.(\d+)/.exec(process.version)
  if (!match) return false
  const major = Number(match[1])
  return major >= minMajor
}

async function checkBinary(cmd: string, args: string[] = ['--version']): Promise<CheckResult> {
  try {
    const { exitCode, stdout, stderr } = await runCommand(cmd, args, { timeoutMs: 5_000 })
    const out = (stdout || stderr || '').trim()
    return {
      name: `${cmd} on PATH`,
      ok: exitCode === 0,
      message: exitCode === 0 ? out.split('\n')[0] : `${cmd} not found or failed to execute`,
      details: out,
    }
  } catch (err) {
    return {
      name: `${cmd} on PATH`,
      ok: false,
      message: `${cmd} not found or failed to execute`,
      details: err instanceof Error ? err.message : String(err),
    }
  }
}

async function checkITerm(): Promise<CheckResult> {
  try {
    // Check if on macOS
    if (process.platform !== 'darwin') {
      return {
        name: 'iTerm2',
        ok: false,
        message: 'iTerm2 requires macOS (optional, needed for agent features)',
        optional: true,
      }
    }

    // Check if iTerm2 is installed
    const res = spawnSync('osascript', [
      '-e',
      'tell application "System Events" to name of every application process',
    ])
    const apps = res.stdout.toString()
    const hasITerm = apps.includes('iTerm')
    
    return {
      name: 'iTerm2',
      ok: hasITerm,
      message: hasITerm 
        ? 'iTerm2 found (optional, needed for agent features)' 
        : 'iTerm2 not found (optional, needed for agent features)',
      optional: true,
    }
  } catch {
    return {
      name: 'iTerm2',
      ok: false,
      message: 'Could not check for iTerm2 (optional, needed for agent features)',
      optional: true,
    }
  }
}

async function runChecks(): Promise<CheckResult[]> {
  const results: CheckResult[] = []

  // Node runtime
  const nodeOk = compareNodeVersion(18)
  results.push({
    name: 'Node.js version',
    ok: nodeOk,
    message: `Detected ${process.version}; require >= v18`,
  })

  // Git
  results.push(await checkBinary('git', ['--version']))

  // iTerm2 (optional but needed for agent features)
  results.push(await checkITerm())

  // pnpm (optional but recommended)
  const pnpm = await checkBinary('pnpm', ['--version'])
  pnpm.optional = true
  pnpm.name = 'pnpm on PATH (recommended)'
  results.push(pnpm)

  return results
}

function printResults(results: CheckResult[]): { hasErrors: boolean } {
  let hasErrors = false
  for (const r of results) {
    const label = r.ok ? chalk.green('OK') : r.optional ? chalk.yellow('WARN') : chalk.red('FAIL')
    const name = chalk.bold(r.name)
    console.log(`${label} ${name} — ${r.message}`)
    if (r.details && !r.ok) {
      console.log(chalk.dim(`  details: ${r.details}`))
    }
    if (!r.ok && !r.optional) hasErrors = true
  }

  if (hasErrors) {
    console.log()
    console.log(chalk.red('One or more required prerequisites are missing.'))
    console.log('Required: Node >= 18, git')
    console.log('Optional: iTerm2 (for agent features), pnpm (recommended)')
    console.log()
    console.log('Install hints (macOS):')
    console.log('  - Node:   https://nodejs.org/ (or brew install node)')
    console.log('  - git:    brew install git')
    console.log('  - iTerm2: https://iterm2.com/downloads.html')
    console.log('  - pnpm:   corepack enable && corepack prepare pnpm@latest --activate')
  }

  return { hasErrors }
}

export function registerDoctorCommand(program: Command): void {
  program
    .command('doctor')
    .alias('prereq')
    .description('Check environment prerequisites (Node, git, iTerm2, pnpm)')
    .option('--json', 'Output JSON')
    .action(async (opts: { json?: boolean }) => {
      const results = await runChecks()
      const summary = { results, ok: results.every((r) => r.ok || r.optional) }
      if (opts.json) {
        console.log(JSON.stringify(summary, null, 2))
        process.exit(summary.ok ? 0 : 1)
        return
      }
      const { hasErrors } = printResults(results)
      process.exit(hasErrors ? 1 : 0)
    })
}
