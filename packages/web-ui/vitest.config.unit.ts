/// <reference types="vitest" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
  plugins: [react()],
  
  test: {
    globals: true,
    environment: 'happy-dom',
    setupFiles: ['./tests/setup.ts'],
    include: [
      'src/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
      'tests/unit/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
    ],
    exclude: [
      'node_modules',
      'dist',
      'tests/e2e',
      'tests/integration',
    ],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'dist/',
        'tests/',
        '**/*.d.ts',
        '**/*.config.{js,ts}',
        '**/index.ts',
        'src/client/main.tsx',
        'migrations/',
        'seeds/',
      ],
    },
    testTimeout: 5000,
    hookTimeout: 5000,
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