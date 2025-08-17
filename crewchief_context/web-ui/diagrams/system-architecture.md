# System Architecture Diagrams

## Overall System Architecture

```mermaid
graph TB
    subgraph "Client Layer"
        Browser[Web Browser]
        CLI[CrewChief CLI]
    end
    
    subgraph "Web Server"
        Express[Express Server]
        WS[WebSocket Server]
        GraphQL[GraphQL Endpoint]
        Static[Static File Server]
    end
    
    subgraph "Service Layer"
        MaproomSvc[Maproom Service]
        WorktreeSvc[Worktree Service]
        AgentSvc[Agent Service]
        ConfigSvc[Config Service]
        MonitorSvc[Monitoring Service]
    end
    
    subgraph "External Systems"
        MaproomBin[Maproom Binary]
        Git[Git Repository]
        Postgres[(PostgreSQL)]
        Tmux[Tmux Sessions]
        FS[File System]
    end
    
    Browser --> Express
    Browser <--> WS
    Browser --> GraphQL
    Browser --> Static
    
    CLI --> Express
    
    Express --> MaproomSvc
    Express --> WorktreeSvc
    Express --> AgentSvc
    Express --> ConfigSvc
    Express --> MonitorSvc
    
    WS --> MaproomSvc
    WS --> AgentSvc
    WS --> MonitorSvc
    
    MaproomSvc --> MaproomBin
    MaproomSvc --> Postgres
    
    WorktreeSvc --> Git
    WorktreeSvc --> FS
    
    AgentSvc --> Tmux
    AgentSvc --> FS
    
    ConfigSvc --> FS
    
    MonitorSvc --> Tmux
    MonitorSvc --> Postgres
    MonitorSvc --> FS
```

## Frontend Architecture

```mermaid
graph TB
    subgraph "React Application"
        App[App Component]
        Router[React Router]
        
        subgraph "Pages"
            Dashboard[Dashboard]
            Search[Search Page]
            Worktrees[Worktrees Page]
            Agents[Agents Page]
            Settings[Settings Page]
        end
        
        subgraph "State Management"
            Zustand[Zustand Store]
            ReactQuery[React Query]
            WSContext[WebSocket Context]
        end
        
        subgraph "UI Components"
            Layout[Layout Components]
            DataDisplay[Data Display]
            Forms[Form Components]
            Feedback[Feedback Components]
        end
    end
    
    subgraph "External Libraries"
        Monaco[Monaco Editor]
        D3[D3.js]
        Tailwind[TailwindCSS]
        ShadcnUI[Shadcn/ui]
    end
    
    App --> Router
    Router --> Dashboard
    Router --> Search
    Router --> Worktrees
    Router --> Agents
    Router --> Settings
    
    Dashboard --> Zustand
    Search --> ReactQuery
    Agents --> WSContext
    
    Dashboard --> Layout
    Search --> DataDisplay
    Settings --> Forms
    Agents --> Feedback
    
    DataDisplay --> Monaco
    Dashboard --> D3
    Layout --> Tailwind
    Forms --> ShadcnUI
```

## Backend Service Architecture

```mermaid
graph LR
    subgraph "API Gateway"
        REST[REST Router]
        GQL[GraphQL Schema]
        WS[WebSocket Handler]
    end
    
    subgraph "Business Logic"
        Auth[Auth Middleware]
        Validator[Validation Layer]
        Cache[Cache Layer]
    end
    
    subgraph "Services"
        MaproomService[Maproom Service]
        WorktreeService[Worktree Service]
        AgentService[Agent Service]
        ConfigService[Config Service]
    end
    
    subgraph "Data Access"
        PGClient[PostgreSQL Client]
        FSAdapter[File System Adapter]
        GitAdapter[Git Adapter]
        ProcessManager[Process Manager]
    end
    
    REST --> Auth
    GQL --> Auth
    WS --> Auth
    
    Auth --> Validator
    Validator --> Cache
    
    Cache --> MaproomService
    Cache --> WorktreeService
    Cache --> AgentService
    Cache --> ConfigService
    
    MaproomService --> PGClient
    MaproomService --> ProcessManager
    
    WorktreeService --> GitAdapter
    WorktreeService --> FSAdapter
    
    AgentService --> ProcessManager
    AgentService --> FSAdapter
    
    ConfigService --> FSAdapter
```

