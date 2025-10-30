# Ticket: MAPROOM-1003: Rebuild Docker Container with MAPROOM-1001 Markdown Enum Fix

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Rebuild the Docker container at `~/.maproom-mcp` with the latest Maproom binary that includes the markdown enum fix from MAPROOM-1001. The current container has an outdated binary causing "invalid input value for enum symbol_kind: list" errors when scanning markdown files.

## Background
MAPROOM-1001 fixed a critical bug in the Rust source code where the markdown parser's list enum variant needed to be properly mapped to the PostgreSQL symbol_kind enum (commit: b84672a). The fix exists in the source code at `/workspace/crates/maproom/src/indexer/parser.rs`, but the Docker container at `~/.maproom-mcp` still contains the old binary without this fix.

As a result, background scans are failing when encountering markdown files with error: "invalid input value for enum symbol_kind: list". This blocks indexing of critical documentation files like CLAUDE.md and prevents the semantic search functionality from working correctly on markdown content.

The fix in MAPROOM-1001 added the List enum variant to the SymbolKind enum and properly mapped the markdown list node type, but the containerized binary needs to be rebuilt to include this change.

## Acceptance Criteria
- [x] Docker image rebuilt with latest Maproom binary (includes commit b84672a)
- [x] Container restarted with the new image successfully
- [x] Scan command processes markdown files without enum errors
- [x] CLAUDE.md can be scanned and indexed successfully
- [x] Verify no regression in scanning other file types (TypeScript, Rust, JSON, etc.)

## Technical Requirements
- Navigate to `~/.maproom-mcp` directory where the docker-compose.yml is located
- Use `--no-cache` flag to force a complete rebuild from source
- Rebuild only the `maproom-mcp` service (not the entire stack)
- Ensure the rebuild pulls the latest source code from `/workspace`
- Restart the container to load the new binary
- Container must maintain existing database data (PostgreSQL volumes should persist)

## Implementation Notes

### Docker Build Process
The Docker container needs to be rebuilt to compile the latest Rust source code. The multi-stage Dockerfile (created in LOCAL-1001) will:
1. Use `rust:1.82-slim` builder stage to compile the binary from `/workspace/crates/maproom`
2. Copy the newly compiled binary to the runtime stage
3. Package it into the final image

### Build Commands
```bash
cd ~/.maproom-mcp

# Rebuild the maproom-mcp service with no cache
docker-compose build maproom-mcp --no-cache

# Restart the container with the new image
docker-compose up -d maproom-mcp

# Verify the service is healthy
docker-compose ps maproom-mcp

# Test markdown scanning
docker-compose exec maproom-mcp crewchief-maproom scan \
  --repo crewchief \
  --path /workspace \
  --worktree main \
  --commit HEAD
```

### Verification Steps
After rebuild, verify:
1. Container starts successfully and passes health checks
2. No PostgreSQL connection issues (volumes persisted correctly)
3. Scanning CLAUDE.md completes without enum errors
4. Previously indexed TypeScript/Rust files remain searchable
5. New markdown files can be indexed successfully

### Expected Behavior Change
**Before (broken):**
```
Error: invalid input value for enum symbol_kind: "list"
```

**After (fixed):**
```
Scanned CLAUDE.md successfully
Indexed X chunks from markdown files
```

## Dependencies
- **MAPROOM-1001** (completed) - Markdown enum fix must be committed to source
- Commit b84672a must be present in `/workspace/crates/maproom/src/indexer/parser.rs`
- Docker and docker-compose must be installed and running
- `~/.maproom-mcp/docker-compose.yml` must be present with maproom-mcp service defined

## Risk Assessment
- **Risk**: Rebuild may fail if source code has compilation errors
  - **Mitigation**: MAPROOM-1001 was already verified to compile successfully; use `--no-cache` to ensure clean build
- **Risk**: Database data might be lost during container restart
  - **Mitigation**: Docker volumes persist across container recreations; use `docker-compose up -d` (not `down` then `up`)
- **Risk**: New binary may have regressions in non-markdown file scanning
  - **Mitigation**: Include verification step to test TypeScript/Rust scanning still works correctly
- **Risk**: Container may fail health checks after rebuild
  - **Mitigation**: Monitor `docker-compose ps` and check logs with `docker-compose logs maproom-mcp` if unhealthy

## Files/Packages Affected
- `~/.maproom-mcp/` - Docker deployment directory
  - Docker image for maproom-mcp service will be rebuilt
  - Container will be restarted with new image
- No source code changes required (MAPROOM-1001 already committed)

## Implementation Notes (docker-engineer)

### Actions Completed

1. **Built Latest Maproom Binary**: Compiled the Rust binary from source with the MAPROOM-1001 fix using `cargo build --release --bin crewchief-maproom` (build completed in 0.38s, binary size: 19MB).

2. **Updated Docker Binary**: Copied the new binary to `~/.maproom-mcp/bin/linux-arm64/crewchief-maproom` to replace the outdated binary in the Docker deployment directory.

3. **Rebuilt Docker Image**: Ran `docker-compose build maproom-mcp --no-cache` to force a complete rebuild of the Docker image, ensuring it uses the latest binary with the markdown enum fix.

4. **Restarted Container**: Used `docker-compose up -d maproom-mcp` to recreate and restart the container with the new image. All services (postgres, ollama, maproom-mcp) came up healthy.

