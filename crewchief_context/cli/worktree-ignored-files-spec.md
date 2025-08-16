# Specification: Copying Ignored Files to Worktrees

## Problem Statement

When creating git worktrees for agents, certain files that are intentionally git-ignored (like `.env` files, local configuration, API keys, etc.) are not available in the new worktree. This requires manual copying of these files after worktree creation, which is error-prone and breaks automation.

## Solution Overview

Add a configuration option to `crewchief.config.ts` that specifies which ignored files should be automatically copied from the main repository to newly created worktrees.

## Configuration Schema

### Option 1: Simple Pattern List (Recommended)

```typescript
// crewchief.config.ts
export default {
  // ... existing config ...
  
  worktree: {
    // Copy these ignored files/patterns to new worktrees
    copyIgnoredFiles: [
      '.env',
      '.env.local',
      '.env.*.local',
      'config/secrets.json',
      'certificates/*.pem',
      '!.env.example'  // Explicitly exclude even if matched above
    ],
    
    // Optional: Source directory override (defaults to repository root)
    copyFromPath: '.',
    
    // Optional: How to handle existing files in worktree
    overwriteStrategy: 'skip' | 'overwrite' | 'backup' // default: 'skip'
  }
}
```

### Option 2: Detailed Configuration

```typescript
// crewchief.config.ts
export default {
  worktree: {
    copyIgnoredFiles: [
      // Simple string patterns
      '.env',
      
      // Object configuration for more control
      {
        pattern: '.env.*.local',
        overwrite: true,
        required: false, // Don't fail if source doesn't exist
      },
      
      // Copy with transformation
      {
        pattern: 'config/dev.secrets.json',
        destination: 'config/secrets.json',
        transform: (content: string) => {
          // Optional: Transform content during copy
          return content.replace(/DEV_/g, 'LOCAL_')
        }
      },
      
      // Directory copy
      {
        pattern: 'certificates/',
        recursive: true,
        exclude: ['*.old', '*.backup']
      }
    ]
  }
}
```

## Implementation Details

### 1. File Detection

```typescript
// src/git/worktrees.ts

import { minimatch } from 'minimatch'
import ignore from 'ignore'

async function getIgnoredFilesToCopy(
  config: CrewChiefConfig,
  repoRoot: string
): Promise<string[]> {
  const patterns = config.worktree?.copyIgnoredFiles || []
  const gitignore = await fs.readFile(path.join(repoRoot, '.gitignore'), 'utf-8')
  const ig = ignore().add(gitignore)
  
  const filesToCopy: string[] = []
  
  for (const pattern of patterns) {
    // Handle exclusion patterns
    if (pattern.startsWith('!')) continue
    
    // Find matching files
    const matches = await glob(pattern, {
      cwd: repoRoot,
      dot: true,
      absolute: false
    })
    
    // Only include files that are actually ignored
    for (const file of matches) {
      if (ig.ignores(file)) {
        filesToCopy.push(file)
      }
    }
  }
  
  return filesToCopy
}
```

### 2. Copy Process

```typescript
// src/git/worktrees.ts

async function copyIgnoredFiles(
  sourceRoot: string,
  worktreeRoot: string,
  config: CrewChiefConfig
): Promise<void> {
  const files = await getIgnoredFilesToCopy(config, sourceRoot)
  const overwriteStrategy = config.worktree?.overwriteStrategy || 'skip'
  
  for (const file of files) {
    const sourcePath = path.join(sourceRoot, file)
    const destPath = path.join(worktreeRoot, file)
    
    // Check if source exists
    if (!await fs.pathExists(sourcePath)) {
      console.warn(`Source file not found: ${file}`)
      continue
    }
    
    // Check if destination exists
    if (await fs.pathExists(destPath)) {
      switch (overwriteStrategy) {
        case 'skip':
          console.log(`Skipping existing file: ${file}`)
          continue
        case 'backup':
          const backupPath = `${destPath}.backup.${Date.now()}`
          await fs.move(destPath, backupPath)
          console.log(`Backed up existing file: ${file} -> ${backupPath}`)
          break
        case 'overwrite':
          // Will overwrite below
          break
      }
    }
    
    // Ensure destination directory exists
    await fs.ensureDir(path.dirname(destPath))
    
    // Copy the file
    await fs.copy(sourcePath, destPath)
    console.log(`Copied ignored file: ${file}`)
  }
}
```

### 3. Integration with Worktree Creation

```typescript
// src/git/worktrees.ts

export async function createWorktree(
  branchName: string,
  options?: CreateWorktreeOptions
): Promise<string> {
  // ... existing worktree creation logic ...
  
  const worktreePath = path.join(WORKTREES_DIR, branchName)
  
  // Create the git worktree
  await simpleGit.worktree('add', worktreePath, branchName)
  
  // Copy ignored files if configured
  const config = await loadConfig()
  if (config.worktree?.copyIgnoredFiles?.length > 0) {
    console.log('Copying ignored files to worktree...')
    await copyIgnoredFiles(repoRoot, worktreePath, config)
  }
  
  return worktreePath
}
```

