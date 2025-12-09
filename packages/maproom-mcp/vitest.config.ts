import { defineConfig } from 'vitest/config'
import { homedir } from 'node:os'

// Get test database URL (SQLite only)
function getTestDatabaseUrl(): string {
  // Explicit override takes precedence
  if (process.env.TEST_MAPROOM_DATABASE_URL) {
    return process.env.TEST_MAPROOM_DATABASE_URL
  }
  // Default: use standard maproom database location
  return `sqlite://${homedir()}/.maproom/maproom.db`
}

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts', 'tests/e2e/**/*_test.ts', 'src/**/__tests__/**/*.test.ts'],
    env: {
      // Use SQLite test database
      MAPROOM_DATABASE_URL: getTestDatabaseUrl()
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/**/*.ts'],
      exclude: ['src/postinstall.ts'],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 75,
        statements: 80
      }
    },
    // E2E tests may need longer timeout
    testTimeout: 30000,
    hookTimeout: 30000,
    // Run tests sequentially to avoid database race conditions
    poolOptions: {
      threads: {
        singleThread: true,
        minThreads: 1,
        maxThreads: 1
      }
    }
  }
})