## Data Flow Architecture

```mermaid
sequenceDiagram
    participant User
    participant Browser
    participant WebServer
    participant Service
    participant Database
    participant FileSystem
    
    User->>Browser: Initiate Action
    Browser->>WebServer: HTTP Request
    WebServer->>Service: Process Request
    
    alt Query Operation
        Service->>Database: Query Data
        Database-->>Service: Return Results
    else File Operation
        Service->>FileSystem: Read/Write Files
        FileSystem-->>Service: File Data
    end
    
    Service-->>WebServer: Response Data
    WebServer-->>Browser: JSON Response
    Browser-->>User: Update UI
    
    Note over WebServer,Browser: WebSocket Updates
    Service->>WebServer: Push Update
    WebServer->>Browser: WebSocket Message
    Browser->>Browser: Update State
    Browser-->>User: Real-time Update
```

## Component Hierarchy

```mermaid
graph TD
    App[App Root]
    App --> Header[Header Bar]
    App --> Sidebar[Sidebar Nav]
    App --> Main[Main Content]
    App --> Footer[Status Footer]
    
    Header --> GlobalSearch[Global Search]
    Header --> UserMenu[User Menu]
    Header --> Notifications[Notifications]
    
    Sidebar --> NavMenu[Navigation Menu]
    Sidebar --> QuickActions[Quick Actions]
    
    Main --> Router[Route Container]
    Router --> Dashboard[Dashboard View]
    Router --> SearchView[Search View]
    Router --> WorktreeView[Worktree View]
    Router --> AgentView[Agent View]
    
    Dashboard --> StatsGrid[Stats Grid]
    Dashboard --> ActivityFeed[Activity Feed]
    Dashboard --> AgentStatus[Agent Status]
    
    SearchView --> SearchBar[Search Bar]
    SearchView --> FilterPanel[Filter Panel]
    SearchView --> ResultsList[Results List]
    SearchView --> CodePreview[Code Preview]
    
    WorktreeView --> WorktreeList[Worktree List]
    WorktreeView --> FileExplorer[File Explorer]
    WorktreeView --> GitStatus[Git Status]
    
    AgentView --> AgentGrid[Agent Grid]
    AgentView --> MessageCenter[Message Center]
    AgentView --> LogViewer[Log Viewer]
```

## WebSocket Event Flow

```mermaid
graph LR
    subgraph "Server Events"
        IndexProgress[Index Progress]
        AgentStatus[Agent Status]
        LogStream[Log Stream]
        FileChange[File Change]
        GitEvent[Git Event]
    end
    
    subgraph "WebSocket Server"
        EventEmitter[Event Emitter]
        RoomManager[Room Manager]
        ClientManager[Client Manager]
    end
    
    subgraph "Client Handlers"
        WSProvider[WS Provider]
        EventDispatcher[Event Dispatcher]
        StateUpdater[State Updater]
    end
    
    subgraph "UI Updates"
        ProgressBar[Progress Bar]
        StatusBadge[Status Badge]
        LogDisplay[Log Display]
        FileTree[File Tree]
        BranchInfo[Branch Info]
    end
    
    IndexProgress --> EventEmitter
    AgentStatus --> EventEmitter
    LogStream --> EventEmitter
    FileChange --> EventEmitter
    GitEvent --> EventEmitter
    
    EventEmitter --> RoomManager
    RoomManager --> ClientManager
    
    ClientManager --> WSProvider
    WSProvider --> EventDispatcher
    EventDispatcher --> StateUpdater
    
    StateUpdater --> ProgressBar
    StateUpdater --> StatusBadge
    StateUpdater --> LogDisplay
    StateUpdater --> FileTree
    StateUpdater --> BranchInfo
```

## Database Schema

