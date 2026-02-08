import fs from 'node:fs'
import path from 'node:path'
import type { Platform } from './platforms.js'

/**
 * Scan a platform's agent directory and return the full file paths of all
 * agent definition files whose extensions match `platform.agentExtensions`.
 *
 * Returns an empty array when:
 * - The platform has no `agentDir` (e.g. codex, aider)
 * - The resolved directory does not exist on disk
 */
export function findAgentDefinitions(platform: Platform, projectDir: string): string[] {
  if (!platform.agentDir) {
    return []
  }

  const agentPath = path.join(projectDir, platform.agentDir)

  let entries: fs.Dirent[]
  try {
    entries = fs.readdirSync(agentPath, { withFileTypes: true })
  } catch (err: unknown) {
    if (err instanceof Error && 'code' in err && (err as NodeJS.ErrnoException).code === 'ENOENT') {
      return []
    }
    throw err
  }

  const extensionSet = new Set(platform.agentExtensions)

  return entries
    .filter((e) => e.isFile() && extensionSet.has(path.extname(e.name)))
    .map((e) => path.join(agentPath, e.name))
}
