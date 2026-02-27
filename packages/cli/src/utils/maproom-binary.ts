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
 * 1. MAPROOM_BIN environment variable
 * 2. CREWCHIEF_MAPROOM_BIN environment variable (deprecated fallback)
 * 3. configPath from configuration file
 * 4. Global installation (on PATH)
 * 5. Packaged binary (platform-specific)
 *
 * @param options - Binary resolution options
 * @returns Resolution result with path and source
 */
export function findMaproomBinary(options?: MaproomBinaryOptions): BinaryResolutionResult {
  const newExecName = process.platform === 'win32' ? 'maproom.exe' : 'maproom'
  const legacyExecName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'

  // 1. Check MAPROOM_BIN environment variable first
  const maproomBinEnv = process.env.MAPROOM_BIN
  if (maproomBinEnv && fs.existsSync(maproomBinEnv)) {
    return { path: maproomBinEnv, source: 'env' }
  }

  // 2. Check deprecated CREWCHIEF_MAPROOM_BIN environment variable (backward compat)
  const legacyEnvBin = process.env.CREWCHIEF_MAPROOM_BIN
  if (legacyEnvBin && fs.existsSync(legacyEnvBin)) {
    console.warn(
      'Warning: CREWCHIEF_MAPROOM_BIN is deprecated and will be removed in a future release. Use MAPROOM_BIN instead.',
    )
    return { path: legacyEnvBin, source: 'env' }
  }

  // 3. Check config path
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

  // 4. Check global installation — try 'maproom' first, then 'crewchief-maproom' as fallback
  const whichNew = spawnSync('bash', ['-lc', 'command -v maproom'])
  if (whichNew.status === 0) {
    return { path: 'maproom', source: 'global' }
  }

  const whichLegacy = spawnSync('bash', ['-lc', 'command -v crewchief-maproom'])
  if (whichLegacy.status === 0) {
    return { path: 'crewchief-maproom', source: 'global' }
  }

  // 5. Check packaged binary locations
  const arch = process.arch === 'x64' ? 'x64' : process.arch === 'arm64' ? 'arm64' : process.arch
  const platform = `${process.platform}-${arch}`

  try {
    const __dirname = path.dirname(fileURLToPath(import.meta.url))

    // Try platform-specific directory first — new name, then legacy fallback
    const platformPath = path.join(__dirname, '..', 'bin', platform, newExecName)
    if (fs.existsSync(platformPath)) {
      return { path: platformPath, source: 'packaged' }
    }

    const legacyPlatformPath = path.join(__dirname, '..', 'bin', platform, legacyExecName)
    if (fs.existsSync(legacyPlatformPath)) {
      return { path: legacyPlatformPath, source: 'packaged' }
    }

    // Try bin root (backwards compatibility)
    const binRootPath = path.join(__dirname, '..', 'bin', newExecName)
    if (fs.existsSync(binRootPath)) {
      return { path: binRootPath, source: 'packaged' }
    }

    const legacyBinRootPath = path.join(__dirname, '..', 'bin', legacyExecName)
    if (fs.existsSync(legacyBinRootPath)) {
      return { path: legacyBinRootPath, source: 'packaged' }
    }

    // Try sibling maproom-mcp package (monorepo dev convenience)
    const mcpPath = path.join(__dirname, '..', '..', 'maproom-mcp', 'bin', platform, newExecName)
    if (fs.existsSync(mcpPath)) {
      return { path: mcpPath, source: 'packaged' }
    }

    const legacyMcpPath = path.join(__dirname, '..', '..', 'maproom-mcp', 'bin', platform, legacyExecName)
    if (fs.existsSync(legacyMcpPath)) {
      return { path: legacyMcpPath, source: 'packaged' }
    }
  } catch {
    // Ignore errors during packaged binary resolution
  }

  // No binary found
  return { path: null, source: 'not-found' }
}
