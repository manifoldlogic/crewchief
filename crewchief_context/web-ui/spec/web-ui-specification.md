# CrewChief Web UI Specification

## Overview

The CrewChief Web UI is a browser-based control center for managing git worktrees, Maproom code indexing, AI agents, and development workflows. Launched via `crewchief web`, it provides a visual interface to all CrewChief CLI capabilities while adding real-time monitoring and collaborative features.

## Command Specification

### `crewchief web` Command

```bash
crewchief web [options]

Options:
  -p, --port <port>      Port to run the web server on (default: 3456)
  -h, --host <host>      Host to bind to (default: localhost)
  --open                  Open browser automatically after starting
  --config <path>         Path to crewchief.config.js (default: ./crewchief.config.js)
  --readonly              Start in read-only mode (no modifications allowed)
  --auth <type>           Authentication type: none|basic|oauth (default: none)
  --ssl                   Enable HTTPS with self-signed certificate
  --help                  Display help for command
```

### Example Usage

```bash
# Start with defaults (http://localhost:3456)
crewchief web

# Start and open browser
crewchief web --open

# Custom port with HTTPS
crewchief web --port 8080 --ssl

# Read-only mode for monitoring
crewchief web --readonly --auth basic
```

## Core Features

### 1. Maproom Management

**Visual Code Search Interface**
- Semantic search with instant results
- Syntax-highlighted code preview
- Search history and saved queries
- Filter by file type, worktree, or date range
- Export search results

**Index Management**
- Real-time indexing status dashboard
- Statistics: files indexed, chunks, languages
- Manual re-indexing triggers
- Index health monitoring
- Scheduled indexing configuration

**Search Analytics**
- Most searched terms
- Query performance metrics
- Result relevance tracking
- Search pattern insights

### 2. Worktree Management

**Worktree Dashboard**
- Visual tree of all worktrees
- Status indicators (active, stale, merging)
- One-click creation with branch selection
- Drag-and-drop file copying between worktrees
- Bulk operations (clean stale, archive old)

**Worktree Inspector**
- File browser with diff highlighting
- Git status integration
- Recent commits view
- Branch comparison tools
- Direct file editing (Monaco editor)

### 3. Agent Orchestration

**Agent Control Panel**
- Spawn agents with visual configuration
- Real-time agent status (idle, working, blocked)
- Message passing interface
- Resource usage monitoring (CPU, memory)
- Agent capability browser

**Agent Collaboration View**
- Visual representation of agent communication
- Message bus inspector
- Task assignment UI
- Competition mode setup
- Performance comparisons

**Run History**
- Searchable run archive
- Log viewer with filtering
- Event timeline visualization
- Performance metrics
- Export capabilities

### 4. Branch Management

**Branch Visualizer**
- Interactive branch graph (like GitKraken)
- Merge conflict preview
- Cherry-pick interface
- Branch protection rules
- PR creation shortcuts

**Merge Assistant**
- Guided merge workflows
- Conflict resolution tools
- Auto-merge configuration
- Quality gate visualization
- Rollback capabilities

### 5. Settings Management

**Configuration Editor**
- Visual editor for crewchief.config.js
- Schema validation with autocomplete
- Environment variable management
- Profile switching (dev, staging, prod)
- Config diff and history

**System Monitoring**
- Database connection status
- Tmux session health
- Agent platform availability
- Resource usage graphs
- Error log aggregation

## Technical Architecture

### Backend (Node.js/Express)

**API Server**
- RESTful API for CRUD operations
- WebSocket server for real-time updates
- GraphQL endpoint for complex queries
- File system operations via Node.js fs
- Git operations via simple-git
- PostgreSQL queries via pg driver

**Services**
- MaproomService: Interfaces with Maproom binary
- WorktreeService: Git worktree management
- AgentService: Agent lifecycle management
- ConfigService: Configuration CRUD
- MonitoringService: System health checks

### Frontend (React/TypeScript)

**UI Framework**
- React 18+ with TypeScript
- Vite for build tooling
- TailwindCSS for styling
- Shadcn/ui components
- React Query for data fetching
- Zustand for state management

**Key Components**
- SearchInterface: Maproom search UI
- WorktreeExplorer: File browser component
- AgentDashboard: Agent management
- BranchGraph: D3.js branch visualization
- ConfigEditor: Monaco-based editor
- LogViewer: Virtual scrolling log display

### Data Flow

**Real-time Updates via WebSocket**
- Agent status changes
- Indexing progress
- Log streaming
- File system events
- Git repository changes

**REST API Endpoints**
```
GET    /api/maproom/search
POST   /api/maproom/index
GET    /api/worktrees
POST   /api/worktrees
DELETE /api/worktrees/:id
GET    /api/agents
POST   /api/agents/spawn
POST   /api/agents/:id/message
GET    /api/config
PUT    /api/config
GET    /api/runs
GET    /api/runs/:id/logs
```

## User Experience Principles

### Progressive Disclosure
- Simple dashboard for common tasks
- Advanced features behind expandable panels
- Keyboard shortcuts for power users
- Customizable workspace layouts

### Real-time Feedback
- Live progress indicators
- Instant search results
- Push notifications for long operations
- Status badges and health indicators

### Error Prevention
- Confirmation dialogs for destructive actions
- Validation before operations
- Undo/redo capabilities
- Dry-run previews

### Accessibility
- WCAG 2.1 AA compliance
- Keyboard navigation throughout
- Screen reader support
- High contrast mode
- Responsive design (mobile → desktop)

## Security Considerations

### Authentication Options
1. **None** (default): Local development
2. **Basic Auth**: Simple username/password
3. **OAuth**: GitHub/GitLab integration
4. **API Keys**: For programmatic access

### Authorization
- Read-only mode for monitoring
- Role-based access (viewer, operator, admin)
- Operation-level permissions
- Audit logging

### Data Protection
- HTTPS support with SSL
- Input sanitization
- SQL injection prevention
- XSS protection
- CORS configuration

## Implementation Phases

### Phase 1: Foundation (MVP)
- Basic web server with static UI
- Maproom search interface
- Worktree list and creation
- Simple configuration viewer
- WebSocket for logs

### Phase 2: Agent Features
- Agent spawning UI
- Message passing interface
- Run history browser
- Basic monitoring dashboard

### Phase 3: Advanced Tools
- Branch visualization
- Merge assistant
- Monaco editor integration
- Advanced search filters
- Batch operations

### Phase 4: Polish
- Authentication system
- Customizable dashboard
- Export/import capabilities
- Plugin architecture
- Performance optimizations

## Success Metrics

### Performance
- Search results < 100ms
- Page load < 2s
- WebSocket latency < 50ms
- 60fps UI animations

### Usability
- 80% of operations achievable via UI
- < 3 clicks for common tasks
- Keyboard shortcuts for all actions
- Mobile-responsive design

### Reliability
- 99.9% uptime
- Graceful degradation
- Offline capabilities
- Auto-save and recovery

## Future Enhancements

### Collaboration Features
- Multi-user support
- Shared search sessions
- Agent handoff workflows
- Team dashboards

### AI Integration
- Natural language commands
- Intelligent suggestions
- Automated workflows
- Performance predictions

### Extensibility
- Plugin API
- Custom widgets
- Theme marketplace
- Webhook integrations