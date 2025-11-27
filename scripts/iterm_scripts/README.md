# iTerm2 Integration Scripts for CrewChief

This directory contains Python scripts that provide iTerm2 integration for CrewChief, enabling agent management directly in iTerm2 panes.

## Prerequisites

1. **iTerm2 Version**: Ensure you have iTerm2 version 3.3.0 or later installed
2. **Python API Enabled**: Enable Python API in iTerm2:
   - Open iTerm2 Preferences
   - Go to "General" → "Magic"
   - Enable "Enable Python API"
3. **Python Environment**: Python 3.8 or later

## Installation

Install Python dependencies:
```bash
pip install -r requirements.txt
```

## Core Scripts

### Agent Management

- **spawn_agent.py**: Spawn a new agent in an iTerm2 pane
  ```bash
  python3 spawn_agent.py --name "my-agent" --type claude --project-dir /path/to/project
  ```

- **send_to_pane.py**: Send a message/command to an agent pane
  ```bash
  python3 send_to_pane.py --to "pane-id" --text "Hello"
  ```

- **list_panes.py**: List all iTerm2 panes with their IDs
  ```bash
  python3 list_panes.py
  ```

- **list_agents.py**: List active agent panes (filtered by naming convention)
  ```bash
  python3 list_agents.py
  ```

- **kill_agent.py**: Stop and close an agent pane
  ```bash
  python3 kill_agent.py --name "agent-name"
  ```

### Multi-Agent

- **spawn_multi_agents.py**: Spawn multiple agents at once
  ```bash
  python3 spawn_multi_agents.py --agents claude,gemini --project-dir /path/to/project
  ```

### Pane Operations

- **split_horizontal.py**: Split current pane horizontally
- **split_vertical.py**: Split current pane vertically
- **label_pane.py**: Set a label/badge on a pane

### Configuration

- **agent_config.py**: Agent type definitions and configurations

## Naming Convention

Agent panes follow the `{name}__{type}` naming convention:
- `fix-bug__claude` - Agent named "fix-bug" of type "claude"
- `feature__gemini` - Agent named "feature" of type "gemini"

This convention enables:
- Agent filtering in `list_agents.py`
- Type detection in `send_to_pane.py` for proper Enter key handling
- Visual identification via badges

## Features

### Visual Indicators
- **Badges**: Each agent session displays a badge with agent name and type
- **Labels**: Panes can be labeled for easy identification

### Agent-Specific Behavior
- Different agent types (claude, gemini, etc.) may have different Enter key requirements
- The scripts handle these differences automatically based on agent type

## Troubleshooting

### Connection Issues
- Ensure iTerm2 is running
- Check Python API is enabled in iTerm2 preferences

### Permission Errors
- Grant iTerm2 accessibility permissions in System Preferences
- Ensure scripts have execute permissions: `chmod +x *.py`

### Pane Not Found
- Use `list_panes.py` to see available pane IDs
- Panes may have been closed manually

## License

Part of the CrewChief project. See main project LICENSE for details.
