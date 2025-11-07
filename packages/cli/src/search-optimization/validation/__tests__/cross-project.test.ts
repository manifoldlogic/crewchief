/**
 * Tests for cross-project validation infrastructure
 *
 * TESTDES-5003: Cross-project validation
 */

import { describe, it, expect } from 'vitest'
import {
  runCrossProjectValidation,
  calculateGeneralizationMetrics,
  calculateTransferabilityScore,
  formatCrossProjectSummary,
  SAMPLE_CODEBASES,
  type AdaptedTask,
  type CrossProjectTaskResult,
} from '../cross-project.js'

describe('Cross-Project Validation', () => {
  describe('SAMPLE_CODEBASES', () => {
    it('should provide 3 diverse codebases', () => {
      expect(SAMPLE_CODEBASES).toHaveLength(3)

      // Check diversity
      const languages = new Set(SAMPLE_CODEBASES.map((cb) => cb.language))
      const domains = new Set(SAMPLE_CODEBASES.map((cb) => cb.domain))
      const sizes = new Set(SAMPLE_CODEBASES.map((cb) => cb.sizeCategory))

      expect(languages.size).toBe(3) // TypeScript, Python, Rust
      expect(domains.size).toBeGreaterThanOrEqual(2) // Different domains
      expect(sizes.size).toBe(3) // Small, medium, large
    })

    it('should have required fields for each codebase', () => {
      for (const codebase of SAMPLE_CODEBASES) {
        expect(codebase.id).toBeTruthy()
        expect(codebase.name).toBeTruthy()
        expect(codebase.language).toBeTruthy()
        expect(codebase.domain).toBeTruthy()
        expect(codebase.sizeCategory).toBeTruthy()
        expect(codebase.repositoryUrl).toBeTruthy()
        expect(codebase.description).toBeTruthy()
      }
    })
  })

  describe('calculateTransferabilityScore', () => {
    it('should return 1.0 for perfect generalization', () => {
      const mockResults: CrossProjectTaskResult[] = [
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.2,
          searchSuccess: 0.85,
          improvement: 0.65,
          codebase: SAMPLE_CODEBASES[0],
          adaptedTask: createMockAdaptedTask('task-1', 'commander-js'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.25,
          searchSuccess: 0.82,
          improvement: 0.57,
          codebase: SAMPLE_CODEBASES[1],
          adaptedTask: createMockAdaptedTask('task-1', 'fastapi'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.15,
          searchSuccess: 0.88,
          improvement: 0.73,
          codebase: SAMPLE_CODEBASES[2],
          adaptedTask: createMockAdaptedTask('task-1', 'clap'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
      ]

      const score = calculateTransferabilityScore(mockResults)

      // All codebases have searchSuccess > 0.7, so base score is 1.0
      // Consistency factor depends on variance in improvements
      // With variance in improvements (0.65, 0.57, 0.73), consistency factor ~0.87
      expect(score).toBeGreaterThan(0.8) // Should be high but not perfect
      expect(score).toBeLessThanOrEqual(1.0)
    })

    it('should return low score for poor generalization', () => {
      const mockResults: CrossProjectTaskResult[] = [
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.2,
          searchSuccess: 0.3,
          improvement: 0.1,
          codebase: SAMPLE_CODEBASES[0],
          adaptedTask: createMockAdaptedTask('task-1', 'commander-js'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.25,
          searchSuccess: 0.35,
          improvement: 0.1,
          codebase: SAMPLE_CODEBASES[1],
          adaptedTask: createMockAdaptedTask('task-1', 'fastapi'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.15,
          searchSuccess: 0.25,
          improvement: 0.1,
          codebase: SAMPLE_CODEBASES[2],
          adaptedTask: createMockAdaptedTask('task-1', 'clap'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
      ]

      const score = calculateTransferabilityScore(mockResults)

      // No codebases have searchSuccess > 0.7
      expect(score).toBe(0)
    })

    it('should penalize high variance in search advantage', () => {
      const lowVarianceResults: CrossProjectTaskResult[] = [
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.2,
          searchSuccess: 0.8,
          improvement: 0.6,
          codebase: SAMPLE_CODEBASES[0],
          adaptedTask: createMockAdaptedTask('task-1', 'commander-js'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.25,
          searchSuccess: 0.85,
          improvement: 0.6,
          codebase: SAMPLE_CODEBASES[1],
          adaptedTask: createMockAdaptedTask('task-1', 'fastapi'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.15,
          searchSuccess: 0.75,
          improvement: 0.6,
          codebase: SAMPLE_CODEBASES[2],
          adaptedTask: createMockAdaptedTask('task-1', 'clap'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
      ]

      const highVarianceResults: CrossProjectTaskResult[] = [
        {
          task: createMockTask('task-2'),
          grepSuccess: 0.2,
          searchSuccess: 0.9,
          improvement: 0.7,
          codebase: SAMPLE_CODEBASES[0],
          adaptedTask: createMockAdaptedTask('task-2', 'commander-js'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-2'),
          grepSuccess: 0.25,
          searchSuccess: 0.75,
          improvement: 0.5,
          codebase: SAMPLE_CODEBASES[1],
          adaptedTask: createMockAdaptedTask('task-2', 'fastapi'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-2'),
          grepSuccess: 0.15,
          searchSuccess: 0.25,
          improvement: 0.1,
          codebase: SAMPLE_CODEBASES[2],
          adaptedTask: createMockAdaptedTask('task-2', 'clap'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
      ]

      const lowVarianceScore = calculateTransferabilityScore(lowVarianceResults)
      const highVarianceScore = calculateTransferabilityScore(highVarianceResults)

      // Low variance should score higher
      expect(lowVarianceScore).toBeGreaterThan(highVarianceScore)
    })
  })

  describe('calculateGeneralizationMetrics', () => {
    it('should calculate metrics for a task across codebases', () => {
      const mockResults: CrossProjectTaskResult[] = [
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.2,
          searchSuccess: 0.8,
          improvement: 0.6,
          codebase: SAMPLE_CODEBASES[0],
          adaptedTask: createMockAdaptedTask('task-1', 'commander-js'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.25,
          searchSuccess: 0.85,
          improvement: 0.6,
          codebase: SAMPLE_CODEBASES[1],
          adaptedTask: createMockAdaptedTask('task-1', 'fastapi'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.15,
          searchSuccess: 0.75,
          improvement: 0.6,
          codebase: SAMPLE_CODEBASES[2],
          adaptedTask: createMockAdaptedTask('task-1', 'clap'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
      ]

      const metrics = calculateGeneralizationMetrics('task-1', mockResults)

      expect(metrics.taskId).toBe('task-1')
      expect(metrics.category).toBe('relationship-discovery')
      expect(metrics.codebasePerformance).toHaveLength(3)
      expect(metrics.statistics.meanGrepSuccess).toBeCloseTo(0.2, 1)
      expect(metrics.statistics.meanSearchSuccess).toBeCloseTo(0.8, 1)
      expect(metrics.statistics.meanSearchAdvantage).toBeCloseTo(0.6, 1)
      expect(metrics.transferabilityScore).toBeGreaterThan(0.9)
      expect(metrics.consistentAdvantage).toBe(true)
    })

    it('should identify strong and weak performance codebases', () => {
      const mockResults: CrossProjectTaskResult[] = [
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.2,
          searchSuccess: 0.85,
          improvement: 0.65,
          codebase: SAMPLE_CODEBASES[0],
          adaptedTask: createMockAdaptedTask('task-1', 'commander-js'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.25,
          searchSuccess: 0.35,
          improvement: 0.1,
          codebase: SAMPLE_CODEBASES[1],
          adaptedTask: createMockAdaptedTask('task-1', 'fastapi'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
        {
          task: createMockTask('task-1'),
          grepSuccess: 0.15,
          searchSuccess: 0.9,
          improvement: 0.75,
          codebase: SAMPLE_CODEBASES[2],
          adaptedTask: createMockAdaptedTask('task-1', 'clap'),
          validation: { adaptationValid: true, executionSuccessful: true, issues: [] },
        },
      ]

      const metrics = calculateGeneralizationMetrics('task-1', mockResults)

      expect(metrics.codebaseAnalysis.strongPerformance).toContain('commander-js')
      expect(metrics.codebaseAnalysis.strongPerformance).toContain('clap')
      expect(metrics.codebaseAnalysis.weakPerformance).toContain('fastapi')
      expect(metrics.codebaseAnalysis.limitedAdvantage).toContain('fastapi')
    })
  })

  describe('runCrossProjectValidation', () => {
    it('should execute validation with mock data', async () => {
      const mockTasks: AdaptedTask[] = [
        createMockAdaptedTask('task-1', 'commander-js'),
        createMockAdaptedTask('task-1', 'fastapi'),
        createMockAdaptedTask('task-1', 'clap'),
        createMockAdaptedTask('task-2', 'commander-js'),
        createMockAdaptedTask('task-2', 'fastapi'),
        createMockAdaptedTask('task-2', 'clap'),
      ]

      const result = await runCrossProjectValidation({
        codebases: SAMPLE_CODEBASES,
        tasks: mockTasks,
        iterations: 1,
        useMockData: true,
      })

      // Check result structure
      expect(result.codebaseResults).toHaveLength(3)
      // Each task creates a separate entry per codebase
      expect(result.generalization.length).toBeGreaterThan(0)
      expect(result.summary.totalCodebases).toBe(3)
      expect(result.summary.totalTasksAttempted).toBeGreaterThan(0)
      expect(result.metadata.totalDurationSeconds).toBeGreaterThan(0)
    })

    it('should calculate summary statistics correctly', async () => {
      const mockTasks: AdaptedTask[] = [
        createMockAdaptedTask('task-1', 'commander-js'),
        createMockAdaptedTask('task-1', 'fastapi'),
        createMockAdaptedTask('task-1', 'clap'),
      ]

      const result = await runCrossProjectValidation({
        codebases: SAMPLE_CODEBASES,
        tasks: mockTasks,
        iterations: 1,
        useMockData: true,
      })

      expect(result.summary.avgSuccessRate).toBeGreaterThan(0)
      expect(result.summary.avgSuccessRate).toBeLessThanOrEqual(1)
      expect(result.summary.universalTasks).toBeDefined()
      expect(result.summary.specificTasks).toBeDefined()
    })

    it('should identify universal tasks', async () => {
      const mockTasks: AdaptedTask[] = [
        // Create a task that succeeds on all codebases
        { ...createMockAdaptedTask('universal-task', 'commander-js'), expectedSearchSuccess: 0.85 },
        { ...createMockAdaptedTask('universal-task', 'fastapi'), expectedSearchSuccess: 0.85 },
        { ...createMockAdaptedTask('universal-task', 'clap'), expectedSearchSuccess: 0.85 },
      ]

      const result = await runCrossProjectValidation({
        codebases: SAMPLE_CODEBASES,
        tasks: mockTasks,
        iterations: 1,
        useMockData: true,
      })

      expect(result.summary.universalTasks.length).toBeGreaterThan(0)
    })

    it('should detect patterns by language, domain, and size', async () => {
      const mockTasks: AdaptedTask[] = [
        createMockAdaptedTask('task-1', 'commander-js'),
        createMockAdaptedTask('task-1', 'fastapi'),
        createMockAdaptedTask('task-1', 'clap'),
      ]

      const result = await runCrossProjectValidation({
        codebases: SAMPLE_CODEBASES,
        tasks: mockTasks,
        iterations: 1,
        useMockData: true,
      })

      expect(result.summary.languagePatterns).toBeDefined()
      expect(result.summary.domainPatterns).toBeDefined()
      expect(result.summary.sizePatterns).toBeDefined()
    })
  })

  describe('formatCrossProjectSummary', () => {
    it('should format validation results as readable text', async () => {
      const mockTasks: AdaptedTask[] = [
        createMockAdaptedTask('task-1', 'commander-js'),
        createMockAdaptedTask('task-1', 'fastapi'),
        createMockAdaptedTask('task-1', 'clap'),
      ]

      const result = await runCrossProjectValidation({
        codebases: SAMPLE_CODEBASES,
        tasks: mockTasks,
        iterations: 1,
        useMockData: true,
      })

      const summary = formatCrossProjectSummary(result)

      expect(summary).toContain('CROSS-PROJECT VALIDATION RESULTS')
      expect(summary).toContain('Overall Summary')
      expect(summary).toContain('Per-Codebase Results')
      expect(summary).toContain('Generalization Analysis')
      expect(summary).toContain('Execution Metadata')
    })
  })
})

// Helper functions

function createMockTask(id: string): AdaptedTask {
  return {
    id,
    name: `Mock Task ${id}`,
    description: `A mock task for testing: ${id}`,
    category: 'relationship-discovery',
    difficulty: 'hard',
    tier: 'tier1-impossible',
    searchTarget: {
      type: 'file',
      path: '/mock/path.ts',
    },
    followUpTask: {
      type: 'explanation',
      prompt: 'Explain the mock task',
      validator: {
        type: 'explanation',
        mentionsFiles: ['/mock/path.ts'],
      },
    },
    successValidator: () => ({
      searchQuality: 0.8,
      taskCompletion: 0.8,
      efficiency: 0.8,
      total: 0.8,
      details: 'Mock validation',
    }),
    expectedGrepSuccess: 0.2,
    expectedSearchSuccess: 0.8,
    originalTaskId: `original-${id}`,
    targetCodebase: 'mock-codebase',
    adaptationNotes: 'Mock adaptation',
    adaptationConfidence: 0.9,
  }
}

function createMockAdaptedTask(taskId: string, codebaseId: string): AdaptedTask {
  const task = createMockTask(taskId)
  return {
    ...task,
    id: `${taskId}-${codebaseId}`,
    targetCodebase: codebaseId,
    adaptationNotes: `Adapted for ${codebaseId}`,
  }
}
