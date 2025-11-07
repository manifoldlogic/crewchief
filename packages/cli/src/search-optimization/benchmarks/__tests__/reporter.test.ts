/**
 * Tests for suite report generation
 */

import { describe, it, expect } from 'vitest'
import { generateSuiteReport, formatReportMarkdown, formatReportText, formatCompactSummary } from '../reporter.js'
import { TIER1_GREP_IMPOSSIBLE_SUITE } from '../tier1-impossible.js'

describe('generateSuiteReport', () => {
  const report = generateSuiteReport(TIER1_GREP_IMPOSSIBLE_SUITE)

  describe('Report Structure', () => {
    it('should include suite name', () => {
      expect(report.suiteName).toBe('Tier 1: Grep-Impossible Tasks')
    })

    it('should include suite version', () => {
      expect(report.suiteVersion).toBe('1.0.0')
    })

    it('should include summary', () => {
      expect(report.summary).toBeDefined()
    })

    it('should include task breakdown', () => {
      expect(report.taskBreakdown).toBeDefined()
      expect(Array.isArray(report.taskBreakdown)).toBe(true)
    })

    it('should include category breakdown', () => {
      expect(report.categoryBreakdown).toBeDefined()
      expect(report.categoryBreakdown).toBeInstanceOf(Map)
    })

    it('should include validation results', () => {
      expect(report.validation).toBeDefined()
    })
  })

  describe('Summary', () => {
    it('should have correct total tasks', () => {
      expect(report.summary.totalTasks).toBe(8)
    })

    it('should have 3 categories', () => {
      expect(report.summary.categories).toHaveLength(3)
    })

    it('should have valid grep success rate', () => {
      expect(report.summary.expectedGrepSuccess).toBeGreaterThanOrEqual(0)
      expect(report.summary.expectedGrepSuccess).toBeLessThanOrEqual(1)
    })

    it('should have valid search success rate', () => {
      expect(report.summary.expectedSearchSuccess).toBeGreaterThanOrEqual(0)
      expect(report.summary.expectedSearchSuccess).toBeLessThanOrEqual(1)
    })

    it('should have positive improvement', () => {
      expect(report.summary.expectedImprovement).toBeGreaterThan(0)
    })

    it('should calculate improvement correctly', () => {
      expect(report.summary.expectedImprovement).toBeCloseTo(
        report.summary.expectedSearchSuccess - report.summary.expectedGrepSuccess,
        5,
      )
    })

    it('should have high grep failure rate', () => {
      expect(report.summary.grepFailureRate).toBeGreaterThanOrEqual(0.8)
    })
  })

  describe('Task Breakdown', () => {
    it('should have 8 tasks', () => {
      expect(report.taskBreakdown).toHaveLength(8)
    })

    it('should have all required fields for each task', () => {
      for (const task of report.taskBreakdown) {
        expect(task.id).toBeTruthy()
        expect(task.name).toBeTruthy()
        expect(task.category).toBeTruthy()
        expect(task.difficulty).toBeTruthy()
        expect(task.expectedGrepSuccess).toBeGreaterThanOrEqual(0)
        expect(task.expectedSearchSuccess).toBeGreaterThanOrEqual(0)
        expect(task.improvement).toBeTypeOf('number')
      }
    })

    it('should calculate improvement correctly for each task', () => {
      for (const task of report.taskBreakdown) {
        expect(task.improvement).toBeCloseTo(task.expectedSearchSuccess - task.expectedGrepSuccess, 5)
      }
    })

    it('should be sorted by improvement (highest first)', () => {
      for (let i = 1; i < report.taskBreakdown.length; i++) {
        expect(report.taskBreakdown[i - 1].improvement).toBeGreaterThanOrEqual(report.taskBreakdown[i].improvement)
      }
    })

    it('should have unique task IDs', () => {
      const ids = report.taskBreakdown.map((t) => t.id)
      const uniqueIds = new Set(ids)
      expect(uniqueIds.size).toBe(ids.length)
    })
  })

  describe('Category Breakdown', () => {
    it('should have 3 categories', () => {
      expect(report.categoryBreakdown.size).toBe(3)
    })

    it('should have statistics for relationship-discovery', () => {
      const stats = report.categoryBreakdown.get('relationship-discovery')
      expect(stats).toBeDefined()
      expect(stats!.taskCount).toBe(3)
    })

    it('should have statistics for architectural-understanding', () => {
      const stats = report.categoryBreakdown.get('architectural-understanding')
      expect(stats).toBeDefined()
      expect(stats!.taskCount).toBe(3)
    })

    it('should have statistics for negative-space', () => {
      const stats = report.categoryBreakdown.get('negative-space')
      expect(stats).toBeDefined()
      expect(stats!.taskCount).toBe(2)
    })

    it('should have valid success rates for each category', () => {
      for (const [, stats] of report.categoryBreakdown) {
        expect(stats.avgGrepSuccess).toBeGreaterThanOrEqual(0)
        expect(stats.avgGrepSuccess).toBeLessThanOrEqual(1)
        expect(stats.avgSearchSuccess).toBeGreaterThanOrEqual(0)
        expect(stats.avgSearchSuccess).toBeLessThanOrEqual(1)
      }
    })
  })

  describe('Validation', () => {
    it('should include validation results', () => {
      expect(report.validation.passed).toBe(true)
    })

    it('should have validation metrics', () => {
      expect(report.validation.grepFailureRate).toBeDefined()
      expect(report.validation.categoryCoverage).toBeDefined()
    })
  })
})

