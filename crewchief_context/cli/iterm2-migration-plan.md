# iTerm2 Migration Plan for CrewChief

## Overview
This document outlines the migration from tmux to iTerm2 for managing agent terminal sessions in CrewChief. iTerm2's Python API provides programmatic control over terminal sessions, enabling us to replicate and enhance the current tmux-based orchestration.

## Current Tmux Architecture

### Key Functions
1. **Session Management**: Creating and managing named tmux sessions
2. **Pane/Window Creation**: Splitting panes and creating new windows for agents
3. **Command Execution**: Sending commands to specific panes
4. **Output Capture**: Reading pane content for monitoring
5. **Process Isolation**: Each agent runs in its own pane with dedicated worktree

### Tmux Service Methods (from `packages/cli/src/tmux/tmux.service.ts`)
- `ensureSession()`: Create session if it doesn't exist
- `createPane()`: Split window to create new pane
- `createWindow()`: Create new window in session
- `sendKeys()`: Send commands to specific pane
- `captureOutput()`: Get current pane content
- `attach()`: Attach to session for viewing

## iTerm2 Architecture

### Advantages over Tmux
1. **Native macOS Integration**: Better performance and stability on macOS
2. **Rich API**: More comprehensive programmatic control
3. **Visual Feedback**: Built-in UI for monitoring agent activity
4. **Advanced Features**: Badges, marks, annotations for better tracking
5. **Persistent State**: Sessions survive terminal closure

### Key iTerm2 Concepts
- **App**: The iTerm2 application instance
- **Window**: A terminal window (can have multiple tabs)
- **Tab**: A tab within a window (can have multiple panes)
- **Session**: A terminal session within a pane
- **Profile**: Configuration template for new sessions

## Implementation Strategy

### Phase 1: Python Scripts Foundation
Create core Python scripts that provide iTerm2 control functionality:

1. **`iterm_controller.py`**: Main controller script
   - Session management
   - Window/tab/pane creation
   - Command execution
   - Output monitoring

2. **`iterm_agent_manager.py`**: Agent-specific management
   - Create agent workspace (tab with panes)
   - Send commands to agent panes
   - Monitor agent output
   - Clean up agent sessions

3. **`iterm_bridge.py`**: Bridge between TypeScript and Python
   - JSON-RPC server for TypeScript communication
   - Command queue management
   - Event streaming

### Phase 2: TypeScript Integration
Replace `TmuxService` with `ITermService`:

1. **`packages/cli/src/iterm/iterm.service.ts`**
   - Interface matching TmuxService API
   - Communication with Python bridge
   - Fallback to direct AppleScript if needed

2. **Configuration Updates**
   - Add iTerm2-specific settings to `crewchief.config.ts`
   - Profile selection for agent sessions
   - Layout preferences

### Phase 3: Enhanced Features
Leverage iTerm2-specific capabilities:

1. **Visual Indicators**
   - Badges showing agent status
   - Color coding for different agent types
   - Marks for important events

2. **Output Management**
   - Automatic scrollback capture
   - Session logging integration
   - Real-time output streaming

## File Structure

```
crewchief/
├── iterm_scripts/
│   ├── __init__.py
│   ├── iterm_controller.py      # Core iTerm2 control
│   ├── iterm_agent_manager.py   # Agent-specific management
│   ├── iterm_bridge.py          # TypeScript bridge
│   ├── requirements.txt         # Python dependencies
│   └── README.md                # Setup instructions
├── packages/cli/src/
│   ├── iterm/
│   │   ├── iterm.service.ts    # TypeScript iTerm2 service
│   │   ├── iterm.types.ts      # Type definitions
│   │   └── iterm.config.ts     # Configuration
│   └── terminal/
│       ├── terminal.interface.ts # Common interface
│       ├── tmux.adapter.ts      # Tmux implementation
│       └── iterm.adapter.ts     # iTerm2 implementation
```

## Migration Path

### Step 1: Parallel Implementation
- Keep tmux implementation intact
- Build iTerm2 implementation alongside
- Use feature flag to switch between implementations

### Step 2: Testing & Validation
- Unit tests for Python scripts
- Integration tests for TypeScript service
- End-to-end tests with real agents

### Step 3: Gradual Rollout
- Beta testing with select users
- Documentation updates
- Migration guide for existing users

## Technical Requirements

### Python Environment
- Python 3.8+
- iterm2 Python package
- asyncio for async operations
- JSON-RPC for TypeScript communication

### iTerm2 Requirements
- iTerm2 version 3.3.0 or later
- Python API enabled in iTerm2 preferences
- Script permissions configured

### TypeScript Updates
- Abstract terminal interface
- Adapter pattern for tmux/iTerm2
- Configuration-based selection

## Risk Mitigation

### Compatibility
- Maintain tmux support for non-macOS platforms
- Detect iTerm2 availability at runtime
- Graceful fallback mechanisms

### Performance
- Async operations for non-blocking execution
- Connection pooling for Python bridge
- Efficient output streaming

### Reliability
- Error handling and recovery
- Session persistence
- Automatic reconnection

## Timeline

- **Week 1**: Python scripts development
- **Week 2**: TypeScript integration
- **Week 3**: Testing and refinement
- **Week 4**: Documentation and rollout

## Success Criteria

1. Feature parity with tmux implementation
2. Improved performance on macOS
3. Enhanced visual feedback for users
4. Backward compatibility maintained
5. Comprehensive test coverage