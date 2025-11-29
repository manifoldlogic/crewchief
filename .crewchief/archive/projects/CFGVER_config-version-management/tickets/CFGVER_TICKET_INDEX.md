# CFGVER: Config Version Management - Ticket Index (Simplified)

## Project Overview

**Goal:** Prevent config drift by tracking package version

**Timeline:** 1-2 days (9-11 hours)

**Status:** Ready for Implementation

---

## Tickets (4 total)

### CFGVER-001: Version File Implementation
- **File:** `CFGVER-001_version-file.md`
- **Agent:** database-engineer
- **Summary:** Create `.version` file to track package version
- **Deliverables:** `readVersion()`, `writeVersion()` functions
- **Dependencies:** None (first ticket)
- **Estimated:** 2 hours

---

### CFGVER-002: Version Comparison Logic
- **File:** `CFGVER-002_version-comparison.md`
- **Agent:** database-engineer
- **Summary:** Detect when cached version differs from package version
- **Deliverables:** `needsConfigUpdate()` function returning boolean
- **Dependencies:** CFGVER-001
- **Estimated:** 2 hours

---

### CFGVER-003: Config Update with .env Preservation
- **File:** `CFGVER-003_config-update.md`
- **Agent:** database-engineer
- **Summary:** Copy fresh configs from package, preserve user `.env` file
- **Deliverables:** `updateConfigs()` function
- **Dependencies:** CFGVER-002
- **Estimated:** 3-4 hours

---

### CFGVER-004: CLI Integration
- **File:** `CFGVER-004_cli-integration.md`
- **Agent:** mcp-tools-engineer
- **Summary:** Call version check on CLI startup, update if needed
- **Deliverables:** Modified `bin/cli.cjs` with update logic
- **Dependencies:** CFGVER-003
- **Estimated:** 2-3 hours

---

## Summary

| Ticket | Hours | Cumulative |
|--------|-------|------------|
| CFGVER-001 | 2 | 2 |
| CFGVER-002 | 2 | 4 |
| CFGVER-003 | 3-4 | 7-8 |
| CFGVER-004 | 2-3 | 9-11 |
| **Total** | **9-11** | **1-2 days** |

## Workflow

For each ticket:
1. **Implement** following technical requirements
2. **Test** manually (first run, version update, .env preservation)
3. **Commit** with descriptive message

**No formal verification needed** - this is a simple feature, manual testing is sufficient.

---

## Dependencies Graph

```
001 (version file)
  └─→ 002 (version comparison)
       └─→ 003 (config update)
            └─→ 004 (CLI integration)
```

Linear dependency chain - complete tickets in order.

---

## Success Criteria

- ✅ First run creates `.version` file
- ✅ Version change triggers config update
- ✅ User `.env` file preserved after update
- ✅ Clear progress messages during update
- ✅ Helpful error messages if update fails

---

## Archive Reference

Comprehensive plan with backup/rollback/testing available at:
`.crewchief/archive/projects/CFGVER_config-version-management-comprehensive/`

Use if we need enterprise-grade safety features later.
