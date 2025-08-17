# CrewChief Web UI Testing Framework

This document provides comprehensive guidance for testing the CrewChief Web UI application.

## Overview

The testing framework consists of multiple layers:

- **Unit Tests**: Test individual functions, classes, and components in isolation
- **Integration Tests**: Test API endpoints and database interactions
- **End-to-End (E2E) Tests**: Test complete user workflows in a browser environment

## Technology Stack

- **Vitest**: Fast unit test runner with native TypeScript support
- **React Testing Library**: Component testing utilities focused on user behavior
- **Playwright**: Modern E2E testing framework with cross-browser support
- **Supertest**: HTTP assertion library for API testing
- **Happy DOM**: Lightweight DOM implementation for unit tests
- **MSW**: Mock Service Worker for API mocking

## Directory Structure

```
tests/
├── setup.ts                    # Global test setup for unit tests
├── setup.integration.ts        # Global setup for integration tests
├── utils/                      # Shared test utilities
│   ├── index.ts                # Re-exports all utilities
│   ├── database.ts             # Database setup/teardown utilities
│   ├── test-server.ts          # Test server management
│   └── react-testing-utils.tsx # React testing utilities
├── fixtures/                   # Mock data and test fixtures
│   └── mock-data.ts            # Shared mock data factories
├── mocks/                      # Service and API mocks
├── unit/                       # Unit tests
│   ├── components/             # React component tests
│   └── services/               # Service layer tests
├── integration/                # Integration tests
│   ├── api/                    # API endpoint tests
│   └── database/               # Database integration tests
└── e2e/                        # End-to-end tests
    ├── global-setup.ts         # E2E test environment setup
    ├── global-teardown.ts      # E2E test environment cleanup
    ├── dashboard.e2e.test.ts   # Dashboard workflow tests
    ├── search.e2e.test.ts      # Search functionality tests
    └── agents.e2e.test.ts      # Agent management tests
```

## Running Tests

### All Tests
```bash
pnpm test:all          # Run all test types
pnpm test:ci           # Run tests with coverage for CI
```

### Unit Tests
```bash
pnpm test              # Run unit tests once
pnpm test:watch        # Run unit tests in watch mode
pnpm test:unit         # Run only unit tests
pnpm test:coverage     # Run with coverage report
```

### Integration Tests
```bash
pnpm test:integration  # Run integration tests
```

### E2E Tests
```bash
pnpm test:e2e          # Run E2E tests headlessly
pnpm test:e2e:ui       # Run E2E tests with UI (useful for debugging)
```

## Writing Tests

### Unit Tests

Unit tests should be placed alongside the source files or in the `tests/unit/` directory:

```typescript
// Example: tests/unit/services/maproom.test.ts
import { describe, it, expect, vi } from 'vitest';
import { MaproomService } from '../../../src/services/maproom.js';

describe('MaproomService', () => {
  it('should perform search', async () => {
    const service = new MaproomService();
    const result = await service.search('test query');
    expect(result.query).toBe('test query');
  });
});
```

### Component Tests

Use React Testing Library for component tests:

```typescript
// Example: tests/unit/components/Header.test.tsx
import { describe, it, expect } from 'vitest';
import { render, screen } from '@test-utils';
import Header from '../../../src/client/components/Header.js';

describe('Header', () => {
  it('renders dashboard title', () => {
    render(<Header onMenuClick={() => {}} />);
    expect(screen.getByRole('heading', { name: 'Dashboard' })).toBeInTheDocument();
  });
});
```

### Integration Tests

Test API endpoints and database interactions:

```typescript
// Example: tests/integration/api/maproom.integration.test.ts
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import request from 'supertest';
import { setupTestDatabase, startTestServer } from '@test-utils';

describe('Maproom API', () => {
  let server;
  let baseURL;

  beforeAll(async () => {
    await setupTestDatabase();
    const serverInfo = await startTestServer();
    server = serverInfo.server;
    baseURL = serverInfo.url;
  });

  it('should search code', async () => {
    const response = await request(baseURL)
      .post('/api/maproom/search')
      .send({ query: 'test', filters: {} })
      .expect(200);
      
    expect(response.body).toHaveProperty('results');
  });
});
```

### E2E Tests

Test complete user workflows:

```typescript
// Example: tests/e2e/search.e2e.test.ts
import { test, expect } from '@playwright/test';

test('should perform search', async ({ page }) => {
  await page.goto('/search');
  await page.getByRole('textbox', { name: /search/i }).fill('test');
  await page.getByRole('button', { name: /search/i }).click();
  await expect(page.getByTestId('search-results')).toBeVisible();
});
```

