# iTerm2 Integration for CrewChief

## Overview

CrewChief now supports iTerm2 as an alternative to tmux for managing agent terminal sessions on macOS. This integration provides a more native and visually rich experience for macOS users.

## Key Benefits

### Over Tmux
- **Native macOS Experience**: Seamless integration with macOS UI
- **Rich Visual Feedback**: Badges, colors, and annotations for agent status
- **Better Performance**: Native application vs. terminal multiplexer
- **Persistent Sessions**: Sessions survive terminal closure
- **Advanced Features**: Split views, tabs, profiles, and more

## Setup

### Prerequisites
1. macOS (required for iTerm2)
2. iTerm2 version 3.3.0 or later
3. Python 3.8 or later
4. Python API enabled in iTerm2:
   - Open iTerm2 → Preferences → General → Magic
   - Enable "Enable Python API"

### Installation

1. **Install Python dependencies**:
```bash
pip3 install -r iterm_scripts/requirements.txt
```

2. **Configure CrewChief**:
Update your `crewchief.config.ts`:
```typescript
{
  terminal: {
    backend: 'iterm', // or 'auto' for automatic detection
    iterm: {
      sessionName: 'crewchief',
      bridgePort: 8765,
      agentBadges: true,
    }
  }
}
```

3. **Start the bridge** (automatic when using CrewChief):
```bash
./iterm_scripts/start_bridge.sh
```

## Usage

### Basic Commands

All existing CrewChief commands work seamlessly with iTerm2:

```bash
# Start CrewChief with iTerm2
crewchief

# Spawn an agent (creates iTerm2 tab/pane)
crewchief agent spawn worker "implement feature X"

# Send message to agent
crewchief agent message agent-001 "run tests"

# View agent in iTerm2 (automatic focus)
crewchief agent view agent-001
```

### Visual Features

#### Agent Badges
Each agent session displays a badge showing:
- Agent type (WORKER, REVIEWER, etc.)
- Agent ID (first 8 characters)
- Status (BUSY, IDLE, STOPPED)

#### Grid Layout
Multiple agents can be arranged in a grid:
```bash
# Create 4 agents in a 2x2 grid
crewchief competition start "task" agent-1 agent-2 agent-3 agent-4
```

#### Color Coding
- 🟢 Green badge: Agent idle/ready
- 🟡 Yellow badge: Agent running
- 🔴 Red badge: Agent stopped/error

### Advanced Features

#### Custom Profiles
Create dedicated iTerm2 profiles for agents:

1. Create a profile named "CrewChief Agent" in iTerm2
2. Configure in `crewchief.config.ts`:
```typescript
iterm: {
  profile: 'CrewChief Agent'
}
```

#### Output Monitoring
Real-time agent output is captured and can be:
- Viewed in iTerm2 tabs/panes
- Logged to files
- Streamed to the orchestrator

#### Keyboard Shortcuts
Use iTerm2's native shortcuts:
- `Cmd+D`: Split pane vertically
- `Cmd+Shift+D`: Split pane horizontally
- `Cmd+[/]`: Navigate between panes
- `Cmd+T`: New tab
- `Cmd+Number`: Switch to tab

## Architecture

### Component Interaction

```
┌──────────────┐     JSON-RPC      ┌─────────────┐     Python API    ┌─────────┐
│  TypeScript  │ ◄─────────────────► │   Bridge    │ ◄────────────────► │ iTerm2  │
│     CLI      │                    │   Server    │                    │   App   │
└──────────────┘                    └─────────────┘                    └─────────┘
      │                                    │                                  │
      └── ITermService ──────────► iterm_bridge.py ──────► iTerm2 Sessions ──┘
```

### File Structure

```
crewchief/
├── iterm_scripts/              # Python scripts for iTerm2
│   ├── iterm_controller.py     # Low-level iTerm2 API
│   ├── iterm_agent_manager.py  # Agent management
│   ├── iterm_bridge.py         # JSON-RPC bridge
│   └── start_bridge.sh         # Startup script
├── packages/cli/src/
│   ├── iterm/                  # TypeScript iTerm2 integration
│   │   ├── iterm.service.ts    # Main service
│   │   └── iterm.types.ts      # Type definitions
│   └── terminal/               # Terminal abstraction
│       ├── factory.ts          # Backend selection
│       ├── iterm.adapter.ts    # iTerm2 adapter
│       └── tmux.adapter.ts     # Tmux adapter (fallback)
```

## Testing

### Manual Testing
```bash
# Test the bridge
python3 iterm_scripts/test_bridge.py

# Interactive test mode
python3 iterm_scripts/test_bridge.py --interactive
```

### Automated Tests
```bash
# Run TypeScript tests
pnpm test

# Run Python tests
python3 -m pytest iterm_scripts/
```

## Troubleshooting

### Bridge Connection Issues
- Ensure iTerm2 is running
- Check Python API is enabled in iTerm2 preferences
- Verify no other process is using port 8765
- Check firewall settings

### Session Not Found
- Sessions may have been manually closed
- Restart the bridge: `./iterm_scripts/start_bridge.sh`
- Clear stale sessions: `crewchief worktree clean`

### Performance Issues
- Reduce number of concurrent agents
- Adjust grid layout (fewer panes)
- Check system resources (CPU, memory)

### Fallback to Tmux
If iTerm2 is unavailable, CrewChief automatically falls back to tmux:
```typescript
terminal: {
  backend: 'auto'  // Automatic fallback
}
```

## Migration from Tmux

### For Users
1. Install iTerm2 if not already installed
2. Update configuration to use iTerm2 backend
3. Run CrewChief normally - it handles the rest

### For Developers
The terminal abstraction layer ensures code compatibility:
```typescript
// Works with both tmux and iTerm2
const terminal = TerminalFactory.create({ backend: 'auto' })
await terminal.createPane()
await terminal.sendCommand(paneId, 'echo hello')
```

## Platform Support

| Platform | iTerm2 | Tmux | Default |
|----------|--------|------|---------|
| macOS    | ✅     | ✅   | iTerm2  |
| Linux    | ❌     | ✅   | Tmux    |
| Windows  | ❌     | ⚠️   | WSL/Tmux |

## Future Enhancements

### Planned Features
- [ ] WebSocket support for real-time updates
- [ ] Session recording and replay
- [ ] Agent performance metrics dashboard
- [ ] Automatic session restoration
- [ ] Multi-window orchestration
- [ ] Custom themes for agent types

### Experimental Features
- Semantic highlighting of agent output
- AI-powered output analysis
- Cross-session clipboard sharing
- Collaborative agent sessions

## Contributing

To contribute to the iTerm2 integration:

1. Fork the repository
2. Create a feature branch
3. Implement changes in appropriate directories:
   - Python: `iterm_scripts/`
   - TypeScript: `packages/cli/src/iterm/`
4. Add tests
5. Submit a pull request

## Support

For issues related to iTerm2 integration:
- Check the [troubleshooting guide](#troubleshooting)
- Search existing GitHub issues
- Create a new issue with the `iterm2` label
- Include iTerm2 version and macOS version

## License

The iTerm2 integration is part of the CrewChief project and follows the same license terms.