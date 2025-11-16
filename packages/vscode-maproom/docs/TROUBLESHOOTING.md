# Troubleshooting Guide

Comprehensive troubleshooting guide for Maproom Semantic Search extension.

## Table of Contents

- [Quick Diagnostics](#quick-diagnostics)
- [Common Issues](#common-issues)
  - [Docker Issues](#docker-issues)
  - [Binary Permission Issues](#binary-permission-issues)
  - [Provider Configuration Issues](#provider-configuration-issues)
  - [Process Crash Issues](#process-crash-issues)
  - [Performance Issues](#performance-issues)
  - [File Watching Issues](#file-watching-issues)
- [Platform-Specific Issues](#platform-specific-issues)
  - [macOS](#macos)
  - [Linux](#linux)
  - [Windows](#windows)
- [Advanced Troubleshooting](#advanced-troubleshooting)
- [How to Report Bugs](#how-to-report-bugs)
- [Log Collection](#log-collection)

---

## Quick Diagnostics

Before diving into specific issues, run these quick checks:

### 1. Check Extension Status

Look at the Maproom status bar item (bottom right):
- **Green "Watching"** = Everything working
- **Blue "Indexing"** = Currently scanning files
- **Gray "Starting"** = Services initializing
- **Red "Error"** = Something wrong (click for details)

### 2. View Output Channel

`View → Output → Maproom` (dropdown in Output panel)

This shows detailed logs of everything the extension is doing.

### 3. Check Docker Status

```bash
# Verify Docker is running
docker ps

# Should show 3 containers:
# - maproom-postgres
# - maproom-ollama
# - maproom-mcp
```

### 4. Verify Binary Exists

```bash
# Linux/macOS
ls -la ~/.vscode/extensions/crewchief.vscode-maproom-*/bin/

# Should show platform-specific binary
```

---

## Common Issues

### Docker Issues

#### "Docker command not found"

**Symptoms:**
- Error message: "Docker command not found"
- Status bar shows "Error"
- Output channel shows "DOCKER_NOT_FOUND"

**Cause:** Docker is not installed or not in system PATH.

**Solutions:**

1. **Install Docker Desktop**
   - Download from: https://www.docker.com/products/docker-desktop
   - macOS: Install .dmg, drag to Applications
   - Windows: Run installer, enable WSL2 if prompted
   - Linux: Follow distribution-specific instructions

2. **Verify installation**
   ```bash
   docker --version
   # Should show: Docker version 24.0.0 or higher
   ```

3. **Add Docker to PATH** (if installed but not found)
   - macOS/Linux: Add to `~/.bashrc` or `~/.zshrc`:
     ```bash
     export PATH="/usr/local/bin:$PATH"
     ```
   - Windows: Check "Add to PATH" during installation, or add manually

4. **Restart VSCode** after installing Docker

---

#### "Docker daemon is not running"

**Symptoms:**
- Error: "Docker daemon is not running"
- `docker ps` fails with "Cannot connect to Docker daemon"
- Status bar shows "Error"

**Cause:** Docker Desktop is not started.

**Solutions:**

1. **Launch Docker Desktop**
   - macOS: Open from Applications folder
   - Windows: Start menu → Docker Desktop
   - Linux: `systemctl start docker`

2. **Wait for Docker to be ready** (can take 30-60 seconds)
   - macOS/Windows: Look for Docker icon in system tray
   - Icon should be stable, not animated

3. **Verify Docker is running**
   ```bash
   docker ps
   # Should return empty list or running containers
   ```

4. **Restart VSCode** after Docker is running

5. **Check Docker Desktop settings**
   - Ensure "Start Docker Desktop when you log in" is enabled
   - macOS/Windows: Docker Desktop → Settings → General

---

#### "Health check timeout" or "Services failed to start"

**Symptoms:**
- Error: "Health check timeout" or "Services didn't become healthy"
- Services appear in `docker ps` but extension reports error
- Initial startup takes >30 seconds
- Output channel shows repeated health check attempts

**Cause:** Docker services are starting slowly or failing health checks.

**Solutions:**

1. **Check Docker resource allocation**
   - Docker Desktop → Settings → Resources
   - **Memory**: Increase to at least 4GB (8GB recommended)
   - **CPUs**: Allocate at least 2 cores
   - **Disk**: Ensure 5GB+ available
   - Click "Apply & Restart"

2. **Check container logs**
   ```bash
   # PostgreSQL logs
   docker logs maproom-postgres

   # Look for errors like:
   # - "out of memory"
   # - "could not create shared memory segment"
   # - "database system is starting up" (still initializing)

   # Ollama logs
   docker logs maproom-ollama

   # MCP server logs
   docker logs maproom-mcp
   ```

3. **Manually verify services**
   ```bash
   # Check PostgreSQL is ready
   docker exec maproom-postgres pg_isready -U maproom -d maproom
   # Should output: "accepting connections"

   # Check Ollama API
   curl http://localhost:11434
   # Should return Ollama version info
   ```

4. **Reset Docker services**
   ```bash
   # Stop all services
   cd ~/.vscode/extensions/crewchief.vscode-maproom-*
   docker compose -f config/docker-compose.yml down

   # Remove volumes (fresh start)
   docker compose -f config/docker-compose.yml down -v

   # Restart extension
   # VSCode: Reload Window (Cmd/Ctrl+R)
   ```

5. **Check for port conflicts**
   ```bash
   # Check if port 5433 is already in use
   lsof -i :5433  # macOS/Linux
   netstat -ano | findstr :5433  # Windows

   # If port is in use, stop the conflicting service
   ```

6. **Update Docker Desktop** to latest version
   - Old versions may have resource management bugs

---

### Binary Permission Issues

#### "EACCES: permission denied" (Linux/macOS)

**Symptoms:**
- Error: "EACCES: permission denied"
- Error: "spawn EACCES"
- Process fails to start immediately
- Output channel shows spawn error

**Cause:** Binary file doesn't have execute permissions.

**Solutions:**

1. **Make binary executable**
   ```bash
   # Find extension directory
   EXT_DIR=$(ls -d ~/.vscode/extensions/crewchief.vscode-maproom-* | head -1)
   echo $EXT_DIR

   # Make binary executable
   chmod +x "$EXT_DIR/bin/"*/crewchief-maproom

   # Verify permissions
   ls -la "$EXT_DIR/bin/"*/crewchief-maproom
   # Should show: -rwxr-xr-x (executable)
   ```

2. **Reload VSCode**
   - Command Palette → "Developer: Reload Window"
   - Or restart VSCode completely

3. **If issue persists**, check SELinux (Linux only)
   ```bash
   # Check SELinux status
   getenforce
   # If "Enforcing", SELinux may be blocking execution

   # Temporarily disable (testing only)
   sudo setenforce 0

   # Or add SELinux exception
   chcon -t bin_t "$EXT_DIR/bin/"*/crewchief-maproom
   ```

---

#### "No such file or directory" (Binary missing)

**Symptoms:**
- Error: "ENOENT: no such file or directory"
- Binary path shown in error doesn't exist
- Extension just installed or updated

**Cause:** Binary not included in VSIX package or corrupted installation.

**Solutions:**

1. **Verify platform detection**
   - Check output channel for: "Platform detected: darwin-arm64" (or similar)
   - Ensure your platform is supported (see README)

2. **Check binary exists**
   ```bash
   # Find extension
   EXT_DIR=$(ls -d ~/.vscode/extensions/crewchief.vscode-maproom-* | head -1)

   # List binaries
   ls -R "$EXT_DIR/bin/"
   # Should show platform-specific directories and binaries
   ```

3. **Reinstall extension**
   ```bash
   # Uninstall
   code --uninstall-extension crewchief.vscode-maproom

   # Remove extension directory
   rm -rf ~/.vscode/extensions/crewchief.vscode-maproom-*

   # Reinstall
   code --install-extension vscode-maproom-0.1.0.vsix
   ```

4. **Check VSIX package integrity**
   ```bash
   # Extract VSIX (it's a ZIP file)
   unzip -l vscode-maproom-0.1.0.vsix | grep bin/
   # Should show binaries for all platforms
   ```

---

### Provider Configuration Issues

#### "Could not connect to Ollama"

**Symptoms:**
- Setup wizard doesn't mark Ollama as "Recommended"
- Error: "Failed to connect to Ollama"
- Chose Ollama but indexing fails

**Cause:** Ollama not running or not accessible on port 11434.

**Solutions:**

1. **Verify Ollama is installed**
   ```bash
   ollama --version
   # Should show version number
   ```

2. **Install Ollama** (if not installed)
   - Download from: https://ollama.ai/download
   - macOS: `brew install ollama` or use installer
   - Linux: `curl https://ollama.ai/install.sh | sh`
   - Windows: Download installer from website

3. **Start Ollama service**
   ```bash
   # macOS/Linux
   ollama serve

   # Or run as background service
   # macOS: Ollama.app stays running in menu bar
   # Linux: systemctl start ollama (if installed via package)
   ```

4. **Verify Ollama is running**
   ```bash
   # Test API endpoint
   curl http://localhost:11434
   # Should return: Ollama is running

   # List models
   ollama list
   # Should show installed models
   ```

5. **Download embedding model**
   ```bash
   # Pull required model (if not already installed)
   ollama pull nomic-embed-text

   # Verify model is available
   ollama list | grep nomic-embed-text
   ```

6. **Check firewall settings**
   - macOS: System Settings → Network → Firewall
   - Linux: `sudo ufw status`
   - Ensure port 11434 is not blocked

7. **If Ollama is on different port**
   - Currently extension assumes port 11434
   - Workaround: Use OpenAI or Google provider
   - Feature request: Configurable Ollama port (future version)

---

#### "Invalid API key" or "Authentication failed"

**Symptoms:**
- Error: "Invalid API key"
- Error: "401 Unauthorized"
- Error: "Authentication failed"
- Selected OpenAI or Google but indexing fails

**Cause:** API key is incorrect, expired, or has insufficient permissions.

**Solutions:**

1. **Re-run setup with new API key**
   - Command Palette → `Maproom: Setup`
   - Select provider again
   - Carefully paste API key (avoid leading/trailing whitespace)

2. **Verify OpenAI API key**
   - Visit: https://platform.openai.com/api-keys
   - Check key is not revoked
   - Check you have credits/billing set up
   - Test key manually:
     ```bash
     curl https://api.openai.com/v1/models \
       -H "Authorization: Bearer YOUR_API_KEY"
     # Should return list of models
     ```

3. **Verify Google API credentials**
   - Visit: https://console.cloud.google.com/
   - Ensure Vertex AI API is enabled
   - Check service account has correct permissions
   - Verify API key format is correct

4. **Check for quota limits**
   - OpenAI: https://platform.openai.com/account/limits
   - Google: https://console.cloud.google.com/apis/dashboard
   - You may have hit rate limits or quota

5. **Try different provider**
   - If one provider fails, try another
   - Ollama is free and has no API limits

---

### Process Crash Issues

#### "Maproom watcher crashed after 5 restart attempts"

**Symptoms:**
- Error notification: "Maproom watcher crashed after 5 restart attempts"
- Status bar shows "Error"
- Output channel shows repeated crash/restart cycle
- Circuit breaker enters OPEN state

**Cause:** Background process keeps crashing, exhausted retry attempts.

**Solutions:**

1. **Check output channel for crash details**
   - `View → Output → Maproom`
   - Look for error messages before each restart
   - Common errors:
     - "out of memory" → Need more RAM
     - "connection refused" → Database not ready
     - "permission denied" → File permission issue
     - "too many open files" → System limit reached

2. **Check system resources**
   ```bash
   # macOS/Linux - check RAM usage
   top -l 1 | grep PhysMem  # macOS
   free -h                   # Linux

   # Check Docker memory usage
   docker stats --no-stream

   # Check available disk space
   df -h
   ```

3. **Increase system limits** (Linux)
   ```bash
   # Check current limits
   ulimit -a

   # Increase file descriptor limit
   ulimit -n 4096

   # Make permanent (add to ~/.bashrc)
   echo "ulimit -n 4096" >> ~/.bashrc
   ```

4. **Manually restart watchers**
   - Command Palette → `Maproom: Restart Watchers`
   - This resets the crash recovery circuit breaker
   - Extension will try again from clean state

5. **Check for corrupted workspace state**
   ```bash
   # Clear workspace state (last resort)
   # Note: This will lose your provider selection
   rm -rf ~/Library/Application\ Support/Code/User/workspaceStorage/*  # macOS
   rm -rf ~/.config/Code/User/workspaceStorage/*                       # Linux
   ```

6. **Restart VSCode**
   - Command Palette → "Developer: Reload Window"
   - Or fully quit and restart VSCode

7. **Report the issue** (if none of above work)
   - See "How to Report Bugs" section below
   - Include output channel logs

---

#### "Process crashed: exit code 137"

**Symptoms:**
- Process exits with code 137 specifically
- Crashes during large file scans
- More common with large repositories

**Cause:** Exit code 137 = killed by OS due to out-of-memory (OOM).

**Solutions:**

1. **Increase Docker memory**
   - Docker Desktop → Settings → Resources → Memory
   - Set to 8GB or higher
   - Apply & Restart

2. **Close other memory-intensive applications**
   - Check Activity Monitor (macOS) or Task Manager (Windows)
   - Close unused Chrome tabs, IDEs, etc.

3. **Increase system swap space** (Linux)
   ```bash
   # Check current swap
   swapon --show

   # Add swap file if needed (2GB example)
   sudo fallocate -l 2G /swapfile
   sudo chmod 600 /swapfile
   sudo mkswap /swapfile
   sudo swapon /swapfile
   ```

4. **Scan workspace in smaller batches** (future feature)
   - Current version scans everything at once
   - Future versions will support incremental scanning

---

### Performance Issues

#### "Indexing is very slow or never completes"

**Symptoms:**
- Status bar shows "Indexing" for >10 minutes
- Progress notification stuck at same file count
- System becomes unresponsive
- Fans running at high speed

**Cause:** Large repository, insufficient resources, or system under heavy load.

**Solutions:**

1. **Check repository size**
   ```bash
   # Count files in workspace
   find . -type f | wc -l

   # If >10,000 files, indexing can take 5-10 minutes
   ```

2. **Monitor progress in output channel**
   - `View → Output → Maproom`
   - Look for "FileScanned" events
   - Progress should show continuous activity
   - If stopped, process may have crashed

3. **Increase Docker resources**
   - Docker Desktop → Settings → Resources
   - Memory: 8GB
   - CPUs: 4 cores
   - Swap: 2GB
   - Apply & Restart

4. **Close other applications**
   - Free up CPU and RAM
   - Especially other Docker containers
   - Check `docker ps` for other running containers

5. **Check disk I/O**
   ```bash
   # macOS
   sudo fs_usage -f filesys | grep crewchief

   # Linux
   iotop  # requires sudo
   ```

6. **Wait for initial scan to complete**
   - First scan is always slowest
   - Subsequent updates are incremental (much faster)
   - Don't interrupt the initial scan

7. **Exclude large directories** (future feature)
   - Current version scans everything
   - Future versions will support exclusion patterns
   - Workaround: Use smaller workspace folders

---

#### "High CPU usage"

**Symptoms:**
- CPU at 100% for extended periods
- System becomes sluggish
- Fans at maximum speed
- Battery drains quickly (laptops)

**Cause:** Embedding generation is CPU-intensive, especially with Ollama.

**Solutions:**

1. **This is normal during initial indexing**
   - Embedding generation requires significant CPU
   - Should return to normal after scan completes
   - Check status bar: Wait for "Watching" state

2. **Limit Docker CPU usage**
   - Docker Desktop → Settings → Resources
   - CPUs: Reduce to 2 cores (slower but less impact)

3. **Switch to cloud provider** (if local CPU is a problem)
   - Use OpenAI or Google instead of Ollama
   - Offloads computation to cloud
   - Faster on slow/old computers
   - Requires API key and internet

4. **Schedule indexing during idle time** (manual workaround)
   - Disable extension when you need performance
   - Re-enable during breaks/overnight
   - Extension will catch up when reactivated

---

### File Watching Issues

#### "Changes not detected, index becomes stale"

**Symptoms:**
- Modify file but changes not reflected
- Status bar timestamp doesn't update
- Manual search (future) returns outdated results

**Cause:** File watcher crashed or not monitoring workspace.

**Solutions:**

1. **Restart file watchers**
   - Command Palette → `Maproom: Restart Watchers`
   - Check status bar returns to "Watching"

2. **Check output channel for watcher errors**
   - `View → Output → Maproom`
   - Look for "watcher" related errors

3. **Verify file is within workspace**
   - File must be inside the VSCode workspace root
   - Symlinks may not be followed (platform-dependent)

4. **Check file isn't ignored**
   - `.gitignore` patterns are respected
   - Binary files are skipped
   - Check output channel for "Skipping file" messages

5. **Windows-specific**: Enable legacy watching
   - VSCode Settings → Search "legacy"
   - Enable "Files: Legacy Watcher"
   - Reload window

6. **Large workspace**: Increase watch limit (Linux)
   ```bash
   # Check current limit
   cat /proc/sys/fs/inotify/max_user_watches

   # Increase limit (temporary)
   sudo sysctl -w fs.inotify.max_user_watches=524288

   # Make permanent
   echo "fs.inotify.max_user_watches=524288" | sudo tee -a /etc/sysctl.conf
   sudo sysctl -p
   ```

---

## Platform-Specific Issues

### macOS

#### "crewchief-maproom cannot be opened because the developer cannot be verified"

**Cause:** macOS Gatekeeper blocking unsigned binary.

**Solutions:**

1. **Bypass Gatekeeper for this binary**
   ```bash
   # Find extension directory
   EXT_DIR=$(ls -d ~/.vscode/extensions/crewchief.vscode-maproom-* | head -1)

   # Remove quarantine attribute
   xattr -d com.apple.quarantine "$EXT_DIR/bin/"*/crewchief-maproom

   # Reload VSCode
   ```

2. **Or allow in System Settings**
   - System Settings → Privacy & Security
   - Scroll to "Security" section
   - Click "Open Anyway" next to blocked app
   - Reload VSCode

---

#### "Operation not permitted" (macOS Catalina+)

**Cause:** Full Disk Access required for VSCode.

**Solution:**

1. Grant Full Disk Access to VSCode
   - System Settings → Privacy & Security → Full Disk Access
   - Click the lock to unlock
   - Click "+" and add "Visual Studio Code.app"
   - Restart VSCode

---

### Linux

#### "Too many open files"

**Cause:** System file descriptor limit too low.

**Solution:**

```bash
# Check current limit
ulimit -n

# Increase limit (temporary)
ulimit -n 4096

# Make permanent
echo "* soft nofile 4096" | sudo tee -a /etc/security/limits.conf
echo "* hard nofile 4096" | sudo tee -a /etc/security/limits.conf

# Also add to systemd (for Docker)
sudo mkdir -p /etc/systemd/system/docker.service.d
echo "[Service]" | sudo tee /etc/systemd/system/docker.service.d/limit_nofile.conf
echo "LimitNOFILE=4096" | sudo tee -a /etc/systemd/system/docker.service.d/limit_nofile.conf
sudo systemctl daemon-reload
sudo systemctl restart docker
```

---

#### "Permission denied" with Docker (not using sudo)

**Cause:** User not in `docker` group.

**Solution:**

```bash
# Add user to docker group
sudo usermod -aG docker $USER

# Log out and log back in (or run)
newgrp docker

# Verify
docker ps  # Should work without sudo
```

---

### Windows

#### "Windows support is experimental"

**Known issues:**
- File watching may be slow
- Some file events may be missed
- Binary may not run in some environments

**Workarounds:**

1. Use WSL2 if possible
   - Install extension in WSL2 VSCode
   - Better file watching support
   - Full Linux compatibility

2. Report issues
   - Windows support improving in future versions
   - Your feedback helps prioritize fixes

---

#### "Docker Desktop WSL2 backend error"

**Cause:** WSL2 not properly configured.

**Solution:**

1. Install WSL2
   ```powershell
   # Run in PowerShell as Administrator
   wsl --install
   wsl --set-default-version 2
   ```

2. Enable WSL2 in Docker Desktop
   - Docker Desktop → Settings → General
   - Check "Use the WSL2 based engine"
   - Apply & Restart

---

## Advanced Troubleshooting

### Collecting Diagnostic Information

```bash
# System info
uname -a  # macOS/Linux
systeminfo  # Windows

# Docker version
docker --version
docker compose version

# VSCode version
code --version

# Extension info
code --list-extensions --show-versions | grep maproom

# Check Docker containers
docker ps -a

# Check Docker logs
docker logs maproom-postgres
docker logs maproom-ollama
docker logs maproom-mcp

# Check Docker resource usage
docker stats --no-stream

# Check Docker networks
docker network ls
docker network inspect maproom_default

# Check Docker volumes
docker volume ls | grep maproom
```

---

### Manual Service Testing

Test services outside of VSCode:

```bash
# Navigate to extension directory
cd ~/.vscode/extensions/crewchief.vscode-maproom-*

# Start services manually
docker compose -f config/docker-compose.yml up

# In another terminal, test PostgreSQL
docker exec -it maproom-postgres psql -U maproom -d maproom -c "SELECT version();"

# Test Ollama
curl http://localhost:11434

# Test MCP server (port depends on config)
# Check docker compose file for port mapping
```

---

### Reset Everything

Last resort: Complete reset.

```bash
# 1. Stop all containers
docker compose -f config/docker-compose.yml down -v

# 2. Remove extension
code --uninstall-extension crewchief.vscode-maproom
rm -rf ~/.vscode/extensions/crewchief.vscode-maproom-*

# 3. Clear workspace state
# macOS
rm -rf ~/Library/Application\ Support/Code/User/workspaceStorage/*
# Linux
rm -rf ~/.config/Code/User/workspaceStorage/*
# Windows
rmdir /s /q "%APPDATA%\Code\User\workspaceStorage"

# 4. Reinstall extension
code --install-extension vscode-maproom-0.1.0.vsix

# 5. Restart VSCode completely
```

---

## How to Report Bugs

When reporting issues, please include:

### Required Information

1. **Extension version**
   ```bash
   code --list-extensions --show-versions | grep maproom
   ```

2. **Platform details**
   - OS: macOS 14.2 / Ubuntu 22.04 / Windows 11
   - Architecture: x64 / arm64
   - VSCode version: `code --version`

3. **Docker version**
   ```bash
   docker --version
   docker compose version
   ```

4. **Error message**
   - Exact error text from notification or output channel
   - Screenshot if applicable

5. **Output channel logs**
   - See "Log Collection" section below

### Optional but Helpful

- Workspace size: `find . -type f | wc -l`
- Docker resource allocation (Settings → Resources)
- Steps to reproduce
- Whether issue is consistent or intermittent
- Recent changes (VSCode update, Docker update, etc.)

### Where to Report

- GitHub Issues: https://github.com/crewchief/vscode-maproom/issues
- Include "[BUG]" in title
- Use bug report template if available

---

## Log Collection

### Collect Output Channel Logs

1. **Open output channel**
   - `View → Output`
   - Select "Maproom" from dropdown

2. **Copy all text**
   - Right-click in output panel
   - "Select All"
   - Copy (Cmd/Ctrl+C)

3. **Save to file**
   - Paste into text editor
   - Save as `maproom-logs.txt`

4. **Attach to bug report**
   - GitHub allows attaching .txt files
   - Or paste into code block in issue

### Collect Docker Logs

```bash
# Save all container logs
docker logs maproom-postgres > postgres.log 2>&1
docker logs maproom-ollama > ollama.log 2>&1
docker logs maproom-mcp > mcp.log 2>&1

# Attach these files to bug report
```

### Redact Sensitive Information

Before sharing logs, check for:
- API keys (though they shouldn't be in logs)
- File paths with personal information
- Internal hostnames or IP addresses

Replace with placeholders like `[REDACTED]`.

---

## Getting Help

If this guide doesn't solve your issue:

1. **Search existing issues**
   - https://github.com/crewchief/vscode-maproom/issues
   - Someone may have had the same problem

2. **Ask in discussions**
   - https://github.com/crewchief/vscode-maproom/discussions
   - Community may have quick answers

3. **File a bug report**
   - Follow "How to Report Bugs" section above
   - Provide all requested information

4. **Check for updates**
   - Bug may be fixed in newer version
   - VSCode → Extensions → Check for updates

---

**Last updated:** 2025-11-16 (v0.1.0)
