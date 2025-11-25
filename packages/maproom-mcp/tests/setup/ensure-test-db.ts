/**
 * Test Database Setup
 *
 * Automatically ensures the test database container is running and
 * the schema is initialized before tests.
 * Used as vitest globalSetup.
 *
 * Related: MCPSIMP-4003
 */

import { execSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const CONTAINER_NAME = 'maproom-postgres-test'
const COMPOSE_FILE = resolve(__dirname, '../../../vscode-maproom/config/docker-compose.yml')
const SCHEMA_FILE = resolve(__dirname, './init-schema.sql')
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
    // Use -p to ensure consistent project name matching the docker-compose.yml 'name' field
    execSync(`docker compose -p crewchief-dev-env -f "${COMPOSE_FILE}" --profile test up -d postgres-test`, {
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
 * Check if the maproom schema is initialized
 */
function isSchemaInitialized(): boolean {
  try {
    // Check if the chunks table exists in the maproom schema
    const result = execSync(
      `docker exec ${CONTAINER_NAME} psql -U maproom -d maproom_test -t -c "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='maproom' AND table_name='chunks')"`,
      { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
    )
    return result.trim() === 't'
  } catch {
    return false
  }
}

/**
 * Initialize the database schema
 */
function initializeSchema(): void {
  console.log('📊 Initializing test database schema...')
  try {
    // Read the schema file
    const schemaSQL = readFileSync(SCHEMA_FILE, 'utf-8')

    // Execute the schema via docker exec psql
    // Use a heredoc-style approach by piping SQL through stdin
    execSync(
      `docker exec -i ${CONTAINER_NAME} psql -U maproom -d maproom_test`,
      {
        input: schemaSQL,
        stdio: ['pipe', 'pipe', 'pipe'],
        encoding: 'utf-8'
      }
    )
    console.log('✅ Schema initialized successfully')
  } catch (error) {
    throw new Error(`Failed to initialize schema: ${error}`)
  }
}

/**
 * Verify schema initialization was successful
 */
function verifySchema(): void {
  console.log('🔍 Verifying schema...')

  const requiredTables = ['repos', 'worktrees', 'commits', 'files', 'chunks', 'code_embeddings', 'worktree_index_state']

  for (const table of requiredTables) {
    try {
      const result = execSync(
        `docker exec ${CONTAINER_NAME} psql -U maproom -d maproom_test -t -c "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='maproom' AND table_name='${table}')"`,
        { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
      )
      if (result.trim() !== 't') {
        throw new Error(`Table maproom.${table} not found`)
      }
    } catch (error) {
      throw new Error(`Schema verification failed for table ${table}: ${error}`)
    }
  }

  console.log('✅ Schema verification passed (all required tables exist)')
}

/**
 * Vitest global setup - ensures test database is running and schema is initialized
 */
export async function setup(): Promise<void> {
  // Skip in CI - GitHub Actions has its own postgres-test service with schema
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
      console.log('✅ Test database container already running')
    } else {
      console.log('🔄 Container running but not ready, waiting...')
      await waitForDatabase()
    }
  } else {
    startTestDatabase()
    await waitForDatabase()
  }

  // Initialize schema if needed (idempotent - safe to run multiple times)
  if (!isSchemaInitialized()) {
    initializeSchema()
  } else {
    console.log('✅ Schema already initialized')
  }

  // Verify schema regardless (catches partial initialization)
  verifySchema()
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
      execSync(`docker compose -p crewchief-dev-env -f "${COMPOSE_FILE}" --profile test stop postgres-test`, {
        stdio: 'inherit'
      })
    } catch {
      // Ignore errors during cleanup
    }
  }
}
