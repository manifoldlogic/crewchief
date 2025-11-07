/**
 * Tests for validation report generator
 */

import { promises as fs } from 'node:fs'
import path from 'node:path'
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import type { SearchTask } from '../../types.js'
import {
  ReportGenerator,
  generateSummary,
  generatePerTaskResults,
  identifyPatterns,
  generateRecommendations,
  formatMarkdown,
  formatJSON,
  formatConsole,
  type Report,
  type ReportConfig,
} from '../reporter.js'
import type { ValidationResult, DimensionResult } from '../task-validator.js'

// Test data helpers
function createMockTask(id: string, name: string): SearchTask {
  return {
    id,
    name,
    category: 'test',
    tier: 1,
    difficulty: 'hard',
    query: 'test query',
    followUpTask: {
      instruction: 'test',
      validator: {
        type: 'code_change',
        filePath: '/test/file.ts',
        expectedChange: 'test change',
      },
    },
  } as SearchTask
}

function createMockDimensionResult(
  dimension: string,
  passed: boolean,
  actual: number | string,
  expected: number | string,
): DimensionResult {
  return {
    dimension,
    passed,
    actual,
    expected,
    details: passed ? `${dimension} passed` : `${dimension} failed`,
  }
}

function createMockValidationResult(
  taskId: string,
  passed: boolean,
  customDimensions?: Partial<ValidationResult['dimensions']>,
): ValidationResult {
  const task = createMockTask(taskId, `Task ${taskId}`)

  const defaultDimensions = {
    constructValidity: createMockDimensionResult('Construct Validity', true, 0.25, '≤ 0.3'),
    discriminantValidity: createMockDimensionResult('Discriminant Validity', true, '75% (Δ +45pp)', '≥ 70%'),
    ecologicalValidity: createMockDimensionResult('Ecological Validity', true, 'Score: 75%', 'Score ≥ 60%'),
    reliability: createMockDimensionResult('Reliability', true, 'CV = 8.0%', 'CV ≤ 10%'),
    statisticalPower: createMockDimensionResult('Statistical Power', true, 'n = 5', 'n ≥ 5'),
  }

  return {
    task,
    passed,
    tier: 'tier1-impossible',
    dimensions: { ...defaultDimensions, ...customDimensions },
    recommendations: passed
      ? ['Task passed all criteria']
      : ['Task failed validation. Review failed dimensions and apply fixes.'],
    timestamp: new Date('2025-01-01T00:00:00.000Z'),
  }
}

