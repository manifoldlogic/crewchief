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
  // Minimal env to pass validation
  MAPROOM_DATABASE_URL: 'postgresql://test:test@localhost:5432/test',
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
    expect(stdout).toContain('PostgreSQL')
  })

  it('maproom db --help shows db subcommands', () => {
    const { stdout, exitCode } = runCli('maproom db --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
    expect(stdout).toContain('migrate')
  })
})

describe('Maproom validation integration', () => {
  it('maproom scan without env vars shows validation error', () => {
    const { stderr, stdout, exitCode } = runCli('maproom scan', EMPTY_ENV)
    expect(exitCode).toBe(1)
    // Validation errors appear in stderr OR stdout depending on logger implementation
    const output = stderr + stdout
    expect(output).toContain('database')
  })

  it('validation error message contains MAPROOM_DATABASE_URL', () => {
    const { stderr, stdout, exitCode } = runCli('maproom scan', EMPTY_ENV)
    expect(exitCode).toBe(1)
    const output = stderr + stdout
    expect(output).toContain('MAPROOM_DATABASE_URL')
  })

  it('maproom scan with valid env forwards to binary (or shows binary not found)', () => {
    const { stderr, stdout, exitCode } = runCli('maproom scan', TEST_ENV)
    // Either:
    // - exit 0: forwards successfully
    // - exit 1: binary not found, connection refused, or other runtime error
    // We're testing validation passed (env vars accepted), not binary execution
    expect([0, 1]).toContain(exitCode)

    // If exit 1, should be a runtime error (binary not found OR database not available)
    // NOT a validation error (which would mention MAPROOM_DATABASE_URL missing)
    if (exitCode === 1) {
      const output = stderr + stdout
      // Should NOT be a validation error
      expect(output).not.toContain('MAPROOM_DATABASE_URL is required')
      // Should be either binary not found or database connection error
      const isBinaryError = output.includes('crewchief-maproom') || output.includes('not found')
      const isConnectionError = output.includes('Connection refused') || output.includes('error connecting')
      expect(isBinaryError || isConnectionError).toBe(true)
    }
  })

  it('maproom --help bypasses validation (no env needed)', () => {
    const { stdout, exitCode } = runCli('maproom --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
    expect(stdout).toContain('maproom')
  })

  it('maproom scan --help bypasses validation (no env needed)', () => {
    const { stdout, exitCode } = runCli('maproom scan --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
    expect(stdout).toContain('scan')
  })

  it('maproom search without env vars shows validation error', () => {
    const { stderr, stdout, exitCode } = runCli('maproom search "test"', EMPTY_ENV)
    expect(exitCode).toBe(1)
    const output = stderr + stdout
    expect(output).toContain('database')
  })

  it('maproom cache bypasses validation (no database needed)', () => {
    const { exitCode } = runCli('maproom cache --help', EMPTY_ENV)
    expect(exitCode).toBe(0)
  })
})

describe('Exit code propagation', () => {
  it('validation error returns exit code 1', () => {
    const { exitCode } = runCli('maproom scan', EMPTY_ENV)
    expect(exitCode).toBe(1)
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
