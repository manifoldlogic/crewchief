import { defineConfig } from 'vitest/config'

// Determine the correct database host based on environment
function getTestDatabaseUrl(): string {
  // Explicit override takes precedence
  if (process.env.TEST_MAPROOM_DATABASE_URL) {
    return process.env.TEST_MAPROOM_DATABASE_URL
  }
  // CI uses localhost (GitHub Actions service container)
  if (process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true') {
    return 'postgresql://maproom:maproom@localhost:5434/maproom_test'
  }
  // Default: host.docker.internal works from both devcontainers and host machines
  // (On Mac/Windows Docker Desktop, host.docker.internal resolves to the host)
  // (On Linux, it may need --add-host in docker run, but falls back gracefully)
  return 'postgresql://maproom:maproom@host.docker.internal:5434/maproom_test'
}

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts', 'tests/e2e/**/*_test.ts'],
    // Auto-start test database before tests run
    globalSetup: ['./tests/setup/ensure-test-db.ts'],
    env: {
      // Use the appropriate test database URL
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
    // The test-corpus is shared between test files, parallel execution causes cleanup/index conflicts
    poolOptions: {
      threads: {
        singleThread: true,
        minThreads: 1,
        maxThreads: 1
      }
    }
  }
})
