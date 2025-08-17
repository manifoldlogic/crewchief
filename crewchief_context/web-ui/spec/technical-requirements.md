# Technical Requirements

## Technology Stack

### Backend

**Runtime & Framework**
- Node.js 20+ (LTS)
- Express.js 4.x
- TypeScript 5.x
- ESM modules

**API Technologies**
- REST API (Express Router)
- GraphQL (Apollo Server)
- WebSocket (Socket.io or ws)
- JSON-RPC for MCP compatibility

**Database & Storage**
- PostgreSQL 14+ (existing Maproom DB)
- Redis for caching/sessions
- File system access via fs/promises

**Process Management**
- Child process spawning for agents
- Tmux integration via child_process
- Git operations via simple-git

### Frontend

**Core Framework**
- React 18+
- TypeScript 5.x
- Vite 5.x for bundling

**UI Libraries**
- TailwindCSS 3.x
- Shadcn/ui components
- Radix UI primitives
- Monaco Editor for code editing
- D3.js for visualizations

**State Management**
- Zustand for global state
- React Query for server state
- React Hook Form for forms

**Development Tools**
- ESLint + Prettier
- Vitest for testing
- Playwright for E2E tests
- Storybook for components

### Infrastructure

**Development**
- Docker Compose for services
- Hot module replacement
- TypeScript watch mode
- Concurrent dev servers

**Production**
- Docker containerization
- Nginx reverse proxy
- PM2 process manager
- Let's Encrypt SSL

## System Requirements

### Server Requirements

**Minimum**
- 2 CPU cores
- 4GB RAM
- 10GB disk space
- Ubuntu 20.04+ / macOS 12+

**Recommended**
- 4+ CPU cores
- 8GB+ RAM
- 50GB+ SSD storage
- Dedicated PostgreSQL instance

### Client Requirements

**Browser Support**
- Chrome 100+
- Firefox 100+
- Safari 15+
- Edge 100+

**Screen Resolution**
- Minimum: 1280x720
- Optimal: 1920x1080+
- Mobile responsive: 375px+

## API Specifications

### REST Endpoints

```typescript
// Worktree Management
GET    /api/worktrees
POST   /api/worktrees
GET    /api/worktrees/:id
PUT    /api/worktrees/:id
DELETE /api/worktrees/:id
POST   /api/worktrees/:id/clean
POST   /api/worktrees/:id/copy-ignored

// Maproom Search
GET    /api/maproom/search?q=:query&worktree=:id&limit=:n
POST   /api/maproom/index
GET    /api/maproom/status
DELETE /api/maproom/cache

// Agent Management
GET    /api/agents
POST   /api/agents/spawn
GET    /api/agents/:id
POST   /api/agents/:id/message
POST   /api/agents/:id/close
GET    /api/agents/:id/logs

// Run Management
GET    /api/runs
GET    /api/runs/:id
GET    /api/runs/:id/events
GET    /api/runs/:id/logs?tail=:n

// Configuration
GET    /api/config
PUT    /api/config
POST   /api/config/validate
GET    /api/config/schema

// System
GET    /api/health
GET    /api/metrics
GET    /api/version
```

### GraphQL Schema

```graphql
type Query {
  # Repository queries
  repository(id: ID!): Repository
  repositories: [Repository!]!
  
  # Worktree queries
  worktree(id: ID!): Worktree
  worktrees(repositoryId: ID): [Worktree!]!
  
  # Search queries
  search(query: String!, worktree: ID, limit: Int): SearchResults!
  searchHistory(limit: Int): [SearchQuery!]!
  
  # Agent queries
  agent(id: ID!): Agent
  agents(status: AgentStatus): [Agent!]!
  
  # Run queries
  run(id: ID!): Run
  runs(agentId: ID, limit: Int): [Run!]!
}

type Mutation {
  # Worktree mutations
  createWorktree(input: CreateWorktreeInput!): Worktree!
  deleteWorktree(id: ID!): Boolean!
  
  # Agent mutations
  spawnAgent(input: SpawnAgentInput!): Agent!
  sendAgentMessage(id: ID!, message: String!): Boolean!
  closeAgent(id: ID!): Boolean!
  
  # Config mutations
  updateConfig(input: ConfigInput!): Config!
}

type Subscription {
  # Real-time updates
  indexProgress: IndexProgress!
  agentStatus(id: ID!): AgentStatus!
  logStream(runId: ID!): LogEntry!
  fileChanged(worktreeId: ID!): FileChange!
}
```

### WebSocket Events

```typescript
// Server -> Client Events
interface ServerEvents {
  'index:start': { worktreeId: string }
  'index:progress': { worktreeId: string; progress: number }
  'index:complete': { worktreeId: string; stats: IndexStats }
  'index:error': { worktreeId: string; error: string }
  
  'agent:spawn': { agent: Agent }
  'agent:status': { agentId: string; status: AgentStatus }
  'agent:message': { agentId: string; message: Message }
  'agent:close': { agentId: string }
  
  'worktree:create': { worktree: Worktree }
  'worktree:update': { worktree: Worktree }
  'worktree:delete': { worktreeId: string }
  
  'git:commit': { worktreeId: string; commit: Commit }
  'git:push': { worktreeId: string; branch: string }
  'git:pull': { worktreeId: string; branch: string }
  
  'log:entry': { runId: string; entry: LogEntry }
  'system:health': { status: SystemHealth }
}

// Client -> Server Events
interface ClientEvents {
  'subscribe': { channels: string[] }
  'unsubscribe': { channels: string[] }
  'ping': {}
}
```

