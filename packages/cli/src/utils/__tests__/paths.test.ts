import os from 'node:os'
import path from 'node:path'
import { simpleGit } from 'simple-git'
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from 'vitest'
import { expandTilde, getRepositoryName, expandRepoPlaceholder, expandWorktreePath } from '../paths'

// Mock simple-git
vi.mock('simple-git')

describe('expandTilde', () => {
  const originalHomedir = os.homedir()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('expands ~ to home directory', () => {
    const result = expandTilde('~')
    expect(result).toBe(originalHomedir)
  })

  it('expands ~/foo to $HOME/foo', () => {
    const result = expandTilde('~/foo')
    expect(result).toBe(path.join(originalHomedir, 'foo'))
  })

  it('expands ~/foo/bar to $HOME/foo/bar', () => {
    const result = expandTilde('~/foo/bar')
    expect(result).toBe(path.join(originalHomedir, 'foo', 'bar'))
  })

  it('returns absolute path unchanged', () => {
    const result = expandTilde('/abs/path')
    expect(result).toBe('/abs/path')
  })

  it('returns relative path unchanged', () => {
    const result = expandTilde('relative/path')
    expect(result).toBe('relative/path')
  })

  it('returns ./relative path unchanged', () => {
    const result = expandTilde('./relative')
    expect(result).toBe('./relative')
  })

  it('returns ../parent path unchanged', () => {
    const result = expandTilde('../parent')
    expect(result).toBe('../parent')
  })

  it('does not expand tilde in middle of path', () => {
    const result = expandTilde('/path/~/foo')
    expect(result).toBe('/path/~/foo')
  })

  it('does not expand ~username format', () => {
    const result = expandTilde('~user/path')
    expect(result).toBe('~user/path')
  })

  // Windows-specific tests (run only on Windows or with mocked platform)
  if (process.platform === 'win32') {
    it('handles Windows paths with backslashes', () => {
      const result = expandTilde('~\\foo\\bar')
      expect(result).toBe(path.join(originalHomedir, 'foo', 'bar'))
    })

    it('handles Windows drive letters unchanged', () => {
      const result = expandTilde('C:\\path\\to\\file')
      expect(result).toBe('C:\\path\\to\\file')
    })
  }
})

