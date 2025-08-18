# CrewChief Database Schema Documentation

## Overview

The CrewChief Web UI uses a PostgreSQL database that integrates with the existing Maproom schema. This document provides comprehensive documentation of the database schema, including entity relationships, table descriptions, index strategy, and performance considerations.

## Database Architecture

### Schema Organization

- **public** - Web UI tables and application data
- **maproom** - Code indexing and search functionality (from Maproom service)

### Connection Configuration

```javascript
{
  host: process.env.CREWCHIEF_DB_HOST || 'localhost',
  port: process.env.CREWCHIEF_DB_PORT || 5432,
  database: process.env.CREWCHIEF_DB_NAME || 'crewchief',
  user: process.env.CREWCHIEF_DB_USER || 'postgres',
  password: process.env.CREWCHIEF_DB_PASSWORD || '',
  ssl: process.env.CREWCHIEF_DB_SSL === 'true',
  max: 20,  // Maximum pool size
  min: 5,   // Minimum pool size
}
```

## Core Tables

### 1. web_sessions
**Purpose**: User session management and authentication

```sql
CREATE TABLE web_sessions (
  id BIGSERIAL PRIMARY KEY,
  session_id UUID UNIQUE NOT NULL DEFAULT gen_random_uuid(),
  user_id TEXT,
  auth_token TEXT UNIQUE NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_accessed TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ip_address INET,
  user_agent TEXT,
  is_active BOOLEAN NOT NULL DEFAULT true,
  session_data JSONB DEFAULT '{}'
);
```

**Key Features**:
- UUID-based session identifiers
- JWT token storage for API authentication
- Flexible session data storage with JSONB
- Automatic cleanup of expired sessions
- IP address and user agent tracking

**Indexes**:
- `idx_web_sessions_token` - Fast token lookups
- `idx_web_sessions_expires` - Efficient expiry queries
- `idx_web_sessions_user_active` - User-specific active sessions
- `idx_web_sessions_active` - Partial index for active sessions only

### 2. web_search_history
**Purpose**: Search query tracking and results caching

```sql
CREATE TABLE web_search_history (
  id BIGSERIAL PRIMARY KEY,
  session_id UUID REFERENCES web_sessions(session_id),
  query TEXT NOT NULL,
  query_type search_type DEFAULT 'semantic',
  results JSONB,
  result_count INTEGER DEFAULT 0,
  execution_time_ms INTEGER,
  searched_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Key Features**:
- Links to user sessions for personalization
- Supports multiple search types (semantic, text, code)
- Caches search results in JSONB format
- Performance tracking with execution time
- Full-text search capabilities on queries

### 3. web_ui_preferences
**Purpose**: User interface customization and settings

```sql
CREATE TABLE web_ui_preferences (
  id BIGSERIAL PRIMARY KEY,
  session_id UUID REFERENCES web_sessions(session_id),
  preference_key web_preference_key NOT NULL,
  preference_value JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Supported Preferences**:
- `theme` - Dark/light mode settings
- `layout` - Dashboard layout configuration
- `search_filters` - Default search filters
- `notifications` - Notification preferences
- `accessibility` - Accessibility settings

### 4. system_config
**Purpose**: Application-wide configuration storage

```sql
CREATE TABLE system_config (
  id VARCHAR(100) PRIMARY KEY,
  category VARCHAR(100) NOT NULL,
  name VARCHAR(100) NOT NULL,
  value JSONB NOT NULL,
  description TEXT,
  is_sensitive BOOLEAN DEFAULT FALSE,
  encrypted BOOLEAN DEFAULT FALSE,
  schema VARCHAR(100),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Configuration Categories**:
- `database` - Database connection settings
- `auth` - Authentication configuration
- `integrations` - Third-party service settings
- `monitoring` - System monitoring configuration
- `features` - Feature flags and toggles

## Agent Management Tables

### 5. agent_runs
**Purpose**: Agent execution tracking and lifecycle management

```sql
CREATE TABLE agent_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id TEXT NOT NULL,
  agent_type agent_type NOT NULL,
  status agent_status NOT NULL DEFAULT 'pending',
  task_description TEXT,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  working_directory TEXT,
  log_path TEXT,
  config JSONB DEFAULT '{}',
  results JSONB,
  error_message TEXT,
  parent_run_id UUID REFERENCES agent_runs(id)
);
```

**Agent Types**:
- `claude` - Claude AI agent
- `cursor` - Cursor AI agent  
- `continue` - Continue AI agent
- `custom` - Custom agent implementations

**Agent Statuses**:
- `pending` - Queued for execution
- `running` - Currently executing
- `completed` - Successfully finished
- `failed` - Execution failed
- `cancelled` - Manually cancelled

### 6. agent_messages
**Purpose**: Inter-agent communication and message bus

```sql
CREATE TABLE agent_messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  run_id UUID REFERENCES agent_runs(id),
  sender_id TEXT NOT NULL,
  recipient_id TEXT,
  message_type message_type NOT NULL,
  priority message_priority DEFAULT 'normal',
  content JSONB NOT NULL,
  sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  delivered_at TIMESTAMPTZ,
  read_at TIMESTAMPTZ
);
```

**Message Types**:
- `task_assignment` - Task delegation
- `status_update` - Progress reports
- `result_sharing` - Sharing execution results
- `error_report` - Error notifications
- `coordination` - Agent coordination

### 7. worktree_status
**Purpose**: Git worktree status and file tracking

```sql
CREATE TABLE worktree_status (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  worktree_path TEXT NOT NULL,
  branch_name TEXT NOT NULL,
  commit_hash TEXT,
  state worktree_state NOT NULL DEFAULT 'clean',
  agent_id TEXT,
  last_scan TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  file_changes JSONB DEFAULT '[]'
);
```

**Worktree States**:
- `clean` - No uncommitted changes
- `dirty` - Has uncommitted changes
- `conflicted` - Merge conflicts present
- `syncing` - Synchronization in progress

## Authentication & Authorization

### 8. auth_users
**Purpose**: User account management

```sql
CREATE TABLE auth_users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  username VARCHAR(100) UNIQUE NOT NULL,
  email VARCHAR(255) UNIQUE NOT NULL,
  password_hash TEXT,
  full_name VARCHAR(255),
  is_active BOOLEAN DEFAULT TRUE,
  is_verified BOOLEAN DEFAULT FALSE,
  last_login TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### 9. auth_roles
