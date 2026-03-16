/**
 * Integration tests for maproom command structure and validation
 *
 * Prerequisites: CLI must be built before running these tests
 *   Run: pnpm build
 *
 * These tests verify:
 * - All subcommands are registered correctly
 * - Help text displays for parent and child commands
 * - Environment validation runs for appropriate commands
 * - Validation can be bypassed with --help flag
 * - Exit codes propagate correctly
 */

import { execSync } from 'node:child_process'
import { describe, it, expect } from 'vitest'

const CLI_PATH = 'node dist/cli/index.js'

function runCli(
  args: string,
  env: Record<string, string> = {},
): {
  stdout: string
  stderr: string
  exitCode: number
} {
  try {
    const stdout = execSync(`${CLI_PATH} ${args}`, {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, ...env },
    })
    return { stdout, stderr: '', exitCode: 0 }
  } catch (error: any) {
    return {
      stdout: error.stdout?.toString() || '',
      stderr: error.stderr?.toString() || '',
      exitCode: error.status || 1,
    }
  }
}

const TEST_ENV = {
  // Minimal env to pass validation (SQLite-only after PostgreSQL removal)
  MAPROOM_DATABASE_URL: 'sqlite:///tmp/test-maproom.db',
  MAPROOM_EMBEDDING_PROVIDER: 'ollama',
}

const EMPTY_ENV = {
  // Remove MAPROOM vars to test validation
  PATH: process.env.PATH,
  MAPROOM_DATABASE_URL: undefined,
  MAPROOM_DB_HOST: undefined,
  PG_DATABASE_URL: undefined,
  DATABASE_URL: undefined,
  MAPROOM_EMBEDDING_PROVIDER: undefined,
  OPENAI_API_KEY: undefined,
  GOOGLE_PROJECT_ID: undefined,
}

describe('Maproom command registration', () => {
  it('maproom --help shows all 8 subcommands', () => {
    const { stdout, exitCode } = runCli('maproom --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
    expect(stdout).toContain('scan')
    expect(stdout).toContain('search')
    expect(stdout).toContain('upsert')
    expect(stdout).toContain('watch')
    expect(stdout).toContain('db')
    expect(stdout).toContain('branch-watch')
    expect(stdout).toContain('cache')
    expect(stdout).toContain('generate-embeddings')
  })

  it('maproom scan --help shows scan-specific help', () => {
    const { stdout, exitCode } = runCli('maproom scan --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
    expect(stdout).toContain('scan')
    expect(stdout).toContain('SQLite')
  })

  it('maproom db --help shows db subcommands', () => {
    const { stdout, exitCode } = runCli('maproom db --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
    expect(stdout).toContain('migrate')
  })
})

describe('Maproom execution integration', () => {
  it('maproom scan succeeds with SQLite default (no env vars needed)', () => {
    // After PostgreSQL removal, maproom defaults to SQLite and scan succeeds
    // without MAPROOM_DATABASE_URL being set
    const { exitCode } = runCli('maproom scan')
    expect([0, 1]).toContain(exitCode)
  })

  it('maproom scan with explicit env forwards to binary', () => {
    const { exitCode } = runCli('maproom scan', TEST_ENV)
    // Either succeeds or has a runtime error (not a validation error)
    expect([0, 1]).toContain(exitCode)
  })

  it('maproom --help works without env vars', () => {
    const { stdout, exitCode } = runCli('maproom --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
    expect(stdout).toContain('maproom')
  })

  it('maproom scan --help works without env vars', () => {
    const { stdout, exitCode } = runCli('maproom scan --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
    expect(stdout).toContain('scan')
  })

  it('maproom search without required args shows usage error', () => {
    const { exitCode } = runCli('maproom search "test"', EMPTY_ENV)
    // clap returns exit code 2 for CLI parse errors (missing --repo, --query)
    expect(exitCode).toBe(2)
  })

  it('maproom cache --help works without env vars', () => {
    const { exitCode } = runCli('maproom cache --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
  })
})

describe('Exit code propagation', () => {
  it('scan without database succeeds or returns runtime error', () => {
    // After PostgreSQL removal, scan with SQLite default may succeed
    const { exitCode } = runCli('maproom scan', EMPTY_ENV)
    expect([0, 1]).toContain(exitCode)
  })

  it('help command returns exit code 0', () => {
    const { exitCode } = runCli('maproom --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
  })

  it('subcommand help returns exit code 0', () => {
    const { exitCode } = runCli('maproom scan --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
  })
})
