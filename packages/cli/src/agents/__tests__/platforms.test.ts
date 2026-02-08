import fs from 'node:fs'
import path from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  BUILTIN_PLATFORMS,
  MAX_NAME_LENGTH,
  listAgentsForPlatform,
  listPlatforms,
  resolveAgent,
  resolvePlatform,
  validateAgentName,
  validatePlatformName,
} from '../platforms'

// Mock node:fs for filesystem-dependent tests
vi.mock('node:fs')

afterEach(() => {
  vi.restoreAllMocks()
})

// ---------------------------------------------------------------------------
// resolvePlatform
// ---------------------------------------------------------------------------

describe('resolvePlatform', () => {
  it('resolves claude platform', () => {
    const platform = resolvePlatform('claude')
    expect(platform).toEqual({
      name: 'claude',
      command: 'claude',
      agentDir: '.claude/agents',
      agentExtensions: ['.md'],
    })
  })

  it('resolves gemini platform', () => {
    const platform = resolvePlatform('gemini')
    expect(platform).toEqual({
      name: 'gemini',
      command: 'gemini',
      agentDir: '.gemini/agents',
      agentExtensions: ['.txt', '.md'],
    })
  })

  it('resolves codex platform', () => {
    const platform = resolvePlatform('codex')
    expect(platform).toEqual({
      name: 'codex',
      command: 'codex',
      agentDir: null,
      agentExtensions: [],
    })
  })

  it('resolves aider platform', () => {
    const platform = resolvePlatform('aider')
    expect(platform).toEqual({
      name: 'aider',
      command: 'aider',
      agentDir: null,
      agentExtensions: [],
    })
  })

  it('falls back to custom platform for unknown names', () => {
    const platform = resolvePlatform('my-custom-tool')
    expect(platform).toEqual({
      name: 'my-custom-tool',
      command: 'my-custom-tool',
      agentDir: null,
      agentExtensions: [],
    })
  })

  it('rejects empty string platform name', () => {
    expect(() => resolvePlatform('')).toThrow('Invalid platform name')
  })

  it('rejects platform name with shell metacharacters', () => {
    expect(() => resolvePlatform('tool@2.0')).toThrow('Invalid platform name')
  })

  it('returns the exact built-in object (identity check)', () => {
    const platform = resolvePlatform('claude')
    expect(platform).toBe(BUILTIN_PLATFORMS['claude'])
  })
})

// ---------------------------------------------------------------------------
// resolveAgent
// ---------------------------------------------------------------------------

