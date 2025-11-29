# Ticket Index: NPMDEP Project

**Project:** npm Package Deprecation
**Slug:** NPMDEP
**Total Tickets:** 5
**Total Estimated Time:** 70 minutes

## Overview

This project deprecates the old unscoped `maproom-mcp` npm package by publishing a final "tombstone" version (2.0.0) with clear migration guidance, then applying `npm deprecate` to all versions. Users will be directed to the new scoped package `@crewchief/maproom-mcp`.

## Ticket Organization

Tickets are organized by phase following the project plan:

- **Phase 1 (Preparation):** Tickets 1001-1003 - Assess state, create content, validate locally
- **Phase 2 (Publishing):** Ticket 2001 - Publish to npm registry
- **Phase 3 (Deprecation):** Ticket 3001 - Apply deprecation warnings
- **Phase 4 (Validation):** Ticket 4001 - End-to-end validation

## Phase 1: Preparation and Validation

### NPMDEP-1001: npm State Assessment
**File:** `NPMDEP-1001_npm-state-assessment.md`
**Agent:** general-purpose
**Time:** 10 minutes
**Status:** Pending

**Summary:** Verify current state of maproom-mcp on npm, confirm user has publish rights, check version 2.0.0 availability, and document findings.

**Key Deliverables:**
- Current version and all existing versions documented
- User authentication verified (npm whoami)
- Publish rights confirmed (npm owner ls)
- Version 2.0.0 availability checked
- npm registry status verified
- Findings documented in state-assessment.md

**Dependencies:** None (first ticket)

---

### NPMDEP-1002: Content Preparation
**File:** `NPMDEP-1002_content-preparation.md`
**Agent:** general-purpose
**Time:** 15 minutes
**Status:** Pending

**Summary:** Create the three files needed for deprecation package: package.json (v2.0.0), index.js (executable with --help support), README.md (deprecation notice).

**Key Deliverables:**
- Directory `/tmp/maproom-mcp-deprecated/` created
- package.json with correct metadata
- index.js with shebang, executable permissions, exit code 1
- README.md copied from existing deprecation notice
- All content matches architecture specifications

**Dependencies:** Blocks on NPMDEP-1001

---

### NPMDEP-1003: Local Testing
**File:** `NPMDEP-1003_local-testing.md`
**Agent:** general-purpose
**Time:** 10 minutes
**Status:** Pending

**Summary:** Build package with npm pack, extract and verify contents, test executable (normal and --help), validate package size and structure.

**Key Deliverables:**
- npm pack succeeds, creates maproom-mcp-2.0.0.tgz
- Package size < 50 KB
- Exactly 3 files in package
- Executable works and exits with code 1
- --help flag shows correct message
- Validation report documented

**Dependencies:** Blocks on NPMDEP-1002

**Critical:** This is the last chance to catch errors before publishing (irreversible after 72 hours).

---

## Phase 2: Publishing

### NPMDEP-2001: Publish to npm
**File:** `NPMDEP-2001_publish-to-npm.md`
**Agent:** general-purpose (requires user interaction)
**Time:** 15 minutes
**Status:** Pending

**Summary:** Verify npm authentication, publish maproom-mcp v2.0.0 to registry, immediately verify package appears correctly.

**Key Deliverables:**
- npm authentication verified
- Publish command succeeds
- Version 2.0.0 visible in registry
- npm website shows v2.0.0
- README renders on package page
- Deprecated field set in metadata
- Publish output captured

**Dependencies:** Blocks on NPMDEP-1003 (local validation must pass)

**Critical:** This is an irreversible operation. Cannot unpublish after 72 hours per npm policy.

**User Interaction Required:** npm credentials, publish approval

---

## Phase 3: Deprecation Tagging

### NPMDEP-3001: Apply Deprecation
**File:** `NPMDEP-3001_apply-deprecation.md`
**Agent:** general-purpose
**Time:** 10 minutes
**Status:** Pending