## Test Configuration

### Vitest Configuration

- `vitest.config.ts`: Main configuration for unit tests
- `vitest.config.unit.ts`: Specific configuration for unit tests
- `vitest.config.integration.ts`: Configuration for integration tests

Key features:
- TypeScript support out of the box
- Path aliases matching the main application
- Coverage reporting with v8 provider
- Happy DOM environment for fast DOM testing

### Playwright Configuration

- `playwright.config.ts`: E2E test configuration

Key features:
- Multi-browser testing (Chromium, Firefox, WebKit)
- Mobile viewport testing
- Automatic test server startup
- Screenshot and video recording on failure
- Parallel test execution

## Test Utilities

### Database Utilities

```typescript
import { setupTestDatabase, teardownTestDatabase } from '@test-utils';

// Set up clean database state
await setupTestDatabase();

// Clean up after tests
await teardownTestDatabase();
```

### React Testing Utilities

```typescript
import { render, screen, userEvent } from '@test-utils';

// Render with router and providers
render(<MyComponent />, { withRouter: true });

// Simulate user interactions
const user = userEvent.setup();
await user.click(screen.getByRole('button'));
```

### Mock Data

```typescript
import { createMockAgentRun, mockAgentRuns } from '@test-utils';

// Create single mock object
const agentRun = createMockAgentRun({ status: 'completed' });

// Use predefined mock arrays
const runs = mockAgentRuns;
```

## Best Practices

### General Guidelines

1. **Test Behavior, Not Implementation**: Focus on what the component does, not how it does it
2. **Use Descriptive Test Names**: Test names should clearly describe the expected behavior
3. **Follow AAA Pattern**: Arrange, Act, Assert
4. **Keep Tests Independent**: Each test should be able to run in isolation
5. **Mock External Dependencies**: Don't make real API calls or database queries in unit tests

### Component Testing

1. **Test User Interactions**: Focus on what users can see and do
2. **Use Semantic Queries**: Prefer `getByRole`, `getByLabelText` over `getByTestId`
3. **Test Accessibility**: Ensure components are accessible to screen readers
4. **Mock Network Requests**: Use MSW or similar for consistent API mocking

### E2E Testing

1. **Test Critical User Journeys**: Focus on the most important user workflows
2. **Use Page Object Model**: Encapsulate page interactions in reusable objects
3. **Wait for Elements**: Always wait for elements to be stable before interacting
4. **Test Cross-Browser**: Ensure functionality works across different browsers

### Performance Testing

1. **Set Reasonable Timeouts**: Don't let tests hang indefinitely
2. **Use Parallel Execution**: Run tests concurrently when possible
3. **Clean Up Resources**: Properly close database connections and servers
4. **Monitor Test Duration**: Keep tests fast to enable rapid feedback

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:14
        env:
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '20'
      - run: pnpm install
      - run: pnpm test:ci
      - uses: actions/upload-artifact@v3
        with:
          name: coverage
          path: coverage/
```

## Coverage Targets

- **Lines**: 80%+ coverage
- **Functions**: 80%+ coverage  
- **Branches**: 80%+ coverage
- **Statements**: 80%+ coverage

Critical paths (authentication, data persistence, core features) should have higher coverage.

## Debugging Tests

### Unit Tests
```bash
# Run specific test file
pnpm vitest run tests/unit/services/maproom.test.ts

# Run with debugging
pnpm vitest run --reporter=verbose

# Run in watch mode for development
pnpm test:watch
```

### E2E Tests
```bash
# Run with UI for visual debugging
pnpm test:e2e:ui

# Run specific test
npx playwright test tests/e2e/search.e2e.test.ts

# Debug mode
npx playwright test --debug
```

## Troubleshooting

### Common Issues

1. **Database Connection Errors**: Ensure PostgreSQL is running and test database exists
2. **Port Conflicts**: Make sure test ports (3000, 3500) are available
3. **Timeout Issues**: Increase timeout values for slow operations
4. **Mock Data Issues**: Verify mock data matches expected API schemas

### Environment Setup

1. **Database**: PostgreSQL 14+ required for integration tests
2. **Node.js**: Version 20+ required
3. **Browsers**: Playwright will install required browsers automatically

For more detailed information, see the individual test files and configuration files in this directory.