import chalk from 'chalk'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { runCommand } from '../../utils/exec.js'
import { findMaproomBinary } from '../../utils/maproom-binary.js'
import { type CapabilityTiers, buildCapabilityTiers, checkEmbeddingProvider, formatCapabilityTiers } from '../doctor.js'

// Mock external dependencies
vi.mock('../../utils/exec.js', () => ({
  runCommand: vi.fn(),
}))
vi.mock('../../utils/maproom-binary.js', () => ({
  findMaproomBinary: vi.fn(),
}))

const mockRunCommand = vi.mocked(runCommand)
const mockFindMaproomBinary = vi.mocked(findMaproomBinary)

// Disable chalk colors for deterministic assertions
beforeEach(() => {
  chalk.level = 0
})

afterEach(() => {
  vi.restoreAllMocks()
})

// ---------------------------------------------------------------------------
// buildCapabilityTiers
// ---------------------------------------------------------------------------
describe('buildCapabilityTiers', () => {
  beforeEach(() => {
    // Default: git available, tmux available, maproom available, no embedding
    mockRunCommand.mockImplementation(async (cmd: string) => {
      if (cmd === 'git') return { exitCode: 0, stdout: 'git version 2.43.0', stderr: '' }
      if (cmd === 'tmux') return { exitCode: 0, stdout: 'tmux 3.4', stderr: '' }
      if (cmd === 'curl') return { exitCode: 1, stdout: '', stderr: '' }
      return { exitCode: 1, stdout: '', stderr: '' }
    })
    mockFindMaproomBinary.mockReturnValue({ path: '/usr/local/bin/maproom', source: 'global' })
    // Clear embedding env vars.
    //
    // MAPROOM_EMBEDDING_PROVIDER and the AWS variables matter as much as the
    // API keys: checkEmbeddingProvider honors an explicit provider choice and
    // detects Bedrock from ambient AWS credentials, so leaving either set makes
    // these assertions depend on the developer's shell.
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    delete process.env.GOOGLE_APPLICATION_CREDENTIALS
    delete process.env.AWS_ACCESS_KEY_ID
    delete process.env.AWS_PROFILE
    delete process.env.AWS_WEB_IDENTITY_TOKEN_FILE
    delete process.env.AWS_CONTAINER_CREDENTIALS_RELATIVE_URI
    delete process.env.AWS_CONTAINER_CREDENTIALS_FULL_URI
  })

  it('places worktree management, FTS, and tmux under ready when all available', async () => {
    const tiers = await buildCapabilityTiers()
    const readyNames = tiers.ready.map((r) => r.name)
    expect(readyNames).toContain('Worktree management')
    expect(readyNames).toContain('Code search (FTS)')
    expect(readyNames).toContain('Agent orchestration (tmux)')
  })

  it('marks tmux ready item with version note', async () => {
    const tiers = await buildCapabilityTiers()
    const tmuxItem = tiers.ready.find((r) => r.name === 'Agent orchestration (tmux)')
    expect(tmuxItem).toBeDefined()
    expect(tmuxItem!.note).toContain('tmux 3.4')
  })

  it('places tmux under addable when not installed', async () => {
    mockRunCommand.mockImplementation(async (cmd: string) => {
      if (cmd === 'git') return { exitCode: 0, stdout: 'git version 2.43.0', stderr: '' }
      if (cmd === 'tmux') throw new Error('not found')
      if (cmd === 'curl') return { exitCode: 1, stdout: '', stderr: '' }
      return { exitCode: 1, stdout: '', stderr: '' }
    })
    const tiers = await buildCapabilityTiers()
    const addableNames = tiers.addable.map((a) => a.name)
    expect(addableNames).toContain('Agent orchestration (tmux)')
    const tmuxItem = tiers.addable.find((a) => a.name === 'Agent orchestration (tmux)')
    expect(tmuxItem!.actions.length).toBeGreaterThan(0)
    // Should include a concrete install command
    const hasInstallCmd = tmuxItem!.actions.some(
      (a) => a.command.includes('apt install tmux') || a.command.includes('brew install tmux'),
    )
    expect(hasInstallCmd).toBe(true)
  })

  it('places embedding under addable when no provider configured', async () => {
    const tiers = await buildCapabilityTiers()
    const addableNames = tiers.addable.map((a) => a.name)
    expect(addableNames).toContain('Semantic vector search')
    const embItem = tiers.addable.find((a) => a.name === 'Semantic vector search')
    expect(embItem!.reason).toBe('add embedding provider')
    // Should include concrete setup commands
    expect(embItem!.actions.some((a) => a.command.includes('crewchief maproom setup'))).toBe(true)
  })

  it('places embedding under ready when OPENAI_API_KEY is set', async () => {
    process.env.OPENAI_API_KEY = 'sk-test-key'
    const tiers = await buildCapabilityTiers()
    const readyNames = tiers.ready.map((r) => r.name)
    expect(readyNames).toContain('Semantic vector search')
    const embItem = tiers.ready.find((r) => r.name === 'Semantic vector search')
    expect(embItem!.note).toContain('openai')
    delete process.env.OPENAI_API_KEY
  })

  it('always places VSCode extension under notApplicable', async () => {
    const tiers = await buildCapabilityTiers()
    const naNames = tiers.notApplicable.map((n) => n.name)
    expect(naNames).toContain('VSCode extension')
    const vsItem = tiers.notApplicable.find((n) => n.name === 'VSCode extension')
    expect(vsItem!.reason).toContain('deprecated')
    expect(vsItem!.reason).toContain('MCP server')
  })

  it('reports critical error when git is missing', async () => {
    mockRunCommand.mockImplementation(async (cmd: string) => {
      if (cmd === 'git') throw new Error('not found')
      if (cmd === 'tmux') return { exitCode: 0, stdout: 'tmux 3.4', stderr: '' }
      if (cmd === 'curl') return { exitCode: 1, stdout: '', stderr: '' }
      return { exitCode: 1, stdout: '', stderr: '' }
    })
    const tiers = await buildCapabilityTiers()
    expect(tiers.hasCriticalErrors).toBe(true)
    expect(tiers.criticalErrors.some((e) => e.includes('git'))).toBe(true)
    // Worktree management should NOT appear under ready
    const readyNames = tiers.ready.map((r) => r.name)
    expect(readyNames).not.toContain('Worktree management')
  })

  it('places maproom FTS under addable when binary not found', async () => {
    mockFindMaproomBinary.mockReturnValue({ path: null, source: 'not-found' })
    const tiers = await buildCapabilityTiers()
    const addableNames = tiers.addable.map((a) => a.name)
    expect(addableNames).toContain('Code search (FTS)')
  })
})

