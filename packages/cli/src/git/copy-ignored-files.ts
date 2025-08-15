import fs from 'node:fs/promises'
import path from 'node:path'
import { glob } from 'glob'
import ignore from 'ignore'
import { CrewChiefConfig } from '../config/schema'

export interface CopyIgnoredFilesOptions {
  sourceRoot: string
  worktreeRoot: string
  config: CrewChiefConfig
  dryRun?: boolean
}

export interface CopyResult {
  copied: string[]
  skipped: string[]
  errors: Array<{ file: string; error: string }>
}

/**
 * Validates a pattern to ensure it doesn't escape the repository root
 */
function validatePattern(pattern: string): void {
  // Remove leading '!' for exclusion patterns
  const cleanPattern = pattern.startsWith('!') ? pattern.slice(1) : pattern

  // Prevent directory traversal
  if (cleanPattern.includes('../') || cleanPattern.startsWith('/')) {
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
    /api.*key/i,
  ]

  for (const sensitive of sensitivePatterns) {
    if (sensitive.test(cleanPattern)) {
      console.warn(`⚠️  Pattern "${pattern}" may match sensitive files. Ensure proper security.`)
    }
  }
}

/**
 * Gets the list of ignored files that should be copied based on config patterns
 */
async function getIgnoredFilesToCopy(sourceRoot: string, patterns: string[]): Promise<string[]> {
  // Read .gitignore file
  let gitignoreContent = ''
  try {
    gitignoreContent = await fs.readFile(path.join(sourceRoot, '.gitignore'), 'utf-8')
  } catch {
    // No .gitignore file, so no files are ignored
    return []
  }

  const ig = ignore().add(gitignoreContent)
  const filesToCopy: Set<string> = new Set()
  const excludePatterns: string[] = []

  // Separate inclusion and exclusion patterns
  for (const pattern of patterns) {
    validatePattern(pattern)

    if (pattern.startsWith('!')) {
      excludePatterns.push(pattern.slice(1))
    } else {
      // Find matching files
      const matches = await glob(pattern, {
        cwd: sourceRoot,
        dot: true,
        absolute: false,
        nodir: true, // Only match files, not directories
      })

      // Only include files that are actually ignored by git
      for (const file of matches) {
        if (ig.ignores(file)) {
          filesToCopy.add(file)
        }
      }
    }
  }

  // Remove excluded files
  if (excludePatterns.length > 0) {
    const excludeIg = ignore().add(excludePatterns)
    for (const file of filesToCopy) {
      if (excludeIg.ignores(file)) {
        filesToCopy.delete(file)
      }
    }
  }

  return Array.from(filesToCopy)
}

/**
 * Copies ignored files from source to worktree based on config
 */
export async function copyIgnoredFiles(options: CopyIgnoredFilesOptions): Promise<CopyResult> {
  const { sourceRoot, worktreeRoot, config, dryRun = false } = options
  const result: CopyResult = {
    copied: [],
    skipped: [],
    errors: [],
  }

  // Check if copying is configured
  const patterns = config.worktree?.copyIgnoredFiles
  if (!patterns || patterns.length === 0) {
    return result
  }

  const copyFromPath = path.join(sourceRoot, config.worktree?.copyFromPath || '.')
  const overwriteStrategy = config.worktree?.overwriteStrategy || 'skip'

  console.log('🔍 Finding ignored files to copy...')

  // Get list of files to copy
  const files = await getIgnoredFilesToCopy(copyFromPath, patterns)

  if (files.length === 0) {
    console.log('No ignored files found matching patterns.')
    return result
  }

  console.log(`Found ${files.length} file(s) to copy`)

  for (const file of files) {
    const sourcePath = path.join(copyFromPath, file)
    const destPath = path.join(worktreeRoot, file)

    try {
      // Check if source exists
      try {
        await fs.access(sourcePath)
      } catch {
        console.warn(`⚠️  Source file not found: ${file}`)
        result.errors.push({ file, error: 'Source file not found' })
        continue
      }

      // Check if destination exists
      let destExists = false
      try {
        await fs.access(destPath)
        destExists = true
      } catch {
        // Destination doesn't exist, which is fine
      }

      if (destExists) {
        switch (overwriteStrategy) {
          case 'skip':
            console.log(`⏭️  Skipping existing file: ${file}`)
            result.skipped.push(file)
            continue
          case 'backup':
            if (!dryRun) {
              const backupPath = `${destPath}.backup.${Date.now()}`
              await fs.rename(destPath, backupPath)
              console.log(`💾 Backed up: ${file} -> ${path.basename(backupPath)}`)
            } else {
              console.log(`[DRY RUN] Would backup: ${file}`)
            }
            break
          case 'overwrite':
            console.log(`🔄 Overwriting: ${file}`)
            break
        }
      }

      if (!dryRun) {
        // Ensure destination directory exists
        const destDir = path.dirname(destPath)
        await fs.mkdir(destDir, { recursive: true })

        // Copy the file
        await fs.copyFile(sourcePath, destPath)
        console.log(`✅ Copied: ${file}`)
      } else {
        console.log(`[DRY RUN] Would copy: ${file}`)
      }

      result.copied.push(file)
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error)
      console.error(`❌ Failed to copy ${file}: ${errorMsg}`)
      result.errors.push({ file, error: errorMsg })
    }
  }

  // Print summary
  console.log('\n📋 Copy Summary:')
  console.log(`   Copied: ${result.copied.length} file(s)`)
  if (result.skipped.length > 0) {
    console.log(`   Skipped: ${result.skipped.length} file(s)`)
  }
  if (result.errors.length > 0) {
    console.log(`   Errors: ${result.errors.length} file(s)`)
  }

  return result
}
