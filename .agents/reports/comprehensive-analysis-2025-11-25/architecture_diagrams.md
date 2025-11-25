# System Architecture Diagrams

**Date**: November 25, 2025
**Project**: CrewChief

## 1. System Context Diagram

This diagram shows how CrewChief fits into the user's environment.

```mermaid
flowchart TB
    User[Developer]
    subgraph Workstation
        IDE["IDE / Editor\n(Cursor, VSCode)"]
        Term["Terminal\n(iTerm2)"]
        
        subgraph CrewChief_System
            CLI[CrewChief CLI]
            MCP[Maproom MCP Server]
            Daemon["Maproom Daemon\n(Rust)"]
        end
        
        DB[("PostgreSQL\npgvector")]
    end
    
    External["LLM Provider\n(OpenAI/Anthropic)"]

    User -->|Orchestrates| CLI
    User -->|Codes in| IDE
    IDE -->|Uses| MCP
    CLI -->|Manages| Term
    MCP -->|Queries| Daemon
    CLI -->|Spawns| Daemon
    Daemon -->|Reads/Indexes| IDE
    Daemon -->|Stores/Searches| DB
    Daemon -->|Embeds| External
```

## 2. Container Diagram (Detailed Architecture)

This diagram details the internal components and their communication protocols.

```mermaid
flowchart TB
    subgraph "Frontend / Interface Layer"
        CLI_PKG["packages/cli\n(TypeScript)"]
        MCP_PKG["packages/maproom-mcp\n(TypeScript)"]
        VSCODE["packages/vscode-maproom\n(Extension)"]
    end

    subgraph "Client Layer"
        D_CLIENT["packages/daemon-client\n(TypeScript JSON-RPC Client)"]
    end

    subgraph "Engine Layer"
        RUST_BIN["crates/maproom\n(Rust Binary)"]
        
        subgraph "Rust Components"
            TreeSitter[Tree Sitter Parser]
            SearchEng[Search Engine]
            Embedder[Embedding Service]
            BranchWatcher[Branch Watcher]
        end
    end

    subgraph "Infrastructure"
        PG[("PostgreSQL")]
    end

    CLI_PKG -->|Uses| D_CLIENT
    MCP_PKG -->|Uses| D_CLIENT
    VSCODE -->|Uses| D_CLIENT

    D_CLIENT -->|JSON-RPC via Stdio| RUST_BIN
    
    RUST_BIN -->|Parses| TreeSitter
    RUST_BIN -->|Vectors| Embedder
    RUST_BIN -->|Watches| BranchWatcher
    RUST_BIN -->|SQL/pgvector| PG

    SearchEng -->|Queries| PG
```

## 3. Sequence Diagram: Semantic Search Flow

Tracing a search request from the User/Agent to the Database.

```mermaid
sequenceDiagram
    participant User as Agent/User
    participant MCP as Maproom MCP
    participant Client as Daemon Client
    participant Rust as Rust Daemon
    participant DB as PostgreSQL

    User->>MCP: search(query: "auth flow")
    MCP->>Client: search({ query: "auth flow" })
    
    Note over Client, Rust: JSON-RPC Request
    Client->>Rust: method: "search", params: {...}
    
    activate Rust
    Rust->>Rust: Generate Embeddings (if hybrid/vector)
    Rust->>DB: SQL Query (FTS + Vector Similarity)
    DB-->>Rust: Ranked Results
    Rust->>Rust: Re-rank / Format
    Rust-->>Client: JSON-RPC Response
    deactivate Rust
    
    Client-->>MCP: Result Object
    MCP-->>User: Tool Result (Formatted Text)
```

## 4. Agent Workflow Diagram

How the `.agents` system works.

```mermaid
stateDiagram-v2
    [*] --> Planning
    
    state Planning {
        CreateProject --> Analysis
        Analysis --> Architecture
        Architecture --> Plan
        Plan --> Tickets
    }

    state Execution {
        PickTicket --> Implementation
        Implementation --> Tests
        Tests --> Verification
        Verification --> Commit
    }

    Tickets --> PickTicket: Agent picks next ticket
    Commit --> PickTicket: Loop until done
    Commit --> Archive: All tickets done
    
    state Review {
        HumanReview
        AIReview
    }
    
    Tickets --> AIReview
    AIReview --> PickTicket
```

