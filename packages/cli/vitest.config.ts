import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.test.ts', 'tests/**/*.test.ts'],
    // Exclude integration tests from default run
    // - spawner.test.ts requires Claude Code with valid API credentials (run via test:integration)
    exclude: ['**/node_modules/**', '**/.crewchief/**', '**/dist/**', 'tests/sdk/spawner.test.ts'],
  },
})