## Security Considerations

1. **Pattern Validation**: Ensure patterns don't escape the repository root
2. **Sensitive File Detection**: Warn when copying files with sensitive patterns (keys, tokens, etc.)
3. **Audit Logging**: Log all copied files for security audit

```typescript
function validatePattern(pattern: string): void {
  // Prevent directory traversal
  if (pattern.includes('../') || pattern.startsWith('/')) {
    throw new Error(`Invalid pattern: ${pattern}. Patterns must be relative to repo root.`)
  }
  
  // Warn about potentially sensitive files
  const sensitivePatterns = [
    /private.*key/i,
    /\.pem$/,
    /\.pfx$/,
    /\.p12$/,
    /password/i,
    /secret/i,
    /token/i,
    /api.*key/i
  ]
  
  for (const sensitive of sensitivePatterns) {
    if (sensitive.test(pattern)) {
      console.warn(`⚠️  Pattern "${pattern}" may match sensitive files. Ensure proper security.`)
    }
  }
}
```

## Usage Examples

### Basic .env File Copy

```typescript
// crewchief.config.ts
export default {
  worktree: {
    copyIgnoredFiles: ['.env', '.env.local']
  }
}
```

### Development Setup with Multiple Configs

```typescript
// crewchief.config.ts
export default {
  worktree: {
    copyIgnoredFiles: [
      '.env',
      '.env.*.local',
      'config/local-settings.json',
      'certificates/dev-cert.pem',
      'certificates/dev-key.pem',
      '!.env.example',
      '!.env.test'
    ],
    overwriteStrategy: 'skip'
  }
}
```

### Monorepo with Package-Specific Configs

```typescript
// crewchief.config.ts
export default {
  worktree: {
    copyIgnoredFiles: [
      'packages/*/.env',
      'packages/*/config.local.js',
      'packages/api/certificates/*.pem',
      'shared-secrets.json'
    ]
  }
}
```

## CLI Integration

Add command-line options for manual control:

```bash
# Create worktree with ignored files
crewchief worktree create my-feature --copy-ignored

# Create worktree without ignored files (override config)
crewchief worktree create my-feature --no-copy-ignored

# Copy ignored files to existing worktree
crewchief worktree copy-ignored my-feature

# List what would be copied (dry run)
crewchief worktree copy-ignored my-feature --dry-run
```

## Migration Path

1. **Phase 1**: Implement basic pattern matching with simple string patterns
2. **Phase 2**: Add overwrite strategies and basic validation
3. **Phase 3**: Add advanced features (transformations, detailed config objects)
4. **Phase 4**: Add CLI commands for manual control

## Testing Strategy

```typescript
// tests/worktree-ignored-files.test.ts

describe('Ignored Files Copy', () => {
  it('should copy .env file to new worktree', async () => {
    // Create .env in main repo
    await fs.writeFile('.env', 'API_KEY=test123')
    
    // Create worktree with config
    const config = {
      worktree: {
        copyIgnoredFiles: ['.env']
      }
    }
    
    const worktreePath = await createWorktree('test-branch', { config })
    
    // Verify .env exists in worktree
    const envPath = path.join(worktreePath, '.env')
    expect(await fs.pathExists(envPath)).toBe(true)
    expect(await fs.readFile(envPath, 'utf-8')).toBe('API_KEY=test123')
  })
  
  it('should respect overwrite strategy', async () => {
    // ... test different overwrite strategies ...
  })
  
  it('should handle missing source files gracefully', async () => {
    // ... test missing file handling ...
  })
  
  it('should validate patterns for security', async () => {
    // ... test pattern validation ...
  })
})
```

## Alternatives Considered

1. **Symlinks**: Create symlinks to ignored files in main repo
   - Pros: No duplication, always up-to-date
   - Cons: Can break if main repo files change, issues on Windows

2. **Git Sparse Checkout**: Use git's sparse-checkout to include ignored files
   - Pros: Native git feature
   - Cons: Complex configuration, not designed for this use case

3. **Separate Config Repository**: Store sensitive configs in a separate repo
   - Pros: Better security separation
   - Cons: More complex setup, additional dependency

4. **Environment Variable Injection**: Pass configs via environment instead of files
   - Pros: No file copying needed
   - Cons: Limited to simple key-value configs, not suitable for certificates

## Conclusion

This feature would significantly improve the developer experience when working with worktrees that need access to local configuration files. The implementation is straightforward and provides good defaults while allowing customization for complex scenarios.