describe('ReportGenerator', () => {
  let tempDir: string

  beforeEach(async () => {
    // Create temporary directory for test reports
    tempDir = path.join(process.cwd(), 'test-reports-' + Date.now())
    await fs.mkdir(tempDir, { recursive: true })
  })

  afterEach(async () => {
    // Clean up temporary directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  })

  describe('constructor', () => {
    it('should create with default config', () => {
      const generator = new ReportGenerator()
      expect(generator).toBeDefined()
    })

    it('should create with custom config', () => {
      const config: ReportConfig = {
        format: 'json',
        outputDir: tempDir,
        includePatterns: false,
        includeRecommendations: false,
        verbose: true,
      }
      const generator = new ReportGenerator(config)
      expect(generator).toBeDefined()
    })
  })

  describe('generate', () => {
    it('should generate report with passing tasks', () => {
      const generator = new ReportGenerator()
      const results = [createMockValidationResult('TASK-001', true), createMockValidationResult('TASK-002', true)]

      const report = generator.generate(results, 'test-suite', 'tier1')

      expect(report.metadata.totalTasks).toBe(2)
      expect(report.metadata.suiteName).toBe('test-suite')
      expect(report.metadata.tier).toBe('tier1')
      expect(report.summary.passed).toBe(2)
      expect(report.summary.failed).toBe(0)
      expect(report.perTaskResults).toHaveLength(2)
      expect(report.markdown).toContain('Validation Report')
      expect(report.json).toContain('"totalTasks": 2')
    })

    it('should generate report with failing tasks', () => {
      const generator = new ReportGenerator()
      const results = [createMockValidationResult('TASK-001', false), createMockValidationResult('TASK-002', true)]

      const report = generator.generate(results)

      expect(report.summary.passed).toBe(1)
      expect(report.summary.failed).toBe(1)
      expect(report.summary.passRate).toBe(50)
    })

    it('should generate report with empty results', () => {
      const generator = new ReportGenerator()
      const report = generator.generate([])

      expect(report.metadata.totalTasks).toBe(0)
      expect(report.summary.passed).toBe(0)
      expect(report.summary.failed).toBe(0)
      expect(report.summary.passRate).toBe(0)
      expect(report.perTaskResults).toHaveLength(0)
    })

    it('should include patterns when configured', () => {
      const generator = new ReportGenerator({ includePatterns: true })
      const results = [
        createMockValidationResult('TASK-001', false, {
          constructValidity: createMockDimensionResult('Construct Validity', false, 0.7, '≤ 0.3'),
        }),
      ]

      const report = generator.generate(results)

      expect(report.patterns).toBeDefined()
      expect(report.patterns!.length).toBeGreaterThan(0)
    })

    it('should exclude patterns when configured', () => {
      const generator = new ReportGenerator({ includePatterns: false })
      const results = [createMockValidationResult('TASK-001', false)]

      const report = generator.generate(results)

      expect(report.patterns).toBeUndefined()
    })

    it('should include recommendations when configured', () => {
      const generator = new ReportGenerator({ includeRecommendations: true })
      const results = [createMockValidationResult('TASK-001', false)]

      const report = generator.generate(results)

      expect(report.recommendations).toBeDefined()
      expect(report.recommendations!.length).toBeGreaterThan(0)
    })

    it('should exclude recommendations when configured', () => {
      const generator = new ReportGenerator({ includeRecommendations: false })
      const results = [createMockValidationResult('TASK-001', false)]

      const report = generator.generate(results)

      expect(report.recommendations).toBeUndefined()
    })
  })

  describe('save', () => {
    it('should save markdown report to file', async () => {
      const generator = new ReportGenerator({ format: 'markdown', outputDir: tempDir })
      const results = [createMockValidationResult('TASK-001', true)]
      const report = generator.generate(results, 'test-suite')

      await generator.save(report)

      // Check file was created
      const files = await fs.readdir(tempDir)
      expect(files.length).toBe(1)
      expect(files[0]).toMatch(/^validation-report-test-suite-.+\.md$/)

      // Check content
      const content = await fs.readFile(path.join(tempDir, files[0]), 'utf-8')
      expect(content).toContain('# Validation Report')
      expect(content).toContain('TASK-001')
    })

    it('should save JSON report to file', async () => {
      const generator = new ReportGenerator({ format: 'json', outputDir: tempDir })
      const results = [createMockValidationResult('TASK-001', true)]
      const report = generator.generate(results, 'test-suite')

      await generator.save(report)

      // Check file was created
      const files = await fs.readdir(tempDir)
      expect(files.length).toBe(1)
      expect(files[0]).toMatch(/^validation-report-test-suite-.+\.json$/)

      // Check content is valid JSON
      const content = await fs.readFile(path.join(tempDir, files[0]), 'utf-8')
      const json = JSON.parse(content)
      expect(json.metadata.totalTasks).toBe(1)
    })

    it('should save with custom filename', async () => {
      const generator = new ReportGenerator({ outputDir: tempDir })
      const results = [createMockValidationResult('TASK-001', true)]
      const report = generator.generate(results)

      await generator.save(report, 'custom-report.md')

      const files = await fs.readdir(tempDir)
      expect(files).toContain('custom-report.md')
    })

    it('should create output directory if it does not exist', async () => {
      const nestedDir = path.join(tempDir, 'nested', 'dir')
      const generator = new ReportGenerator({ outputDir: nestedDir })
      const results = [createMockValidationResult('TASK-001', true)]
      const report = generator.generate(results)

      await generator.save(report)

      // Check directory was created
      const exists = await fs
        .access(nestedDir)
        .then(() => true)
        .catch(() => false)
      expect(exists).toBe(true)
    })
  })

  describe('print', () => {
    it('should print report to console', () => {
      const generator = new ReportGenerator()
      const results = [createMockValidationResult('TASK-001', true)]
      const report = generator.generate(results)

      // Should not throw
      expect(() => generator.print(report)).not.toThrow()
    })
  })
})

describe('generateSummary', () => {
  it('should calculate summary for all passing tasks', () => {
    const results = [createMockValidationResult('TASK-001', true), createMockValidationResult('TASK-002', true)]

    const summary = generateSummary(results)

    expect(summary.total).toBe(2)
    expect(summary.passed).toBe(2)
    expect(summary.failed).toBe(0)
    expect(summary.passRate).toBe(100)
  })

  it('should calculate summary for mixed results', () => {
    const results = [
      createMockValidationResult('TASK-001', true),
      createMockValidationResult('TASK-002', false),
      createMockValidationResult('TASK-003', false),
    ]

    const summary = generateSummary(results)

    expect(summary.total).toBe(3)
    expect(summary.passed).toBe(1)
    expect(summary.failed).toBe(2)
    expect(summary.passRate).toBeCloseTo(33.33, 1)
  })

  it('should calculate summary for empty results', () => {
    const summary = generateSummary([])

    expect(summary.total).toBe(0)
    expect(summary.passed).toBe(0)
    expect(summary.failed).toBe(0)
    expect(summary.passRate).toBe(0)
  })

  it('should calculate dimension breakdown correctly', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        constructValidity: createMockDimensionResult('Construct Validity', false, 0.7, '≤ 0.3'),
        discriminantValidity: createMockDimensionResult('Discriminant Validity', false, '50%', '≥ 70%'),
      }),
      createMockValidationResult('TASK-002', true),
    ]

    const summary = generateSummary(results)

    expect(summary.dimensionBreakdown.constructValidity.passed).toBe(1)
    expect(summary.dimensionBreakdown.constructValidity.failed).toBe(1)
    expect(summary.dimensionBreakdown.discriminantValidity.passed).toBe(1)
    expect(summary.dimensionBreakdown.discriminantValidity.failed).toBe(1)
    expect(summary.dimensionBreakdown.ecologicalValidity.passed).toBe(2)
    expect(summary.dimensionBreakdown.ecologicalValidity.failed).toBe(0)
  })
})