// ---------------------------------------------------------------------------
// formatCapabilityTiers
// ---------------------------------------------------------------------------
describe('formatCapabilityTiers', () => {
  function makeTiers(overrides: Partial<CapabilityTiers> = {}): CapabilityTiers {
    return {
      ready: [
        { name: 'Worktree management', note: 'git version 2.43.0' },
        { name: 'Code search (FTS)' },
        { name: 'Agent orchestration (tmux)', note: '[tmux 3.4]' },
      ],
      addable: [
        {
          name: 'Semantic vector search',
          reason: 'add embedding provider',
          actions: [
            { label: 'Fastest:  ollama (local)', command: 'crewchief maproom setup --embeddings ollama' },
            { label: 'Simplest: openai (cloud)', command: 'crewchief maproom setup --embeddings openai' },
          ],
        },
        {
          name: 'Agent orchestration (iTerm2)',
          reason: 'macOS + iTerm2 required',
          actions: [{ label: 'Install', command: 'https://iterm2.com' }],
        },
      ],
      notApplicable: [{ name: 'VSCode extension', reason: 'deprecated -- use MCP server instead' }],
      hasCriticalErrors: false,
      criticalErrors: [],
      ...overrides,
    }
  }

  it('includes the section header WHAT YOU CAN DO RIGHT NOW', () => {
    const output = formatCapabilityTiers(makeTiers())
    expect(output).toContain('WHAT YOU CAN DO RIGHT NOW')
  })

  it('includes the section header WHAT YOU CAN ADD', () => {
    const output = formatCapabilityTiers(makeTiers())
    expect(output).toContain('WHAT YOU CAN ADD')
  })

  it('includes the section header WHAT IS NOT APPLICABLE', () => {
    const output = formatCapabilityTiers(makeTiers())
    expect(output).toContain('WHAT IS NOT APPLICABLE')
  })

  it('shows ready items with "ready" status', () => {
    const output = formatCapabilityTiers(makeTiers())
    expect(output).toContain('Worktree management')
    expect(output).toContain('ready')
  })

  it('shows addable items with their reason and commands', () => {
    const output = formatCapabilityTiers(makeTiers())
    expect(output).toContain('Semantic vector search')
    expect(output).toContain('add embedding provider')
    expect(output).toContain('crewchief maproom setup --embeddings ollama')
    expect(output).toContain('crewchief maproom setup --embeddings openai')
  })

  it('shows not-applicable items with their reason', () => {
    const output = formatCapabilityTiers(makeTiers())
    expect(output).toContain('VSCode extension')
    expect(output).toContain('deprecated -- use MCP server instead')
  })

  it('does not contain the word "failed" for optional capabilities', () => {
    const output = formatCapabilityTiers(makeTiers())
    expect(output.toLowerCase()).not.toContain('failed')
  })

  it('does not contain the word "error" for optional capabilities', () => {
    // Only when there are no critical errors
    const output = formatCapabilityTiers(makeTiers())
    expect(output.toLowerCase()).not.toContain('error')
  })

  it('does not contain red X symbols', () => {
    const output = formatCapabilityTiers(makeTiers())
    // Common red X representations
    expect(output).not.toContain('\u274C') // cross mark emoji
    expect(output).not.toContain('FAIL')
  })

  it('shows critical errors when present', () => {
    const output = formatCapabilityTiers(
      makeTiers({
        hasCriticalErrors: true,
        criticalErrors: ['git not found. Install: brew install git'],
      }),
    )
    expect(output).toContain('REQUIRED')
    expect(output).toContain('git not found')
  })

  it('shows (none) when no ready items', () => {
    const output = formatCapabilityTiers(makeTiers({ ready: [] }))
    expect(output).toContain('(none)')
  })

  it('shows informational message when nothing to add', () => {
    const output = formatCapabilityTiers(makeTiers({ addable: [] }))
    expect(output).toContain('nothing to add')
  })

  it('sections appear in correct order: RIGHT NOW before CAN ADD before NOT APPLICABLE', () => {
    const output = formatCapabilityTiers(makeTiers())
    const rightNowIdx = output.indexOf('WHAT YOU CAN DO RIGHT NOW')
    const canAddIdx = output.indexOf('WHAT YOU CAN ADD')
    const notApplicableIdx = output.indexOf('WHAT IS NOT APPLICABLE')
    expect(rightNowIdx).toBeLessThan(canAddIdx)
    expect(canAddIdx).toBeLessThan(notApplicableIdx)
  })
})

