# DAEMIGR Project Tickets

## Ticket Overview

| Ticket ID | Title | Phase | Priority | Status |
|:----------|:------|:------|:---------|:-------|
| [DAEMIGR-1001](./DAEMIGR-1001_daemon-client-library.md) | Daemon Client Library | Foundation | High | Open |
| [DAEMIGR-2001](./DAEMIGR-2001_mcp-migration.md) | MCP Server Migration | Integration | High | Open |
| [DAEMIGR-3001](./DAEMIGR-3001_testing-validation.md) | Testing & Validation | Verification | High | Open |

**Total Estimated Effort:** 7-10 hours

---

## Progress Tracking

- [ ] DAEMIGR-1001: Daemon Client Library
- [ ] DAEMIGR-2001: MCP Server Migration
- [ ] DAEMIGR-3001: Testing & Validation

**Completion:** 0/3 (0%)

---

## Ticket Summaries

### DAEMIGR-1001: Daemon Client Library ⏳ Open
**Phase:** Foundation  
**Effort:** 3-4 hours  
**Dependencies:** None

Create the `@maproom/daemon-client` NPM package with:
- Process lifecycle management (start, stop, restart)
- JSON-RPC protocol handling
- Health checking and crash recovery
- High-level search API

**Deliverables:**
- `packages/daemon-client/` package
- Unit tests (>80% coverage)
- API documentation

---

### DAEMIGR-2001: MCP Server Migration ⏳ Open
**Phase:** Integration  
**Effort:** 2-3 hours  
**Dependencies:** DAEMIGR-1001

Migrate MCP server to use daemon-client:
- Replace `trySpawnWithCandidates` in `tools/search.ts`
- Add daemon singleton management
- Update error handling
- Maintain chunk ID fetching logic

**Deliverables:**
- Updated `packages/maproom-mcp/src/tools/search.ts`
- Updated `packages/maproom-mcp/package.json`
- Integration tests

---

### DAEMIGR-3001: Testing & Validation ⏳ Open
**Phase:** Verification  
**Effort:** 1-2 hours  
**Dependencies:** DAEMIGR-2001

Comprehensive testing and performance validation:
- Integration tests (daemon lifecycle, concurrent requests)
- Performance benchmarks (cold/warm latency)
- Regression testing (all existing features)
- Documentation updates

**Deliverables:**
- Test suite passing (100%)
- Performance report
- Updated README and troubleshooting guide

---

## Out of Scope (Future Work)

### DAEMIGR-4001: VSCode Extension Migration
**Status:** Deferred  
**Effort:** 2-3 hours

Migrate VSCode extension scan command to use daemon. Lower priority since scan is infrequent.

### DAEMIGR-5001: Shared Daemon Mode
**Status:** Deferred  
**Effort:** 5-7 hours

Single daemon shared across multiple clients with IPC coordination. More complex, revisit after Phase 1 success.

---

## Dependencies

### External
- **MAPDAEMON** ✅ Complete - Daemon implementation ready
- **Node.js** >=18 - For daemon-client package
- **TypeScript** >= 5.0 - For type safety

### Internal
- Daemon binary must be available at runtime
- PostgreSQL database configured
- Environment variables set

---

**Created:** 2025-11-21  
**Status:** Ready for execution
