import { Command } from 'commander'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { registerMaproomAliases } from '../maproom-aliases.js'
import { runMaproomForward, runMaproomSearchWithAutoIndex } from '../maproom.js'

// Mock the maproom module to intercept runMaproomForward / runMaproomSearchWithAutoIndex calls
vi.mock('../maproom.js', () => ({
  runMaproomForward: vi.fn().mockResolvedValue(undefined),
  runMaproomSearchWithAutoIndex: vi.fn().mockResolvedValue(undefined),
}))

const mockRunMaproomForward = vi.mocked(runMaproomForward)
const mockRunMaproomSearchWithAutoIndex = vi.mocked(runMaproomSearchWithAutoIndex)

describe('maproom top-level aliases', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  function buildProgram(): Command {
    const program = new Command()
    program.exitOverride() // prevent process.exit in tests
    registerMaproomAliases(program)
    return program
  }

  describe('search alias', () => {
    it('registers the search command', () => {
      const program = buildProgram()
      const searchCmd = program.commands.find((c) => c.name() === 'search')
      expect(searchCmd).toBeDefined()
      expect(searchCmd!.description()).toBe('Search your codebase by concept')
    })

    it('delegates to runMaproomSearchWithAutoIndex with search args', async () => {
      const program = buildProgram()
      await program.parseAsync(['node', 'crewchief', 'search', 'auth flow'], { from: 'node' })
      expect(mockRunMaproomSearchWithAutoIndex).toHaveBeenCalledWith(['auth flow'])
    })

    it('passes through extra flags to maproom', async () => {
      const program = buildProgram()
      await program.parseAsync(['node', 'crewchief', 'search', 'auth flow', '--format', 'agent'], { from: 'node' })
      expect(mockRunMaproomSearchWithAutoIndex).toHaveBeenCalledWith(['auth flow', '--format', 'agent'])
    })
  })

  describe('index alias', () => {
    it('registers the index command', () => {
      const program = buildProgram()
      const indexCmd = program.commands.find((c) => c.name() === 'index')
      expect(indexCmd).toBeDefined()
      expect(indexCmd!.description()).toBe('Index the current repository for code search')
    })

    it('delegates to runMaproomForward with scan subcommand', async () => {
      const program = buildProgram()
      await program.parseAsync(['node', 'crewchief', 'index'], { from: 'node' })
      expect(mockRunMaproomForward).toHaveBeenCalledWith(['scan'])
    })

    it('passes through extra flags to maproom scan', async () => {
      const program = buildProgram()
      await program.parseAsync(['node', 'crewchief', 'index', '--generate-embeddings'], { from: 'node' })
      expect(mockRunMaproomForward).toHaveBeenCalledWith(['scan', '--generate-embeddings'])
    })
  })

  describe('context alias', () => {
    it('registers the context command', () => {
      const program = buildProgram()
      const contextCmd = program.commands.find((c) => c.name() === 'context')
      expect(contextCmd).toBeDefined()
      expect(contextCmd!.description()).toBe('Retrieve context bundle for a code chunk')
    })

    it('delegates to runMaproomForward with context subcommand', async () => {
      const program = buildProgram()
      await program.parseAsync(['node', 'crewchief', 'context', '--chunk-id', '12345'], { from: 'node' })
      expect(mockRunMaproomForward).toHaveBeenCalledWith(['context', '--chunk-id', '12345'])
    })

    it('passes through extra flags to maproom context', async () => {
      const program = buildProgram()
      await program.parseAsync(
        ['node', 'crewchief', 'context', '--chunk-id', '12345', '--callers', '--budget', '4000'],
        { from: 'node' },
      )
      expect(mockRunMaproomForward).toHaveBeenCalledWith([
        'context',
        '--chunk-id',
        '12345',
        '--callers',
        '--budget',
        '4000',
      ])
    })
  })
})
