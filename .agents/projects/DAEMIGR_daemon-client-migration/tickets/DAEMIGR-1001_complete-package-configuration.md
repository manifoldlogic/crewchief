# Ticket: DAEMIGR-1001: Complete Package Configuration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Complete any missing package configuration and build setup for the daemon-client package based on findings from DAEMIGR-1000 review. Ensure package.json exports, TypeScript strict mode, and build configurations are properly set up for ESM module distribution.

## Background
The daemon-client package has basic configuration in place (package.json, tsconfig.json, vitest.config.ts) but may be missing exports, strict mode settings, or build optimizations. This ticket ensures the package is properly configured before completing core implementation in subsequent tickets.

This ticket implements the foundation phase (Phase 1) of the daemon-client migration project, specifically addressing package configuration requirements identified in DAEMIGR-1000 review.

**Context:**
- Package location: `/workspace/packages/daemon-client/`
- Existing config: package.json, tsconfig.json, vitest.config.ts
- Build target: ESM modules for Node.js >= 18
- Test framework: Vitest with coverage reporting
- Gap analysis from: DAEMIGR-1000 review findings

## Acceptance Criteria
- [ ] Package builds successfully (`pnpm build` completes without errors)
- [ ] Tests run successfully (`pnpm test` executes even with 0 tests)
- [ ] Linter passes (`pnpm lint` shows no errors)
- [ ] Package exports configured correctly in package.json (main export points to built DaemonClient, types export points to .d.ts files, ESM module format specified)
- [ ] TypeScript strict mode enabled with proper compiler options
- [ ] Vitest coverage thresholds configured (>80% target for branches, functions, lines, statements)

## Technical Requirements

### package.json
- Correct `name` field (@crewchief/daemon-client or similar monorepo-appropriate name)
- Proper `exports` field configured for ESM module resolution
- `types` field pointing to TypeScript declaration files in dist/
- Build scripts present: build, test, lint, clean
- Dependencies properly declared (child_process is Node.js built-in, no external deps needed for core functionality)
- `type: "module"` specified for ESM

### tsconfig.json
- `strict: true` enabled for maximum type safety
- `module: "ESNext"` or `"NodeNext"` for modern ESM output
- `moduleResolution: "bundler"` or `"node16"` for proper resolution
- `outDir: "dist/"` for compiled output
- Declaration files enabled (`declaration: true`, `declarationMap: true`)
- Source maps enabled for debugging (`sourceMap: true`)
- Target Node.js >= 18 compatibility

### vitest.config.ts
- Coverage provider configured (v8 or istanbul)
- Coverage thresholds set:
  - branches: 80
  - functions: 80
  - lines: 80
  - statements: 80
- Test environment: node
- Coverage directory: coverage/
- Coverage reporters: text, html, json

## Implementation Notes

1. Review DAEMIGR-1000 findings for specific gaps in configuration
2. Compare against architecture.md requirements (Node >= 18, ESM modules)
3. Ensure package can be imported by MCP server: `import { DaemonClient } from '@crewchief/daemon-client'`
4. Verify build produces clean dist/ directory with .js and .d.ts files
5. Add any missing npm scripts (clean, typecheck, etc.)
6. Consider adding `prepublishOnly` script to ensure build runs before publishing
7. Ensure tsconfig.json extends workspace root config if applicable
8. Verify all compiler options are compatible with workspace-level tooling

## Dependencies
- DAEMIGR-1000 (code review complete with gap list) - MUST BE COMPLETED FIRST

## Risk Assessment
- **Risk**: Breaking existing imports if exports field changes
  - **Mitigation**: Test imports after changes, verify MCP server can still import if package is currently in use
- **Risk**: TypeScript strict mode revealing type issues in existing code
  - **Mitigation**: Fix incrementally, document type issues for Tickets 1002-1003 if extensive work needed
- **Risk**: Build configuration changes affecting other packages
  - **Mitigation**: Run workspace-level build/test to ensure no breakage

## Files/Packages Affected
- Modify: `/workspace/packages/daemon-client/package.json`
- Modify: `/workspace/packages/daemon-client/tsconfig.json`
- Modify: `/workspace/packages/daemon-client/vitest.config.ts`
- Verify: Build output in `/workspace/packages/daemon-client/dist/`
- Verify: No breakage in dependent packages (MCP server if applicable)

## Planning References
- Phase: 1 (Foundation)
- Priority: HIGH
- Estimated Effort: 0.25 days (2 hours)
- Project: DAEMIGR - Daemon Client Migration
