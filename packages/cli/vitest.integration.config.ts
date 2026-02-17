import { defineConfig } from 'vitest/config'

/**
 * Integration test configuration for CLI package
 *
 * Includes tests excluded from the default CI run:
 * - tests/sdk/spawner.test.ts — requires Claude Code with valid API credentials
 *
 * Run with: pnpm test:integration
 */
export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/sdk/spawner.test.ts'],
    testTimeout: 60000,
    hookTimeout: 30000,
  },
})
