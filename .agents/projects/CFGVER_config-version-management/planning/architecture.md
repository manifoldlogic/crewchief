# Config Version Management - Architecture

## Overview

This document describes the technical architecture for implementing version-based configuration management in the Maproom MCP CLI. The solution prevents config drift by explicitly tracking configuration versions and automatically updating cached configs when the npm package updates.

## Design Principles

1. **Explicit Over Implicit** - Use version markers, not pattern detection
2. **User Transparency** - Show users what's changing and why
3. **Safety First** - Backup before replacing, handle failures gracefully
4. **Zero Configuration** - No user action required for normal updates
5. **Fail Safe** - If update fails, provide clear error and recovery steps

## Architecture Components

### 1. Version Tracking System

#### Version Marker in docker-compose.yml

Add a comment header to the docker-compose.yml template:

```yaml
# Maproom MCP Configuration
# Version: 1.2.3
# DO NOT EDIT: This file is managed by @crewchief/maproom-mcp
# To customize: Use environment variables or .env file
# Package: @crewchief/maproom-mcp@1.2.3
# Generated: 2024-10-30T15:30:00Z

version: '3.8'
services:
  ...
```

**Key fields:**
- `Version`: Package version (from package.json)
- `Package`: Full package identifier
- `Generated`: ISO 8601 timestamp

#### .maproom-version File

Create a version tracking file alongside cached configs:

```json
{
  "package_version": "1.2.3",
  "config_version": "1.2.3",
  "last_updated": "2024-10-30T15:30:00Z",
  "files": {
    "docker-compose.yml": {
      "hash": "sha256:abc123...",
      "size": 2048,
      "last_modified": "2024-10-30T15:30:00Z"
    },
    "init.sql": {
      "hash": "sha256:def456...",
      "size": 1024,
      "last_modified": "2024-10-30T15:30:00Z"
    }
  }
}
```

**Purpose:**
- Track package version
- Store file hashes for integrity checking
- Record update history
- Enable rollback if needed

### 2. Update Detection Logic

Replace pattern-based detection with version comparison:

```javascript
// packages/maproom-mcp/bin/cli.cjs

const CACHE_DIR = path.join(os.homedir(), '.maproom-mcp');
const VERSION_FILE = path.join(CACHE_DIR, '.maproom-version');
const PACKAGE_VERSION = require('../package.json').version;

function needsConfigUpdate() {
  // First run - no version file exists
  if (!fs.existsSync(VERSION_FILE)) {
    return { needsUpdate: true, reason: 'first_run' };
  }

  // Read version tracking file
  const versionData = JSON.parse(fs.readFileSync(VERSION_FILE, 'utf-8'));

  // Compare package versions
  if (versionData.package_version !== PACKAGE_VERSION) {
    return {
      needsUpdate: true,
      reason: 'version_mismatch',
      oldVersion: versionData.package_version,
      newVersion: PACKAGE_VERSION
    };
  }

  // Verify file integrity
  const integrityCheck = verifyFileIntegrity(versionData.files);
  if (!integrityCheck.valid) {
    return {
      needsUpdate: true,
      reason: 'integrity_failure',
      corruptedFiles: integrityCheck.corruptedFiles
    };
  }

  return { needsUpdate: false };
}
```

### 3. Safe Update Process

Multi-step update with rollback capability:

