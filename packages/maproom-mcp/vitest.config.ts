import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts', 'tests/e2e/**/*_test.ts'],
    env: {
      MAPROOM_DATABASE_URL: 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
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
