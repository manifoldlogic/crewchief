# iTerm2 Agent Management

Scripts for spawning and managing CLI agents (Claude, Gemini, GPT, etc.) in iTerm2 with dedicated worktrees.

## Quick Start

```bash
# Spawn a Claude agent
python3 spawn_agent.py claude

# List all active agents
python3 list_agents.py

# Send command to an agent
python3 send_to_pane.py --to claude-20240115-143022-a3f2 --text "implement the login feature"

# Kill an agent when done
python3 kill_agent.py claude-20240115-143022-a3f2
```

## Scripts

### `spawn_agent.py`

Spawns a new CLI agent in a split pane with its own worktree.

**What it does:**
1. Splits the current pane (horizontal by default)
2. Creates a new worktree using `crewchief worktree create`
3. Changes to the worktree directory
4. Launches the specified CLI agent
5. Labels the pane with agent name and type

**Usage:**
```bash
# Basic spawning
python3 spawn_agent.py claude              # Spawn Claude with auto-generated name
python3 spawn_agent.py gemini              # Spawn Gemini
python3 spawn_agent.py gpt                 # Spawn GPT

# With custom name
python3 spawn_agent.py claude --name my-feature-agent

# With additional arguments
python3 spawn_agent.py claude --args "--model claude-3-opus"

# Vertical split
python3 spawn_agent.py claude --vertical

# Custom command
python3 spawn_agent.py custom "my-ai-cli --flag"
```

**Supported agents:**
- `claude` - Anthropic's Claude
- `gemini` - Google's Gemini
- `gpt` / `openai` - OpenAI GPT
- `cursor` - Cursor AI
- `aider` - Aider coding assistant
- `custom` - Any custom command

**Auto-generated names:**
Format: `{agent-type}-{timestamp}-{random}`
Example: `claude-20240115-143022-a3f2`

### `list_agents.py`

Lists all active agents with their details.

```bash
# List all agents
python3 list_agents.py

# Output as JSON
python3 list_agents.py --json

# Filter by type
python3 list_agents.py --type claude
```

**Output shows:**
- Agent name
- Agent type
- Command used
- Window/Tab location
- Session ID

### `kill_agent.py`

Terminates an agent session.

```bash
# Kill specific agent
python3 kill_agent.py claude-20240115-143022-a3f2

# Kill all agents
python3 kill_agent.py --all

# Also suggest worktree cleanup
python3 kill_agent.py my-agent --cleanup
```

## Workflow Examples

### Example 1: Simple Agent Spawn
```bash
# Spawn a Claude agent
python3 spawn_agent.py claude

# Output:
# 🚀 Spawning claude agent: claude-20240115-143022-a3f2
#    1️⃣ Splitting pane...
#    2️⃣ Creating worktree: claude-20240115-143022-a3f2
#    3️⃣ Changing to worktree directory...
#    4️⃣ Launching claude agent...
#    5️⃣ Labeling pane as: claude-20240115-143022-a3f2
#
# ✅ Agent spawned successfully!
```

### Example 2: Multiple Agents for Different Tasks
```bash
# Spawn agents for different purposes
python3 spawn_agent.py claude --name backend-dev
python3 spawn_agent.py claude --name frontend-dev
python3 spawn_agent.py gemini --name code-review

# List all agents
python3 list_agents.py

# Send specific tasks to each
python3 send_to_pane.py --to backend-dev --text "create the user authentication API"
python3 send_to_pane.py --to frontend-dev --text "build the login form component"
python3 send_to_pane.py --to code-review --text "review the pull request #123"
```

### Example 3: Agent Grid Layout
```bash
# Create a 2x2 grid of agents
python3 spawn_agent.py claude --name agent1
python3 spawn_agent.py claude --name agent2 --vertical
python3 spawn_agent.py gemini --name agent3
python3 spawn_agent.py gpt --name agent4 --vertical
```

### Example 4: Competition Mode
```bash
# Spawn multiple agents for the same task
task="implement a binary search function in Python"

python3 spawn_agent.py claude --name claude-solver
python3 spawn_agent.py gemini --name gemini-solver
python3 spawn_agent.py gpt --name gpt-solver

# Send the same task to all
for agent in claude-solver gemini-solver gpt-solver; do
    python3 send_to_pane.py --to "$agent" --text "$task"
done

# Compare their solutions...

# Clean up when done
python3 kill_agent.py --all
```

## Integration with CrewChief

Each spawned agent:
1. **Gets its own worktree** - Isolated git workspace at `.crewchief/worktrees/{agent-name}`
2. **Can be managed via CrewChief** - Use `crewchief worktree list` to see all worktrees
3. **Works on the same repository** - All agents share the same codebase but in isolation
4. **Can be merged back** - Use `crewchief worktree merge` when agent work is complete

### Worktree Management
```bash
# List all worktrees (including agent worktrees)
crewchief worktree list

# Merge agent's work back to main branch
crewchief worktree merge claude-20240115-143022-a3f2

# Clean up stale worktrees
crewchief worktree clean
```

## Visual Indicators

Each agent pane shows:
- **Badge**: 🤖 {agent-name}
- **Session Name**: [{agent-type}] {agent-name}
- **User Variables**: Queryable metadata for scripts

## Tips

1. **Use descriptive names** when spawning agents for specific tasks:
   ```bash
   python3 spawn_agent.py claude --name auth-feature
   python3 spawn_agent.py claude --name bug-fix-login
   ```

2. **Check agent status** before sending commands:
   ```bash
   python3 list_agents.py
   ```

3. **Clean up finished agents** to free resources:
   ```bash
   python3 kill_agent.py agent-name
   crewchief worktree clean
   ```

4. **Use different agent types** for different strengths:
   - Claude for complex reasoning
   - Gemini for creative solutions
   - GPT for general tasks

5. **Coordinate agents** by sending commands between them:
   ```bash
   # Agent 1 creates a feature
   python3 send_to_pane.py --to agent1 --text "create user model"
   
   # Agent 2 reviews it
   python3 send_to_pane.py --to agent2 --text "review the user model in models/user.py"
   ```

## Troubleshooting

**"crewchief: command not found"**
- Ensure CrewChief is installed and in your PATH
- Or use the full path to the crewchief binary

**Agent doesn't start**
- Check that the CLI tool is installed (claude, gemini, etc.)
- Verify the command works in a regular terminal

**Worktree creation fails**
- Ensure you're in a git repository
- Check that `.crewchief/worktrees/` directory exists
- Verify you have write permissions

**Can't send to agent**
- Use `list_agents.py` to get the exact agent name
- Check that the agent session is still active
- Verify the agent pane hasn't been manually closed

## Requirements

- iTerm2 3.3.0+
- Python 3.8+
- CrewChief CLI installed
- Git repository
- CLI agents installed (claude, gemini, etc.)