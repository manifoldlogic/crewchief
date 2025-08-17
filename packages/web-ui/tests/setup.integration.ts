import { vi } from 'vitest';

// Mock environment variables for integration tests
vi.mock('process', () => ({
  env: {
    NODE_ENV: 'test',
    PORT: '0', // Use random port for integration tests
    DATABASE_URL: 'postgresql://test:test@localhost:5432/crewchief_integration_test',
    PGUSER: 'test',
    PGPASSWORD: 'test',
    PGHOST: 'localhost',
    PGPORT: '5432',
    PGDATABASE: 'crewchief_integration_test',
  },
}));

// Reset all mocks before each test
beforeEach(() => {
  vi.clearAllMocks();
});

// Set test timeout for integration tests
vi.setConfig({
  testTimeout: 30000,
  hookTimeout: 30000,
});