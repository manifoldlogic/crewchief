/// <reference types="vitest" />
import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    setupFiles: ['./tests/setup.integration.ts'],
    include: [
      'tests/integration/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
    ],
    exclude: [
      'node_modules',
      'dist',
      'tests/e2e',
      'tests/unit',
      'src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
    ],
    testTimeout: 30000,
    hookTimeout: 30000,
    pool: 'forks',
    poolOptions: {
      forks: {
        singleFork: true,
      },
    },
  },
  
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src/client'),
      '@components': resolve(__dirname, 'src/client/components'),
      '@pages': resolve(__dirname, 'src/client/pages'),
      '@hooks': resolve(__dirname, 'src/client/hooks'),
      '@utils': resolve(__dirname, 'src/client/utils'),
      '@types': resolve(__dirname, 'src/client/types'),
      '@server': resolve(__dirname, 'src'),
      '@test-utils': resolve(__dirname, 'tests/utils'),
    },
  },
});