**Summary:** Execute npm deprecate command with user-specified message (including --help reference), verify deprecation applies to all versions.

**Key Deliverables:**
- npm deprecate command succeeds
- Message: "This package has been replaced by @crewchief/maproom-mcp. Please use the new package: npx @crewchief/maproom-mcp --help"
- Deprecation visible in package metadata
- Installation warning appears
- Command output documented

**Dependencies:** Blocks on NPMDEP-2001 (must publish before deprecating)

**Note:** --help reference is user-required feature, must be included.

---

## Phase 4: End-to-End Validation

### NPMDEP-4001: E2E Validation
**File:** `NPMDEP-4001_e2e-validation.md`
**Agent:** general-purpose
**Time:** 15 minutes
**Status:** Pending

**Summary:** Comprehensive validation from user perspective: test installation warnings, executable behavior, verify npm website, test all links, document results.

**Key Deliverables:**
- Fresh install shows deprecation warning
- npx execution shows migration message
- --help flag shows help-specific message
- Both exit with code 1
- npm website shows DEPRECATED badge
- README renders correctly
- All links functional
- Complete validation report
- Project completion summary

**Dependencies:** Blocks on NPMDEP-3001 (deprecation must be applied)

**Critical:** All 9 project success criteria must pass before project completion.

---

## Success Criteria

The project is complete when all tickets are verified and these criteria are met:

1. ✅ Version 2.0.0 published to npm registry
2. ✅ README visible on https://www.npmjs.com/package/maproom-mcp
3. ✅ `npm install maproom-mcp` shows deprecation warning
4. ✅ Warning mentions `@crewchief/maproom-mcp`
5. ✅ Warning includes `npx @crewchief/maproom-mcp --help` reference
6. ✅ `npx maproom-mcp` shows migration message
7. ✅ `npx maproom-mcp --help` shows help-specific message
8. ✅ All links work
9. ✅ Documentation complete

## Dependency Graph

```
NPMDEP-1001 (npm state)
    ↓
NPMDEP-1002 (content prep)
    ↓
NPMDEP-1003 (local testing) ← CRITICAL GATE
    ↓
NPMDEP-2001 (publish) ← IRREVERSIBLE
    ↓
NPMDEP-3001 (deprecation)
    ↓
NPMDEP-4001 (E2E validation)
    ↓
PROJECT COMPLETE
```

## Critical Path

All tickets must be executed sequentially. No parallelization possible due to dependencies.

**Total Duration:** 70 minutes (assuming no issues)

## Risk Summary

- **High Risk:** NPMDEP-2001 (irreversible publish)
- **Medium Risk:** NPMDEP-1001 (user may lack publish rights)
- **Low Risk:** All other tickets (can be retried/fixed)

## Planning References

- **Project Plan:** `.crewchief/projects/NPMDEP_npm-deprecation/planning/plan.md`
- **Architecture:** `.crewchief/projects/NPMDEP_npm-deprecation/planning/architecture.md`
- **Quality Strategy:** `.crewchief/projects/NPMDEP_npm-deprecation/planning/quality-strategy.md`
- **Security Review:** `.crewchief/projects/NPMDEP_npm-deprecation/planning/security-review.md`
- **Project README:** `.crewchief/projects/NPMDEP_npm-deprecation/README.md`

## Execution

To execute this project:

1. **Review all tickets** - Understand the flow and dependencies
2. **Execute sequentially** - `/single-ticket NPMDEP-1001`, then NPMDEP-1002, etc.
3. **Or run all** - `/work-on-project NPMDEP`
4. **User presence required** - Phase 2 (publishing) requires user interaction

## Notes

- This is a **one-time operation** with no ongoing maintenance
- User must be available for npm authentication in Phase 2
- Phase 1.3 (local testing) is the critical quality gate
- Phase 2 (publishing) is irreversible after 72 hours
- --help flag support is a user-specified requirement (critical)
- Exact deprecation message specified by user must be used verbatim
