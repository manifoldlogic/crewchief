/**
 * Tests for SearchCompetitionManager
 */

import { rmSync } from 'fs'
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { SearchCompetitionManager } from '../../src/orchestrator/search-competition.js'
import type { SearchTask } from '../../src/orchestrator/task.types.js'
import type { Variant } from '../../src/sdk/types.js'

describe('SearchCompetitionManager', () => {
  let manager: SearchCompetitionManager
  const testBaseDir = '/tmp/test-search-competitions'

  beforeEach(() => {
    manager = new SearchCompetitionManager(testBaseDir)
  })

  afterEach(() => {
    try {
      rmSync(testBaseDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  })

  const createTestTask = (): SearchTask => ({
    id: 'test-task-1',
    description: 'Find the authentication implementation',
    requirements: ['Use semantic search', 'Find auth files'],
    acceptanceCriteria: [],
    targets: ['src/auth/login.ts', 'authenticateUser'],
    context: 'The codebase uses JWT for authentication',
  })

  const createTestVariants = (): Variant[] => [
    {
      id: 'variant-minimal',
      name: 'Minimal',
      description: 'Simple search tool description',
    },
    {
      id: 'variant-detailed',
      name: 'Detailed',
      description: 'Detailed search tool with examples and best practices',
    },
  ]

  it('should create a search competition', async () => {
    const task = createTestTask()
    const variants = createTestVariants()

    const competition = await manager.startSearchCompetition(task, variants)

    expect(competition.id).toBeDefined()
    expect(competition.task).toEqual(task)
    expect(competition.participants).toHaveLength(2)
    expect(competition.participants[0].variant).toEqual(variants[0])
    expect(competition.participants[1].variant).toEqual(variants[1])
  })

  it('should initialize search metrics for participants', async () => {
    const task = createTestTask()
    const variants = createTestVariants()

    const competition = await manager.startSearchCompetition(task, variants)

    for (const participant of competition.participants) {
      expect(participant.searchMetrics).toBeDefined()
      expect(participant.searchMetrics?.searchCount).toBe(0)
      expect(participant.searchMetrics?.queriesIssued).toEqual([])
      expect(participant.searchMetrics?.toolCallCount).toBe(0)
    }
  })

  it('should assign unique agent IDs based on variant', async () => {
    const task = createTestTask()
    const variants = createTestVariants()

    const competition = await manager.startSearchCompetition(task, variants)

    expect(competition.participants[0].agentId).toBe('variant-variant-minimal')
    expect(competition.participants[1].agentId).toBe('variant-variant-detailed')
  })

  it('should persist competition state', async () => {
    const task = createTestTask()
    const variants = createTestVariants()

    const competition = await manager.startSearchCompetition(task, variants)

    // Retrieve the competition
    const retrieved = manager.get(competition.id)
    expect(retrieved).toBeDefined()
    expect(retrieved?.id).toBe(competition.id)
  })

  it('should list all competitions', async () => {
    const task = createTestTask()
    const variants = createTestVariants()

    await manager.startSearchCompetition(task, variants)
    await manager.startSearchCompetition(task, variants)

    const all = manager.list()
    expect(all.length).toBeGreaterThanOrEqual(2)
  })

  // Note: Full integration tests with actual agent execution
  // would be added in AGENTOPT-1005 (evaluation implementation)
})
