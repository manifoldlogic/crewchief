# iTerm2 Pane Communication Scripts

Simple scripts for managing and communicating between iTerm2 panes.

## Quick Start

1. **Label your panes** for easy identification:
```bash
python3 label_pane.py "main"      # Label current pane as "main"
python3 label_pane.py "worker"    # Label current pane as "worker"
```

2. **List all panes** to see what's available:
```bash
python3 list_panes.py
```

3. **Send commands** to other panes:
```bash
python3 send_to_pane.py --to main --text "echo Hello from another pane"
python3 send_to_pane.py --to worker --text "ls -la"
python3 send_to_pane.py --to 2 --text "pwd"     # Use pane index
python3 send_to_pane.py                         # Interactive mode
```

## Scripts

### `split_horizontal.py` / `split_vertical.py`
Split the current pane:
```bash
python3 split_horizontal.py   # Split top/bottom
python3 split_vertical.py     # Split left/right
```

### `label_pane.py`
Label panes for easy identification:
```bash
python3 label_pane.py "name"           # Label current pane
python3 label_pane.py "name" --session SESSION_ID  # Label specific session
python3 label_pane.py --clear          # Remove label from current pane
```

Labels appear as:
- **Badges** - Visual overlay in the terminal pane (📌 label)
- **Session names** - Shown in iTerm2 tabs ([label])
- **Variables** - Queryable by other scripts

### `list_panes.py`
Show all available panes:
```bash
python3 list_panes.py        # Human-readable list
python3 list_panes.py --json # JSON output
```

Output shows:
- Index number (for quick selection)
- Label (if set)
- Window and tab location
- Session ID
- Current pane indicator

### `send_to_pane.py`
Send text/commands to another pane:

**By label** (easiest):
```bash
python3 send_to_pane.py --to main --text "echo hello"
```

**By index** (from list_panes output):
```bash
python3 send_to_pane.py --to 3 --text "pwd"
```

**By session ID** (partial match supported):
```bash
python3 send_to_pane.py --id 3F4A2B --text "ls"
```

**Interactive mode**:
```bash
python3 send_to_pane.py
# Shows numbered list to choose from
```

**Pipe input**:
```bash
echo "git status" | python3 send_to_pane.py --to worker
cat script.sh | python3 send_to_pane.py --to main
```

## Workflow Examples

### Example 1: Basic Setup
```bash
# In pane 1
python3 label_pane.py "main"

# Split and label a worker pane
python3 split_vertical.py
python3 label_pane.py "worker"

# From main pane, send commands to worker
python3 send_to_pane.py --to worker --text "cd /tmp"
python3 send_to_pane.py --to worker --text "ls -la"
```

### Example 2: Multiple Workers
```bash
# Create and label multiple panes
python3 label_pane.py "controller"
python3 split_horizontal.py
python3 label_pane.py "worker1"
python3 split_vertical.py
python3 label_pane.py "worker2"

# Send different commands to each
python3 send_to_pane.py --to worker1 --text "npm run dev"
python3 send_to_pane.py --to worker2 --text "npm test --watch"
```

### Example 3: Broadcast to Multiple Panes
```bash
# Simple broadcast script
for pane in worker1 worker2 worker3; do
    python3 send_to_pane.py --to "$pane" --text "git pull"
done
```

## Tips

1. **Label panes immediately** after creating them for easy management
2. **Use descriptive labels** like "server", "client", "logs", "tests"
3. **Check available panes** with `list_panes.py` before sending
4. **Use interactive mode** when you're not sure which pane to target
5. **Pipe complex commands** from files or other scripts

## Requirements

- iTerm2 3.3.0 or later
- Python 3.8+
- `pip install iterm2`

## Troubleshooting

**"No active session found"**
- Make sure you're running the script from within iTerm2
- Check that Python API is enabled in iTerm2 preferences

**"Target pane not found"**
- Run `list_panes.py` to see available panes
- Check that the label/index/ID is correct
- Labels are case-insensitive but must match exactly

**Scripts not working**
- Ensure iTerm2 Python API is enabled: iTerm2 → Preferences → General → Magic → Enable Python API
- Check Python version: `python3 --version` (needs 3.8+)
- Install requirements: `pip3 install iterm2`