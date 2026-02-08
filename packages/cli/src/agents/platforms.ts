import fs from 'node:fs'
import path from 'node:path'

/**
 * Describes a supported AI coding platform and how to discover its agent definitions.
 */
export interface Platform {
  /** Unique platform identifier (e.g. 'claude', 'gemini') */
  name: string
  /** CLI command used to invoke the platform */
  command: string
  /** Relative directory containing agent definition files, or null if the platform has none */
  agentDir: string | null
  /** File extensions used for agent definitions (e.g. ['.md']) */
  agentExtensions: string[]
}

/**
 * The result of resolving a platform + optional agent name into a concrete invocation.
 */
export interface ResolvedAgent {
  /** The resolved platform configuration */
  platform: Platform
  /** The requested agent name, or null if no agent was specified */
  agentName: string | null
  /** Absolute path to the agent definition file, or null if not found / not applicable */
  agentPath: string | null
  /** The final CLI command string to execute */
  command: string
}

/**
 * Built-in platform definitions. Keyed by platform name for O(1) lookup.
 *
 * - claude/gemini: Have agent directories with definition files
 * - codex/aider: Standalone commands with no agent file conventions
 */
export const BUILTIN_PLATFORMS: Record<string, Platform> = {
  claude: {
    name: 'claude',
    command: 'claude',
    agentDir: '.claude/agents',
    agentExtensions: ['.md'],
  },
  gemini: {
    name: 'gemini',
    command: 'gemini',
    agentDir: '.gemini/agents',
    agentExtensions: ['.txt', '.md'],
  },
  codex: {
    name: 'codex',
    command: 'codex',
    agentDir: null,
    agentExtensions: [],
  },
  aider: {
    name: 'aider',
    command: 'aider',
    agentDir: null,
    agentExtensions: [],
  },
}

/**
 * Platforms that support the `--agent <path>` flag for specifying agent definitions.
 */
const PLATFORMS_WITH_AGENT_FLAG = new Set(['claude', 'gemini'])

/**
 * Resolve a platform by name. Returns the built-in definition if known,
 * otherwise creates a custom platform with no agent directory support.
 */
export function resolvePlatform(name: string): Platform {
  // Built-in platforms are trusted; only validate unknown/custom names
  // that would become shell commands directly
  if (!BUILTIN_PLATFORMS[name]) {
    validatePlatformName(name)
  }

  return (
    BUILTIN_PLATFORMS[name] ?? {
      name,
      command: name,
      agentDir: null,
      agentExtensions: [],
    }
  )
}

/**
 * Fully resolve a platform and optional agent into a concrete invocation.
 *
 * Resolution flow:
 * 1. Look up the platform (built-in or custom fallback)
 * 2. If no agentName, return with null agent fields and bare command
 * 3. If agentName and platform has agentDir, scan for a matching file by trying
 *    each extension in order and returning the first match
 * 4. Build the final command string with platform-specific flags
 */
export function resolveAgent(platformName: string, agentName?: string, projectDir?: string): ResolvedAgent {
  const platform = resolvePlatform(platformName)

  // No agent requested — return bare platform command
  if (!agentName) {
    return {
      platform,
      agentName: null,
      agentPath: null,
      command: platform.command,
    }
  }

  // Agent requested but platform has no agent directory convention
  if (!platform.agentDir) {
    return {
      platform,
      agentName,
      agentPath: null,
      command: platform.command,
    }
  }

  // Resolve agent file within the platform's agent directory
  const agentPath = findAgentFile(platform, agentName, projectDir)
  const command = buildCommand(platform, agentPath)

  return {
    platform,
    agentName,
    agentPath,
    command,
  }
}

/**
 * Return all built-in platforms.
 */
export function listPlatforms(): Platform[] {
  return Object.values(BUILTIN_PLATFORMS)
}

/**
 * Scan a platform's agent directory and return agent names (without extensions).
 * Returns an empty array if the platform has no agentDir or the directory does not exist.
 */
export function listAgentsForPlatform(platformName: string, projectDir: string): string[] {
  const platform = resolvePlatform(platformName)

  if (!platform.agentDir) {
    return []
  }

  const dir = path.join(projectDir, platform.agentDir)

  let entries: fs.Dirent[]
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true })
  } catch (err: unknown) {
    // Directory doesn't exist or is inaccessible — not an error
    if (err instanceof Error && 'code' in err && (err as NodeJS.ErrnoException).code === 'ENOENT') {
      return []
    }
    throw err
  }

  const extensionSet = new Set(platform.agentExtensions)

  return entries.filter((e) => e.isFile() && extensionSet.has(path.extname(e.name))).map((e) => stripExtension(e.name))
}

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

/**
 * Pattern for safe platform and agent names: must start with an alphanumeric
 * character and contain only alphanumerics, dots, hyphens, and underscores.
 * This prevents shell injection via metacharacters (;|&`$) and path traversal
 * via sequences like `../`.
 */
const SAFE_NAME_PATTERN = /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/

/**
 * Validate a platform name to prevent shell injection.
 * Platform names are interpolated into shell commands, so they must not
 * contain shell metacharacters.
 */
export function validatePlatformName(name: string): void {
  if (!SAFE_NAME_PATTERN.test(name)) {
    throw new Error(
      `Invalid platform name: "${name}". Platform names must start with an alphanumeric character and contain only alphanumeric characters, dots, hyphens, and underscores.`,
    )
  }
}

/**
 * Validate an agent name to prevent path traversal.
 * Agent names are used in path construction, so they must not contain
 * path separators or traversal sequences.
 */
export function validateAgentName(name: string): void {
  if (!SAFE_NAME_PATTERN.test(name)) {
    throw new Error(
      `Invalid agent name: "${name}". Agent names must start with an alphanumeric character and contain only alphanumeric characters, dots, hyphens, and underscores.`,
    )
  }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Search for an agent definition file by trying each of the platform's
 * extensions in order. Returns the absolute path of the first match, or null.
 */
function findAgentFile(platform: Platform, agentName: string, projectDir?: string): string | null {
  if (!platform.agentDir || !projectDir) {
    return null
  }

  validateAgentName(agentName)

  const dir = path.join(projectDir, platform.agentDir)

  for (const ext of platform.agentExtensions) {
    const candidate = path.join(dir, `${agentName}${ext}`)

    // Path containment check: ensure the resolved candidate stays within the agent directory
    if (!path.resolve(candidate).startsWith(path.resolve(dir))) {
      throw new Error(`Agent name "${agentName}" resolves outside the agent directory.`)
    }

    try {
      if (fs.existsSync(candidate)) {
        return candidate
      }
    } catch {
      // existsSync should not throw, but guard against unexpected errors
      continue
    }
  }

  return null
}

/**
 * Build the CLI command string for a platform invocation.
 * Platforms that support `--agent` get the flag appended when an agent file is found.
 */
function buildCommand(platform: Platform, agentPath: string | null): string {
  if (agentPath && PLATFORMS_WITH_AGENT_FLAG.has(platform.name)) {
    return `${platform.command} --agent ${agentPath}`
  }
  return platform.command
}

/**
 * Strip the file extension from a filename.
 */
function stripExtension(filename: string): string {
  const ext = path.extname(filename)
  return ext ? filename.slice(0, -ext.length) : filename
}