describe('getRepositoryName', () => {
  let mockGit: {
    raw: Mock<[string[]], Promise<string>>
  }

  beforeEach(() => {
    vi.clearAllMocks()

    // Create mock git instance
    mockGit = {
      raw: vi.fn(),
    }

    // Mock simpleGit to return our mock instance
    vi.mocked(simpleGit).mockReturnValue(mockGit as ReturnType<typeof simpleGit>)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('extracts name from git@github.com:org/repo.git format', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await getRepositoryName()
    expect(result).toBe('myrepo')
    expect(simpleGit).toHaveBeenCalledWith({
      baseDir: process.cwd(),
      timeout: { block: 5000 },
    })
    expect(mockGit.raw).toHaveBeenCalledWith(['config', '--get', 'remote.origin.url'])
  })

  it('extracts name from https://github.com/org/repo.git format', async () => {
    mockGit.raw.mockResolvedValue('https://github.com/org/myrepo.git\n')

    const result = await getRepositoryName()
    expect(result).toBe('myrepo')
  })

  it('extracts name from URLs without .git suffix', async () => {
    mockGit.raw.mockResolvedValue('https://github.com/org/myrepo\n')

    const result = await getRepositoryName()
    expect(result).toBe('myrepo')
  })

  it('extracts name from git@gitlab.com format', async () => {
    mockGit.raw.mockResolvedValue('git@gitlab.com:org/myrepo.git\n')

    const result = await getRepositoryName()
    expect(result).toBe('myrepo')
  })

  it('extracts name from https://gitlab.com format', async () => {
    mockGit.raw.mockResolvedValue('https://gitlab.com/org/myrepo\n')

    const result = await getRepositoryName()
    expect(result).toBe('myrepo')
  })

  it('falls back to directory basename when git command fails', async () => {
    mockGit.raw.mockRejectedValue(new Error('not a git repository'))

    const result = await getRepositoryName('/path/to/mydir')
    expect(result).toBe('mydir')
  })

  it('falls back to directory basename when git times out', async () => {
    mockGit.raw.mockRejectedValue(new Error('timeout'))

    const result = await getRepositoryName('/path/to/mydir')
    expect(result).toBe('mydir')
  })

  it('falls back to directory basename when URL does not match regex', async () => {
    mockGit.raw.mockResolvedValue('invalid-url-format\n')

    const result = await getRepositoryName('/path/to/mydir')
    expect(result).toBe('mydir')
  })

  it('extracts last segment from path with slashes', async () => {
    // Regex extracts the last segment after the final /
    mockGit.raw.mockResolvedValue('git@github.com:org/repo/with/slashes.git\n')

    const result = await getRepositoryName()
    expect(result).toBe('slashes')
  })

  it('sanitizes backslashes in repo name', async () => {
    // Regex extracts 'repo\with\backslashes', then sanitization replaces backslashes
    mockGit.raw.mockResolvedValue('https://example.com/org/repo\\with\\backslashes.git\n')

    const result = await getRepositoryName()
    expect(result).not.toContain('\\')
    expect(result).toBe('repo-with-backslashes')
  })

  it('handles colons as separator in regex', async () => {
    // Regex pattern /[/:]([^/:]+?)(\.git)?$/ uses : as separator
    // So 'repo:name' extracts 'name' (after the colon)
    mockGit.raw.mockResolvedValue('https://example.com/org/repo:name.git\n')

    const result = await getRepositoryName()
    expect(result).toBe('name')
  })

  it('sanitizes asterisks in repo name', async () => {
    mockGit.raw.mockResolvedValue('https://example.com/org/repo*name.git\n')

    const result = await getRepositoryName()
    expect(result).not.toContain('*')
    expect(result).toBe('repo-name')
  })

  it('sanitizes question marks in repo name', async () => {
    mockGit.raw.mockResolvedValue('https://example.com/org/repo?name.git\n')

    const result = await getRepositoryName()
    expect(result).not.toContain('?')
    expect(result).toBe('repo-name')
  })

  it('sanitizes double quotes in repo name', async () => {
    mockGit.raw.mockResolvedValue('https://example.com/org/repo"name".git\n')

    const result = await getRepositoryName()
    expect(result).not.toContain('"')
    expect(result).toBe('repo-name-')
  })

  it('sanitizes angle brackets in repo name', async () => {
    mockGit.raw.mockResolvedValue('https://example.com/org/repo<name>.git\n')

    const result = await getRepositoryName()
    expect(result).not.toContain('<')
    expect(result).not.toContain('>')
    expect(result).toBe('repo-name-')
  })

  it('sanitizes pipe characters in repo name', async () => {
    mockGit.raw.mockResolvedValue('https://example.com/org/repo|name.git\n')

    const result = await getRepositoryName()
    expect(result).not.toContain('|')
    expect(result).toBe('repo-name')
  })

  it('sanitizes multiple special characters', async () => {
    // Test sanitization of special chars within repo name
    mockGit.raw.mockResolvedValue('https://example.com/org/repo*na?me.git\n')

    const result = await getRepositoryName()
    expect(result).toBe('repo-na-me')
  })

  it('limits result to 255 characters', async () => {
    const longName = 'a'.repeat(300)
    mockGit.raw.mockResolvedValue(`https://github.com/org/${longName}.git\n`)

    const result = await getRepositoryName()
    expect(result.length).toBeLessThanOrEqual(255)
    expect(result).toBe(longName.slice(0, 255))
  })

  it('uses custom cwd parameter', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    await getRepositoryName('/custom/path')
    expect(simpleGit).toHaveBeenCalledWith({
      baseDir: '/custom/path',
      timeout: { block: 5000 },
    })
  })

  it('uses process.cwd() when cwd not provided', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    await getRepositoryName()
    expect(simpleGit).toHaveBeenCalledWith({
      baseDir: process.cwd(),
      timeout: { block: 5000 },
    })
  })

  it('handles empty git output gracefully', async () => {
    mockGit.raw.mockResolvedValue('')

    const result = await getRepositoryName('/path/to/mydir')
    expect(result).toBe('mydir')
  })

  it('trims whitespace from git output', async () => {
    mockGit.raw.mockResolvedValue('  git@github.com:org/myrepo.git  \n')

    const result = await getRepositoryName()
    expect(result).toBe('myrepo')
  })

  // Windows-specific test
  if (process.platform === 'win32') {
    it('sanitizes directory basename on Windows', async () => {
      mockGit.raw.mockRejectedValue(new Error('not a git repository'))

      const result = await getRepositoryName('C:\\path\\to\\my:dir')
      expect(result).not.toContain(':')
    })
  }
})

