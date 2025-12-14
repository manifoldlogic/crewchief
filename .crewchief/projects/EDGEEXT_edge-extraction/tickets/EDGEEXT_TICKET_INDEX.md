# EDGEEXT Ticket Index

## Phase 1: TypeScript/JavaScript Calls (MVP)

### EDGEEXT-1001: Create Edge Extractor Module
**Status:** Not Started
**Dependencies:** None
**Blocks:** EDGEEXT-1002, EDGEEXT-1003
**Summary:** Create foundational edge extractor module with shared types, public API, and common utilities

### EDGEEXT-1002: TypeScript Call Extraction
**Status:** Not Started
**Dependencies:** EDGEEXT-1001
**Blocks:** EDGEEXT-1003
**Summary:** Implement TypeScript/JavaScript call expression extraction using tree-sitter

### EDGEEXT-1003: Scan/Upsert Integration
**Status:** Not Started
**Dependencies:** EDGEEXT-1001, EDGEEXT-1002
**Blocks:** EDGEEXT-1004
**Summary:** Integrate edge extraction into scan_worktree(), upsert_files(), and EdgeUpdater

### EDGEEXT-1004: Testing & Validation Infrastructure
**Status:** Not Started
**Dependencies:** EDGEEXT-1003
**Blocks:** None
**Summary:** Create synthetic test repos, integration tests, accuracy validation, and performance benchmarks

## Dependency Chain

```
EDGEEXT-1001 (Module + Shared Types)
    ↓
EDGEEXT-1002 (TypeScript Extractor)
    ↓
EDGEEXT-1003 (Integration)
    ↓
EDGEEXT-1004 (Testing)
```

## Coverage

Phase 1 Deliverables:
- ✅ Edge extractor module → EDGEEXT-1001
- ✅ TypeScript call extraction → EDGEEXT-1002
- ✅ Integration with scan/upsert → EDGEEXT-1003
- ✅ EdgeUpdater enhancement → EDGEEXT-1003
- ✅ Unit tests → EDGEEXT-1002
- ✅ Integration test → EDGEEXT-1004

Coverage: 100% (all 6 deliverables mapped to tickets)
