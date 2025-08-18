# Testing Framework Documentation

## Overview

This document outlines the comprehensive testing framework setup for the CrewChief Web UI project. The testing infrastructure includes unit tests, integration tests, E2E tests, performance monitoring, and security validation.

## Testing Stack

- **Unit Testing**: Vitest with React Testing Library
- **Integration Testing**: Vitest with Supertest
- **E2E Testing**: Playwright
- **Coverage**: Vitest Coverage (V8)
- **Performance**: Lighthouse CI
- **Security**: pnpm audit, Trivy, Snyk

## Build Optimization

### Production Build Features
- **Console Removal**: All `console.log` statements removed in production
- **Source Maps**: Excluded from production builds for security
- **Code Splitting**: Vendor and router chunks separated
- **Compression**: Terser minification enabled
- **Size Limit**: Total bundle < 500KB gzipped (currently ~69KB)

## Test Commands

```bash
# Run all tests
pnpm test:all

# Unit tests only
pnpm test:unit
pnpm test:unit --watch  # Watch mode

# Integration tests
pnpm test:integration

# E2E tests
pnpm test:e2e
pnpm test:e2e:ui  # With UI

# Coverage reports
pnpm test:coverage

# Security audit
pnpm audit
pnpm security:check
```

## Test Structure

```
tests/
├── examples/           # Template and example tests
│   ├── unit.example.test.ts
│   ├── integration.example.test.ts
│   └── e2e.example.test.ts
├── unit/              # Component and utility tests
│   ├── components/
│   ├── pages/
│   ├── services/
│   └── utils/
├── integration/       # API and database tests
│   ├── api/
│   └── database/
├── e2e/              # End-to-end browser tests
├── fixtures/         # Test data and mocks
└── utils/           # Test utilities and helpers
```

## Testing Guidelines

### Unit Tests
- Test individual components and functions in isolation
- Mock external dependencies
- Focus on public interfaces and behavior
- Achieve 60%+ code coverage minimum
- Use descriptive test names following AAA pattern

### Integration Tests
- Test multiple components working together
- Use real database connections (test DB)
- Verify API endpoints and data flow
- Test error handling and edge cases

### E2E Tests
- Test complete user workflows
- Use Page Object Model pattern
- Test across multiple browsers/devices
- Include accessibility and performance checks

## Coverage Requirements

- **Minimum Coverage**: 60% (configurable in vitest.config.ts)
- **Target Coverage**: 80%
- **Excluded from Coverage**:
  - Configuration files
  - Type definitions
  - Main entry points
  - Database migrations

## CI/CD Pipeline

The automated testing pipeline includes:

1. **Code Quality**: Linting, formatting, type checking
2. **Security**: Dependency audit, vulnerability scanning
3. **Build**: Production build with size checks
4. **Testing**: Unit, integration, and E2E tests
5. **Performance**: Lighthouse CI for performance metrics
6. **Deployment**: Automated staging and production deploys

## Security Testing

- **Dependency Audit**: Checks for known vulnerabilities
- **Static Analysis**: Trivy filesystem scanning
- **Runtime Security**: Snyk continuous monitoring
- **Build Security**: Removes debug code and source maps

## Performance Testing

- **Lighthouse CI**: Automated performance audits
- **Performance Budget**: 
  - First Contentful Paint < 2s
  - Largest Contentful Paint < 3s
  - Cumulative Layout Shift < 0.1
  - Time to Interactive < 4s

## Best Practices

### Writing Tests
1. **Arrange-Act-Assert** pattern for clarity
2. **Descriptive test names** that explain the scenario
3. **Single responsibility** - one concept per test
4. **Isolated tests** that don't depend on each other
5. **Mock external dependencies** appropriately

### Test Data
- Use factories for consistent test data
- Clean up after tests (database, files, etc.)
- Use realistic but anonymous test data
- Separate test data by test type

### Assertions
- Use specific assertions over generic ones
- Test both positive and negative cases
- Include boundary value testing
- Verify error messages and states

## Troubleshooting

### Common Issues

1. **Tests timing out**: Increase timeout in vitest config
2. **Mock issues**: Ensure mocks are reset between tests
3. **Database conflicts**: Use separate test databases
4. **Flaky E2E tests**: Add explicit waits and better selectors

### Debugging

```bash
# Run tests in debug mode
pnpm test:unit --inspect-brk

# Run specific test files
pnpm test:unit path/to/test.ts

# Generate detailed coverage report
pnpm test:coverage --reporter=html
```

## Example Test Files

See the `tests/examples/` directory for comprehensive examples of:
- Unit testing with mocks and spies
- Integration testing with API endpoints
- E2E testing with Playwright
- Performance and accessibility testing

## Contributing

When adding new features:
1. Write tests first (TDD approach)
2. Ensure all tests pass
3. Meet coverage requirements
4. Update this documentation if needed
5. Add example tests for new patterns

## Configuration Files

- `vitest.config.ts` - Main test configuration
- `vitest.config.unit.ts` - Unit test specific config
- `vitest.config.integration.ts` - Integration test config
- `playwright.config.ts` - E2E test configuration
- `.lighthouserc.json` - Performance testing config
- `.github/workflows/ci.yml` - CI/CD pipeline

## Resources

- [Vitest Documentation](https://vitest.dev/)
- [React Testing Library](https://testing-library.com/docs/react-testing-library/intro/)
- [Playwright Documentation](https://playwright.dev/)
- [Testing Best Practices](https://kentcdodds.com/blog/common-mistakes-with-react-testing-library)