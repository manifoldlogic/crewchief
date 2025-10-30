# CrewChief Project Cleanup Plan

**Date:** 2025-10-22
**Branch:** cleanup-container-cruft
**Goal:** Remove all web-ui, redis, and pgAdmin remnants and focus solely on CrewChief CLI, Maproom (Rust), and Maproom MCP.

## Executive Summary

The CrewChief project once had a web-ui component with redis and pgAdmin dependencies. These have been removed from the codebase, but numerous configuration files, documentation, scripts, and references remain throughout the repository. This document outlines a comprehensive cleanup to align the repository state with the actual working code.

## Current State Analysis

### What We're Keeping
1. **CrewChief CLI** (`packages/cli/`) - Multi-agent orchestration and git worktree management
2. **Maproom** (`crates/maproom/`) - Rust-based semantic code search engine
3. **Maproom MCP** (`packages/maproom-mcp/`) - MCP server for AI assistants

### What Was Removed (But Remnants Exist)
1. **Web UI** - Previously in `packages/web-ui/` (already deleted)
2. **Redis** - Session/cache storage for web-ui
3. **pgAdmin** - Database management tool for development

## Identified Remnants

### 1. Docker Configuration Files

#### Files to DELETE:
- **`/workspace/docker-compose.yml`**
  - Still references web-ui migrations: `./packages/web-ui/migrations:/docker-entrypoint-initdb.d/migrations:ro`
  - Only contains PostgreSQL service for web-ui (not needed for CLI/Maproom)
  - **Rationale:** Maproom has its own database setup via CLI commands; docker-compose not required

- **`/workspace/Makefile`**
  - Entirely web-ui focused (all commands reference web-ui, redis, pgAdmin)
  - Commands: `dev`, `prod`, `build`, `logs-web`, `logs-redis`, `shell`, etc.
  - **Rationale:** No longer needed; CLI has its own build system

#### Files to KEEP but MODIFY:
- **`.devcontainer/docker-compose.yml`**
  - Keep: PostgreSQL service (used by Maproom)
  - Remove: References to web-ui environment variables
  - Keep: Core dev container setup
  - **Action:** Clean up environment variables related to web-ui

### 2. Docker Scripts

#### Files to DELETE:
- **`/workspace/scripts/docker-build.sh`**
  - Purpose: Build web-ui Docker images
  - References: `packages/web-ui/package.json`, `packages/web-ui/Dockerfile`
  - **Rationale:** No web-ui to build

- **`/workspace/scripts/docker-run.sh`**
  - Purpose: Manage web-ui Docker Compose deployments
  - All commands target web-ui services
  - **Rationale:** No web-ui services to manage

- **`/workspace/scripts/docker-test.sh`**
  - Purpose: Test web-ui Docker setup
  - Tests redis, web-ui container, Dockerfiles
  - **Rationale:** No web-ui to test

- **`/workspace/scripts/build-opsdeck.sh`**
  - Contains: `echo "opsdeck"`
  - **Rationale:** Appears to be a stub/placeholder with no real functionality

- **`/workspace/scripts/release.sh`**
  - Contains: `echo "release"`
  - **Rationale:** Appears to be a stub; actual release logic is in `packages/cli/src/cli/release.ts`

#### Files to KEEP:
- **`/workspace/scripts/build-maproom.sh`** - Used for building Rust components
- **`/workspace/scripts/build-and-package.sh`** - Comprehensive build script for all platforms

### 3. Documentation Files

#### Files to DELETE:
- **`/workspace/DOCKER.md`**
  - Entirely about running web-ui via Docker
  - Covers redis, pgAdmin, web-ui services
  - **Rationale:** No longer applicable to project

- **`/workspace/DOCKER_SETUP_SUMMARY.md`**
  - Summary of web-ui Docker setup (TICKET-007)
  - Documents all the docker files we're removing
  - **Rationale:** Historical document for removed features

