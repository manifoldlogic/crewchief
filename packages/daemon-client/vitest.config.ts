import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    globalSetup: './vitest.setup.ts',
    // Exclude integration tests and CI-flaky tests from default run
    // - tests/** requires a running daemon (performance.test.ts, etc.)
    // - client.test.ts has async mock timing issues in CI (run via test:integration)
    exclude: ['**/node_modules/**', '**/dist/**', 'tests/**', 'src/__tests__/client.test.ts'],
    // Run discovery tests sequentially to avoid PID file conflicts
    // (Rust daemon uses shared PID file - will be fixed in future ticket)
    pool: 'forks',
    poolOptions: {
      forks: {
        singleFork: true,
      },
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/**',
        'dist/**',
        'tests/**',
        '**/*.test.ts',
      ],
      thresholds: {
        statements: 80,
        branches: 80,
        functions: 80,
        lines: 80,
      },
    },
    testTimeout: 10000,
    hookTimeout: 10000,
  },
})
