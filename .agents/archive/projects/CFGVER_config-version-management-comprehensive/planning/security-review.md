# Config Version Management - Security Review

## Security Context

This feature manages configuration files and Docker containers on users' local machines. While not handling sensitive user data directly, it has elevated permissions (file system access, Docker control) and must be implemented with security best practices.

## Threat Model

### Assets to Protect

1. **User's Configuration Files** - docker-compose.yml, init.sql, .env
2. **Docker Containers** - Running database with indexed code
3. **File System** - User's home directory (~/.maproom-mcp/)
4. **User's Trust** - In the npm package and update mechanism

### Threat Actors

1. **Malicious npm Package** - Compromised package on registry
2. **Local Attacker** - Someone with access to user's machine
3. **Supply Chain Attack** - Compromised dependency
4. **Accidental Misconfiguration** - Bugs causing security issues

### Attack Vectors

1. **File System Access** - Reading/writing arbitrary files
2. **Command Injection** - Via Docker commands
3. **Path Traversal** - Escaping cache directory
4. **Race Conditions** - Concurrent operations
5. **Data Exposure** - Leaking sensitive information

## Security Risks by Component

### 1. Version File Management

**Risk: Path Traversal**
- **Severity:** Medium
- **Attack:** Malicious version file with paths like `../../../etc/passwd`
- **Impact:** Read/write outside cache directory

**Mitigation:**
```javascript
function sanitizePath(filename) {
  // Ensure filename contains no path separators
  if (filename.includes('/') || filename.includes('\\')) {
    throw new Error('Invalid filename: must not contain path separators');
  }

  // Ensure path stays within cache directory
  const resolved = path.resolve(CACHE_DIR, filename);
  if (!resolved.startsWith(CACHE_DIR)) {
    throw new Error('Invalid path: outside cache directory');
  }

  return resolved;
}
```

**Risk: JSON Injection**
- **Severity:** Low
- **Attack:** Malformed JSON in version file
- **Impact:** DoS (crash), potential prototype pollution

**Mitigation:**
```javascript
function readVersionFile() {
  try {
    const content = fs.readFileSync(VERSION_FILE, 'utf-8');

    // Validate JSON structure before parsing
    const data = JSON.parse(content);

    // Validate schema
    if (!isValidVersionFile(data)) {
      throw new Error('Invalid version file schema');
    }

    return data;
  } catch (error) {
    // Treat corrupted file as missing (triggers update)
    console.warn(`Version file corrupted: ${error.message}`);
    return null;
  }
}

function isValidVersionFile(data) {
  return (
    data &&
    typeof data.package_version === 'string' &&
    typeof data.files === 'object' &&
    !Array.isArray(data.files)
  );
}
```

### 2. File Integrity Checking

**Risk: Hash Collision**
- **Severity:** Low
- **Attack:** Craft file with same SHA-256 hash as legitimate config
- **Impact:** Bypass integrity check

**Mitigation:**
- Use SHA-256 (industry standard, collision-resistant)
- No practical collision attacks known
- Risk acceptable for this use case

**Risk: TOCTOU (Time-of-Check-Time-of-Use)**
- **Severity:** Low
- **Attack:** Modify file between hash check and use
- **Impact:** Use tampered config

**Mitigation:**
```javascript
function safeReadAndVerify(filename, expectedHash) {
  const filepath = sanitizePath(filename);

  // Read file once
  const content = fs.readFileSync(filepath);

  // Compute hash of what we read
  const actualHash = crypto.createHash('sha256')
    .update(content)
    .digest('hex');

  // Verify before using
  if (actualHash !== expectedHash) {
    throw new Error(`Hash mismatch for ${filename}`);
  }

  return content; // Use the verified content
}
```

### 3. Docker Integration

**Risk: Command Injection**
- **Severity:** High
- **Attack:** Inject shell commands via file paths or environment variables
- **Impact:** Arbitrary code execution

**Mitigation:**
```javascript
// UNSAFE:
await execAsync(`docker compose -f ${composeFile} down`);

// SAFE: Use array syntax, no shell interpolation
const { execFile } = require('child_process');
const { promisify } = require('util');
const execFileAsync = promisify(execFile);

async function stopContainers() {
  try {
    await execFileAsync('docker', [
      'compose',
      '-f', COMPOSE_FILE, // Passed as argument, not interpolated
      'down'
    ], {
      cwd: CACHE_DIR,
      timeout: 30000 // 30 second timeout
    });
  } catch (error) {
    // Handle error safely
    throw new Error(`Failed to stop containers: ${error.message}`);
  }
}
```

