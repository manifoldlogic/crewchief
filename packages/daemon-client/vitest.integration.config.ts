import { defineConfig } from 'vitest/config'

/**
 * Integration test configuration
 *
 * Includes tests excluded from the default CI run:
 * - tests/** — require a running maproom daemon
 * - src/__tests__/client.test.ts — mock-based but has async timing issues in CI
 *
 * Run with: pnpm test:integration
 */
export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts', 'src/__tests__/client.test.ts'],
    testTimeout: 60000,
    hookTimeout: 30000,
  },
})
