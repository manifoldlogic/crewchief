# Project: npm Package Deprecation

**SLUG:** NPMDEP
**Status:** Planning Complete
**Estimated Duration:** 70 minutes

## Problem

The old unscoped `maproom-mcp` npm package needs to be deprecated in favor of the new scoped package `@crewchief/maproom-mcp`. Currently:

- Users may install the old package unknowingly
- No clear migration path visible
- Old package doesn't receive updates
- Creates confusion in npm ecosystem

## Solution

Publish a final "tombstone" version (2.0.0) of `maproom-mcp` that:

1. Shows clear deprecation message when executed
2. Displays migration README on npm package page
3. Uses `npm deprecate` command for installation warnings
4. Supports `--help` flag (user requirement)

All messages direct users to `@crewchief/maproom-mcp` with specific migration commands.

## Relevant Agents

**Primary:** general-purpose
- Content creation and validation
- npm publishing execution (interactive with user)
- End-to-end verification
- Documentation

**Why general-purpose?**
- Simple sequential tasks
- Requires user interaction (credentials)
- Manual verification steps
- One-time operation (no automation needed)

## Planning Documents

### Core Planning
- [analysis.md](planning/analysis.md) - Problem space analysis, npm deprecation best practices, current state assessment
- [architecture.md](planning/architecture.md) - Solution design, package structure, publishing strategy, file specifications
- [quality-strategy.md](planning/quality-strategy.md) - Manual verification approach, validation checklists, risk mitigation
- [security-review.md](planning/security-review.md) - Threat model, vulnerability analysis, security gates
- [plan.md](planning/plan.md) - 4-phase implementation plan, timeline, agent assignment, success criteria

## Key Features

✅ **Deprecation Message:** User-specified message including `--help` reference
✅ **README Update:** Full migration guide visible on npm package page
✅ **Executable:** Shows migration message when run via `npx`
✅ **Zero Dependencies:** Minimal attack surface, no supply chain risk
✅ **Industry Standard:** Follows npm best practices (request, babel-core, istanbul)

## Success Criteria

1. ✅ Version 2.0.0 published to npm
2. ✅ README visible on package page
3. ✅ `npm install maproom-mcp` shows deprecation warning
4. ✅ Warning mentions new package and includes `--help` reference
5. ✅ `npx maproom-mcp` shows migration message
6. ✅ `npx maproom-mcp --help` shows help-specific message
7. ✅ All links work

## Implementation Phases

### Phase 1: Preparation and Validation (30 min)
- Assess current npm package state
- Create package.json, index.js, README.md
- Validate content locally with `npm pack`
- Test executable and --help flag

### Phase 2: Publishing (15 min)
- Verify npm authentication and publish rights
- Publish version 2.0.0 to npm registry
- Verify package appears and README renders

### Phase 3: Deprecation Tagging (10 min)
- Apply `npm deprecate` with user-specified message
- Verify deprecation applied to all versions
- Test installation warning appears

### Phase 4: End-to-End Validation (15 min)
- Fresh installation testing
- Executable testing (normal and --help)
- Web validation (npm package page)
- Documentation of results

## Next Steps

1. Run `/create-project-tickets NPMDEP` to generate implementation tickets
2. Review tickets with `/review-tickets NPMDEP`
3. Execute with `/work-on-project NPMDEP` or `/single-ticket NPMDEP-XXXX`

## Notes

- **User interaction required:** npm login, credentials, approval to publish
- **One-time operation:** No ongoing maintenance needed
- **Simple approach:** Manual steps acceptable, no automation overhead
- **Specific requirement:** Must support `--help` flag as requested by user
- **Exact message:** User specified deprecation message must be used verbatim

## Assets

**Existing:**
- `/workspace/packages/maproom-mcp/README.deprecated.md` - Deprecation README (ready to use)

**To Create:**
- `/tmp/maproom-mcp-deprecated/` - Temporary directory for deprecation package
  - `package.json` - Version 2.0.0 metadata
  - `index.js` - Executable showing migration message
  - `README.md` - Copy of README.deprecated.md
