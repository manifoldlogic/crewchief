import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  
  // Entry point for the client application
  root: '.',
  
  // Build configuration
  build: {
    outDir: 'dist/client',
    emptyOutDir: true,
    sourcemap: true,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
      },
    },
  },
  
  // Development server configuration
  server: {
    port: 3000,
    proxy: {
      // Proxy API requests to the backend server
      '/api': {
        target: 'http://localhost:3500',
        changeOrigin: true,
        secure: false,
      },
    },
  },
  
  // Path aliases
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src/client'),
      '@components': resolve(__dirname, 'src/client/components'),
      '@pages': resolve(__dirname, 'src/client/pages'),
      '@hooks': resolve(__dirname, 'src/client/hooks'),
      '@utils': resolve(__dirname, 'src/client/utils'),
      '@types': resolve(__dirname, 'src/client/types'),
    },
  },
  
  // CSS configuration
  css: {
    postcss: './postcss.config.js',
  },
  
  // Environment variables
  envPrefix: 'VITE_',
  
  // TypeScript configuration
  esbuild: {
    tsconfigRaw: {
      compilerOptions: {
        jsx: 'react-jsx'
      }
    }
  }
});