#### Files to MODIFY:
- **`/workspace/.env.example`**
  - **Remove sections:**
    - Database Configuration (lines 4-16) - Maproom uses `PG_DATABASE_URL` env var instead
    - Application Configuration (lines 18-40) - Session/JWT secrets for web-ui
    - Security Settings (lines 42-53) - Rate limiting, sessions, cookies for web-ui
    - Monitoring & Observability (lines 56-63) - Metrics for web-ui
    - File Upload Settings (lines 66-72) - Web-ui feature
    - Database Migration Settings (lines 75-81) - Web-ui auto-migration
    - Cache Settings (lines 84-91) - Redis cache TTLs
    - Docker Compose Overrides (lines 93-100) - Postgres env vars for docker-compose
    - Production Security (lines 103-115) - SSL, CSP for web-ui
  - **Keep:** Environment variables needed for Maproom (if any)
  - **Rationale:** File is entirely web-ui focused; Maproom uses simpler config

- **`/workspace/README.md`**
  - **Keep:** Overall project description and structure
  - **Review:** Ensure it accurately describes current state (CLI + Maproom + MCP)
  - **Current state:** Looks good, accurately describes the project

### 4. DevContainer Configuration

#### Files to MODIFY:
- **`.devcontainer/devcontainer.json`**
  - **Remove:**
    - Port 3000 (Frontend dev server) from `forwardPorts`
    - Port 3500 (Backend API) from `forwardPorts`
    - Port attributes for 3000 and 3500
  - **Keep:**
    - Port 5432 (PostgreSQL) - needed for Maproom
  - **Rationale:** Only Maproom uses PostgreSQL

- **`.devcontainer/scripts/post-create.sh`**
  - **Remove:**
    - Lines referring to `packages/web-ui`
    - `webui` alias commands
    - `REDIS_URL` environment variable setup
  - **Keep:**
    - Core dependency installation
    - Maproom binary build
  - **Rationale:** Remove web-ui specific setup

- **`.devcontainer/scripts/post-start.sh`**
  - **Remove:**
    - Redis ping check
    - web-ui tmux window creation
  - **Keep:**
    - Core startup logic
  - **Rationale:** No redis or web-ui to start

- **`.devcontainer/scripts/post-attach.sh`**
  - **Remove:**
    - Redis connectivity check
  - **Keep:**
    - Other attach logic
  - **Rationale:** No redis service

- **`.devcontainer/README.md`**
  - **Remove:**
    - References to redis, web-ui, pgAdmin
    - REDIS_URL environment variable documentation
    - pgadmin-servers.json reference
  - **Update:**
    - Focus on CLI and Maproom development
  - **Rationale:** Align with actual dev environment

- **`.devcontainer/TROUBLESHOOTING.md`**
  - **Remove:**
    - Redis troubleshooting sections
    - Web-ui troubleshooting sections
  - **Keep:**
    - PostgreSQL troubleshooting (for Maproom)
  - **Rationale:** Only relevant troubleshooting

- **`.devcontainer/pgadmin-servers.json`**
  - **DELETE:** Not needed without web-ui and pgAdmin
  - **Rationale:** pgAdmin was only used for web-ui development

### 5. Source Code References

#### Files to MODIFY:
- **`packages/cli/src/cli/build.ts`**
  - **Remove:**
    - Lines 241-261: `buildWebUI()` function
    - Lines 12, 269, 292-294: `skipWeb` option and related logic
    - Lines 305: Output reference to Web UI dist
  - **Update:**
    - Remove web-ui from build targets
  - **Rationale:** No web-ui package to build

### 6. Git Status Review

Based on git status:
```
M .env.example
M DOCKER.md
M DOCKER_SETUP_SUMMARY.md
D docker-compose.dev.yml
M docker-compose.yml
```

- **`docker-compose.dev.yml`** - Already deleted ✓
- **`docker-compose.yml`** - Modified but should be DELETED entirely
- **`DOCKER.md`** - Modified but should be DELETED entirely
- **`DOCKER_SETUP_SUMMARY.md`** - Modified but should be DELETED entirely
- **`.env.example`** - Modified but should be more aggressively cleaned

## Cleanup Actions Summary

