# Architecture: Test Database Isolation

## Overview

Establish parallel database infrastructure where development and testing environments run independently on the same machine without interference. This enables true test isolation while maintaining developer ergonomics.

## Architecture Decisions

### Decision 1: Single Compose File with Dual Databases

**Chosen Approach**: Add `postgres-test` service to existing `docker-compose.yml`

**Rationale**:
- Developer experience: `docker compose up` starts everything
- Resource efficient: Modern machines easily handle two postgres containers
- Discoverable: All infrastructure visible in one file
- Maintainable: One source of truth for database configuration

**Alternatives Considered**:
- Separate compose files: Rejected due to cognitive overhead
- Environment-based overrides: Rejected as over-engineered for this use case

### Decision 2: Port Allocation

**Development Database**:
- Host Port: `5433` (existing)
- Container Port: `5432`
- Database: `maproom`
- Container: `maproom-postgres`

**Test Database**:
- Host Port: `5434` (new)
- Container Port: `5432`
- Database: `maproom_test`
- Container: `maproom-postgres-test`

**Rationale**:
- Clear separation prevents accidental cross-connection
- Sequential ports (5433, 5434) are memorable
- Standard PostgreSQL port (5432) used inside containers

### Decision 3: Schema Initialization

**Current Reality**: Manual schema initialization (init.sql mount disabled)

**Current Setup**:
The existing docker-compose.yml has init.sql mount disabled due to Docker-in-Docker limitations:
```yaml
# Note: init.sql mount disabled in dev container due to Docker-in-Docker limitations
# Schema will be initialized via migrations or manual SQL execution
```

**Chosen Approach**: Follow existing pattern - manual schema initialization

Both dev and test databases use the same manual initialization process:
1. **Option A**: Execute SQL from init.sql manually after container starts
2. **Option B**: Run existing migration scripts against the database

**Rationale**:
- Matches current dev database setup (consistency)
- Test database has identical schema to dev (parity)
- No schema drift between environments (same SQL source)
- Single source of truth for database structure (init.sql)

**Schema Initialization Steps** (documented in Phase 1):
```bash
# After postgres-test container starts:
docker exec maproom-postgres-test psql -U maproom -d maproom_test < /path/to/init.sql

# OR use existing migration system if available
```

### Decision 4: Volume Isolation

**Development**:
```yaml
volumes:
  - maproom-data:/var/lib/postgresql/data
```

**Test**:
```yaml
volumes:
  - maproom-test-data:/var/lib/postgresql/data
```

**Rationale**:
- Separate volumes guarantee data isolation
- Test volume can be destroyed/recreated without affecting dev data
- Supports "clean slate" test runs

### Decision 5: Environment Variable Hierarchy

**Priority Order**:
1. `TEST_MAPROOM_DATABASE_URL` (highest - test-specific)
2. `MAPROOM_DATABASE_URL` (fallback - development)

Implemented in:
- `vitest.config.ts`: Test framework configuration
- `package.json` scripts: Test execution commands
- Test helpers: Already supports this pattern (no changes needed)

**Rationale**:
- Backward compatible: Existing code works without changes
- Explicit intent: TEST_ prefix makes purpose clear
- Flexible: Can override in different contexts (local, CI, debugging)

## Component Architecture

### Docker Infrastructure

```
┌─────────────────────────────────────────────────────┐
│          docker-compose.yml (Single Project)         │
├─────────────────────────────────────────────────────┤
│                                                       │
│  ┌──────────────────┐      ┌──────────────────┐    │
│  │    postgres      │      │  postgres-test   │    │
│  ├──────────────────┤      ├──────────────────┤    │
│  │ Port: 5433:5432  │      │ Port: 5434:5432  │    │
│  │ DB: maproom      │      │ DB: maproom_test │    │
│  │ Vol: maproom-    │      │ Vol: maproom-    │    │
│  │      data        │      │      test-data   │    │
│  └──────────────────┘      └──────────────────┘    │
│         ▲                           ▲               │
│         │                           │               │
│         │                           │               │
│  ┌──────┴────────┐          ┌──────┴────────┐     │
│  │  Dev Workflow │          │  Test Suite    │     │
│  │  (MCP Server) │          │  (Vitest)      │     │
│  └───────────────┘          └────────────────┘     │
│                                                       │
│  MAPROOM_DATABASE_URL        TEST_MAPROOM_DATABASE_URL      │
│  localhost:5433              localhost:5434          │
│                                                       │
└─────────────────────────────────────────────────────┘
```

### Network Configuration

Both databases on same Docker network (`maproom-network`) for:
- Container-to-container communication
- Service health checks
- Consistent DNS resolution

From inside containers:
- Dev: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- Test: `postgresql://maproom:maproom@maproom-postgres-test:5432/maproom_test`

From host machine:
- Dev: `postgresql://maproom:maproom@localhost:5433/maproom`
- Test: `postgresql://maproom:maproom@localhost:5434/maproom_test`

### Container vs Host Context

**Execution Context**: Tests run on **host machine** (not inside Docker containers)

**Hostname Resolution Table**:

| **Component** | **Execution Context** | **Database Hostname** | **Port** | **Full URL** |
|---------------|----------------------|----------------------|----------|--------------|
| vitest.config.ts | Host → Docker network | `maproom-postgres-test` | `5432` | `postgresql://maproom:maproom@maproom-postgres-test:5432/maproom_test` |
| package.json scripts | Host machine | `localhost` | `5434` | `postgresql://maproom:maproom@localhost:5434/maproom_test` |
| MCP Server (container) | Inside Docker | `maproom-postgres-test` | `5432` | `postgresql://maproom:maproom@maproom-postgres-test:5432/maproom_test` |