describe('formatReportMarkdown', () => {
  const report = generateSuiteReport(TIER1_GREP_IMPOSSIBLE_SUITE)
  const markdown = formatReportMarkdown(report)

  it('should return a non-empty string', () => {
    expect(markdown).toBeTruthy()
    expect(markdown.length).toBeGreaterThan(100)
  })

  it('should have markdown heading for suite name', () => {
    expect(markdown).toContain('# Tier 1: Grep-Impossible Tasks')
  })

  it('should have executive summary section', () => {
    expect(markdown).toContain('## Executive Summary')
  })

  it('should have validation status section', () => {
    expect(markdown).toContain('## Validation Status')
  })

  it('should have task breakdown section', () => {
    expect(markdown).toContain('## Task Breakdown')
  })

  it('should have category breakdown section', () => {
    expect(markdown).toContain('## Category Breakdown')
  })

  it('should have markdown table for tasks', () => {
    expect(markdown).toContain('| Task ID | Name | Category | Difficulty | Grep | Search | Improvement |')
    expect(markdown).toContain('|---------|------|----------|------------|------|--------|-------------|')
  })

  it('should include all 8 tasks in table', () => {
    const tableRows = markdown.split('\n').filter((line) => line.startsWith('| ') && !line.includes('---'))
    // Header row + 8 data rows
    expect(tableRows.length).toBeGreaterThanOrEqual(9)
  })

  it('should format success rates as percentages', () => {
    expect(markdown).toMatch(/\d+%/)
  })

  it('should show improvement with plus sign', () => {
    expect(markdown).toMatch(/\+\d+%/)
  })

  it('should include validation status checkmark or X', () => {
    expect(markdown).toMatch(/✓ PASSED|✗ FAILED/)
  })

  it('should include category subsections', () => {
    expect(markdown).toContain('### relationship-discovery')
    expect(markdown).toContain('### architectural-understanding')
    expect(markdown).toContain('### negative-space')
  })

  it('should include category statistics', () => {
    expect(markdown).toContain('Avg Grep Success:')
    expect(markdown).toContain('Avg Search Success:')
    expect(markdown).toContain('Avg Improvement:')
  })
})

