# Ticket: CFGVER-0001: Setup Testing Infrastructure

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary
Install and configure Vitest testing framework to establish the testing foundation for the CFGVER project. This ticket is critical and must be completed BEFORE any test-related tickets (CFGVER-1901, CFGVER-2902, CFGVER-3903, CFGVER-5001, CFGVER-5002) as the codebase currently has no test runner configured.

## Background
The CFGVER project requires comprehensive unit and integration testing to achieve 80%+ code coverage and ensure safe config management. Currently, `packages/maproom-mcp/package.json` has no test runner configured - only a placeholder script that echoes "Test mode not yet implemented". All subsequent test tickets assume Vitest exists and will fail without this foundation.

Current state:
```json
{
  "devDependencies": {
    "typescript": "^5.3.3",
    "@types/node": "^20.10.5",
    "@types/pg": "^8.10.9"
  },
  "scripts": {
    "test": "node bin/cli.cjs --test || echo 'Test mode not yet implemented'"
  }
}
```

Reference: `.agents/projects/CFGVER_config-version-management/planning/quality-strategy.md` lines 177-201 for testing tools and framework selection.

## Acceptance Criteria
- [ ] Vitest installed as devDependency with version ^1.0.0 or later
- [ ] @vitest/coverage-v8 installed for coverage reporting
- [ ] @vitest/ui installed for interactive test interface
- [ ] vitest.config.js created with proper configuration (node environment, coverage thresholds, path aliases)
- [ ] package.json scripts updated with test commands (test, test:watch, test:ui, test:coverage, test:coverage-check)
- [ ] Sample test file `tests/config-manager.test.ts` created and runs successfully
- [ ] Coverage report generates correctly and creates coverage/ directory
- [ ] Coverage thresholds configured: lines 80%, functions 80%, branches 75%, statements 80%

## Technical Requirements

### Install Dependencies
```bash
cd packages/maproom-mcp
pnpm add -D vitest @vitest/coverage-v8 @vitest/ui
```

### Create vitest.config.js
File: `packages/maproom-mcp/vitest.config.js`
```javascript
import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      exclude: [
        'node_modules/',
        'dist/',
        'tests/',
        '**/*.test.ts',
        '**/*.test.js',
        'vitest.config.js'
      ],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 75,
        statements: 80
      }
    },
    include: ['tests/**/*.test.ts', 'tests/**/*.test.js'],
    exclude: ['node_modules/', 'dist/']
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  }
});
```

### Update package.json scripts
Replace the current `test` script with:
```json
{
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest",
    "test:ui": "vitest --ui",
    "test:coverage": "vitest run --coverage",
    "test:coverage-check": "vitest run --coverage --coverage.thresholds.lines=80"
  }
}
```

### Create Sample Test
File: `packages/maproom-mcp/tests/config-manager.test.ts`
```typescript
import { describe, it, expect } from 'vitest';

describe('Testing Infrastructure', () => {
  it('should run tests with Vitest', () => {
    expect(true).toBe(true);
  });

  it('should support async tests', async () => {
    const result = await Promise.resolve(42);
    expect(result).toBe(42);
  });
});
```

### Verification Steps
1. Run: `pnpm test` - Should execute tests and show 2 passing tests
2. Run: `pnpm test:coverage` - Should generate coverage report with 0% baseline
3. Check: `coverage/` directory created in packages/maproom-mcp/
4. Check: `coverage/index.html` viewable in browser with coverage visualization
5. Verify: Sample tests pass without errors
6. Verify: pnpm install completes without errors

## Implementation Notes

**Technology Choice:**
- Use Vitest (not Jest) for:
  - Faster execution (Vite-powered)
  - Native ES module support (matches package.json "type": "module")
  - Better TypeScript support out-of-the-box
  - Smaller dependency footprint
  - Modern API compatible with Jest

**Coverage Tool:**
- Use @vitest/coverage-v8 (not c8) - official Vitest plugin with better integration
- Configure 80% coverage threshold (can enable strict mode later)
- Exclude test files, config files, and build artifacts from coverage

**Configuration Details:**
- Enable `globals: true` for less verbose test syntax (describe/it/expect available globally)
- Use `environment: 'node'` for Node.js runtime (not browser)
- Add @vitest/ui for interactive development (useful for debugging tests)
- Configure path alias '@' to './src' for cleaner imports in tests

**Success Metrics:**
- All commands run without errors
- Coverage report shows 0% baseline (expected - no code tested yet)
- Foundation ready for subsequent test tickets
- CI can run `pnpm test:coverage` successfully

## Dependencies
None (this is the foundation ticket, blocks all test tickets)

## Blocks
- **CFGVER-1901**: Unit tests for core logic (requires Vitest)
- **CFGVER-2902**: Integration tests for update flow (requires Vitest)
- **CFGVER-3903**: Integration tests for Docker orchestration (requires Vitest)
- **CFGVER-5001**: Complete unit test coverage (requires Vitest)
- **CFGVER-5002**: Complete integration test coverage (requires Vitest)

## Risk Assessment
- **Risk**: TypeScript configuration conflicts with Vitest
  - **Mitigation**: Verify tsconfig.json has "module": "ESNext" or "NodeNext" and "moduleResolution": "node" or "bundler"

- **Risk**: Coverage thresholds too strict for initial baseline
  - **Mitigation**: Thresholds configured but only enforced in `test:coverage-check` script, not in default `test` script

- **Risk**: Test file patterns don't match existing test structure
  - **Mitigation**: Use standard `tests/**/*.test.ts` pattern, flexible for future expansion

- **Risk**: Path resolution issues with @/ alias
  - **Mitigation**: Configure both vitest.config.js and ensure tsconfig.json includes proper path mapping

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/package.json` (add devDependencies and update scripts)
- **Create**: `packages/maproom-mcp/vitest.config.js` (Vitest configuration)
- **Create**: `packages/maproom-mcp/tests/config-manager.test.ts` (sample test to verify setup)
- **Create**: `packages/maproom-mcp/coverage/` (directory created by coverage tool, git-ignored)