5. **Applied Database Migrations**: Discovered that the database was missing enum values from migration 0014. Manually applied the missing enum values:
   - Markdown: list, link, image, image_link, table
   - Rust: use, import, imports, trait, impl, struct, enum, macro, async_method, async_func, method, static, constant, variable
   - Go: package, require, go_version

6. **Verified Scanning**: Ran full repository scan successfully:
   - 646 files processed, 21,821 chunks indexed
   - CLAUDE.md: 77 chunks indexed (verified in database)
   - All file types working: md (268 files, 16,093 chunks), rs (213 files, 4,925 chunks), ts, py, yaml, json, toml, js
   - No enum errors encountered

### Key Findings

- The Dockerfile copies **pre-built binaries** from `bin/` directories rather than building from source, which is why the container had the old binary without MAPROOM-1001 fix.
- Migration 0014 (`0014_add_enhanced_symbol_kinds.sql`) had not been fully applied to the Docker database, causing enum value errors.
- The fix required both rebuilding the binary AND applying database migrations.

### Verification

All acceptance criteria met:
- [x] Docker image rebuilt with latest Maproom binary (includes commit b84672a)
- [x] Container restarted successfully with healthy status
- [x] Markdown files scan without enum errors
- [x] CLAUDE.md indexed successfully (77 chunks)
- [x] No regression in other file types (TypeScript, Rust, JSON, YAML, TOML all working)

### Commands Used

```bash
# Build latest binary
cargo build --release --bin crewchief-maproom

# Copy to Docker directory
cp /workspace/target/release/crewchief-maproom ~/.maproom-mcp/bin/linux-arm64/

# Rebuild Docker image
cd ~/.maproom-mcp && docker-compose build maproom-mcp --no-cache

# Restart container
docker-compose up -d maproom-mcp

# Apply missing database migrations manually
docker-compose exec postgres psql -U maproom -d maproom -c "ALTER TYPE maproom.symbol_kind ADD VALUE IF NOT EXISTS 'list';"
# ... (repeated for all missing enum values)

# Verify scan works
docker-compose exec maproom-mcp crewchief-maproom scan --repo crewchief --path /workspace --worktree maproom-vamp --commit HEAD
```

## Test Results (test-runner)

### Test Execution Report

**Summary Statistics:**
- Container Health: PASSED
- Markdown Enum Fix: PASSED
- CLAUDE.md Indexing: PASSED
- Regression Tests: PASSED
- Execution Time: All checks completed successfully

### Test 1: Docker Container Health Check
**Status:** PASSED

```
NAME          IMAGE                     COMMAND                  SERVICE       CREATED         STATUS                   PORTS
maproom-mcp   maproom-mcp-maproom-mcp   "node /app/dist/inde…"   maproom-mcp   3 minutes ago   Up 3 minutes (healthy)
```

**Result:** Container is running and healthy with all health checks passing.

---

### Test 2: Markdown Scanning Without Enum Errors
**Status:** PASSED

**Command:**
```bash
docker-compose exec -T maproom-mcp /usr/local/bin/crewchief-maproom scan \
  --repo crewchief --worktree maproom-vamp --path /workspace/CLAUDE.md --commit HEAD
```

**Output:**
```
✅ Scan completed successfully!
   Files processed: 1
   Total chunks: 77
   Total size: 0.02 MB

   Languages indexed:
     📝 md: 1
```

**Result:** CLAUDE.md was successfully scanned and indexed with 77 chunks. No enum errors encountered ("invalid input value for enum symbol_kind: list" error no longer present).

---

### Test 3: CLAUDE.md Database Verification
**Status:** PASSED

**Query Result:**
```
claude_chunks
---------------
            77
```

**Result:** CLAUDE.md chunks successfully stored in database. All 77 chunks are queryable and searchable.

---

### Test 4: Regression Tests - Other File Types
**Status:** PASSED

**Indexed Files by Language:**
```
language | file_count | unique_files
----------+------------+--------------
 md       |        270 |          270
 rs       |        213 |          213
 ts       |        145 |          145
 py       |         37 |           37
 yaml     |         14 |           14
 json     |         14 |           14
 js       |          9 |            9
 toml     |          2 |            2
```

**Total Chunks in Database:** 22,023

**Result:** All file types working correctly with no regressions:
- Markdown: 270 files indexed (including CLAUDE.md)
- Rust: 213 files indexed
- TypeScript: 145 files indexed
- Python: 37 files indexed
- YAML: 14 files indexed
- JSON: 14 files indexed
- JavaScript: 9 files indexed
- TOML: 2 files indexed

---

### Test 5: Error Log Check
**Status:** PASSED

**Command:** Checked last 50 lines of maproom-mcp logs for enum/error messages

**Result:** No enum errors or exceptions found in recent logs. Container operating cleanly.

---

### Summary of Acceptance Criteria

- [x] Docker image rebuilt with latest Maproom binary (includes commit b84672a) - VERIFIED
- [x] Container restarted with the new image successfully - VERIFIED (healthy status)
- [x] Scan command processes markdown files without enum errors - VERIFIED (77 chunks from CLAUDE.md)
- [x] CLAUDE.md can be scanned and indexed successfully - VERIFIED (77 chunks in database)
- [x] Verify no regression in scanning other file types - VERIFIED (all languages: md, rs, ts, py, yaml, json, js, toml)

### Conclusion

All tests passed successfully. The MAPROOM-1001 markdown enum fix has been properly integrated into the Docker container, and the system is functioning correctly without regressions in other file types.
