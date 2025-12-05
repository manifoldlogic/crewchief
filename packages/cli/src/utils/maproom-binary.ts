import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { logger } from './logger'

export interface MaproomBinaryOptions {
  configPath?: string // from config.repository.maproomBinaryPath
  configFileLocation?: string // location of the config file for relative path resolution
}

export interface BinaryResolutionResult {
  path: string | null
  source: 'env' | 'config' | 'global' | 'packaged' | 'not-found'
}

/**
 * Find the maproom binary using the following precedence order:
 * 1. CREWCHIEF_MAPROOM_BIN environment variable
 * 2. configPath from configuration file
 * 3. Global installation (on PATH)
 * 4. Packaged binary (platform-specific)
 *
 * @param options - Binary resolution options
 * @returns Resolution result with path and source
 */
export function findMaproomBinary(options?: MaproomBinaryOptions): BinaryResolutionResult {
  const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'

  // 1. Check environment variable first
  const envBin = process.env.CREWCHIEF_MAPROOM_BIN
  if (envBin && fs.existsSync(envBin)) {
    return { path: envBin, source: 'env' }
  }

  // 2. Check config path
  if (options?.configPath) {
    let resolvedConfigPath = options.configPath

    // If it's a relative path and we have a config file location, resolve from there
    if (!path.isAbsolute(options.configPath) && options.configFileLocation) {
      const configDir = path.dirname(options.configFileLocation)
      resolvedConfigPath = path.resolve(configDir, options.configPath)
    }

    if (fs.existsSync(resolvedConfigPath)) {
      return { path: resolvedConfigPath, source: 'config' }
    } else {
      logger.warn(`Configured maproom binary path not found: ${resolvedConfigPath}`)
    }
  }

  // 3. Check global installation
  const which = spawnSync('bash', ['-lc', 'command -v crewchief-maproom'])
  if (which.status === 0) {
    return { path: 'crewchief-maproom', source: 'global' }
  }

  // 4. Check packaged binary locations
  const arch = process.arch === 'x64' ? 'x64' : process.arch === 'arm64' ? 'arm64' : process.arch
  const platform = `${process.platform}-${arch}`

  try {
    const __dirname = path.dirname(fileURLToPath(import.meta.url))

    // Try platform-specific directory first
    const platformPath = path.join(__dirname, '..', 'bin', platform, execName)
    if (fs.existsSync(platformPath)) {
      return { path: platformPath, source: 'packaged' }
    }

    // Try bin root (backwards compatibility)
    const binRootPath = path.join(__dirname, '..', 'bin', execName)
    if (fs.existsSync(binRootPath)) {
      return { path: binRootPath, source: 'packaged' }
    }

    // Try sibling maproom-mcp package (monorepo dev convenience)
    const mcpPath = path.join(__dirname, '..', '..', 'maproom-mcp', 'bin', platform, execName)
    if (fs.existsSync(mcpPath)) {
      return { path: mcpPath, source: 'packaged' }
    }
  } catch {
    // Ignore errors during packaged binary resolution
  }

  // No binary found
  return { path: null, source: 'not-found' }
}