describe('generatePerTaskResults', () => {
  it('should generate per-task results', () => {
    const results = [createMockValidationResult('TASK-001', true), createMockValidationResult('TASK-002', false)]

    const perTaskResults = generatePerTaskResults(results)

    expect(perTaskResults).toHaveLength(2)
    expect(perTaskResults[0].taskId).toBe('TASK-001')
    expect(perTaskResults[0].passed).toBe(true)
    expect(perTaskResults[0].dimensions).toHaveLength(5)
    expect(perTaskResults[1].taskId).toBe('TASK-002')
    expect(perTaskResults[1].passed).toBe(false)
  })

  it('should include all dimension details', () => {
    const results = [createMockValidationResult('TASK-001', true)]

    const perTaskResults = generatePerTaskResults(results)

    const dimensions = perTaskResults[0].dimensions
    expect(dimensions).toHaveLength(5)
    expect(dimensions[0].name).toBe('Construct Validity')
    expect(dimensions[0].passed).toBeDefined()
    expect(dimensions[0].actual).toBeDefined()
    expect(dimensions[0].expected).toBeDefined()
    expect(dimensions[0].details).toBeDefined()
  })
})

describe('identifyPatterns', () => {
  it('should identify too-easy pattern', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        constructValidity: createMockDimensionResult('Construct Validity', false, 0.7, '≤ 0.3'),
      }),
    ]

    const patterns = identifyPatterns(results)

    const tooEasy = patterns.find((p) => p.pattern === 'too-easy')
    expect(tooEasy).toBeDefined()
    expect(tooEasy!.count).toBe(1)
    expect(tooEasy!.taskIds).toContain('TASK-001')
    expect(tooEasy!.characteristics.length).toBeGreaterThan(0)
    expect(tooEasy!.fixes.length).toBeGreaterThan(0)
  })

  it('should identify too-hard pattern', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        constructValidity: createMockDimensionResult('Construct Validity', true, 0.05, '≤ 0.3'),
        discriminantValidity: createMockDimensionResult('Discriminant Validity', false, '40% (Δ +35pp)', '≥ 70%'),
      }),
    ]

    const patterns = identifyPatterns(results)

    const tooHard = patterns.find((p) => p.pattern === 'too-hard')
    expect(tooHard).toBeDefined()
    expect(tooHard!.count).toBe(1)
    expect(tooHard!.taskIds).toContain('TASK-001')
  })

  it('should identify insufficient-advantage pattern', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        discriminantValidity: createMockDimensionResult('Discriminant Validity', false, '75% (Δ +15pp)', '≥ 70%'),
      }),
    ]

    const patterns = identifyPatterns(results)

    const insufficientAdvantage = patterns.find((p) => p.pattern === 'insufficient-advantage')
    expect(insufficientAdvantage).toBeDefined()
    expect(insufficientAdvantage!.count).toBe(1)
    expect(insufficientAdvantage!.taskIds).toContain('TASK-001')
  })

  it('should identify unreliable pattern', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        reliability: createMockDimensionResult('Reliability', false, 'CV = 25.0%', 'CV ≤ 10%'),
      }),
    ]

    const patterns = identifyPatterns(results)

    const unreliable = patterns.find((p) => p.pattern === 'unreliable')
    expect(unreliable).toBeDefined()
    expect(unreliable!.count).toBe(1)
    expect(unreliable!.taskIds).toContain('TASK-001')
  })

  it('should identify ecologically-invalid pattern', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        ecologicalValidity: createMockDimensionResult('Ecological Validity', false, 'Score: 45%', 'Score ≥ 60%'),
      }),
    ]

    const patterns = identifyPatterns(results)

    const ecologicallyInvalid = patterns.find((p) => p.pattern === 'ecologically-invalid')
    expect(ecologicallyInvalid).toBeDefined()
    expect(ecologicallyInvalid!.count).toBe(1)
    expect(ecologicallyInvalid!.taskIds).toContain('TASK-001')
  })

  it('should identify multiple patterns', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        constructValidity: createMockDimensionResult('Construct Validity', false, 0.7, '≤ 0.3'),
      }),
      createMockValidationResult('TASK-002', false, {
        reliability: createMockDimensionResult('Reliability', false, 'CV = 25.0%', 'CV ≤ 10%'),
      }),
    ]

    const patterns = identifyPatterns(results)

    expect(patterns.length).toBeGreaterThanOrEqual(2)
    expect(patterns.some((p) => p.pattern === 'too-easy')).toBe(true)
    expect(patterns.some((p) => p.pattern === 'unreliable')).toBe(true)
  })

  it('should return empty array for all passing tasks', () => {
    const results = [createMockValidationResult('TASK-001', true), createMockValidationResult('TASK-002', true)]

    const patterns = identifyPatterns(results)

    expect(patterns).toHaveLength(0)
  })
})

