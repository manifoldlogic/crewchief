# iTerm2 Integration Scripts for CrewChief

This directory contains Python scripts that provide iTerm2 integration for CrewChief, replacing the tmux-based terminal management.

## Prerequisites

1. **iTerm2 Version**: Ensure you have iTerm2 version 3.3.0 or later installed
2. **Python API Enabled**: Enable Python API in iTerm2:
   - Open iTerm2 Preferences
   - Go to "General" → "Magic"
   - Enable "Enable Python API"
3. **Python Environment**: Python 3.8 or later

## Installation

1. Install Python dependencies:
```bash
pip install -r requirements.txt
```

2. Register the scripts with iTerm2:
```bash
# Option 1: Run as standalone script
python3 iterm_bridge.py

# Option 2: Install as iTerm2 script
cp *.py ~/Library/Application\ Support/iTerm2/Scripts/
```

## Architecture

### Core Components

1. **iterm_controller.py**: Low-level iTerm2 API wrapper
   - Session management
   - Window/tab/pane creation
   - Command execution
   - Output capture

2. **iterm_agent_manager.py**: Agent-specific functionality
   - Agent workspace creation
   - Task distribution
   - Output monitoring
   - Grid layouts for multiple agents

3. **iterm_bridge.py**: JSON-RPC bridge for TypeScript integration
   - HTTP server for RPC calls
   - Async operation handling
   - Event streaming support

## Usage

### Starting the Bridge Server

```bash
# Start the bridge server (default port 8765)
python3 iterm_bridge.py

# Or with custom port
python3 iterm_bridge.py --port 9000
```

### TypeScript Integration

The TypeScript CLI communicates with the bridge via JSON-RPC:

```typescript
// Example RPC call
const response = await fetch('http://localhost:8765/rpc', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'createAgent',
    params: {
      agentId: 'agent-001',
      agentType: 'worker',
      workingDir: '/path/to/worktree'
    },
    id: 1
  })
});
```

## Available RPC Methods

### Session Management
- `createSession`: Create a new iTerm2 session
- `closeSession`: Close a session
- `listSessions`: List all active sessions

### Agent Management
- `createAgent`: Create a new agent workspace
- `stopAgent`: Stop and close an agent
- `sendTask`: Send a task to an agent
- `getAgentOutput`: Get recent output from an agent
- `listAgents`: List all active agents
- `getAgentStatus`: Get status of a specific agent

### Command Execution
- `sendCommand`: Send a command (with newline)
- `sendText`: Send raw text
- `getContents`: Get session screen contents

### Layout Management
- `createAgentGrid`: Create a grid layout of agents
- `splitPane`: Split a pane

### Utilities
- `setBadge`: Set a visual badge on a session
- `focusSession`: Bring a session to focus
- `broadcast`: Send command to multiple agents

## Features

### Visual Indicators
- **Badges**: Each agent session displays a badge with agent ID and status
- **Status Colors**: Different colors for running/idle/stopped states
- **Grid Layouts**: Organize multiple agents in grid formation

### Monitoring
- Real-time output capture
- Status tracking
- Event-based updates via callbacks

### Agent Isolation
- Each agent runs in its own session
- Dedicated working directories
- Environment variable isolation

## Testing

Run the test script to verify functionality:

```bash
# Test basic controller functions
python3 iterm_controller.py

# Test agent management
python3 iterm_agent_manager.py

# Test the bridge server
python3 test_bridge.py
```

## Troubleshooting

### Connection Issues
- Ensure iTerm2 is running
- Check Python API is enabled in iTerm2 preferences
- Verify no other process is using the bridge port

### Permission Errors
- Grant iTerm2 accessibility permissions in System Preferences
- Ensure scripts have execute permissions

### Session Not Found
- Sessions may have been closed manually
- Restart the bridge server to refresh connections

## Development

### Adding New Methods

1. Add handler in `iterm_bridge.py`:
```python
async def my_new_method(self, params: Dict) -> Any:
    # Implementation
    pass
```

2. Register in dispatch table:
```python
handlers = {
    # ...
    "myNewMethod": self.my_new_method,
}
```

3. Update TypeScript interface accordingly

### Event Streaming

For real-time updates, consider adding WebSocket support:

```python
# In iterm_bridge.py
async def websocket_handler(request):
    ws = web.WebSocketResponse()
    await ws.prepare(request)
    # Handle WebSocket connections
```

## Migration from Tmux

### Equivalent Commands

| Tmux Command | iTerm2 Method |
|--------------|---------------|
| `tmux new-session` | `createSession()` |
| `tmux split-window` | `splitPane()` |
| `tmux send-keys` | `sendCommand()` |
| `tmux capture-pane` | `getContents()` |
| `tmux kill-pane` | `closeSession()` |

### Key Differences

1. **Session Persistence**: iTerm2 sessions persist across terminal restarts
2. **Visual Feedback**: Rich UI with badges, colors, and annotations
3. **Native Integration**: Better performance on macOS
4. **API Flexibility**: More comprehensive control options

## License

Part of the CrewChief project. See main project LICENSE for details.