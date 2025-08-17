# CrewChief Web UI

A modern web interface for managing and monitoring CrewChief multi-agent orchestration operations.

## Overview

This package provides both a backend API server and a frontend React application for the CrewChief system, featuring:

- **Dashboard**: Real-time monitoring of agent activities and system health
- **Search**: Semantic code search powered by Maproom
- **Worktrees**: Management of isolated git development environments  
- **Agents**: Monitoring and control of AI development agents
- **Settings**: Configuration for system preferences

## Tech Stack

- **Frontend**: React 18+ with TypeScript, React Router, TailwindCSS
- **Backend**: Express.js with TypeScript, CORS, security middleware
- **Build Tools**: Vite (frontend), tsup (backend)
- **Development**: Hot reload, proxy configuration, concurrent servers

## Quick Start

```bash
# Install dependencies
pnpm install

# Start both frontend and backend in development mode
pnpm dev

# Frontend will be available at http://localhost:3000
# Backend API at http://localhost:3500
```

## Development Commands

```bash
# Start both servers concurrently
pnpm dev

# Start servers individually
pnpm dev:server  # Backend API server
pnpm dev:client  # Frontend development server

# Build for production
pnpm build          # Build both frontend and backend
pnpm build:client   # Build React app only
pnpm build:server   # Build Express server only

# Preview production build
pnpm preview

# Start production server
pnpm start

# Run tests
pnpm test

# Lint and format
pnpm lint
pnpm format
```

## API Endpoints

- `GET /api/health` - Server health check with uptime and version
- `GET /api` - API information and available endpoints
- Static files served from `dist/client` in production

## Configuration

Environment variables:

- `PORT` - Server port (default: 3456, development: 3500)
- `NODE_ENV` - Environment mode (development/production)

## Project Structure

```
packages/web-ui/
├── src/
│   ├── client/          # React frontend application
│   │   ├── components/  # Reusable UI components
│   │   ├── pages/      # Page components (Dashboard, Search, etc.)
│   │   ├── App.tsx     # Main app with routing
│   │   ├── main.tsx    # React entry point
│   │   └── index.css   # TailwindCSS styles
│   └── server.ts       # Express backend server
├── public/             # Static assets
├── dist/
│   ├── client/         # Built frontend assets
│   └── server.js       # Built backend server
├── index.html          # Vite entry point
├── vite.config.ts      # Vite configuration
├── tailwind.config.js  # TailwindCSS configuration
└── tsconfig*.json      # TypeScript configurations
```

## Features

### Dashboard
- Real-time system health monitoring
- Server status, uptime, and version information
- Quick action buttons for common tasks

### Search
- Semantic code search interface
- Integration with Maproom for intelligent code discovery
- File and line number navigation

### Worktrees
- Git worktree management and monitoring
- Status tracking (active, idle, error states)
- Agent assignment and activity monitoring

### Agents
- AI agent lifecycle management
- Performance metrics and success rates
- Task execution monitoring

### Settings
- System configuration options
- Theme selection (light/dark/system)
- API and performance settings

## Development Setup

The application uses a proxy configuration in development:
- Frontend dev server (Vite): `http://localhost:3000`
- Backend API server (Express): `http://localhost:3500`
- API requests from frontend are proxied to backend

In production, the Express server serves both the API and the built React application.

## Build Output

Production builds create:
- `dist/client/` - Optimized React application with assets
- `dist/server.js` - Compiled Express server with source maps

The server automatically serves the frontend in production mode.

## Integration with CrewChief

This web UI will be launched via the `crewchief web` command and will integrate with:

- Agent management and monitoring
- Worktree operations
- Message bus for real-time updates
- Configuration management
- Run orchestration