**Risk: Volume Deletion**
- **Severity:** Medium
- **Attack:** Accidentally delete user's important Docker volumes
- **Impact:** Data loss

**Mitigation:**
```javascript
async function cleanupOldResources() {
  // Only remove volumes with specific label
  await execFileAsync('docker', [
    'volume',
    'prune',
    '-f',
    '--filter', 'label=com.crewchief.maproom=true'
  ]);

  // Never use 'docker system prune' or 'docker volume rm' without filter
}
```

### 4. Backup and Rollback

**Risk: Backup Directory Traversal**
- **Severity:** Medium
- **Attack:** Create backup outside cache directory
- **Impact:** Write arbitrary files on file system

**Mitigation:**
```javascript
function createBackupDir() {
  const timestamp = new Date().toISOString()
    .replace(/[^0-9T-]/g, '-'); // Sanitize timestamp

  const backupDir = path.join(CACHE_DIR, 'backups', timestamp);

  // Ensure backup dir is inside cache dir
  if (!backupDir.startsWith(CACHE_DIR)) {
    throw new Error('Invalid backup directory');
  }

  fs.mkdirSync(backupDir, { recursive: true, mode: 0o700 });
  return backupDir;
}
```

**Risk: Symlink Attack**
- **Severity:** Medium
- **Attack:** Replace config file with symlink to sensitive file
- **Impact:** Backup/modify arbitrary files

**Mitigation:**
```javascript
async function copyFile(src, dest) {
  // Check that source is a regular file, not a symlink
  const stats = fs.lstatSync(src);
  if (!stats.isFile()) {
    throw new Error(`Not a regular file: ${src}`);
  }

  // Copy with permissions preserved
  await fs.promises.copyFile(src, dest);

  // Ensure copied file has safe permissions
  await fs.promises.chmod(dest, 0o600);
}
```

### 5. User Communication

**Risk: Information Disclosure**
- **Severity:** Low
- **Attack:** Error messages reveal sensitive paths or data
- **Impact:** Information leakage

**Mitigation:**
```javascript
function safeErrorMessage(error) {
  // Don't expose full stack traces to users
  console.error(`❌ ${error.message}`);

  // Don't log sensitive paths
  // UNSAFE: console.error(`Failed to read ${filepath}`);
  // SAFE: console.error(`Failed to read configuration file`);

  // Provide recovery steps, not technical details
  console.error('   Try: npx -y @crewchief/maproom-mcp@latest');
}
```

## Security Best Practices

### File Permissions

```javascript
// Cache directory: Only user can access
fs.mkdirSync(CACHE_DIR, { recursive: true, mode: 0o700 });

// Config files: Only user can read/write
fs.writeFileSync(configFile, content, { mode: 0o600 });

// Backups: Only user can access
fs.mkdirSync(backupDir, { recursive: true, mode: 0o700 });
```

### Input Validation

```javascript
// Validate package version format (semver)
function isValidVersion(version) {
  return /^\d+\.\d+\.\d+(-[a-z0-9.]+)?(\+[a-z0-9.]+)?$/i.test(version);
}

// Validate filenames
function isValidFilename(filename) {
  // Only alphanumeric, dash, underscore, dot
  return /^[a-z0-9._-]+$/i.test(filename);
}

// Validate file hashes
function isValidHash(hash) {
  // SHA-256 produces 64 hex characters
  return /^sha256:[a-f0-9]{64}$/i.test(hash);
}
```

### Error Handling

```javascript
// Never expose sensitive information in errors
try {
  await updateConfigs();
} catch (error) {
  // Log detailed error for debugging (not shown to user)
  console.debug('Update error:', error.stack);

  // Show safe error to user
  console.error('❌ Configuration update failed');
  console.error('   Please check permissions and try again');

  // Exit with error code
  process.exit(1);
}
```

## Dependency Security

### npm Audit

Run regularly to check for vulnerable dependencies:

```bash
cd packages/maproom-mcp
npm audit

# Fix vulnerabilities automatically
npm audit fix
```

### Minimal Dependencies

