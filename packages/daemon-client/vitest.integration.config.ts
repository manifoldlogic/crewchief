import { defineConfig } from 'vitest/config'

/**
 * Integration test configuration
 *
 * These tests require a running crewchief-maproom daemon and PostgreSQL database.
 * Run with: pnpm test:integration
 */
export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    // Only include integration tests (the tests/ folder)
    include: ['tests/**/*.test.ts'],
    testTimeout: 60000,
    hookTimeout: 30000,
  },
})