**Purpose**: Role-based access control

```sql
CREATE TABLE auth_roles (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(100) UNIQUE NOT NULL,
  description TEXT,
  permissions JSONB DEFAULT '[]',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### 10. auth_oauth
**Purpose**: OAuth provider integration

```sql
CREATE TABLE auth_oauth (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES auth_users(id),
  provider VARCHAR(50) NOT NULL,
  provider_user_id VARCHAR(255) NOT NULL,
  access_token TEXT,
  refresh_token TEXT,
  expires_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Service Layer Tables

### 11. audit_log
**Purpose**: Operation tracking for security and debugging

```sql
CREATE TABLE audit_log (
  id SERIAL PRIMARY KEY,
  correlation_id VARCHAR(36) NOT NULL,
  service VARCHAR(100) NOT NULL,
  operation VARCHAR(100) NOT NULL,
  user_id VARCHAR(100),
  resource VARCHAR(255),
  action VARCHAR(100) NOT NULL,
  success BOOLEAN NOT NULL,
  error TEXT,
  metadata JSONB DEFAULT '{}',
  timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
  ip_address INET,
  user_agent TEXT
);
```

### 12. system_metrics
**Purpose**: System performance monitoring

```sql
CREATE TABLE system_metrics (
  id SERIAL PRIMARY KEY,
  timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
  cpu_usage DECIMAL(5,2) NOT NULL,
  cpu_cores INTEGER NOT NULL,
  load_average JSONB DEFAULT '[]',
  memory_total BIGINT NOT NULL,
  memory_used BIGINT NOT NULL,
  memory_free BIGINT NOT NULL,
  memory_usage DECIMAL(5,2) NOT NULL,
  disk_total BIGINT NOT NULL,
  disk_used BIGINT NOT NULL,
  disk_free BIGINT NOT NULL,
  disk_usage DECIMAL(5,2) NOT NULL,
  processes_total INTEGER DEFAULT 0,
  processes_running INTEGER DEFAULT 0
);
```

### 13. service_health
**Purpose**: Service status monitoring

```sql
CREATE TABLE service_health (
  id SERIAL PRIMARY KEY,
  service VARCHAR(100) NOT NULL,
  healthy BOOLEAN NOT NULL,
  last_check TIMESTAMP WITH TIME ZONE NOT NULL,
  response_time INTEGER NOT NULL,
  details JSONB DEFAULT '{}',
  error TEXT
);
```

### 14. system_alerts
**Purpose**: Alert management and notifications

```sql
CREATE TABLE system_alerts (
  id VARCHAR(100) PRIMARY KEY,
  type VARCHAR(20) NOT NULL CHECK (type IN ('info', 'warning', 'error', 'critical')),
  title VARCHAR(255) NOT NULL,
  message TEXT NOT NULL,
  source VARCHAR(100) NOT NULL,
  severity INTEGER NOT NULL CHECK (severity BETWEEN 1 AND 5),
  timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
  acknowledged BOOLEAN DEFAULT FALSE,
  acknowledged_by VARCHAR(100),
  acknowledged_at TIMESTAMP WITH TIME ZONE,
  resolved BOOLEAN DEFAULT FALSE,
  resolved_by VARCHAR(100),
  resolved_at TIMESTAMP WITH TIME ZONE,
  metadata JSONB DEFAULT '{}'
);
```

### 15. config_backups
**Purpose**: Configuration versioning and rollback

```sql
CREATE TABLE config_backups (
  id VARCHAR(100) PRIMARY KEY,
  config_id VARCHAR(100) NOT NULL,
  version VARCHAR(50) NOT NULL,
  data JSONB NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL,
  created_by VARCHAR(100),
  reason VARCHAR(100)
);
```

## Maproom Schema Integration

The Web UI integrates with the Maproom schema for code indexing and search functionality:

### Foreign Key Relationships

```sql
-- Example relationships to maproom schema
-- These would be implemented as needed for specific integrations

-- Search history could reference maproom chunks
ALTER TABLE web_search_history 
ADD COLUMN maproom_repo_id BIGINT REFERENCES maproom.repos(id);

-- Agent runs could be associated with specific worktrees
ALTER TABLE agent_runs 
ADD COLUMN maproom_worktree_id BIGINT REFERENCES maproom.worktrees(id);
```

### Maproom Schema Tables

1. **maproom.repos** - Repository metadata
2. **maproom.worktrees** - Git worktree tracking
3. **maproom.commits** - Commit information
4. **maproom.files** - File metadata and content hashes
5. **maproom.chunks** - Code chunks with embeddings
6. **maproom.chunk_edges** - Code relationship graph
7. **maproom.file_owners** - Code ownership tracking
8. **maproom.test_links** - Test-to-code relationships

## Index Strategy

### Performance Indexes

1. **Session Management**:
   - `idx_web_sessions_token` - Fast authentication lookups
   - `idx_web_sessions_expires` - Efficient session cleanup
   - `idx_web_sessions_active` - Active session queries

2. **Search History**:
   - `idx_web_search_history_session` - User search history
   - `idx_web_search_history_query_gin` - Full-text search on queries
   - `idx_web_search_history_searched_at` - Time-based queries

3. **Agent Management**:
   - `idx_agent_runs_status` - Status-based filtering
   - `idx_agent_runs_agent_id` - Agent-specific queries
   - `idx_agent_runs_started_at` - Time-based sorting

4. **Audit and Monitoring**:
   - `idx_audit_log_timestamp` - Time-based audit queries
   - `idx_audit_log_service_operation` - Service operation tracking
   - `idx_system_metrics_timestamp` - Metrics time series queries

### Composite Indexes

```sql
-- Multi-column indexes for complex queries
CREATE INDEX idx_agent_runs_status_type ON agent_runs(status, agent_type);
CREATE INDEX idx_web_sessions_user_active ON web_sessions(user_id, is_active) 
WHERE user_id IS NOT NULL;
CREATE INDEX idx_audit_log_service_success ON audit_log(service, success, timestamp);
```

### Partial Indexes

```sql
-- Indexes on filtered data sets
CREATE INDEX idx_web_sessions_active ON web_sessions(session_id, expires_at) 
WHERE is_active = true;

CREATE INDEX idx_agent_runs_running ON agent_runs(started_at) 
WHERE status = 'running';

CREATE INDEX idx_system_alerts_unresolved ON system_alerts(severity, timestamp) 
WHERE resolved = false;
```

## Performance Considerations

### Connection Pooling

- **Pool Size**: 5-20 connections based on load
- **Connection Timeout**: 5 seconds
- **Idle Timeout**: 30 seconds
- **Statement Timeout**: 30 seconds

### Query Optimization

1. **Prepared Statements**: Used for all parameterized queries
2. **Transaction Boundaries**: Clearly defined for data consistency
3. **Batch Operations**: For bulk inserts and updates
4. **Materialized Views**: For dashboard aggregations

### Data Retention

```sql
-- Automatic cleanup functions
CREATE OR REPLACE FUNCTION cleanup_old_service_data(retention_days INTEGER DEFAULT 30)
RETURNS void AS $$
BEGIN
  -- Clean up old audit logs
  DELETE FROM audit_log WHERE timestamp < NOW() - INTERVAL '1 day' * retention_days;
  
  -- Clean up old system metrics
  DELETE FROM system_metrics WHERE timestamp < NOW() - INTERVAL '1 day' * retention_days;
  
  -- Clean up expired sessions
  DELETE FROM web_sessions WHERE expires_at < NOW() - INTERVAL '7 days';
END;
$$ LANGUAGE plpgsql;
```

### Monitoring Queries

```sql
-- Connection pool monitoring
SELECT 
  count(*) as total_connections,
  count(*) FILTER (WHERE state = 'active') as active_connections,
  count(*) FILTER (WHERE state = 'idle') as idle_connections
FROM pg_stat_activity 
WHERE datname = current_database();

-- Table size monitoring
SELECT 
  schemaname,
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
FROM pg_tables 
WHERE schemaname IN ('public', 'maproom')
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Index usage monitoring
SELECT 
  schemaname,
  tablename,
  indexname,
  idx_tup_read,
  idx_tup_fetch,
  idx_tup_read / NULLIF(idx_tup_fetch, 0) as selectivity
FROM pg_stat_user_indexes
ORDER BY idx_tup_read DESC;
```

## Security Features

### Data Protection

1. **Password Hashing**: bcrypt for user passwords
2. **JWT Tokens**: Secure session management
3. **SQL Injection Prevention**: Parameterized queries only
4. **Data Encryption**: Sensitive config values encrypted at rest
5. **Audit Logging**: All operations tracked for compliance

### Access Control

1. **Role-Based Permissions**: Granular access control
2. **Session Validation**: Token expiry and validation
3. **IP Address Tracking**: Security monitoring
4. **Rate Limiting**: API protection (handled at application layer)

### Compliance

1. **Data Retention**: Configurable retention policies
2. **Audit Trail**: Complete operation tracking
3. **Privacy Controls**: User data anonymization capabilities
4. **Backup Strategy**: Point-in-time recovery with config versioning

## Migration Management

### Migration Files

All migrations are located in `/migrations/` directory:

1. `0001_web_sessions.sql` - Session management
2. `0002_web_search_history.sql` - Search tracking
3. `0003_web_ui_preferences.sql` - User preferences
4. `0004_agent_runs.sql` - Agent lifecycle
5. `0005_agent_messages.sql` - Inter-agent communication
6. `0006_worktree_status.sql` - Git worktree tracking
7. `0007_system_config.sql` - Configuration storage
8. `0008_auth_users.sql` - User management
9. `0009_auth_roles.sql` - Role-based access
10. `0010_auth_oauth.sql` - OAuth integration
11. `0011_service_layer_tables.sql` - Service monitoring

### Migration Tracking

```sql
CREATE TABLE web_migrations (
  id TEXT PRIMARY KEY,
  filename TEXT NOT NULL,
  checksum TEXT NOT NULL,
  executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  execution_time_ms INTEGER NOT NULL,
  success BOOLEAN NOT NULL DEFAULT true,
  error_message TEXT,
  UNIQUE(filename)
);
```

### Safe Migration Practices

1. **Checksums**: Prevent modification of executed migrations
2. **Transactions**: All migrations run in transactions where possible
3. **Rollback Scripts**: DOWN migrations for all schema changes
4. **Testing**: Migrations tested against production-like data
5. **Concurrent Safety**: Uses `CREATE INDEX CONCURRENTLY` for large tables

## Troubleshooting

### Common Issues

1. **Connection Pool Exhaustion**:
   ```sql
   SELECT count(*), state FROM pg_stat_activity GROUP BY state;
   ```

2. **Slow Queries**:
   ```sql
   SELECT query, calls, total_time, mean_time 
   FROM pg_stat_statements 
   ORDER BY total_time DESC LIMIT 10;
   ```

3. **Lock Contention**:
   ```sql
   SELECT * FROM pg_locks WHERE NOT granted;
   ```

4. **Index Usage**:
   ```sql
   SELECT * FROM pg_stat_user_indexes WHERE idx_scan = 0;
   ```

### Performance Tuning

1. **Vacuum Strategy**: Regular VACUUM and ANALYZE
2. **Connection Limits**: Monitor and adjust pool sizes
3. **Query Plans**: Regular EXPLAIN ANALYZE for critical queries
4. **Statistics**: Keep table statistics up to date

This schema is designed for scalability, performance, and maintainability while providing comprehensive functionality for the CrewChief Web UI.