This feature should use only:
- Node.js built-ins (fs, path, crypto, child_process)
- No additional dependencies

**Rationale:** Fewer dependencies = smaller attack surface

### Supply Chain Security

**Package Integrity:**
```json
{
  "scripts": {
    "prepublishOnly": "npm audit && npm test"
  }
}
```

**Lock File:**
- Commit package-lock.json
- Use `npm ci` in CI/CD (not `npm install`)
- Regularly update dependencies

## Docker Security

### Image Trust

```yaml
# Use official images from trusted sources
services:
  maproom-postgres:
    image: ankane/pgvector:latest
    # Consider pinning to specific version:
    # image: ankane/pgvector:v0.5.1
```

### Container Isolation

```yaml
services:
  maproom-postgres:
    # Run as non-root user
    user: postgres

    # Drop unnecessary capabilities
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - DAC_OVERRIDE
      - SETGID
      - SETUID

    # Read-only root filesystem where possible
    read_only: true
    tmpfs:
      - /tmp
      - /var/run/postgresql
```

### Network Isolation

```yaml
networks:
  maproom-network:
    driver: bridge
    internal: true  # No external access
```

## Compliance Considerations

### GDPR (if applicable)

This feature doesn't collect user data, but:
- Cache directory contains user's code (sensitive)
- Backups contain configuration (may include credentials in .env)
- All data stays local (good for privacy)

**Recommendation:** Document in privacy policy that code is indexed locally, not transmitted to external services.

### License Compliance

- Use only MIT/Apache-2.0 licensed dependencies
- Clearly state license in package.json
- Include LICENSE file in package

## Security Checklist

Before release:

- [ ] All file operations use sanitizePath()
- [ ] All Docker commands use execFileAsync (no shell interpolation)
- [ ] File permissions set to 0o600 for configs, 0o700 for directories
- [ ] Error messages don't leak sensitive information
- [ ] Input validation on all external inputs (version file, filenames, hashes)
- [ ] No dependencies with known vulnerabilities (npm audit)
- [ ] Code review by second developer
- [ ] Manual security testing (attempt path traversal, command injection)

## Incident Response

If a security issue is discovered:

1. **Assess Severity** - Critical, High, Medium, Low
2. **Patch Quickly** - Fix and publish patched version
3. **Notify Users** - GitHub security advisory + npm deprecate old versions
4. **Post-Mortem** - Document what happened and how to prevent

## Known Security Limitations

### Not Addressed (Out of Scope for MVP)

1. **Code Signing** - npm packages aren't signed
   - **Risk:** Package could be compromised on registry
   - **Mitigation:** Trust npm's infrastructure
   - **Future:** Use npm provenance when available

2. **Encrypted Backups** - Backups stored in plaintext
   - **Risk:** Attacker with file system access can read backups
   - **Mitigation:** File permissions (0o700) prevent other users
   - **Future:** Encrypt backups if needed

3. **Audit Logging** - No log of update operations
   - **Risk:** Can't detect unauthorized changes
   - **Mitigation:** Version file provides some audit trail
   - **Future:** Add structured logging if needed

## Security Assessment

### Overall Risk Level: **LOW**

**Reasoning:**
1. Runs locally (not a network service)
2. No sensitive data transmission
3. Limited attack surface (file operations only)
4. User already trusts npm package (running npx)

### Key Security Properties

✅ **Defense in Depth** - Multiple validation layers
✅ **Least Privilege** - Only accesses ~/.maproom-mcp/
✅ **Fail Safe** - Errors don't leave system in bad state
✅ **Input Validation** - All external inputs validated
✅ **Safe Defaults** - Secure permissions and settings

### Recommended Security Enhancements (Future)

1. **File Locking** - Prevent concurrent updates
2. **Integrity Checks** - Verify npm package signature
3. **Audit Logging** - Log all config changes
4. **Encryption** - Encrypt backups at rest

## Conclusion

This feature has **acceptable security risk** for an MVP release. The mitigations in place cover the most likely attack vectors, and the failure modes are safe (rollback, clear error messages, no data loss).

Focus on:
1. **Safe file operations** (path sanitization, permissions)
2. **Safe Docker commands** (no shell injection)
3. **Input validation** (version files, filenames, hashes)
4. **Clear error handling** (no information disclosure)

Ship with confidence, monitor for issues, iterate based on feedback.