describe('expandRepoPlaceholder', () => {
  let mockGit: {
    raw: Mock<[string[]], Promise<string>>
  }

  beforeEach(() => {
    vi.clearAllMocks()

    mockGit = {
      raw: vi.fn(),
    }

    vi.mocked(simpleGit).mockReturnValue(mockGit as ReturnType<typeof simpleGit>)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('replaces single <repo-name> occurrence', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandRepoPlaceholder('~/.crewchief/<repo-name>')
    expect(result).toBe('~/.crewchief/myrepo')
  })

  it('replaces multiple <repo-name> occurrences', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandRepoPlaceholder('/data/<repo-name>/<repo-name>-backup')
    expect(result).toBe('/data/myrepo/myrepo-backup')
  })

  it('returns path unchanged when no placeholder exists', async () => {
    const result = await expandRepoPlaceholder('~/.crewchief/worktrees')
    expect(result).toBe('~/.crewchief/worktrees')
    // Should not call git when no placeholder exists
    expect(mockGit.raw).not.toHaveBeenCalled()
  })

  it('passes cwd parameter to getRepositoryName', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    await expandRepoPlaceholder('/path/<repo-name>', '/custom/path')
    expect(simpleGit).toHaveBeenCalledWith({
      baseDir: '/custom/path',
      timeout: { block: 5000 },
    })
  })

  it('replaces placeholder with sanitized name', async () => {
    // Regex extracts 'colons' (after final colon), then sanitization (no colons to sanitize)
    mockGit.raw.mockResolvedValue('git@github.com:org/repo:with:colons.git\n')

    const result = await expandRepoPlaceholder('~/.crewchief/<repo-name>')
    expect(result).toBe('~/.crewchief/colons')
  })

  it('works with complex paths', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandRepoPlaceholder('~/projects/<repo-name>/worktrees/<repo-name>-feature')
    expect(result).toBe('~/projects/myrepo/worktrees/myrepo-feature')
  })
})