describe('resolveAgent', () => {
  describe('without agent name', () => {
    it('returns null agent fields and bare command for known platform', () => {
      const result = resolveAgent('claude')
      expect(result.platform.name).toBe('claude')
      expect(result.agentName).toBeNull()
      expect(result.agentPath).toBeNull()
      expect(result.command).toBe('claude')
    })

    it('returns bare command for unknown platform', () => {
      const result = resolveAgent('my-tool')
      expect(result.platform.name).toBe('my-tool')
      expect(result.agentName).toBeNull()
      expect(result.agentPath).toBeNull()
      expect(result.command).toBe('my-tool')
    })
  })

  describe('with agent name and platform with agentDir', () => {
    it('resolves agent file when it exists (claude, .md)', () => {
      vi.mocked(fs.existsSync).mockImplementation((p) => {
        return String(p) === path.join('/project', '.claude/agents', 'backend-developer.md')
      })

      const result = resolveAgent('claude', 'backend-developer', '/project')
      expect(result.agentName).toBe('backend-developer')
      expect(result.agentPath).toBe(path.join('/project', '.claude/agents', 'backend-developer.md'))
      expect(result.command).toBe(`claude --agent ${result.agentPath}`)
    })

    it('resolves agent file when it exists (gemini, .txt first extension match)', () => {
      vi.mocked(fs.existsSync).mockImplementation((p) => {
        return String(p) === path.join('/project', '.gemini/agents', 'my-agent.txt')
      })

      const result = resolveAgent('gemini', 'my-agent', '/project')
      expect(result.agentName).toBe('my-agent')
      expect(result.agentPath).toBe(path.join('/project', '.gemini/agents', 'my-agent.txt'))
      expect(result.command).toBe(`gemini --agent ${result.agentPath}`)
    })

    it('resolves gemini agent with .md when .txt not found (second extension)', () => {
      vi.mocked(fs.existsSync).mockImplementation((p) => {
        // .txt does not exist, .md does
        return String(p) === path.join('/project', '.gemini/agents', 'my-agent.md')
      })

      const result = resolveAgent('gemini', 'my-agent', '/project')
      expect(result.agentPath).toBe(path.join('/project', '.gemini/agents', 'my-agent.md'))
    })

    it('returns null agentPath when agent file not found', () => {
      vi.mocked(fs.existsSync).mockReturnValue(false)

      const result = resolveAgent('claude', 'nonexistent-agent', '/project')
      expect(result.agentName).toBe('nonexistent-agent')
      expect(result.agentPath).toBeNull()
      expect(result.command).toBe('claude')
    })

    it('returns null agentPath when projectDir is not provided', () => {
      const result = resolveAgent('claude', 'backend-developer')
      expect(result.agentName).toBe('backend-developer')
      expect(result.agentPath).toBeNull()
      expect(result.command).toBe('claude')
    })
  })

  describe('with agent name on platform without agentDir', () => {
    it('returns null agentPath for codex with agent name', () => {
      const result = resolveAgent('codex', 'some-agent', '/project')
      expect(result.platform.name).toBe('codex')
      expect(result.agentName).toBe('some-agent')
      expect(result.agentPath).toBeNull()
      expect(result.command).toBe('codex')
    })

    it('returns null agentPath for aider with agent name', () => {
      const result = resolveAgent('aider', 'some-agent', '/project')
      expect(result.platform.name).toBe('aider')
      expect(result.agentName).toBe('some-agent')
      expect(result.agentPath).toBeNull()
      expect(result.command).toBe('aider')
    })

    it('returns null agentPath for custom platform with agent name', () => {
      const result = resolveAgent('my-custom-tool', 'some-agent', '/project')
      expect(result.platform.name).toBe('my-custom-tool')
      expect(result.agentName).toBe('some-agent')
      expect(result.agentPath).toBeNull()
      expect(result.command).toBe('my-custom-tool')
    })
  })

  describe('command construction', () => {
    it('adds --agent flag for claude when agentPath exists', () => {
      vi.mocked(fs.existsSync).mockReturnValue(true)

      const result = resolveAgent('claude', 'dev', '/project')
      expect(result.command).toContain('--agent')
    })

    it('adds --agent flag for gemini when agentPath exists', () => {
      vi.mocked(fs.existsSync).mockImplementation((p) => {
        return String(p) === path.join('/project', '.gemini/agents', 'dev.txt')
      })

      const result = resolveAgent('gemini', 'dev', '/project')
      expect(result.command).toContain('--agent')
    })

    it('does not add --agent flag when agentPath is null', () => {
      vi.mocked(fs.existsSync).mockReturnValue(false)

      const result = resolveAgent('claude', 'missing', '/project')
      expect(result.command).toBe('claude')
      expect(result.command).not.toContain('--agent')
    })
  })

  describe('first extension wins', () => {
    it('prefers first matching extension for gemini (.txt over .md)', () => {
      // Both .txt and .md exist; .txt is checked first per agentExtensions order
      vi.mocked(fs.existsSync).mockReturnValue(true)

      const result = resolveAgent('gemini', 'shared-agent', '/project')
      expect(result.agentPath).toBe(path.join('/project', '.gemini/agents', 'shared-agent.txt'))
    })
  })
})

// ---------------------------------------------------------------------------
// listPlatforms
// ---------------------------------------------------------------------------

