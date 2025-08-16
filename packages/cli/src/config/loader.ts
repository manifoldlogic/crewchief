import { existsSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { ConfigSchema, CrewChiefConfig } from './schema'
import { logger } from '../utils/logger'

/**
 * Load CrewChief configuration from JavaScript files.
 * Priority order:
 * 1. crewchief.config.local.js (gitignored, for local overrides)
 * 2. crewchief.config.js (committed to repo)
 * 3. crewchief.config.ts (legacy TypeScript format)
 * 
 * @param cwd - Current working directory (defaults to process.cwd())
 * @returns Parsed and validated configuration
 */
export async function loadConfig(cwd: string = process.cwd()): Promise<CrewChiefConfig> {
  // Define config file paths in priority order
  const configPaths = [
    { path: path.join(cwd, 'crewchief.config.local.js'), type: 'local JavaScript' },
    { path: path.join(cwd, 'crewchief.config.js'), type: 'JavaScript' },
    { path: path.join(cwd, 'crewchief.config.ts'), type: 'TypeScript' },
  ]
  
  // Find which config files exist
  const existingConfigs = configPaths.filter(c => existsSync(c.path))
  
  if (existingConfigs.length === 0) {
    throw new Error(
      'Missing configuration file. Create one of:\n' +
      '  - crewchief.config.js (plain JavaScript, recommended)\n' +
      '  - crewchief.config.local.js (for local overrides, gitignored)\n' +
      '  - crewchief.config.ts (TypeScript, requires tsx/ts-node)'
    )
  }
  
  // Use the first existing config (highest priority)
  const selectedConfig = existingConfigs[0]
  
  // Log which config is being used and which are being ignored
  if (existingConfigs.length > 1) {
    logger.info(`Using ${selectedConfig.type} config: ${path.basename(selectedConfig.path)}`)
    const ignored = existingConfigs.slice(1).map(c => path.basename(c.path))
    logger.warn(`Ignoring lower priority config(s): ${ignored.join(', ')}`)
  }
  
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
    // Provide helpful error for TypeScript configs that need tsx
    if (selectedConfig.type === 'TypeScript' && error instanceof Error) {
      if (error.message.includes('Cannot use import statement') || 
          error.message.includes('Unexpected token') ||
          error.message.includes('export')) {
        throw new Error(
          `Failed to load TypeScript config. TypeScript configs require tsx or ts-node.\n` +
          `Consider converting to JavaScript:\n` +
          `  1. Rename crewchief.config.ts to crewchief.config.js\n` +
          `  2. Change 'export default' to 'module.exports ='\n` +
          `Original error: ${error.message}`
        )
      }
    }
    throw error
  }
}