### Files to DELETE (18 files):
1. `/workspace/docker-compose.yml` - Web-ui docker compose config
2. `/workspace/Makefile` - Web-ui make commands
3. `/workspace/DOCKER.md` - Web-ui Docker documentation
4. `/workspace/DOCKER_SETUP_SUMMARY.md` - Web-ui Docker setup summary
5. `/workspace/scripts/docker-build.sh` - Web-ui Docker build script
6. `/workspace/scripts/docker-run.sh` - Web-ui Docker run script
7. `/workspace/scripts/docker-test.sh` - Web-ui Docker test script
8. `/workspace/scripts/build-opsdeck.sh` - Empty stub script
9. `/workspace/scripts/release.sh` - Empty stub script
10. `/workspace/.devcontainer/pgadmin-servers.json` - pgAdmin config

### Files to MODIFY (10 files):
1. `/workspace/.env.example` - Remove all web-ui config, keep minimal example for Maproom
2. `/workspace/.devcontainer/devcontainer.json` - Remove web-ui ports
3. `/workspace/.devcontainer/scripts/post-create.sh` - Remove web-ui and redis setup
4. `/workspace/.devcontainer/scripts/post-start.sh` - Remove redis and web-ui startup
5. `/workspace/.devcontainer/scripts/post-attach.sh` - Remove redis checks
6. `/workspace/.devcontainer/README.md` - Remove web-ui, redis, pgAdmin references
7. `/workspace/.devcontainer/TROUBLESHOOTING.md` - Remove web-ui, redis troubleshooting
8. `/workspace/packages/cli/src/cli/build.ts` - Remove web-ui build function

### Files to KEEP (No changes):
1. `/workspace/README.md` - Already accurate
2. `/workspace/CLAUDE.md` - Already accurate
3. `/workspace/scripts/build-maproom.sh` - Needed for Rust builds
4. `/workspace/scripts/build-and-package.sh` - Needed for comprehensive builds
5. All files in `packages/cli/` (except build.ts modifications)
6. All files in `packages/maproom-mcp/`
7. All files in `crates/maproom/`

## Validation Steps

After cleanup:
1. ✅ Verify no references to `web-ui` in codebase (except historical git commits)
2. ✅ Verify no references to `redis` in codebase
3. ✅ Verify no references to `pgadmin` in codebase
4. ✅ Ensure CLI build still works: `cd packages/cli && pnpm build`
5. ✅ Ensure Maproom MCP build still works: `cd packages/maproom-mcp && pnpm build`
6. ✅ Ensure Maproom Rust build still works: `cd crates/maproom && cargo build --release`
7. ✅ Review all documentation for accuracy
8. ✅ Test devcontainer still works with remaining services

## Risk Assessment

**Low Risk:**
- Deleting Docker files - Not used by CLI/Maproom
- Deleting Makefile - Not used by current workflow
- Deleting stubs - No functionality
- Deleting web-ui docs - Historical only

**Medium Risk:**
- Modifying .env.example - Need to ensure Maproom still has needed examples
- Modifying devcontainer - Need to ensure PostgreSQL service still works
- Modifying build.ts - Need to ensure builds still work

**Mitigation:**
- All changes on feature branch `cleanup-container-cruft`
- User will review before committing
- Can easily revert if issues found

## Post-Cleanup State

The repository will contain:
1. **Packages:**
   - `packages/cli/` - CrewChief CLI with agent orchestration
   - `packages/maproom-mcp/` - MCP server for AI assistants

2. **Rust Crates:**
   - `crates/maproom/` - Semantic search engine

3. **Build System:**
   - `scripts/build-maproom.sh` - Rust build script
   - `scripts/build-and-package.sh` - Comprehensive build script
   - `packages/cli/src/cli/build.ts` - TypeScript build command

4. **DevContainer:**
   - PostgreSQL service for Maproom
   - Development tools for TypeScript and Rust
   - No web-ui, redis, or pgAdmin

5. **Documentation:**
   - `README.md` - Project overview
   - `CLAUDE.md` - Development guide
   - Package-specific READMEs
   - No Docker-specific docs

## Conclusion

This cleanup will remove ~18 files and modify ~10 files to eliminate all traces of the web-ui, redis, and pgAdmin features that were previously removed from the codebase. The result will be a cleaner, more focused repository that accurately represents the current state of the CrewChief project: a CLI tool for agent orchestration with semantic code search capabilities.
