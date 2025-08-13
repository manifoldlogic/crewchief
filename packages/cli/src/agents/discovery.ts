import fs from 'node:fs'
import path from 'node:path'

export function findAgentDefinitions(baseDir: string): string[] {
  if (!fs.existsSync(baseDir)) return []
  const entries = fs.readdirSync(baseDir, { withFileTypes: true })
  return entries.filter((e) => e.isFile()).map((e) => path.join(baseDir, e.name))
}