## Security Requirements

### Authentication

**Methods**
- None (local development)
- Basic Authentication
- OAuth 2.0 (GitHub, GitLab)
- API Key authentication
- JWT tokens

**Session Management**
- Secure session cookies
- Token expiration (configurable)
- Refresh token rotation
- Remember me functionality

### Authorization

**Access Control**
- Role-based (viewer, operator, admin)
- Resource-based permissions
- Operation-level control
- API rate limiting

**Security Headers**
```typescript
{
  'Content-Security-Policy': "default-src 'self'",
  'X-Frame-Options': 'DENY',
  'X-Content-Type-Options': 'nosniff',
  'X-XSS-Protection': '1; mode=block',
  'Strict-Transport-Security': 'max-age=31536000'
}
```

### Data Protection

**Encryption**
- HTTPS/TLS 1.3 in production
- Encrypted database connections
- Secure WebSocket (wss://)
- Environment variable encryption

**Input Validation**
- Zod schema validation
- SQL injection prevention
- XSS sanitization
- Path traversal protection

## Performance Requirements

### Response Times

**Target Metrics**
- API response: < 200ms (p95)
- Search results: < 100ms
- Page load: < 2s
- WebSocket latency: < 50ms

### Scalability

**Concurrent Users**
- Support 100+ concurrent users
- 1000+ WebSocket connections
- 10,000+ req/sec API throughput

**Resource Limits**
- Max file upload: 100MB
- Max search results: 1000
- Max log tail: 10,000 lines
- WebSocket message: 1MB

### Caching Strategy

**Cache Layers**
1. Browser cache (static assets)
2. CDN cache (if deployed)
3. Redis cache (API responses)
4. PostgreSQL query cache
5. Application memory cache

**Cache Policies**
```typescript
{
  searchResults: '5 minutes',
  worktreeList: '30 seconds',
  agentStatus: 'no-cache',
  config: '1 minute',
  staticAssets: '1 year'
}
```

## Monitoring Requirements

### Logging

**Log Levels**
- ERROR: System errors, crashes
- WARN: Degraded performance
- INFO: Normal operations
- DEBUG: Detailed debugging
- TRACE: Full execution trace

**Log Outputs**
- Console (development)
- File rotation (production)
- Centralized logging (optional)
- Structured JSON format

### Metrics

**Application Metrics**
- Request rate
- Response time
- Error rate
- Active users
- WebSocket connections

**System Metrics**
- CPU usage
- Memory usage
- Disk I/O
- Network traffic
- Database connections

### Alerting

**Alert Conditions**
- Service down
- High error rate (> 1%)
- Slow response (> 1s)
- Resource exhaustion
- Security violations

## Development Requirements

### Local Development

**Setup Script**
```bash
#!/bin/bash
# Install dependencies
pnpm install

# Setup database
createdb crewchief_dev
pnpm run db:migrate

# Generate types
pnpm run codegen

# Start dev servers
pnpm run dev
```

### Testing Requirements

**Test Coverage**
- Unit tests: 80%+ coverage
- Integration tests: Critical paths
- E2E tests: User workflows
- Performance tests: Load testing

**Test Frameworks**
```json
{
  "unit": "vitest",
  "integration": "supertest",
  "e2e": "playwright",
  "performance": "k6"
}
```

### CI/CD Pipeline

**Build Steps**
1. Lint and format check
2. Type checking
3. Unit tests
4. Integration tests
5. Build artifacts
6. Docker image
7. E2E tests
8. Deploy to staging
9. Smoke tests
10. Deploy to production

## Deployment Requirements

### Docker Support

**Dockerfile**
```dockerfile
FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --production
COPY . .
EXPOSE 3456
CMD ["node", "dist/server.js"]
```

**Docker Compose**
```yaml
version: '3.8'
services:
  web:
    build: .
    ports:
      - "3456:3456"
    environment:
      - DATABASE_URL=postgres://...
    depends_on:
      - postgres
      - redis
  
  postgres:
    image: postgres:14
    volumes:
      - pgdata:/var/lib/postgresql/data
  
  redis:
    image: redis:7-alpine
```

### Environment Configuration

**Required Variables**
```env
NODE_ENV=production
PORT=3456
DATABASE_URL=postgres://...
REDIS_URL=redis://...
SESSION_SECRET=...
CREWCHIEF_MAPROOM_BIN=/usr/local/bin/crewchief-maproom
```

**Optional Variables**
```env
AUTH_PROVIDER=github
GITHUB_CLIENT_ID=...
GITHUB_CLIENT_SECRET=...
LOG_LEVEL=info
CORS_ORIGIN=https://example.com
SSL_CERT_PATH=/etc/ssl/cert.pem
SSL_KEY_PATH=/etc/ssl/key.pem
```