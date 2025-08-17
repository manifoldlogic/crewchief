# Cursor IDE DevContainer Setup

This DevContainer is configured to work with both VS Code and Cursor IDE. If you're experiencing issues with Cursor showing host paths instead of container paths, follow these steps:

## Quick Fix

1. **Open in Cursor**:
   - Open the repository folder in Cursor
   - Press `Cmd+Shift+P` (Mac) or `Ctrl+Shift+P` (Windows/Linux)
   - Type "Remote-Containers: Reopen in Container"
   - Select it and wait for the container to build

2. **If Terminal Shows Wrong Path**:
   - Close the current terminal
   - Open a new terminal (`Cmd+`` ` or `Ctrl+`` `)
   - The terminal should now show `/workspace` as the current directory

3. **Manual Fix** (if needed):
   ```bash
   cd /workspace
   ```

## Features for Cursor

The DevContainer includes Cursor-specific configurations:

- **Working Directory**: Automatically sets to `/workspace`
- **Terminal Settings**: Configured for zsh with proper workspace path
- **Extensions**: Cursor-compatible extensions are pre-installed
- **Aliases**: All commands use absolute paths for consistency

## Troubleshooting

### Terminal Still Shows Host Path

1. Restart Cursor completely
2. Delete the container:
   ```bash
   docker ps -a | grep crewchief
   docker rm -f <container-id>
   ```
3. Rebuild the container in Cursor

### Extensions Not Loading

1. Open Command Palette (`Cmd+Shift+P`)
2. Run "Developer: Reload Window"
3. Extensions should load after reload

### Can't Find Project Files

The workspace is mounted at `/workspace`. If you see different paths:
1. Check Docker Desktop is running
2. Verify the container is running: `docker ps`
3. Ensure you opened the folder containing `.devcontainer`

## Differences from VS Code

While VS Code and Cursor share similar DevContainer support, there are some differences:

- **Extension Storage**: Cursor uses `.cursor-server` instead of `.vscode-server`
- **Settings Sync**: Cursor settings are stored separately
- **Terminal Detection**: We detect Cursor via environment variables

## Verifying Setup

Run this command to verify your environment:

```bash
echo "Current Dir: $(pwd)"
echo "Workspace: $WORKSPACE_DIR"
echo "Terminal: $TERM_PROGRAM"
```

You should see:
```
Current Dir: /workspace
Workspace: /workspace
Terminal: Cursor (or empty if not detected)
```

## Using Claude Code in Cursor DevContainer

The DevContainer includes Claude Code with dangerous mode enabled:

```bash
# Run Claude Code
claude

# Or use the alias
claude --dangerous-mode
```

Network access is configured to allow internet while blocking host access for security.