describe('listPlatforms', () => {
  it('returns exactly 4 built-in platforms', () => {
    const platforms = listPlatforms()
    expect(platforms).toHaveLength(4)
  })

  it('includes all expected platform names', () => {
    const platforms = listPlatforms()
    const names = platforms.map((p) => p.name)
    expect(names).toContain('claude')
    expect(names).toContain('gemini')
    expect(names).toContain('codex')
    expect(names).toContain('aider')
  })

  it('every platform has required fields', () => {
    const platforms = listPlatforms()
    for (const p of platforms) {
      expect(p).toHaveProperty('name')
      expect(p).toHaveProperty('command')
      expect(p).toHaveProperty('agentDir')
      expect(p).toHaveProperty('agentExtensions')
      expect(typeof p.name).toBe('string')
      expect(typeof p.command).toBe('string')
      expect(Array.isArray(p.agentExtensions)).toBe(true)
    }
  })

  it('claude and gemini have agentDir set', () => {
    const platforms = listPlatforms()
    const claude = platforms.find((p) => p.name === 'claude')!
    const gemini = platforms.find((p) => p.name === 'gemini')!
    expect(claude.agentDir).toBe('.claude/agents')
    expect(gemini.agentDir).toBe('.gemini/agents')
  })

  it('codex and aider have null agentDir', () => {
    const platforms = listPlatforms()
    const codex = platforms.find((p) => p.name === 'codex')!
    const aider = platforms.find((p) => p.name === 'aider')!
    expect(codex.agentDir).toBeNull()
    expect(aider.agentDir).toBeNull()
  })

  it('returns references to the BUILTIN_PLATFORMS objects', () => {
    const platforms = listPlatforms()
    const claude = platforms.find((p) => p.name === 'claude')!
    expect(claude).toBe(BUILTIN_PLATFORMS['claude'])
  })
})

// ---------------------------------------------------------------------------
// listAgentsForPlatform
// ---------------------------------------------------------------------------

describe('listAgentsForPlatform', () => {
  /**
   * Helper to create a mock Dirent object.
   */
  function makeDirent(name: string, isFile: boolean) {
    return {
      name,
      isFile: () => isFile,
      isDirectory: () => !isFile,
      isBlockDevice: () => false,
      isCharacterDevice: () => false,
      isFIFO: () => false,
      isSocket: () => false,
      isSymbolicLink: () => false,
      path: '',
      parentPath: '',
    }
  }

  // Cast the mock once so we don't fight Dirent generics on every call
  const mockReaddirSync = vi.mocked(fs.readdirSync) as unknown as ReturnType<typeof vi.fn>

  it('lists agents from directory with correct extensions (claude)', () => {
    mockReaddirSync.mockReturnValue([
      makeDirent('backend-developer.md', true),
      makeDirent('frontend-dev.md', true),
      makeDirent('ignored.txt', true),
      makeDirent('some-dir', false),
    ])

    const agents = listAgentsForPlatform('claude', '/project')
    expect(agents).toEqual(['backend-developer', 'frontend-dev'])
  })

  it('lists agents from directory with multiple extensions (gemini)', () => {
    mockReaddirSync.mockReturnValue([
      makeDirent('agent-a.txt', true),
      makeDirent('agent-b.md', true),
      makeDirent('ignored.json', true),
    ])

    const agents = listAgentsForPlatform('gemini', '/project')
    expect(agents).toEqual(['agent-a', 'agent-b'])
  })

  it('returns empty array for empty directory', () => {
    mockReaddirSync.mockReturnValue([])

    const agents = listAgentsForPlatform('claude', '/project')
    expect(agents).toEqual([])
  })

  it('returns empty array for missing directory (ENOENT)', () => {
    const err = new Error('ENOENT: no such file or directory') as NodeJS.ErrnoException
    err.code = 'ENOENT'
    mockReaddirSync.mockImplementation(() => {
      throw err
    })

    const agents = listAgentsForPlatform('claude', '/project')
    expect(agents).toEqual([])
  })

  it('throws non-ENOENT filesystem errors', () => {
    const err = new Error('EACCES: permission denied') as NodeJS.ErrnoException
    err.code = 'EACCES'
    mockReaddirSync.mockImplementation(() => {
      throw err
    })

    expect(() => listAgentsForPlatform('claude', '/project')).toThrow('EACCES')
  })

  it('returns empty array for platform without agentDir (codex)', () => {
    const agents = listAgentsForPlatform('codex', '/project')
    expect(agents).toEqual([])
    // readdirSync should not be called since platform has no agentDir
    expect(fs.readdirSync).not.toHaveBeenCalled()
  })

  it('returns empty array for platform without agentDir (aider)', () => {
    const agents = listAgentsForPlatform('aider', '/project')
    expect(agents).toEqual([])
    expect(fs.readdirSync).not.toHaveBeenCalled()
  })

  it('returns empty array for unknown platform (no agentDir)', () => {
    const agents = listAgentsForPlatform('unknown-tool', '/project')
    expect(agents).toEqual([])
    expect(fs.readdirSync).not.toHaveBeenCalled()
  })

  it('filters out directories, only includes files', () => {
    mockReaddirSync.mockReturnValue([
      makeDirent('real-agent.md', true),
      makeDirent('sub-directory.md', false), // directory with .md name
    ])

    const agents = listAgentsForPlatform('claude', '/project')
    expect(agents).toEqual(['real-agent'])
  })

  it('strips extensions from agent names', () => {
    mockReaddirSync.mockReturnValue([makeDirent('my-agent.md', true)])

    const agents = listAgentsForPlatform('claude', '/project')
    expect(agents).toEqual(['my-agent'])
    expect(agents[0]).not.toContain('.md')
  })

  it('reads from correct directory path', () => {
    mockReaddirSync.mockReturnValue([])

    listAgentsForPlatform('claude', '/my/project')
    expect(fs.readdirSync).toHaveBeenCalledWith(path.join('/my/project', '.claude/agents'), {
      withFileTypes: true,
    })
  })

  it('reads from gemini agent directory path', () => {
    mockReaddirSync.mockReturnValue([])

    listAgentsForPlatform('gemini', '/my/project')
    expect(fs.readdirSync).toHaveBeenCalledWith(path.join('/my/project', '.gemini/agents'), {
      withFileTypes: true,
    })
  })
})