```javascript
async function updateConfigs() {
  const updateCheck = needsConfigUpdate();

  if (!updateCheck.needsUpdate) {
    return { success: true, skipped: true };
  }

  console.log(`⚡ Updating Maproom configuration...`);
  console.log(`   From: ${updateCheck.oldVersion || 'none'}`);
  console.log(`   To: ${PACKAGE_VERSION}`);

  try {
    // Step 1: Backup existing configs
    const backupDir = await backupConfigs();
    console.log(`   Backed up to: ${backupDir}`);

    // Step 2: Stop containers
    await stopContainers();
    console.log(`   Stopped containers`);

    // Step 3: Copy new configs
    await copyNewConfigs();
    console.log(`   Copied new configuration files`);

    // Step 4: Update version tracking
    await updateVersionFile();
    console.log(`   Updated version tracking`);

    // Step 5: Cleanup old containers/volumes if needed
    await cleanupOldResources();
    console.log(`   Cleaned up old resources`);

    console.log(`✅ Configuration updated successfully`);
    return { success: true, version: PACKAGE_VERSION };

  } catch (error) {
    console.error(`❌ Update failed: ${error.message}`);

    // Attempt rollback
    try {
      await rollbackConfigs(backupDir);
      console.log(`⚠️  Rolled back to previous configuration`);
    } catch (rollbackError) {
      console.error(`❌ Rollback failed: ${rollbackError.message}`);
      console.error(`   Manual recovery required. See: ~/.maproom-mcp/backups/`);
    }

    throw error;
  }
}
```

### 4. Backup Strategy

Backup configs before any update:

```javascript
async function backupConfigs() {
  const timestamp = new Date().toISOString().replace(/:/g, '-');
  const backupDir = path.join(CACHE_DIR, 'backups', timestamp);

  await fs.promises.mkdir(backupDir, { recursive: true });

  // Copy all config files
  const filesToBackup = [
    'docker-compose.yml',
    'init.sql',
    'Dockerfile.mcp-server',
    '.maproom-version',
    '.env' // If exists
  ];

  for (const file of filesToBackup) {
    const src = path.join(CACHE_DIR, file);
    const dest = path.join(backupDir, file);

    if (fs.existsSync(src)) {
      await fs.promises.copyFile(src, dest);
    }
  }

  // Keep only last 5 backups
  await cleanupOldBackups();

  return backupDir;
}
```

### 5. Docker Container Cleanup

Handle running containers during updates:

```javascript
async function stopContainers() {
  const composeFile = path.join(CACHE_DIR, 'docker-compose.yml');

  if (!fs.existsSync(composeFile)) {
    return;
  }

  // Stop containers
  await execAsync('docker compose down', { cwd: CACHE_DIR });
}

async function cleanupOldResources() {
  // Remove dangling volumes from old configs
  await execAsync('docker volume prune -f', {
    cwd: CACHE_DIR,
    // Only remove volumes created by maproom
    filter: 'label=com.crewchief.maproom'
  });

  // Remove old images if using different image names
  // (Be careful not to remove user's other images)
}
```

## Integration Points

### CLI Entry Point

Update the CLI entry point to check for updates on every run:

```javascript
#!/usr/bin/env node

const { needsConfigUpdate, updateConfigs } = require('./config-manager');

async function main() {
  // Check for config updates
  const updateCheck = needsConfigUpdate();

  if (updateCheck.needsUpdate) {
    await updateConfigs();
  }

  // Continue with normal CLI flow
  // ...
}

main().catch(console.error);
```

### Docker Compose Template

Update the template to include version markers:

```yaml
# packages/maproom-mcp/config/docker-compose.yml

# Maproom MCP Configuration
# Version: ${PACKAGE_VERSION}
# DO NOT EDIT: This file is managed by @crewchief/maproom-mcp

version: '3.8'

services:
  maproom-postgres:
    image: ankane/pgvector:latest
    # ...
```

### Version File Schema

```typescript
interface VersionFile {
  package_version: string;      // npm package version
  config_version: string;        // config schema version (for future use)
  last_updated: string;          // ISO 8601 timestamp
  files: {
    [filename: string]: {
      hash: string;               // SHA-256 hash
      size: number;               // File size in bytes
      last_modified: string;      // ISO 8601 timestamp
    }
  };
}
```

## Error Handling

### Update Failures

1. **Network Failure** - Can't download new package
   - Keep using existing cached config
   - Show warning but don't fail

2. **Docker Not Available** - Can't stop containers
   - Show error with instructions to stop manually
   - Provide command: `docker compose -f ~/.maproom-mcp/docker-compose.yml down`

3. **Permission Denied** - Can't write to cache directory
   - Show error with fix instructions
   - Suggest: `chmod -R u+w ~/.maproom-mcp`