```mermaid
erDiagram
    repositories {
        uuid id PK
        string name
        string path
        string current_branch
        string remote_url
        timestamp last_fetch
        jsonb config
    }
    
    worktrees {
        uuid id PK
        uuid repository_id FK
        string name
        string path
        string branch
        string status
        timestamp created_at
        timestamp last_modified
    }
    
    maproom_chunks {
        uuid id PK
        uuid repository_id FK
        uuid worktree_id FK
        string file_path
        int start_line
        int end_line
        text content
        tsvector ts_doc
        jsonb metadata
        timestamp indexed_at
    }
    
    search_queries {
        uuid id PK
        string query
        jsonb filters
        jsonb results
        float execution_time
        int result_count
        timestamp created_at
        uuid user_id FK
    }
    
    agents {
        uuid id PK
        uuid worktree_id FK
        string type
        string status
        jsonb task
        jsonb resources
        timestamp created_at
        timestamp updated_at
    }
    
    runs {
        uuid id PK
        uuid agent_id FK
        uuid worktree_id FK
        string status
        string task
        timestamp start_time
        timestamp end_time
        jsonb evaluation
        jsonb artifacts
    }
    
    run_events {
        uuid id PK
        uuid run_id FK
        string event_type
        jsonb data
        timestamp created_at
    }
    
    repositories ||--o{ worktrees : has
    repositories ||--o{ maproom_chunks : indexes
    worktrees ||--o{ agents : hosts
    worktrees ||--o{ runs : executes
    agents ||--o{ runs : performs
    runs ||--o{ run_events : contains
```

## Deployment Architecture

```mermaid
graph TB
    subgraph "Development"
        DevServer[Node.js Dev Server]
        DevDB[(Local PostgreSQL)]
        DevFS[Local File System]
    end
    
    subgraph "Production"
        subgraph "Container"
            WebApp[Web Application]
            APIServer[API Server]
            WSServer[WebSocket Server]
        end
        
        subgraph "Services"
            ProdDB[(PostgreSQL)]
            Redis[(Redis Cache)]
            S3[S3 Storage]
        end
        
        subgraph "Infrastructure"
            LoadBalancer[Load Balancer]
            CDN[CloudFront CDN]
            Monitoring[CloudWatch]
        end
    end
    
    subgraph "CI/CD"
        GitHub[GitHub]
        Actions[GitHub Actions]
        Docker[Docker Registry]
    end
    
    DevServer --> GitHub
    GitHub --> Actions
    Actions --> Docker
    Docker --> Container
    
    LoadBalancer --> WebApp
    LoadBalancer --> APIServer
    LoadBalancer --> WSServer
    
    WebApp --> CDN
    APIServer --> ProdDB
    APIServer --> Redis
    WSServer --> Redis
    
    Container --> S3
    Container --> Monitoring
```

## Security Architecture

```mermaid
graph TB
    subgraph "Client Security"
        HTTPS[HTTPS/TLS]
        CSP[Content Security Policy]
        CORS[CORS Headers]
    end
    
    subgraph "Authentication"
        AuthProvider[Auth Provider]
        JWT[JWT Tokens]
        Session[Session Management]
    end
    
    subgraph "Authorization"
        RBAC[Role-Based Access]
        Permissions[Permission System]
        RateLimiting[Rate Limiting]
    end
    
    subgraph "Data Protection"
        Encryption[Encryption at Rest]
        Sanitization[Input Sanitization]
        Validation[Schema Validation]
    end
    
    subgraph "Monitoring"
        AuditLog[Audit Logging]
        Alerts[Security Alerts]
        Analytics[Usage Analytics]
    end
    
    HTTPS --> AuthProvider
    AuthProvider --> JWT
    JWT --> Session
    
    Session --> RBAC
    RBAC --> Permissions
    Permissions --> RateLimiting
    
    RateLimiting --> Validation
    Validation --> Sanitization
    Sanitization --> Encryption
    
    Encryption --> AuditLog
    AuditLog --> Alerts
    Alerts --> Analytics
```