// ---------------------------------------------------------------------------
// BUILTIN_PLATFORMS (structural assertions)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Security: input validation
// ---------------------------------------------------------------------------

describe('validatePlatformName', () => {
  it('accepts valid alphanumeric names', () => {
    expect(() => validatePlatformName('claude')).not.toThrow()
    expect(() => validatePlatformName('my-tool')).not.toThrow()
    expect(() => validatePlatformName('tool.v2')).not.toThrow()
    expect(() => validatePlatformName('my_tool')).not.toThrow()
    expect(() => validatePlatformName('Tool123')).not.toThrow()
  })

  it('rejects shell injection: semicolon', () => {
    expect(() => validatePlatformName('foo;rm -rf /')).toThrow('Invalid platform name')
  })

  it('rejects shell injection: pipe', () => {
    expect(() => validatePlatformName('foo|cat /etc/passwd')).toThrow('Invalid platform name')
  })

  it('rejects shell injection: ampersand', () => {
    expect(() => validatePlatformName('foo&&evil')).toThrow('Invalid platform name')
  })

  it('rejects shell injection: backtick', () => {
    expect(() => validatePlatformName('foo`evil`')).toThrow('Invalid platform name')
  })

  it('rejects shell injection: dollar sign', () => {
    expect(() => validatePlatformName('foo$(evil)')).toThrow('Invalid platform name')
  })

  it('rejects shell injection: spaces', () => {
    expect(() => validatePlatformName('foo bar')).toThrow('Invalid platform name')
  })

  it('rejects empty string', () => {
    expect(() => validatePlatformName('')).toThrow('Invalid platform name')
  })

  it('rejects names starting with hyphen', () => {
    expect(() => validatePlatformName('-flag')).toThrow('Invalid platform name')
  })

  it('rejects names starting with dot', () => {
    expect(() => validatePlatformName('.hidden')).toThrow('Invalid platform name')
  })

  it('accepts name at MAX_NAME_LENGTH boundary', () => {
    expect(() => validatePlatformName('a'.repeat(MAX_NAME_LENGTH))).not.toThrow()
  })

  it('rejects name exceeding MAX_NAME_LENGTH', () => {
    expect(() => validatePlatformName('a'.repeat(MAX_NAME_LENGTH + 1))).toThrow('Input too long')
  })

  it('rejects extremely long input', () => {
    expect(() => validatePlatformName('a'.repeat(10000))).toThrow('Input too long')
  })

  it('truncates long input in error message', () => {
    const longName = 'a'.repeat(10000)
    expect(() => validatePlatformName(longName)).toThrow('(10000 chars)')
  })
})

