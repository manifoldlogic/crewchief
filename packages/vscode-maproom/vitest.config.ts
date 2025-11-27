import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    // Run tests that use port 11434 sequentially to avoid conflicts
    // (setupWizard.test.ts and ollama/client.test.ts both bind to this port)
    fileParallelism: false,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'json-summary'],
      exclude: [
        'node_modules/**',
        'dist/**',
        '**/*.test.ts',
        '**/*.config.ts',
        'src/test/**',
      ],
      thresholds: {
        lines: 50,
        functions: 50,
        branches: 50,
        statements: 50,
      },
    },
  },
})