describe('checkEmbeddingProvider: AWS Bedrock', () => {
  beforeEach(() => {
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    delete process.env.GOOGLE_APPLICATION_CREDENTIALS
    delete process.env.AWS_ACCESS_KEY_ID
    delete process.env.AWS_PROFILE
    delete process.env.AWS_WEB_IDENTITY_TOKEN_FILE
    delete process.env.AWS_CONTAINER_CREDENTIALS_RELATIVE_URI
    delete process.env.AWS_CONTAINER_CREDENTIALS_FULL_URI
  })

  // Mirrors beforeEach exactly. A test that sets OPENAI_API_KEY (the
  // precedence cases below do) must not leak it into a later suite in the same
  // file, where it would silently satisfy a different provider check.
  afterEach(() => {
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    delete process.env.GOOGLE_APPLICATION_CREDENTIALS
    delete process.env.AWS_ACCESS_KEY_ID
    delete process.env.AWS_PROFILE
    delete process.env.AWS_WEB_IDENTITY_TOKEN_FILE
    delete process.env.AWS_CONTAINER_CREDENTIALS_RELATIVE_URI
    delete process.env.AWS_CONTAINER_CREDENTIALS_FULL_URI
  })

  it('reports no provider for an unsupported explicit name', async () => {
    // doctor must not claim readiness for a provider maproom would reject —
    // the first search would exit 2.
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'voyage'
    await expect(checkEmbeddingProvider()).resolves.toBeNull()
  })

  it('does not let an invalid explicit name mask a real credential', async () => {
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'not-a-provider'
    process.env.OPENAI_API_KEY = 'sk-test-key'
    await expect(checkEmbeddingProvider()).resolves.toBeNull()
  })

  it('reports bedrock when AWS_PROFILE is set', async () => {
    process.env.AWS_PROFILE = 'my-profile'
    await expect(checkEmbeddingProvider()).resolves.toEqual({ provider: 'bedrock' })
  })

  it('reports bedrock for EKS IRSA', async () => {
    process.env.AWS_WEB_IDENTITY_TOKEN_FILE = '/var/run/secrets/token'
    await expect(checkEmbeddingProvider()).resolves.toEqual({ provider: 'bedrock' })
  })

  it('prefers an explicit provider over credential sniffing', async () => {
    // The user said bedrock; a stray OPENAI_API_KEY must not override that.
    process.env.OPENAI_API_KEY = 'sk-test-key'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'bedrock'
    await expect(checkEmbeddingProvider()).resolves.toEqual({ provider: 'bedrock' })
  })

  it('normalizes the aws alias to bedrock', async () => {
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'aws-bedrock'
    await expect(checkEmbeddingProvider()).resolves.toEqual({ provider: 'bedrock' })
  })

  it('prefers OpenAI over Bedrock when neither is explicit', async () => {
    // Preserves the existing precedence; Bedrock is checked last because AWS
    // credentials are ambient on far more machines than an OpenAI key is.
    process.env.OPENAI_API_KEY = 'sk-test-key'
    process.env.AWS_PROFILE = 'my-profile'
    await expect(checkEmbeddingProvider()).resolves.toEqual({ provider: 'openai' })
  })
})
