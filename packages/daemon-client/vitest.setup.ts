/**
 * Global test setup for daemon-client
 *
 * Cleans up any existing daemon processes before tests run
 * to prevent PID file conflicts.
 */

import { execSync } from 'node:child_process'

export async function setup() {
  // Kill any existing daemon processes
  try {
    execSync('pkill -f "crewchief-maproom serve"', { stdio: 'ignore' })
    // Wait for processes to terminate
    await new Promise((resolve) => setTimeout(resolve, 1000))
  } catch {
    // pkill returns non-zero if no processes found, ignore
  }

  // Clean up stale PID, lock, and socket files using shell glob
  try {
    execSync('rm -f /tmp/maproom-*.pid /tmp/maproom-*.lock /tmp/maproom-*.sock /tmp/test-maproom-*.sock /tmp/test-maproom-*.lock', {
      stdio: 'ignore',
      shell: '/bin/bash',
    })
  } catch {
    // Ignore errors (files may not exist)
  }

  console.log('✓ Cleaned up daemon processes and files')
}

export async function teardown() {
  // Clean up after all tests
  await setup()
}
