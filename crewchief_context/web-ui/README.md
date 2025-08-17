# CrewChief Web UI Documentation

This folder contains comprehensive documentation for the CrewChief Web UI feature, accessible via the `crewchief web` command.

## Structure

- **[spec/](spec/)** - Technical specifications and requirements
  - [web-ui-specification.md](spec/web-ui-specification.md) - Complete feature specification
  - [technical-requirements.md](spec/technical-requirements.md) - Technical stack and requirements
  
- **[architecture/](architecture/)** - Information architecture and system design
  - [information-architecture.md](architecture/information-architecture.md) - Data models and relationships
  - [api-design.md](architecture/api-design.md) - REST/WebSocket API specifications
  
- **[ux/](ux/)** - User experience design and wireframes
  - [ux-plan.md](ux/ux-plan.md) - UX strategy and design principles
  - [user-flows.md](ux/user-flows.md) - Key user journeys
  - [wireframes.md](ux/wireframes.md) - Interface mockups and layouts
  
- **[diagrams/](diagrams/)** - Visual documentation
  - [system-architecture.md](diagrams/system-architecture.md) - System architecture diagrams
  - [data-flow.md](diagrams/data-flow.md) - Data flow and state management
  - [component-hierarchy.md](diagrams/component-hierarchy.md) - UI component structure

## Quick Start

To understand the web UI system:

1. Start with the [Web UI Specification](spec/web-ui-specification.md) for an overview
2. Review the [Information Architecture](architecture/information-architecture.md) to understand data models
3. Explore the [UX Plan](ux/ux-plan.md) for design principles and user experience goals
4. Check the [System Architecture](diagrams/system-architecture.md) for technical implementation details

## Key Features

The CrewChief Web UI provides:

- **Maproom Management** - Visual search interface and index statistics
- **Worktree Control** - Create, switch, and manage git worktrees
- **Agent Orchestration** - Spawn, monitor, and coordinate AI agents
- **Branch Management** - Visual git branch operations and merging
- **Settings Configuration** - Edit crewchief.config.js through a web interface
- **Real-time Monitoring** - Live updates via WebSocket connections
- **Run History** - Review past agent runs and their outputs