describe('generateRecommendations', () => {
  it('should generate recommendations for failed tasks only', () => {
    const results = [createMockValidationResult('TASK-001', false), createMockValidationResult('TASK-002', true)]

    const recommendations = generateRecommendations(results)

    expect(recommendations).toHaveLength(1)
    expect(recommendations[0].taskId).toBe('TASK-001')
    expect(recommendations[0].actions.length).toBeGreaterThan(0)
  })

  it('should prioritize high-severity failures', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        constructValidity: createMockDimensionResult('Construct Validity', false, 0.7, '≤ 0.3'),
        discriminantValidity: createMockDimensionResult('Discriminant Validity', false, '50%', '≥ 70%'),
        ecologicalValidity: createMockDimensionResult('Ecological Validity', false, 'Score: 45%', 'Score ≥ 60%'),
      }),
      createMockValidationResult('TASK-002', false, {
        reliability: createMockDimensionResult('Reliability', false, 'CV = 25.0%', 'CV ≤ 10%'),
      }),
    ]

    const recommendations = generateRecommendations(results)

    expect(recommendations[0].priority).toBe('high') // Multiple failures
    expect(recommendations[1].priority).toBe('medium') // Single failure
  })

  it('should mark ecological validity failures as high priority', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        ecologicalValidity: createMockDimensionResult('Ecological Validity', false, 'Score: 45%', 'Score ≥ 60%'),
      }),
    ]

    const recommendations = generateRecommendations(results)

    expect(recommendations[0].priority).toBe('high')
  })

  it('should return empty array for all passing tasks', () => {
    const results = [createMockValidationResult('TASK-001', true), createMockValidationResult('TASK-002', true)]

    const recommendations = generateRecommendations(results)

    expect(recommendations).toHaveLength(0)
  })

  it('should sort recommendations by priority', () => {
    const results = [
      createMockValidationResult('TASK-001', false, {
        reliability: createMockDimensionResult('Reliability', false, 'CV = 25.0%', 'CV ≤ 10%'),
      }),
      createMockValidationResult('TASK-002', false, {
        ecologicalValidity: createMockDimensionResult('Ecological Validity', false, 'Score: 45%', 'Score ≥ 60%'),
      }),
      createMockValidationResult('TASK-003', false, {
        constructValidity: createMockDimensionResult('Construct Validity', false, 0.7, '≤ 0.3'),
        discriminantValidity: createMockDimensionResult('Discriminant Validity', false, '50%', '≥ 70%'),
        ecologicalValidity: createMockDimensionResult('Ecological Validity', false, 'Score: 45%', 'Score ≥ 60%'),
      }),
    ]

    const recommendations = generateRecommendations(results)

    // High priority should come first
    expect(recommendations[0].priority).toBe('high')
    expect(recommendations[1].priority).toBe('high')
    expect(recommendations[2].priority).toBe('medium')
  })
})

