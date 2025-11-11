import { existsSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { ConfigSchema, CrewChiefConfig } from './schema'
import { logger } from '../utils/logger'

/**
 * Find config file by traversing up directory tree
 * Similar to how tsconfig.json is discovered
 */
function findConfigFile(startDir: string): string | null {
  let currentDir = startDir
  const root = path.parse(currentDir).root

  while (currentDir !== root) {
    // Check for local config first (higher priority)
    const localConfigPath = path.join(currentDir, 'crewchief.config.local.js')
    if (existsSync(localConfigPath)) {
      return localConfigPath
    }

    // Then check for standard config
    const configPath = path.join(currentDir, 'crewchief.config.js')
    if (existsSync(configPath)) {
      return configPath
    }

    // Move up one directory
    const parentDir = path.dirname(currentDir)
    if (parentDir === currentDir) break // Reached root
    currentDir = parentDir
  }

  return null
}

/**
 * Load CrewChief configuration from JavaScript files.
 * Priority order:
 * 1. crewchief.config.local.js (gitignored, for local overrides)
 * 2. crewchief.config.js (committed to repo)
 *
 * Searches current directory and parent directories (like tsconfig.json)
 *
 * @param cwd - Current working directory (defaults to process.cwd())
 * @returns Parsed and validated configuration
 */
export async function loadConfig(cwd: string = process.cwd()): Promise<CrewChiefConfig> {
  // Find config file by traversing up directory tree
  const configPath = findConfigFile(cwd)

  if (!configPath) {
    throw new Error(
      'Missing configuration file. Create one of:\n' +
        '  - crewchief.config.js (standard configuration)\n' +
        '  - crewchief.config.local.js (for local overrides, gitignored)',
    )
  }

  const configType = path.basename(configPath).includes('.local.') ? 'local JavaScript' : 'JavaScript'
  const selectedConfig = { path: configPath, type: configType }

  // Log which config is being used
  logger.info(`Using ${selectedConfig.type} config: ${path.basename(selectedConfig.path)}`)

  // Load the config module
  const moduleUrl = pathToFileURL(selectedConfig.path).href

  try {
    const mod = await import(moduleUrl)
    const raw = mod.default ?? mod

    // Validate the configuration
    const parsed = ConfigSchema.safeParse(raw)
    if (!parsed.success) {
      const issues = parsed.error.issues.map((i) => `${i.path.join('.')}: ${i.message}`).join('\n')
      throw new Error(`Invalid ${path.basename(selectedConfig.path)}:\n${issues}`)
    }

    return parsed.data
  } catch (error) {
    // Re-throw the error with config filename for context
    if (error instanceof Error && !error.message.includes(selectedConfig.path)) {
      throw new Error(`Error loading ${path.basename(selectedConfig.path)}: ${error.message}`)
    }
    throw error
  }
}