describe('formatReportText', () => {
  const report = generateSuiteReport(TIER1_GREP_IMPOSSIBLE_SUITE)
  const text = formatReportText(report)

  it('should return a non-empty string', () => {
    expect(text).toBeTruthy()
    expect(text.length).toBeGreaterThan(100)
  })

  it('should have header with suite name', () => {
    expect(text).toContain('Tier 1: Grep-Impossible Tasks')
  })

  it('should have section dividers', () => {
    expect(text).toContain('='.repeat(80))
    expect(text).toContain('-'.repeat(80))
  })

  it('should have SUMMARY section', () => {
    expect(text).toContain('SUMMARY')
  })

  it('should have VALIDATION section', () => {
    expect(text).toContain('VALIDATION')
  })

  it('should have TASKS section', () => {
    expect(text).toContain('TASKS')
  })

  it('should have CATEGORIES section', () => {
    expect(text).toContain('CATEGORIES')
  })

  it('should format success rates as percentages', () => {
    expect(text).toMatch(/\d+\.\d%/)
  })

  it('should show validation status', () => {
    expect(text).toMatch(/Status: (PASSED|FAILED)/)
  })

  it('should list all tasks', () => {
    // Should have at least 8 lines with task info (one per task)
    const taskLines = text.split('\n').filter((line) => line.includes('Grep:') && line.includes('Search:'))
    expect(taskLines.length).toBeGreaterThanOrEqual(8)
  })

  it('should show category statistics', () => {
    expect(text).toContain('relationship-discovery:')
    expect(text).toContain('architectural-understanding:')
    expect(text).toContain('negative-space:')
  })

  it('should be properly aligned', () => {
    // Check for consistent padding/alignment
    const lines = text.split('\n')
    // Task lines in TASKS section contain both "Grep:" and "Search:" and start with task ID
    // They should all use pipe separators for alignment
    let inTasksSection = false
    const taskLines: string[] = []

    for (const line of lines) {
      if (line === 'TASKS') {
        inTasksSection = true
      } else if (line === 'CATEGORIES') {
        inTasksSection = false
      } else if (inTasksSection && line.includes('Grep:') && line.includes('Search:')) {
        taskLines.push(line)
      }
    }

    // All task lines in TASKS section should have pipes
    expect(taskLines.length).toBeGreaterThan(0)
    expect(taskLines.every((line) => line.includes('|'))).toBe(true)
  })
})

describe('formatCompactSummary', () => {
  const report = generateSuiteReport(TIER1_GREP_IMPOSSIBLE_SUITE)
  const compact = formatCompactSummary(report)

  it('should return a non-empty string', () => {
    expect(compact).toBeTruthy()
  })

  it('should be a single line', () => {
    expect(compact.split('\n').length).toBe(1)
  })

  it('should include suite name', () => {
    expect(compact).toContain('Tier 1: Grep-Impossible Tasks')
  })

  it('should include version', () => {
    expect(compact).toContain('v1.0.0')
  })

  it('should include task count', () => {
    expect(compact).toContain('8 tasks')
  })

  it('should include grep success rate', () => {
    expect(compact).toMatch(/\d+% grep/)
  })

  it('should include search success rate', () => {
    expect(compact).toMatch(/\d+% search/)
  })

  it('should include improvement', () => {
    expect(compact).toMatch(/\(\+\d+%\)/)
  })

  it('should include validation status', () => {
    expect(compact).toMatch(/VALID|INVALID/)
  })

  it('should be concise (less than 200 chars)', () => {
    expect(compact.length).toBeLessThan(200)
  })
})

describe('Report Consistency', () => {
  const report = generateSuiteReport(TIER1_GREP_IMPOSSIBLE_SUITE)

  it('markdown and text formats should have same task count', () => {
    const markdown = formatReportMarkdown(report)
    const text = formatReportText(report)

    expect(markdown).toContain('8')
    expect(text).toContain('8')
  })

  it('all formats should show validation passed', () => {
    const markdown = formatReportMarkdown(report)
    const text = formatReportText(report)
    const compact = formatCompactSummary(report)

    expect(markdown).toContain('PASSED')
    expect(text).toContain('PASSED')
    expect(compact).toContain('VALID')
  })

  it('all formats should reference all 3 categories', () => {
    const markdown = formatReportMarkdown(report)
    const text = formatReportText(report)

    expect(markdown).toContain('relationship-discovery')
    expect(markdown).toContain('architectural-understanding')
    expect(markdown).toContain('negative-space')

    expect(text).toContain('relationship-discovery')
    expect(text).toContain('architectural-understanding')
    expect(text).toContain('negative-space')
  })
})