describe('expandWorktreePath', () => {
  let mockGit: {
    raw: Mock<[string[]], Promise<string>>
  }
  const originalHomedir = os.homedir()

  beforeEach(() => {
    vi.clearAllMocks()

    mockGit = {
      raw: vi.fn(),
    }

    vi.mocked(simpleGit).mockReturnValue(mockGit as ReturnType<typeof simpleGit>)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('chains tilde expansion and placeholder replacement', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('~/.crewchief/<repo-name>')
    expect(result).toBe(path.resolve(originalHomedir, '.crewchief', 'myrepo'))
  })

  it('makes path absolute', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('.crewchief/<repo-name>')
    expect(path.isAbsolute(result)).toBe(true)
    expect(result).toBe(path.resolve(process.cwd(), '.crewchief', 'myrepo'))
  })

  it('handles already absolute paths', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('/data/<repo-name>')
    expect(result).toBe('/data/myrepo')
  })

  it('rejects root directory', async () => {
    await expect(expandWorktreePath('/')).rejects.toThrow(/Rejected system directory.*Reason.*Example valid path/s)
  })

  it('rejects /etc directory', async () => {
    await expect(expandWorktreePath('/etc')).rejects.toThrow(
      /Rejected system directory.*\/etc.*Reason.*Example valid path/s,
    )
  })

  it('rejects /etc subdirectory', async () => {
    await expect(expandWorktreePath('/etc/worktrees')).rejects.toThrow(
      /Rejected system directory.*\/etc.*Reason.*Example valid path/s,
    )
  })

  it('rejects /usr directory', async () => {
    await expect(expandWorktreePath('/usr')).rejects.toThrow(
      /Rejected system directory.*\/usr.*Reason.*Example valid path/s,
    )
  })

  it('rejects /usr subdirectory', async () => {
    await expect(expandWorktreePath('/usr/local/worktrees')).rejects.toThrow(
      /Rejected system directory.*\/usr.*Reason.*Example valid path/s,
    )
  })

  it('rejects /System directory on macOS', async () => {
    await expect(expandWorktreePath('/System')).rejects.toThrow(
      /Rejected system directory.*\/System.*Reason.*Example valid path/s,
    )
  })

  it('rejects /System subdirectory on macOS', async () => {
    await expect(expandWorktreePath('/System/Library/worktrees')).rejects.toThrow(
      /Rejected system directory.*\/System.*Reason.*Example valid path/s,
    )
  })

  // Windows-specific tests
  if (process.platform === 'win32') {
    it('rejects C:\\Windows directory on Windows', async () => {
      await expect(expandWorktreePath('C:\\Windows')).rejects.toThrow(
        /Rejected system directory.*C:\\Windows.*Reason.*Example valid path/s,
      )
    })

    it('rejects C:\\Windows subdirectory on Windows', async () => {
      await expect(expandWorktreePath('C:\\Windows\\System32\\worktrees')).rejects.toThrow(
        /Rejected system directory.*C:\\Windows.*Reason.*Example valid path/s,
      )
    })

    it('allows C:\\Users paths on Windows', async () => {
      mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

      const result = await expandWorktreePath('C:\\Users\\username\\<repo-name>')
      expect(result).toBe(path.normalize('C:\\Users\\username\\myrepo'))
    })

    it('expands ~ to USERPROFILE on Windows', async () => {
      mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

      const result = await expandWorktreePath('~\\<repo-name>')
      expect(result).toBe(path.resolve(os.homedir(), 'myrepo'))
    })
  }

  it('allows valid home directory paths', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('~/.crewchief/<repo-name>')
    expect(result).toBe(path.resolve(originalHomedir, '.crewchief', 'myrepo'))
  })

  it('allows valid relative paths', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('.crewchief/<repo-name>')
    expect(result).toBe(path.resolve(process.cwd(), '.crewchief', 'myrepo'))
  })

  it('allows /tmp paths', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('/tmp/<repo-name>')
    expect(result).toBe('/tmp/myrepo')
  })

  it('allows /home paths', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('/home/user/<repo-name>')
    expect(result).toBe('/home/user/myrepo')
  })

  it('allows /opt paths', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('/opt/<repo-name>')
    expect(result).toBe('/opt/myrepo')
  })

  it('passes cwd parameter through the chain', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    await expandWorktreePath('~/<repo-name>', '/custom/path')
    expect(simpleGit).toHaveBeenCalledWith({
      baseDir: '/custom/path',
      timeout: { block: 5000 },
    })
  })

  it('error message includes rejected path', async () => {
    await expect(expandWorktreePath('/etc/worktrees')).rejects.toThrow(/\/etc/)
  })

  it('error message includes reason', async () => {
    await expect(expandWorktreePath('/etc/worktrees')).rejects.toThrow(/Cannot create worktrees in system directories/)
  })

  it('error message includes example valid path', async () => {
    await expect(expandWorktreePath('/etc/worktrees')).rejects.toThrow(/~\/.crewchief\/worktrees\/<repo-name>/)
  })

  it('handles paths that resolve to system directories after expansion', async () => {
    // This tests path normalization catching system directories
    // For example, if someone tries: /home/../etc
    await expect(expandWorktreePath('/home/../etc')).rejects.toThrow(/Rejected system directory/)
  })

  it('works with complex expansion chain', async () => {
    mockGit.raw.mockResolvedValue('git@github.com:org/myrepo.git\n')

    const result = await expandWorktreePath('~/projects/<repo-name>/worktrees/<repo-name>-feature')
    expect(result).toBe(path.resolve(originalHomedir, 'projects', 'myrepo', 'worktrees', 'myrepo-feature'))
  })
})