describe('formatMarkdown', () => {
  it('should format basic report structure', () => {
    const report: Omit<Report, 'markdown' | 'json'> = {
      metadata: {
        timestamp: new Date('2025-01-01T00:00:00.000Z'),
        version: '1.0.0',
        totalTasks: 2,
        suiteName: 'test-suite',
        tier: 'tier1',
      },
      summary: {
        total: 2,
        passed: 2,
        failed: 0,
        passRate: 100,
        dimensionBreakdown: {
          constructValidity: { passed: 2, failed: 0 },
          discriminantValidity: { passed: 2, failed: 0 },
          ecologicalValidity: { passed: 2, failed: 0 },
          reliability: { passed: 2, failed: 0 },
          statisticalPower: { passed: 2, failed: 0 },
        },
      },
      perTaskResults: [
        {
          taskId: 'TASK-001',
          taskName: 'Test Task',
          passed: true,
          dimensions: [
            {
              name: 'Construct Validity',
              passed: true,
              actual: 0.25,
              expected: '≤ 0.3',
              details: 'Passed',
            },
          ],
          recommendations: [],
        },
      ],
    }

    const markdown = formatMarkdown(report)

    expect(markdown).toContain('# Validation Report')
    expect(markdown).toContain('**Suite:** test-suite')
    expect(markdown).toContain('**Tier:** tier1')
    expect(markdown).toContain('## Summary')
    expect(markdown).toContain('100.0%')
    expect(markdown).toContain('## Per-Task Results')
    expect(markdown).toContain('TASK-001')
  })

  it('should include failure patterns section when present', () => {
    const report: Omit<Report, 'markdown' | 'json'> = {
      metadata: {
        timestamp: new Date('2025-01-01T00:00:00.000Z'),
        version: '1.0.0',
        totalTasks: 1,
      },
      summary: {
        total: 1,
        passed: 0,
        failed: 1,
        passRate: 0,
        dimensionBreakdown: {
          constructValidity: { passed: 0, failed: 1 },
          discriminantValidity: { passed: 1, failed: 0 },
          ecologicalValidity: { passed: 1, failed: 0 },
          reliability: { passed: 1, failed: 0 },
          statisticalPower: { passed: 1, failed: 0 },
        },
      },
      perTaskResults: [],
      patterns: [
        {
          pattern: 'too-easy',
          description: 'Tasks that are too easy',
          count: 1,
          taskIds: ['TASK-001'],
          characteristics: ['High grep success'],
          fixes: ['Add complexity'],
        },
      ],
    }

    const markdown = formatMarkdown(report)

    expect(markdown).toContain('## Failure Patterns')
    expect(markdown).toContain('Tasks that are too easy')
    expect(markdown).toContain('Add complexity')
  })

  it('should include recommendations section when present', () => {
    const report: Omit<Report, 'markdown' | 'json'> = {
      metadata: {
        timestamp: new Date('2025-01-01T00:00:00.000Z'),
        version: '1.0.0',
        totalTasks: 1,
      },
      summary: {
        total: 1,
        passed: 0,
        failed: 1,
        passRate: 0,
        dimensionBreakdown: {
          constructValidity: { passed: 0, failed: 1 },
          discriminantValidity: { passed: 1, failed: 0 },
          ecologicalValidity: { passed: 1, failed: 0 },
          reliability: { passed: 1, failed: 0 },
          statisticalPower: { passed: 1, failed: 0 },
        },
      },
      perTaskResults: [],
      recommendations: [
        {
          taskId: 'TASK-001',
          priority: 'high',
          actions: ['Fix the issue', 'Add more tests'],
        },
      ],
    }

    const markdown = formatMarkdown(report)

    expect(markdown).toContain('## Recommendations')
    expect(markdown).toContain('TASK-001')
    expect(markdown).toContain('HIGH Priority')
    expect(markdown).toContain('Fix the issue')
  })

  it('should use correct emojis for status', () => {
    const passingReport: Omit<Report, 'markdown' | 'json'> = {
      metadata: { timestamp: new Date(), version: '1.0.0', totalTasks: 1 },
      summary: {
        total: 1,
        passed: 1,
        failed: 0,
        passRate: 100,
        dimensionBreakdown: {
          constructValidity: { passed: 1, failed: 0 },
          discriminantValidity: { passed: 1, failed: 0 },
          ecologicalValidity: { passed: 1, failed: 0 },
          reliability: { passed: 1, failed: 0 },
          statisticalPower: { passed: 1, failed: 0 },
        },
      },
      perTaskResults: [],
    }

    const failingReport: Omit<Report, 'markdown' | 'json'> = {
      metadata: { timestamp: new Date(), version: '1.0.0', totalTasks: 1 },
      summary: {
        total: 1,
        passed: 0,
        failed: 1,
        passRate: 0,
        dimensionBreakdown: {
          constructValidity: { passed: 0, failed: 1 },
          discriminantValidity: { passed: 1, failed: 0 },
          ecologicalValidity: { passed: 1, failed: 0 },
          reliability: { passed: 1, failed: 0 },
          statisticalPower: { passed: 1, failed: 0 },
        },
      },
      perTaskResults: [],
    }

    const passingMarkdown = formatMarkdown(passingReport)
    const failingMarkdown = formatMarkdown(failingReport)

    expect(passingMarkdown).toContain('✅')
    expect(failingMarkdown).toContain('❌')
  })
})

