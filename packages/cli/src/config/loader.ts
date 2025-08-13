import { existsSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { ConfigSchema, CrewChiefConfig } from './schema'

export async function loadConfig(cwd: string = process.cwd()): Promise<CrewChiefConfig> {
  const configPathTs = path.join(cwd, 'crewchief.config.ts')
  if (!existsSync(configPathTs)) {
    throw new Error('Missing crewchief.config.ts in project root')
  }
  const moduleUrl = pathToFileURL(configPathTs).href
  const mod = await import(moduleUrl)
  const raw = mod.default ?? mod
  const parsed = ConfigSchema.safeParse(raw)
  if (!parsed.success) {
    const issues = parsed.error.issues.map((i) => `${i.path.join('.')}: ${i.message}`).join('\n')
    throw new Error(`Invalid crewchief.config.ts\n${issues}`)
  }
  return parsed.data
}
