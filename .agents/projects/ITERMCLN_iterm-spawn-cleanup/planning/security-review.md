# Security Review: iTerm Spawn Command Cleanup

## Scope

This review covers security implications of:
1. Removing dead JSON-RPC code
2. Consolidating terminal provider implementations
3. Adding headless messaging via stdin pipe (NOT file-based IPC)

## Risk Assessment

### Overall Risk: LOW

This is a cleanup project that primarily removes code and consolidates existing patterns. No new attack surfaces are introduced.

## Current Security Analysis

### 1. Command Injection Risk

**Location**: Python script invocation via `spawnSync`

**Current Pattern**:
```typescript
// iterm-simple.service.ts
const args = [scriptPath, '--to', targetLabel, '--text', text]
spawnSync('python3', args, { encoding: 'utf-8', stdio: 'pipe' })
```

**Assessment**: **LOW RISK**
- Arguments passed as array (not shell string)
- `spawnSync` with array args prevents shell injection
- Python scripts use `argparse` which handles escaping

**Recommendation**: No change needed. Pattern is correct.

### 2. File System Access

**Location**: Python scripts read/write to iTerm2 sessions

**Assessment**: **LOW RISK**
- Scripts only interact with iTerm2 API
- No arbitrary file read/write
- Working directory changes are user-initiated

### 3. Process Spawning

**Location**: HeadlessProvider spawns child processes

**Current Pattern** (HeadlessProvider, lines 65-69):
```typescript
spawn(command, {
  shell: true,  // NOTE: Uses shell execution
  stdio: 'pipe',
  detached: false,
})
```

**Assessment**: **LOW-MEDIUM RISK**
- Commands are from controlled agent registry
- User cannot inject arbitrary commands via CLI
- Working directory is validated path
- **HOWEVER**: Uses `shell: true` which passes command through shell

**Note**: The `shell: true` option is used because the command is a single string (agent command like "claude"). This is acceptable because:
1. Commands come from a controlled agent registry, not user input
2. The command string doesn't include user-controlled arguments
3. CLI tools like Claude typically need shell environment

**Recommendation**:
- Consider validating `agentType` against known list
- Phase 3 should consider switching to array-style args: `spawn(command, args, { shell: false })` if agent commands can be parsed

### 4. Inter-Process Communication (New)

**Proposed**: Stdin pipe messaging for headless agents (NOT file-based)

**Design**:
```typescript
// HeadlessProvider spawns with stdio: 'pipe'
const child = spawn(command, { stdio: 'pipe' })

// Messages sent via stdin
child.stdin.write(message + '\n')
```

**Assessment**: **LOW RISK**
- Stdin pipes are standard OS-level IPC
- No file system exposure
- Messages go directly to process, not stored
- No cleanup required

**Why stdin over file-based IPC**:
1. Simpler implementation
2. No file permission concerns
3. No cleanup needed
4. No race conditions
5. Standard pattern used by many CLI tools

## Security Improvements from Cleanup

### Removing JSON-RPC Server

The dead `iterm_bridge.py` contained:
- HTTP server on localhost:8765
- No authentication mechanism
- JSON-RPC handlers with session access

**Risk Removed**: Local privilege escalation via unauthenticated RPC

While the server was never actually started (broken code path), removing it eliminates future accidental enablement.

### Reducing Attack Surface

| Component | Before | After | Risk Change |
|-----------|--------|-------|-------------|
| TypeScript code | ~1,200 lines | ~600 lines | ↓ 50% less surface |
| Python code | ~2,000 lines | ~500 lines | ↓ 75% less surface |
| HTTP endpoints | 2 (dead) | 0 | ✓ Eliminated |
| IPC mechanisms | 0 | 1 (stdin) | ↔ Standard pattern |

## Threat Model

### In Scope

1. **Malicious agent prompt injection** - Agents receive prompts, could be manipulated
   - Mitigation: Out of scope for this project (agent CLI responsibility)

2. **Stdin tampering** - Another process could write to agent stdin
   - Mitigation: Stdin pipes are process-local, only parent can write

3. **Process escape** - Spawned agents could access system
   - Mitigation: Out of scope (agents run with user privileges by design)

### Out of Scope

- Network-based attacks (no network listeners)
- Authentication/authorization (single-user CLI tool)
- Data encryption (no sensitive data stored)
- Privilege escalation (runs as user)

## Code Patterns to Verify

### Safe Command Execution

```typescript
// GOOD: Array arguments, no shell
spawnSync('python3', [script, '--arg', userInput])

// BAD: Shell string interpolation
spawnSync(`python3 ${script} --arg ${userInput}`, { shell: true })
```

### Safe File Operations

```typescript
// GOOD: Path.join with validated base
const file = join(SAFE_BASE_DIR, sanitizedFilename)

// BAD: Direct path concatenation
const file = userInput + '/config.json'
```

### Verified in Codebase

- [x] All `spawnSync` calls use array arguments (ITermSimpleService)
- [x] All file paths use `path.join` with known bases
- [ ] No `shell: true` options in spawn calls - **FALSE: HeadlessProvider uses `shell: true`**
- [x] No `eval()` or dynamic code execution

**Note on `shell: true`**: HeadlessProvider at line 65-66 uses `shell: true`. This is acceptable because the command string comes from controlled agent registry, not user input. See "Process Spawning" section above for details.

## Recommendations

### Must Fix (Before Release)

None identified. Current patterns are secure.

### Should Fix (This Project)

1. **Validate agent types** against known list
   ```typescript
   const KNOWN_AGENTS = ['claude', 'gemini', 'codex', 'gpt', 'cursor']
   if (!KNOWN_AGENTS.includes(agentType)) {
     throw new AgentError('Unknown agent type', 'INVALID_AGENT')
   }
   ```

2. **Consider array-style spawn in Phase 3** (optional improvement)
   ```typescript
   // Current: shell: true with command string
   spawn('claude', { shell: true })

   // Better: array args without shell
   spawn('claude', [], { shell: false })
   ```
   This would eliminate shell injection risk entirely, but requires agent registry to provide executable paths.

### Nice to Have (Future)

1. **Audit logging** for agent spawn/close operations
2. **Rate limiting** for message sending (prevent spam)
3. **Message size limits** to prevent memory issues

## Compliance Notes

- No PII handling
- No credential storage
- No network services
- User-initiated actions only

This is a developer tool running with user privileges on their local machine. Enterprise security controls are not applicable.

## Sign-off

- [x] No command injection vectors (commands from controlled registry)
- [x] No arbitrary file access
- [x] No unauthenticated network services
- [x] IPC mechanism is stdin pipe (secure by design)
- [x] Dead code with security risks removed (JSON-RPC bridge)
- [ ] `shell: true` acknowledged and risk accepted (controlled command source)