**Why Different Hostnames?**:
- **vitest.config.ts**: Uses container hostname because tests connect through Docker network
- **package.json**: Uses localhost because pnpm executes on host and connects to mapped port
- From **host perspective**: Connect to `localhost:5434` which maps to container's `5432`
- From **container perspective**: Connect to `maproom-postgres-test:5432` directly via Docker network

**Correct Configuration**:
```typescript
// vitest.config.ts - uses container hostname
env: {
  MAPROOM_DATABASE_URL:
    process.env.TEST_MAPROOM_DATABASE_URL ||
    'postgresql://maproom:maproom@maproom-postgres-test:5432/maproom_test'
}

// package.json - uses localhost (host-based execution)
"test:vitest": "TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test vitest run"
```

**Rationale**: Tests executed via `pnpm test` run on the host machine but need to connect to databases inside Docker containers. The vitest configuration uses container hostnames (visible via Docker network), while package.json scripts set environment variables using localhost (host perspective).

### Test Configuration Flow

```
Test Execution (pnpm test)
         │
         ▼
   package.json script
   (sets TEST_MAPROOM_DATABASE_URL)
         │
         ▼
   vitest.config.ts
   (env.TEST_MAPROOM_DATABASE_URL)
         │
         ▼
   Test Suite
         │
         ▼
   tests/helpers/database.ts
   (getDatabaseUrl())
         │
         ▼
   TEST_MAPROOM_DATABASE_URL ||  <── Uses test DB if set
   MAPROOM_DATABASE_URL  <── Falls back to dev DB
```

### CI/CD Architecture

GitHub Actions workflow modifications:

```yaml
jobs:
  test:
    services:
      postgres-test:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_DB: maproom_test
          POSTGRES_USER: maproom
          POSTGRES_PASSWORD: maproom
        ports:
          - 5434:5432
    env:
      TEST_MAPROOM_DATABASE_URL: postgresql://maproom:maproom@localhost:5434/maproom_test
    steps:
      - run: pnpm test
```

## Migration Strategy

### Phase 1: Add Infrastructure (Non-Breaking)
- Add `postgres-test` service to docker-compose.yml
- No changes to existing postgres service
- No code changes required yet

**Impact**: Zero (new service dormant until used)

### Phase 2: Update Configurations (Backward Compatible)
- Update vitest.config.ts to use `TEST_MAPROOM_DATABASE_URL || MAPROOM_DATABASE_URL`
- Update package.json test scripts to set TEST_MAPROOM_DATABASE_URL
- Test helpers already support this pattern

**Impact**: Tests can use either database (backward compatible)

### Phase 3: Update CI/CD (Validation)
- Configure GitHub Actions to use TEST_MAPROOM_DATABASE_URL
- Verify tests pass with isolated database

**Impact**: Increases CI reliability

### Phase 4: Documentation (Developer Experience)
- Update README with two-database setup
- Troubleshooting guide
- Migration guide for existing developers

**Impact**: Improved onboarding and debugging

## Technology Choices

### PostgreSQL Version
- **Choice**: `pgvector/pgvector:pg16` (same as production)
- **Rationale**: Parity with existing database, pgvector extension required

### Configuration Parity
Both databases use identical:
- PostgreSQL version
- Configuration parameters (shared_buffers, work_mem, etc.)
- Extensions (pgvector)
- Schema (init.sql)

**Rationale**: Tests should run against production-like database

## Performance Considerations

### Resource Usage
- Two PostgreSQL containers: ~500MB RAM each
- Minimal CPU overhead (idle when not in use)
- Disk: Separate volumes, test data typically smaller

**Impact**: Negligible on modern development machines

### Startup Time
- Both databases start in parallel
- Health checks ensure readiness before tests run
- Total overhead: ~5-10 seconds

**Mitigation**: Databases run persistently (restart: unless-stopped)

## Constraints and Trade-offs

### Constraints
1. **Docker Required**: Developers must have Docker installed
2. **Port Availability**: Ports 5433 and 5434 must be available
3. **Disk Space**: Two database volumes (test volume typically smaller)

### Trade-offs

**Chosen**: Developer ergonomics over resource minimization
- **Benefit**: One command starts everything
- **Cost**: Extra postgres container (negligible)

**Chosen**: Same schema for both databases
- **Benefit**: Production parity
- **Cost**: Test database may have features not used in tests

**Chosen**: Persistent test database
- **Benefit**: Faster test iteration (no cold starts)
- **Cost**: Must manually reset if needed

## Long-term Maintainability

### Configuration Management
- Single docker-compose.yml reduces fragmentation
- TEST_MAPROOM_DATABASE_URL convention extends to future test infrastructure
- Schema changes automatically propagate to test database (via init.sql)

### Extensibility
This architecture supports future enhancements:
- Multiple test databases for parallel test execution
- Staging database on different port
- Performance testing database with different config

### Deprecation Path
If Docker becomes a constraint:
- Can switch to sqlite for tests (different trade-offs)
- Can use testcontainers for per-test isolation
- TEST_MAPROOM_DATABASE_URL abstraction supports any backend
