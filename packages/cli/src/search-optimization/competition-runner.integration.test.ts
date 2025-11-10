/**
 * Integration tests for competition runner with three-phase validation
 *
 * These tests verify:
 * 1. Full competition flow with validation
 * 2. Fail-fast behavior for setup failures
 * 3. Proper ordering of validation before execution
 */

import { mkdtempSync, rmSync, existsSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { describe, it, expect, vi, beforeEach, afterEach, beforeAll } from 'vitest'
import { runCompetition } from './competition-runner.js'
import type { Variant, SearchTask } from './types.js'
import { PreFlightValidator } from './validation/pre-flight-validator.js'

// Mock task for testing
const MOCK_TASK: SearchTask = {
  id: 'test-task-1',
  name: 'Find Worktree Creation',
  description: 'Find the function that creates git worktrees in the CLI',
  searchTarget: {
    type: 'function',
    name: 'createWorktree',
  },
  followUpTask: {
    type: 'explanation',
    prompt: 'Explain how it works',
    validator: {
      type: 'explanation',
      mentionsFiles: ['packages/cli/src/git/worktree.ts'],
    },
  },
  difficulty: 'easy',
  category: 'code-discovery',
  successValidator: () => ({
    searchQuality: 0.8,
    taskCompletion: 0.7,
    efficiency: 0.9,
    total: 0.8,
    details: 'Mock validation',
  }),
}

// Mock variants for testing
const MOCK_VARIANTS: Variant[] = [
  {
    id: 'variant-control',
    name: 'Control',
    description: 'Standard search description',
  },
  {
    id: 'variant-test',
    name: 'Test Variant',
    description: 'Enhanced search description with more detail',
  },
]

describe('Competition Runner Integration', () => {
  let testDir: string

  beforeAll(async () => {
    // Skip if database is not available (CI environment)
    const validator = new PreFlightValidator()
    const dbAvailable = await validator.checkDatabaseConnection()
    if (!dbAvailable) {
      console.warn('⚠️  PostgreSQL not available - skipping integration tests')
    }
  })

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'comp-test-'))
  })

  afterEach(() => {
    if (existsSync(testDir)) {
      rmSync(testDir, { recursive: true, force: true })
    }
  })

  it('fails fast when database is unavailable', async () => {
    const originalUrl = process.env.MAPROOM_DATABASE_URL
    process.env.MAPROOM_DATABASE_URL = 'postgresql://invalid:invalid@localhost:9999/fake'

    await expect(
      runCompetition({
        task: MOCK_TASK,
        variants: [MOCK_VARIANTS[0]],
        baseDir: testDir,
      }),
    ).rejects.toThrow(/Database connection failed/)

    process.env.MAPROOM_DATABASE_URL = originalUrl
  }, 10000)

  it('fails fast when base branch not indexed', async () => {
    // Check if database is available first
    const validator = new PreFlightValidator()
    const dbAvailable = await validator.checkDatabaseConnection()

    if (!dbAvailable) {
      console.warn('⚠️  Skipping test: PostgreSQL not available')
      return
    }

    // Mock the validation to return false
    const mockVerifyBaseBranch = vi.spyOn(PreFlightValidator.prototype, 'verifyBaseBranchIndexed')
    mockVerifyBaseBranch.mockResolvedValue({ indexed: false, chunkCount: 0 })

    await expect(
      runCompetition({
        task: MOCK_TASK,
        variants: [MOCK_VARIANTS[0]],
        baseDir: testDir,
      }),
    ).rejects.toThrow(/Base branch .* not indexed/)

    mockVerifyBaseBranch.mockRestore()
  }, 10000)

  it('includes setup metrics in competition result', async () => {
    // Check if database is available first
    const validator = new PreFlightValidator()
    const dbAvailable = await validator.checkDatabaseConnection()

    if (!dbAvailable) {
      console.warn('⚠️  Skipping test: PostgreSQL not available')
      return
    }

    // Check if base branch is indexed
    const baseIndexed = await validator.verifyBaseBranchIndexed('crewchief', 'main')
    if (!baseIndexed.indexed) {
      console.warn('⚠️  Skipping test: Base branch not indexed')
      return
    }

    // Mock the expensive operations
    const mockCreateVariantWorktree = vi.fn().mockResolvedValue({
      path: testDir,
      cleanup: vi.fn().mockResolvedValue(undefined),
    })

    const mockScanAllWorktrees = vi.fn().mockResolvedValue([
      {
        success: true,
        worktree: 'variant-control',
        chunkCount: 100,
        durationMs: 1000,
      },
    ])

    const mockValidateVariantEnvironment = vi.fn().mockResolvedValue({
      variantId: 'variant-control',
      worktreePath: testDir,
      checks: {
        worktreeExists: { passed: true, message: 'OK' },
        worktreeScanned: { passed: true, message: 'OK' },
        mcpConfigValid: { passed: true, message: 'OK' },
        toolsAccessible: { passed: true, message: 'OK' },
        filePermissions: { passed: true, message: 'OK' },
      },
      overall: 'pass' as const,
    })

    const mockSpawnAgent = vi.fn().mockResolvedValue({
      success: true,
      messages: [],
      sessionId: 'test-session',
      transcriptPath: testDir,
      performance: { durationMs: 5000, durationApiMs: 4000, numTurns: 3 },
    })

    const mockRunSearchTaskEvaluation = vi.fn().mockResolvedValue({
      compositeScore: 0.8,
      taskScore: {
        searchQuality: 0.8,
        taskCompletion: 0.7,
        efficiency: 0.9,
        total: 0.8,
        details: 'Mock evaluation',
      },
      searchMetrics: {
        searchCount: 2,
        targetFound: true,
      },
      searchUsageScore: 0.8,
    })

    // Apply mocks
    vi.doMock('../sdk/variant-injection.js', () => ({
      createVariantWorktree: mockCreateVariantWorktree,
    }))

    vi.doMock('./scan-orchestrator.js', () => ({
      scanAllWorktrees: mockScanAllWorktrees,
    }))

    vi.spyOn(PreFlightValidator.prototype, 'validateVariantEnvironment').mockImplementation(
      mockValidateVariantEnvironment,
    )

    vi.doMock('../sdk/spawner.js', () => ({
      spawnAgent: mockSpawnAgent,
    }))

    vi.doMock('../evaluation/search-checks.js', () => ({
      runSearchTaskEvaluation: mockRunSearchTaskEvaluation,
    }))

    const result = await runCompetition({
      task: MOCK_TASK,
      variants: [MOCK_VARIANTS[0]],
      parallelExecution: false,
      baseDir: testDir,
      timeout: 60,
    })

    // Verify setup metrics are included
    expect(result.setupMetrics).toBeDefined()
    expect(result.setupMetrics?.scanResults).toHaveLength(1)
    expect(result.setupMetrics?.validationResults).toHaveLength(1)
    expect(result.setupMetrics?.totalSetupTimeMs).toBeGreaterThan(0)

    // Verify scan results
    expect(result.setupMetrics?.scanResults[0].worktree).toBe('variant-control')
    expect(result.setupMetrics?.scanResults[0].chunkCount).toBe(100)

    // Verify validation results
    expect(result.setupMetrics?.validationResults[0].overall).toBe('pass')

    vi.clearAllMocks()
  }, 30000)

  it('validates all worktrees before agent spawn', async () => {
    // Check if database is available first
    const validator = new PreFlightValidator()
    const dbAvailable = await validator.checkDatabaseConnection()

    if (!dbAvailable) {
      console.warn('⚠️  Skipping test: PostgreSQL not available')
      return
    }

    // Check if base branch is indexed
    const baseIndexed = await validator.verifyBaseBranchIndexed('crewchief', 'main')
    if (!baseIndexed.indexed) {
      console.warn('⚠️  Skipping test: Base branch not indexed')
      return
    }

    const validateSpy = vi.spyOn(PreFlightValidator.prototype, 'validateVariantEnvironment')
    const spawnSpy = vi.fn()

    // Mock implementations
    const mockCreateVariantWorktree = vi.fn().mockResolvedValue({
      path: testDir,
      cleanup: vi.fn().mockResolvedValue(undefined),
    })

    const mockScanAllWorktrees = vi.fn().mockResolvedValue([
      {
        success: true,
        worktree: 'variant-control',
        chunkCount: 100,
        durationMs: 1000,
      },
      {
        success: true,
        worktree: 'variant-test',
        chunkCount: 105,
        durationMs: 1100,
      },
    ])

    validateSpy.mockResolvedValue({
      variantId: 'test',
      worktreePath: testDir,
      checks: {
        worktreeExists: { passed: true, message: 'OK' },
        worktreeScanned: { passed: true, message: 'OK' },
        mcpConfigValid: { passed: true, message: 'OK' },
        toolsAccessible: { passed: true, message: 'OK' },
        filePermissions: { passed: true, message: 'OK' },
      },
      overall: 'pass' as const,
    })

    const mockSpawnAgent = spawnSpy.mockResolvedValue({
      success: true,
      messages: [],
      sessionId: 'test-session',
      transcriptPath: testDir,
      performance: { durationMs: 5000, durationApiMs: 4000, numTurns: 3 },
    })

    const mockRunSearchTaskEvaluation = vi.fn().mockResolvedValue({
      compositeScore: 0.8,
      taskScore: {
        searchQuality: 0.8,
        taskCompletion: 0.7,
        efficiency: 0.9,
        total: 0.8,
        details: 'Mock evaluation',
      },
      searchMetrics: {
        searchCount: 2,
        targetFound: true,
      },
      searchUsageScore: 0.8,
    })

    // Apply mocks
    vi.doMock('../sdk/variant-injection.js', () => ({
      createVariantWorktree: mockCreateVariantWorktree,
    }))

    vi.doMock('./scan-orchestrator.js', () => ({
      scanAllWorktrees: mockScanAllWorktrees,
    }))

    vi.doMock('../sdk/spawner.js', () => ({
      spawnAgent: mockSpawnAgent,
    }))

    vi.doMock('../evaluation/search-checks.js', () => ({
      runSearchTaskEvaluation: mockRunSearchTaskEvaluation,
    }))

    await runCompetition({
      task: MOCK_TASK,
      variants: MOCK_VARIANTS,
      parallelExecution: false,
      baseDir: testDir,
    })

    // Validation should be called for each variant
    expect(validateSpy).toHaveBeenCalledTimes(2)
    expect(spawnSpy).toHaveBeenCalledTimes(2)

    // Get call order
    const validateOrder = validateSpy.mock.invocationCallOrder
    const spawnOrder = spawnSpy.mock.invocationCallOrder

    // All validate calls should precede all spawn calls
    const maxValidateOrder = Math.max(...validateOrder)
    const minSpawnOrder = Math.min(...spawnOrder)

    expect(maxValidateOrder).toBeLessThan(minSpawnOrder)

    validateSpy.mockRestore()
    vi.clearAllMocks()
  }, 30000)

  it('fails when worktree scan fails', async () => {
    // Check if database is available first
    const validator = new PreFlightValidator()
    const dbAvailable = await validator.checkDatabaseConnection()

    if (!dbAvailable) {
      console.warn('⚠️  Skipping test: PostgreSQL not available')
      return
    }

    // Check if base branch is indexed
    const baseIndexed = await validator.verifyBaseBranchIndexed('crewchief', 'main')
    if (!baseIndexed.indexed) {
      console.warn('⚠️  Skipping test: Base branch not indexed')
      return
    }

    // Mock implementations
    const mockCreateVariantWorktree = vi.fn().mockResolvedValue({
      path: testDir,
      cleanup: vi.fn().mockResolvedValue(undefined),
    })

    const mockScanAllWorktrees = vi.fn().mockRejectedValue(new Error('Scan failed for variant-a: Permission denied'))

    // Apply mocks
    vi.doMock('../sdk/variant-injection.js', () => ({
      createVariantWorktree: mockCreateVariantWorktree,
    }))

    vi.doMock('./scan-orchestrator.js', () => ({
      scanAllWorktrees: mockScanAllWorktrees,
    }))

    await expect(
      runCompetition({
        task: MOCK_TASK,
        variants: [MOCK_VARIANTS[0]],
        baseDir: testDir,
      }),
    ).rejects.toThrow(/Scan failed/)

    vi.clearAllMocks()
  }, 10000)

  it('cleans up worktrees on validation failure', async () => {
    // Check if database is available first
    const validator = new PreFlightValidator()
    const dbAvailable = await validator.checkDatabaseConnection()

    if (!dbAvailable) {
      console.warn('⚠️  Skipping test: PostgreSQL not available')
      return
    }

    // Check if base branch is indexed
    const baseIndexed = await validator.verifyBaseBranchIndexed('crewchief', 'main')
    if (!baseIndexed.indexed) {
      console.warn('⚠️  Skipping test: Base branch not indexed')
      return
    }

    const cleanupSpy = vi.fn().mockResolvedValue(undefined)

    // Mock implementations
    const mockCreateVariantWorktree = vi.fn().mockResolvedValue({
      path: testDir,
      cleanup: cleanupSpy,
    })

    const mockScanAllWorktrees = vi.fn().mockResolvedValue([
      {
        success: true,
        worktree: 'variant-control',
        chunkCount: 100,
        durationMs: 1000,
      },
    ])

    const mockValidateVariantEnvironment = vi.fn().mockResolvedValue({
      variantId: 'variant-control',
      worktreePath: testDir,
      checks: {
        worktreeExists: { passed: false, message: 'Directory not found' },
        worktreeScanned: { passed: true, message: 'OK' },
        mcpConfigValid: { passed: true, message: 'OK' },
        toolsAccessible: { passed: true, message: 'OK' },
        filePermissions: { passed: true, message: 'OK' },
      },
      overall: 'fail' as const,
      failureReason: 'Worktree does not exist',
    })

    // Apply mocks
    vi.doMock('../sdk/variant-injection.js', () => ({
      createVariantWorktree: mockCreateVariantWorktree,
    }))

    vi.doMock('./scan-orchestrator.js', () => ({
      scanAllWorktrees: mockScanAllWorktrees,
    }))

    vi.spyOn(PreFlightValidator.prototype, 'validateVariantEnvironment').mockImplementation(
      mockValidateVariantEnvironment,
    )

    await expect(
      runCompetition({
        task: MOCK_TASK,
        variants: [MOCK_VARIANTS[0]],
        baseDir: testDir,
      }),
    ).rejects.toThrow(/Pre-flight validation failed/)

    // Verify cleanup was called
    expect(cleanupSpy).toHaveBeenCalled()

    vi.clearAllMocks()
  }, 10000)
})