describe('formatJSON', () => {
  it('should format valid JSON', () => {
    const report: Omit<Report, 'markdown' | 'json'> = {
      metadata: {
        timestamp: new Date('2025-01-01T00:00:00.000Z'),
        version: '1.0.0',
        totalTasks: 1,
      },
      summary: {
        total: 1,
        passed: 1,
        failed: 0,
        passRate: 100,
        dimensionBreakdown: {
          constructValidity: { passed: 1, failed: 0 },
          discriminantValidity: { passed: 1, failed: 0 },
          ecologicalValidity: { passed: 1, failed: 0 },
          reliability: { passed: 1, failed: 0 },
          statisticalPower: { passed: 1, failed: 0 },
        },
      },
      perTaskResults: [],
    }

    const json = formatJSON(report)

    expect(() => JSON.parse(json)).not.toThrow()
    const parsed = JSON.parse(json)
    expect(parsed.metadata.totalTasks).toBe(1)
    expect(parsed.summary.passRate).toBe(100)
  })

  it('should include all report sections', () => {
    const report: Omit<Report, 'markdown' | 'json'> = {
      metadata: { timestamp: new Date(), version: '1.0.0', totalTasks: 1 },
      summary: {
        total: 1,
        passed: 0,
        failed: 1,
        passRate: 0,
        dimensionBreakdown: {
          constructValidity: { passed: 0, failed: 1 },
          discriminantValidity: { passed: 1, failed: 0 },
          ecologicalValidity: { passed: 1, failed: 0 },
          reliability: { passed: 1, failed: 0 },
          statisticalPower: { passed: 1, failed: 0 },
        },
      },
      perTaskResults: [],
      patterns: [{ pattern: 'too-easy', description: 'Test', count: 1, taskIds: [], characteristics: [], fixes: [] }],
      recommendations: [{ taskId: 'TASK-001', priority: 'high', actions: [] }],
    }

    const json = formatJSON(report)
    const parsed = JSON.parse(json)

    expect(parsed.metadata).toBeDefined()
    expect(parsed.summary).toBeDefined()
    expect(parsed.perTaskResults).toBeDefined()
    expect(parsed.patterns).toBeDefined()
    expect(parsed.recommendations).toBeDefined()
  })
})

