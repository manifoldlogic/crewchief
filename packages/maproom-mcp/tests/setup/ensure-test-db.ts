/**
 * Test Database Setup
 *
 * Automatically ensures the test database container is running before tests.
 * Used as vitest globalSetup.
 */

import { execSync, spawn } from 'node:child_process'
import { resolve } from 'node:path'

const CONTAINER_NAME = 'maproom-postgres-test'
const COMPOSE_FILE = resolve(__dirname, '../../../vscode-maproom/config/docker-compose.yml')
const MAX_WAIT_SECONDS = 60
const HEALTH_CHECK_INTERVAL_MS = 1000

/**
 * Check if a Docker container is running
 */
function isContainerRunning(name: string): boolean {
  try {
    const result = execSync(`docker inspect -f '{{.State.Running}}' ${name} 2>/dev/null`, {
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe']
    })
    return result.trim() === 'true'
  } catch {
    return false
  }
}

/**
 * Check if the database is ready to accept connections
 */
function isDatabaseReady(): boolean {
  try {
    execSync(`docker exec ${CONTAINER_NAME} pg_isready -U maproom -d maproom_test`, {
      stdio: ['pipe', 'pipe', 'pipe']
    })
    return true
  } catch {
    return false
  }
}

/**
 * Start the test database container
 */
function startTestDatabase(): void {
  console.log('🐘 Starting test database container...')
  try {
    execSync(`docker compose -f "${COMPOSE_FILE}" --profile test up -d postgres-test`, {
      stdio: 'inherit'
    })
  } catch (error) {
    throw new Error(`Failed to start test database: ${error}`)
  }
}

/**
 * Wait for the database to be healthy
 */
async function waitForDatabase(): Promise<void> {
  console.log('⏳ Waiting for test database to be ready...')
  const startTime = Date.now()

  while (Date.now() - startTime < MAX_WAIT_SECONDS * 1000) {
    if (isDatabaseReady()) {
      console.log('✅ Test database is ready')
      return
    }
    await new Promise(resolve => setTimeout(resolve, HEALTH_CHECK_INTERVAL_MS))
  }

  throw new Error(`Test database did not become ready within ${MAX_WAIT_SECONDS} seconds`)
}

/**
 * Vitest global setup - ensures test database is running
 */
export async function setup(): Promise<void> {
  // Skip in CI - GitHub Actions has its own postgres-test service
  if (process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true') {
    console.log('ℹ️  CI environment detected, skipping local database setup')
    return
  }

  // Check if Docker is available
  try {
    execSync('docker --version', { stdio: ['pipe', 'pipe', 'pipe'] })
  } catch {
    console.warn('⚠️  Docker not available, skipping database auto-start')
    return
  }

  // Check if container is already running
  if (isContainerRunning(CONTAINER_NAME)) {
    if (isDatabaseReady()) {
      console.log('✅ Test database already running and ready')
      return
    }
    console.log('🔄 Container running but not ready, waiting...')
  } else {
    startTestDatabase()
  }

  await waitForDatabase()
}

/**
 * Vitest global teardown - optionally stop the container
 * Note: We leave it running by default for faster subsequent test runs
 */
export async function teardown(): Promise<void> {
  // Only stop if explicitly requested
  if (process.env.STOP_TEST_DB === 'true') {
    console.log('🛑 Stopping test database container...')
    try {
      execSync(`docker compose -f "${COMPOSE_FILE}" --profile test stop postgres-test`, {
        stdio: 'inherit'
      })
    } catch {
      // Ignore errors during cleanup
    }
  }
}
