# Service Layer Implementation

This directory contains the service layer implementation for the CrewChief Web UI project, fulfilling the requirements of TICKET-012.

## Overview

The service layer provides a consistent interface for business logic operations with comprehensive error handling, caching, audit logging, and security features.

## Architecture

### Base Service (`base.ts`)

The `BaseService` class provides common functionality for all services:

- **Result Pattern**: Consistent error handling using `Result<T, E>` type
- **Correlation ID Tracking**: Automatic correlation ID generation and tracking
- **Audit Logging**: Comprehensive audit trail for all operations
- **Caching**: Redis-backed caching with graceful fallback to memory cache
- **Authorization**: Service-level authorization checks
- **Transaction Support**: Database transaction management
- **Encryption**: Sensitive data encryption capabilities
- **Health Checks**: Standard health check interface

### Services Implemented

#### 1. MaproomService (`maproom.ts`)
- **Purpose**: Interface to Maproom binary for code search and indexing
- **Features**:
  - Semantic and full-text search
  - Index management and status reporting
  - File upsert operations
  - Process management for long-running operations
  - Binary path resolution with environment variable support

#### 2. WorktreeService (`worktree.ts`)
- **Purpose**: Git worktree management
- **Features**:
  - Worktree creation and deletion
  - Branch management
  - Merge operations with conflict detection
  - Automatic cleanup of old worktrees
  - Ignored files copying

#### 3. AgentService (`agent.ts`)
- **Purpose**: Agent lifecycle management
- **Features**:
  - Agent spawning and termination
  - Real-time monitoring and metrics
  - Log management and aggregation
  - Process resource tracking
  - Event-based notifications

#### 4. ConfigService (`config.ts`)
- **Purpose**: Configuration management with validation
- **Features**:
  - Schema-based validation using Zod
  - Configuration versioning and backups
  - Encryption for sensitive configuration
  - File and database persistence
  - Rollback capabilities

#### 5. MonitoringService (`monitoring.ts`)
- **Purpose**: System monitoring and alerting
- **Features**:
  - Real-time system metrics collection
  - Service health monitoring
  - Alert management and notifications
  - Performance tracking
  - Configurable thresholds

## Key Features

### Error Handling (Result Pattern)
```typescript
type Result<T, E = Error> = 
  | { success: true; data: T; correlationId: string }
  | { success: false; error: E; correlationId: string };
```

All service methods return a `Result` type for consistent error handling.

### Caching Strategy
- **Redis Primary**: High-performance distributed caching
- **Memory Fallback**: Graceful degradation when Redis unavailable
- **Configurable TTL**: Per-operation cache duration settings
- **Pattern-based Invalidation**: Cache clearing by key patterns

### Audit Logging
Every operation is logged with:
- Correlation ID for request tracing
- User identification
- Operation details and metadata
- Success/failure status
- Timestamp and duration

### Security Features
- **Authorization Checks**: Role-based access control
- **Audit Trail**: Complete operation logging
- **Data Encryption**: Sensitive data encryption at rest
- **Input Validation**: Schema-based validation
- **Rate Limiting**: Built-in protection mechanisms

### Dependency Injection
All services support dependency injection for:
- Cache providers
- Audit loggers
- Database connections
- External service dependencies

## Usage

### Service Container
```typescript
import { createServiceContainer, initializeServices } from './services/index.js';

// Initialize services
const services = initializeServices({
  global: {
    redis: { enabled: true, url: 'redis://localhost:6379' },
    audit: { enabled: true, level: 'info' },
  },
  maproom: { timeout: 30000, retries: 2 },
  worktree: { maxWorktrees: 50, autoCleanup: true },
  agent: { maxAgents: 20, defaultTimeout: 3600000 },
  config: { validateOnLoad: true, encryptSensitive: true },
  monitoring: { healthCheckInterval: 60000 },
});

// Use services
const result = await services.maproom.search('query', {}, userId);
if (result.success) {
  console.log('Search results:', result.data.results);
} else {
  console.error('Search failed:', result.error.message);
}
```

### Individual Service Usage
```typescript
import { createMaproomService } from './services/maproom.js';

const maproom = createMaproomService({
  binaryPath: '/path/to/maproom',
  timeout: 30000,
  cacheEnabled: true,
});

const searchResult = await maproom.search('function', {
  language: 'typescript',
  maxResults: 10,
}, 'user-123');
```

## Testing

### Unit Tests
The implementation includes comprehensive unit tests demonstrating:
- **Mocked Dependencies**: Complete service isolation
- **Error Scenarios**: All error paths tested
- **Cache Behavior**: Cache hit/miss scenarios
- **Authorization**: Access control testing
- **Audit Logging**: Complete audit trail verification

### Test Structure
```
tests/unit/services/
├── base.test.ts       # Base service functionality
├── maproom.test.ts    # Maproom service tests
└── ...
```

### Running Tests
```bash
# Run all service tests
pnpm test:unit tests/unit/services/

# Run specific service tests
pnpm test:unit tests/unit/services/maproom.test.ts
```

## Database Schema

The service layer requires additional database tables (see `migrations/0011_service_layer_tables.sql`):

- `audit_log`: Audit trail for all operations
- `system_metrics`: System performance metrics
- `service_health`: Service health monitoring
- `system_alerts`: Alert management
- `config_backups`: Configuration backup storage

## Configuration

### Environment Variables
- `CREWCHIEF_MAPROOM_BIN`: Path to Maproom binary
- `REDIS_URL`: Redis connection URL
- `DB_*`: Database connection parameters

### Service Configuration
```typescript
interface ServicesConfig {
  global?: {
    redis?: { url?: string; enabled?: boolean; ttl?: number };
    audit?: { enabled?: boolean; level?: string };
    security?: { encryptionKey?: string };
  };
  maproom?: { binaryPath?: string; timeout?: number; retries?: number };
  worktree?: { maxWorktrees?: number; autoCleanup?: boolean };
  agent?: { maxAgents?: number; resourceLimits?: object };
  config?: { validateOnLoad?: boolean; encryptSensitive?: boolean };
  monitoring?: { healthCheckInterval?: number; alertThresholds?: object };
}
```

## Health Monitoring

All services implement health checks accessible via:
```typescript
import { checkServicesHealth } from './services/index.js';

const health = await checkServicesHealth();
console.log('Service health:', health);
```

## Performance Considerations

- **Connection Pooling**: Database connections are pooled
- **Caching**: Aggressive caching reduces external calls
- **Async Operations**: Non-blocking operations throughout
- **Resource Limits**: Configurable resource constraints
- **Monitoring**: Real-time performance tracking

## Security Considerations

- **Input Validation**: All inputs validated against schemas
- **Authorization**: Role-based access control on all operations
- **Audit Logging**: Complete audit trail for compliance
- **Encryption**: Sensitive data encrypted at rest
- **Rate Limiting**: Protection against abuse
- **SQL Injection**: Parameterized queries used throughout

## Future Enhancements

- **Distributed Tracing**: OpenTelemetry integration
- **Advanced Caching**: Multi-level cache hierarchy
- **Circuit Breakers**: Fault tolerance patterns
- **Metrics Export**: Prometheus/Grafana integration
- **Advanced Security**: JWT token validation, OAuth integration