4. **Corrupted Backup** - Rollback fails
   - Show error with manual recovery steps
   - List backup locations
   - Provide restore commands

### Integrity Failures

If file integrity check fails:

```javascript
function verifyFileIntegrity(files) {
  const corruptedFiles = [];

  for (const [filename, metadata] of Object.entries(files)) {
    const filepath = path.join(CACHE_DIR, filename);

    if (!fs.existsSync(filepath)) {
      corruptedFiles.push({ filename, reason: 'missing' });
      continue;
    }

    const hash = computeHash(filepath);
    if (hash !== metadata.hash) {
      corruptedFiles.push({ filename, reason: 'hash_mismatch' });
    }
  }

  return {
    valid: corruptedFiles.length === 0,
    corruptedFiles
  };
}
```

## User Communication

### Update Messages

**First Run:**
```
⚡ Initializing Maproom configuration...
   Version: 1.2.3
   Location: ~/.maproom-mcp/
✅ Configuration initialized
```

**Version Update:**
```
⚡ Updating Maproom configuration...
   From: 1.2.2
   To: 1.2.3
   Backed up to: ~/.maproom-mcp/backups/2024-10-30T15-30-00Z/
   Stopped containers
   Copied new configuration files
   Updated version tracking
   Cleaned up old resources
✅ Configuration updated successfully
```

**Update Failure:**
```
❌ Update failed: Cannot stop running containers
   Please stop containers manually:
   $ docker compose -f ~/.maproom-mcp/docker-compose.yml down

   Then run again:
   $ npx -y @crewchief/maproom-mcp@latest
```

## Testing Strategy

### Unit Tests

1. **Version Comparison**
   - Test semver comparison logic
   - Edge cases: pre-releases, build metadata

2. **File Integrity**
   - Test hash computation
   - Test missing file detection
   - Test corrupted file detection

3. **Backup/Rollback**
   - Test backup creation
   - Test rollback execution
   - Test cleanup of old backups

### Integration Tests

1. **First Run**
   - No existing config
   - Creates version file
   - Copies all templates

2. **Update Detection**
   - Existing config with old version
   - Triggers update
   - Preserves user .env

3. **Integrity Check**
   - Modify cached file
   - Detects corruption
   - Triggers update

4. **Rollback**
   - Simulate update failure
   - Verifies rollback works
   - Containers can restart

## Performance Considerations

### Optimization Strategies

1. **Fast Path** - Skip checks if version matches (< 10ms)
2. **Lazy Hashing** - Only compute hashes if version matches (integrity check optional)
3. **Parallel Operations** - Stop containers and create backup simultaneously
4. **Incremental Updates** - Only copy changed files (future enhancement)

### Benchmarks

Target performance:
- Version check: < 10ms
- Full update: < 5 seconds
- Rollback: < 3 seconds

## Security Considerations

### Hash Algorithm

Use SHA-256 for file integrity:
- Collision-resistant
- Fast enough for config files
- Standard in Node.js crypto module

### Backup Location

Store backups in user's home directory:
- `~/.maproom-mcp/backups/`
- Readable only by user (chmod 700)
- Automatic cleanup (keep last 5)

### Version File Integrity

The `.maproom-version` file itself should be validated:
- JSON schema validation
- Required fields check
- Graceful degradation if corrupted

## Future Enhancements

### Phase 2 (Optional)

1. **Config Migration Scripts** - Support schema changes between versions
2. **Partial Updates** - Only update changed files
3. **User Config Overrides** - Support custom docker-compose.override.yml
4. **Rollback Command** - Allow manual rollback: `npx maproom-mcp rollback`
5. **Update Notifications** - Show changelog when updating
6. **Dry Run Mode** - Preview updates without applying: `--dry-run`

## References

- Analysis: `analysis.md` - Problem space and industry solutions
- Plan: `plan.md` - Implementation phases and deliverables
- Quality Strategy: `quality-strategy.md` - Testing approach
- Security Review: `security-review.md` - Security considerations