describe('validateAgentName', () => {
  it('accepts valid alphanumeric names', () => {
    expect(() => validateAgentName('backend-developer')).not.toThrow()
    expect(() => validateAgentName('my.agent')).not.toThrow()
    expect(() => validateAgentName('agent_v2')).not.toThrow()
    expect(() => validateAgentName('Agent123')).not.toThrow()
  })

  it('rejects path traversal: ../', () => {
    expect(() => validateAgentName('../../../etc/passwd')).toThrow('Invalid agent name')
  })

  it('rejects path traversal: embedded ..', () => {
    expect(() => validateAgentName('foo/../bar')).toThrow('Invalid agent name')
  })

  it('rejects forward slashes', () => {
    expect(() => validateAgentName('foo/bar')).toThrow('Invalid agent name')
  })

  it('rejects backslashes', () => {
    expect(() => validateAgentName('foo\\bar')).toThrow('Invalid agent name')
  })

  it('rejects shell metacharacters', () => {
    expect(() => validateAgentName('agent;rm -rf /')).toThrow('Invalid agent name')
    expect(() => validateAgentName('agent|evil')).toThrow('Invalid agent name')
    expect(() => validateAgentName('agent`evil`')).toThrow('Invalid agent name')
  })

  it('rejects empty string', () => {
    expect(() => validateAgentName('')).toThrow('Invalid agent name')
  })

  it('rejects names starting with hyphen', () => {
    expect(() => validateAgentName('-flag-inject')).toThrow('Invalid agent name')
  })

  it('accepts name at MAX_NAME_LENGTH boundary', () => {
    expect(() => validateAgentName('a'.repeat(MAX_NAME_LENGTH))).not.toThrow()
  })

  it('rejects name exceeding MAX_NAME_LENGTH', () => {
    expect(() => validateAgentName('a'.repeat(MAX_NAME_LENGTH + 1))).toThrow('Input too long')
  })

  it('rejects extremely long input', () => {
    expect(() => validateAgentName('a'.repeat(10000))).toThrow('Input too long')
  })
})

describe('security: resolveAgent with malicious inputs', () => {
  it('rejects shell injection in custom platform name', () => {
    expect(() => resolveAgent(';rm -rf /')).toThrow('Invalid platform name')
  })

  it('rejects pipe injection in custom platform name', () => {
    expect(() => resolveAgent('evil|cat /etc/passwd')).toThrow('Invalid platform name')
  })

  it('rejects path traversal in agent name', () => {
    expect(() => resolveAgent('claude', '../../../etc/passwd', '/project')).toThrow('Invalid agent name')
  })

  it('rejects agent name with shell metacharacters', () => {
    expect(() => resolveAgent('claude', 'agent;evil', '/project')).toThrow('Invalid agent name')
  })

  it('allows built-in platform names without validation error', () => {
    // Built-in platforms are trusted and should not trigger validation
    expect(() => resolveAgent('claude')).not.toThrow()
    expect(() => resolveAgent('gemini')).not.toThrow()
    expect(() => resolveAgent('codex')).not.toThrow()
    expect(() => resolveAgent('aider')).not.toThrow()
  })

  it('allows valid custom platform names', () => {
    expect(() => resolveAgent('my-custom-tool')).not.toThrow()
  })

  it('allows valid agent names on known platforms', () => {
    vi.mocked(fs.existsSync).mockReturnValue(false)
    expect(() => resolveAgent('claude', 'backend-developer', '/project')).not.toThrow()
  })
})

// ---------------------------------------------------------------------------
// BUILTIN_PLATFORMS (structural assertions)
// ---------------------------------------------------------------------------

describe('BUILTIN_PLATFORMS', () => {
  it('has exactly 4 entries', () => {
    expect(Object.keys(BUILTIN_PLATFORMS)).toHaveLength(4)
  })

  it('keys match platform name fields', () => {
    for (const [key, platform] of Object.entries(BUILTIN_PLATFORMS)) {
      expect(key).toBe(platform.name)
    }
  })

  it('all commands are non-empty strings', () => {
    for (const platform of Object.values(BUILTIN_PLATFORMS)) {
      expect(platform.command.length).toBeGreaterThan(0)
    }
  })

  it('agentExtensions arrays contain only strings starting with a dot', () => {
    for (const platform of Object.values(BUILTIN_PLATFORMS)) {
      for (const ext of platform.agentExtensions) {
        expect(ext).toMatch(/^\./)
      }
    }
  })
})