describe('formatConsole', () => {
  it('should format console output', () => {
    const report: Report = {
      metadata: {
        timestamp: new Date('2025-01-01T00:00:00.000Z'),
        version: '1.0.0',
        totalTasks: 2,
        suiteName: 'test-suite',
      },
      summary: {
        total: 2,
        passed: 1,
        failed: 1,
        passRate: 50,
        dimensionBreakdown: {
          constructValidity: { passed: 1, failed: 1 },
          discriminantValidity: { passed: 2, failed: 0 },
          ecologicalValidity: { passed: 2, failed: 0 },
          reliability: { passed: 2, failed: 0 },
          statisticalPower: { passed: 2, failed: 0 },
        },
      },
      perTaskResults: [
        {
          taskId: 'TASK-001',
          taskName: 'Test Task',
          passed: false,
          dimensions: [],
          recommendations: [],
        },
      ],
      markdown: '',
      json: '',
    }

    const console = formatConsole(report)

    expect(console).toContain('VALIDATION REPORT')
    expect(console).toContain('test-suite')
    expect(console).toContain('50.0%')
    expect(console).toContain('TASK-001')
  })

  it('should include patterns section when present', () => {
    const report: Report = {
      metadata: { timestamp: new Date(), version: '1.0.0', totalTasks: 1 },
      summary: {
        total: 1,
        passed: 0,
        failed: 1,
        passRate: 0,
        dimensionBreakdown: {
          constructValidity: { passed: 0, failed: 1 },
          discriminantValidity: { passed: 1, failed: 0 },
          ecologicalValidity: { passed: 1, failed: 0 },
          reliability: { passed: 1, failed: 0 },
          statisticalPower: { passed: 1, failed: 0 },
        },
      },
      perTaskResults: [],
      patterns: [
        {
          pattern: 'too-easy',
          description: 'Tasks that are too easy',
          count: 1,
          taskIds: [],
          characteristics: [],
          fixes: [],
        },
      ],
      markdown: '',
      json: '',
    }

    const console = formatConsole(report)

    expect(console).toContain('Failure Patterns')
    expect(console).toContain('Tasks that are too easy')
  })
})
