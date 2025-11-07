import { describe, it, expect } from 'vitest'
import {
  TRANSITIVE_RELATIONSHIP_PATTERN,
  CONCEPTUAL_PATTERN_MATCH,
  ARCHITECTURAL_FLOW_PATTERN,
  NEGATIVE_CONSTRAINT_PATTERN,
  MULTI_PATTERN_AGGREGATION,
  CROSS_CUTTING_CONCERN_PATTERN,
  ALL_PATTERNS,
  getPatternsByCategory,
  getPatternByTemplate,
  getPatternsByGrepDifficulty,
  getPatternsBySearchAdvantage,
} from '../patterns.js'

describe('Patterns', () => {
  describe('pattern definitions', () => {
    it('should define TRANSITIVE_RELATIONSHIP_PATTERN', () => {
      expect(TRANSITIVE_RELATIONSHIP_PATTERN.category).toBe('relationship-discovery')
      expect(TRANSITIVE_RELATIONSHIP_PATTERN.pattern).toContain('{X}')
      expect(TRANSITIVE_RELATIONSHIP_PATTERN.pattern).toContain('{Y}')
      expect(TRANSITIVE_RELATIONSHIP_PATTERN.example.grepDifficulty).toBe('impossible')
      expect(TRANSITIVE_RELATIONSHIP_PATTERN.example.searchAdvantage).toBe('critical')
    })

    it('should define CONCEPTUAL_PATTERN_MATCH', () => {
      expect(CONCEPTUAL_PATTERN_MATCH.category).toBe('conceptual-similarity')
      expect(CONCEPTUAL_PATTERN_MATCH.pattern).toContain('{concept}')
      expect(CONCEPTUAL_PATTERN_MATCH.example.grepDifficulty).toBe('hard')
      expect(CONCEPTUAL_PATTERN_MATCH.example.searchAdvantage).toBe('significant')
    })

    it('should define ARCHITECTURAL_FLOW_PATTERN', () => {
      expect(ARCHITECTURAL_FLOW_PATTERN.category).toBe('architectural-understanding')
      expect(ARCHITECTURAL_FLOW_PATTERN.pattern).toContain('{data/control}')
      expect(ARCHITECTURAL_FLOW_PATTERN.example.grepDifficulty).toBe('impossible')
      expect(ARCHITECTURAL_FLOW_PATTERN.example.searchAdvantage).toBe('critical')
    })

    it('should define NEGATIVE_CONSTRAINT_PATTERN', () => {
      expect(NEGATIVE_CONSTRAINT_PATTERN.category).toBe('negative-space')
      expect(NEGATIVE_CONSTRAINT_PATTERN.pattern).toContain('{X}')
      expect(NEGATIVE_CONSTRAINT_PATTERN.pattern).toContain('{Y}')
      expect(NEGATIVE_CONSTRAINT_PATTERN.example.grepDifficulty).toBe('impossible')
      expect(NEGATIVE_CONSTRAINT_PATTERN.example.searchAdvantage).toBe('critical')
    })

    it('should define MULTI_PATTERN_AGGREGATION', () => {
      expect(MULTI_PATTERN_AGGREGATION.category).toBe('ambiguity-resolution')
      expect(MULTI_PATTERN_AGGREGATION.pattern).toContain('{concept}')
      expect(MULTI_PATTERN_AGGREGATION.example.grepDifficulty).toBe('hard')
      expect(MULTI_PATTERN_AGGREGATION.example.searchAdvantage).toBe('significant')
    })

    it('should define CROSS_CUTTING_CONCERN_PATTERN', () => {
      expect(CROSS_CUTTING_CONCERN_PATTERN.category).toBe('cross-cutting-concerns')
      expect(CROSS_CUTTING_CONCERN_PATTERN.pattern).toContain('{concern}')
      expect(CROSS_CUTTING_CONCERN_PATTERN.pattern).toContain('{scope}')
      expect(CROSS_CUTTING_CONCERN_PATTERN.example.grepDifficulty).toBe('hard')
      expect(CROSS_CUTTING_CONCERN_PATTERN.example.searchAdvantage).toBe('moderate')
    })
  })

  describe('pattern structure validation', () => {
    it('should have all required fields for each pattern', () => {
      ALL_PATTERNS.forEach((pattern) => {
        expect(pattern).toHaveProperty('category')
        expect(pattern).toHaveProperty('pattern')
        expect(pattern).toHaveProperty('description')
        expect(pattern).toHaveProperty('example')

        expect(pattern.example).toHaveProperty('taskDescription')
        expect(pattern.example).toHaveProperty('grepApproach')
        expect(pattern.example).toHaveProperty('grepDifficulty')
        expect(pattern.example).toHaveProperty('searchApproach')
        expect(pattern.example).toHaveProperty('searchAdvantage')
        expect(pattern.example).toHaveProperty('successCriteria')
      })
    })

    it('should have unique pattern templates', () => {
      const templates = ALL_PATTERNS.map((p) => p.pattern)
      const uniqueTemplates = new Set(templates)
      expect(uniqueTemplates.size).toBe(templates.length)
    })

    it('should have valid grep difficulty values', () => {
      const validDifficulties = ['impossible', 'hard', 'possible']
      ALL_PATTERNS.forEach((pattern) => {
        expect(validDifficulties).toContain(pattern.example.grepDifficulty)
      })
    })

    it('should have valid search advantage values', () => {
      const validAdvantages = ['critical', 'significant', 'moderate']
      ALL_PATTERNS.forEach((pattern) => {
        expect(validAdvantages).toContain(pattern.example.searchAdvantage)
      })
    })

    it('should have non-empty descriptions', () => {
      ALL_PATTERNS.forEach((pattern) => {
        expect(pattern.description.length).toBeGreaterThan(10)
        expect(pattern.example.taskDescription.length).toBeGreaterThan(10)
        expect(pattern.example.grepApproach.length).toBeGreaterThan(10)
        expect(pattern.example.searchApproach.length).toBeGreaterThan(10)
      })
    })

    it('should have success criteria objects', () => {
      ALL_PATTERNS.forEach((pattern) => {
        expect(typeof pattern.example.successCriteria).toBe('object')
        expect(Object.keys(pattern.example.successCriteria).length).toBeGreaterThan(0)

        // All criteria should be boolean
        Object.values(pattern.example.successCriteria).forEach((value) => {
          expect(typeof value).toBe('boolean')
        })
      })
    })
  })

  describe('ALL_PATTERNS', () => {
    it('should contain exactly 6 patterns', () => {
      expect(ALL_PATTERNS).toHaveLength(6)
    })

    it('should include all defined patterns', () => {
      expect(ALL_PATTERNS).toContain(TRANSITIVE_RELATIONSHIP_PATTERN)
      expect(ALL_PATTERNS).toContain(CONCEPTUAL_PATTERN_MATCH)
      expect(ALL_PATTERNS).toContain(ARCHITECTURAL_FLOW_PATTERN)
      expect(ALL_PATTERNS).toContain(NEGATIVE_CONSTRAINT_PATTERN)
      expect(ALL_PATTERNS).toContain(MULTI_PATTERN_AGGREGATION)
      expect(ALL_PATTERNS).toContain(CROSS_CUTTING_CONCERN_PATTERN)
    })

    it('should cover all 6 task categories', () => {
      const categories = new Set(ALL_PATTERNS.map((p) => p.category))
      expect(categories.size).toBe(6)
      expect(categories).toContain('relationship-discovery')
      expect(categories).toContain('conceptual-similarity')
      expect(categories).toContain('architectural-understanding')
      expect(categories).toContain('negative-space')
      expect(categories).toContain('ambiguity-resolution')
      expect(categories).toContain('cross-cutting-concerns')
    })
  })

  describe('getPatternsByCategory', () => {
    it('should find patterns by category name', () => {
      const relationshipPatterns = getPatternsByCategory('relationship-discovery')
      expect(relationshipPatterns).toHaveLength(1)
      expect(relationshipPatterns[0]).toBe(TRANSITIVE_RELATIONSHIP_PATTERN)
    })

    it('should return empty array for non-existent category', () => {
      const patterns = getPatternsByCategory('non-existent-category')
      expect(patterns).toHaveLength(0)
    })

    it('should find patterns for all valid categories', () => {
      const categories = [
        'relationship-discovery',
        'conceptual-similarity',
        'architectural-understanding',
        'negative-space',
        'ambiguity-resolution',
        'cross-cutting-concerns',
      ]

      categories.forEach((category) => {
        const patterns = getPatternsByCategory(category)
        expect(patterns.length).toBeGreaterThan(0)
        patterns.forEach((pattern) => {
          expect(pattern.category).toBe(category)
        })
      })
    })
  })

  describe('getPatternByTemplate', () => {
    it('should find pattern by exact template string', () => {
      const pattern = getPatternByTemplate('Find {X} that affects {Y} indirectly')
      expect(pattern).toBeDefined()
      expect(pattern).toBe(TRANSITIVE_RELATIONSHIP_PATTERN)
    })

    it('should return undefined for non-existent template', () => {
      const pattern = getPatternByTemplate('Non-existent template')
      expect(pattern).toBeUndefined()
    })

    it('should be case-sensitive', () => {
      const pattern = getPatternByTemplate('find {x} that affects {y} indirectly')
      expect(pattern).toBeUndefined()
    })
  })

  describe('getPatternsByGrepDifficulty', () => {
    it('should find all impossible patterns', () => {
      const impossible = getPatternsByGrepDifficulty('impossible')
      expect(impossible.length).toBeGreaterThan(0)
      impossible.forEach((pattern) => {
        expect(pattern.example.grepDifficulty).toBe('impossible')
      })
    })

    it('should find all hard patterns', () => {
      const hard = getPatternsByGrepDifficulty('hard')
      expect(hard.length).toBeGreaterThan(0)
      hard.forEach((pattern) => {
        expect(pattern.example.grepDifficulty).toBe('hard')
      })
    })

    it('should return empty array for difficulties not in patterns', () => {
      const possible = getPatternsByGrepDifficulty('possible')
      expect(possible).toHaveLength(0)
    })

    it('should cover all patterns', () => {
      const impossible = getPatternsByGrepDifficulty('impossible')
      const hard = getPatternsByGrepDifficulty('hard')
      const possible = getPatternsByGrepDifficulty('possible')

      expect(impossible.length + hard.length + possible.length).toBe(ALL_PATTERNS.length)
    })
  })

  describe('getPatternsBySearchAdvantage', () => {
    it('should find all critical advantage patterns', () => {
      const critical = getPatternsBySearchAdvantage('critical')
      expect(critical.length).toBeGreaterThan(0)
      critical.forEach((pattern) => {
        expect(pattern.example.searchAdvantage).toBe('critical')
      })
    })

    it('should find all significant advantage patterns', () => {
      const significant = getPatternsBySearchAdvantage('significant')
      expect(significant.length).toBeGreaterThan(0)
      significant.forEach((pattern) => {
        expect(pattern.example.searchAdvantage).toBe('significant')
      })
    })

    it('should find all moderate advantage patterns', () => {
      const moderate = getPatternsBySearchAdvantage('moderate')
      expect(moderate.length).toBeGreaterThan(0)
      moderate.forEach((pattern) => {
        expect(pattern.example.searchAdvantage).toBe('moderate')
      })
    })

    it('should cover all patterns', () => {
      const critical = getPatternsBySearchAdvantage('critical')
      const significant = getPatternsBySearchAdvantage('significant')
      const moderate = getPatternsBySearchAdvantage('moderate')

      expect(critical.length + significant.length + moderate.length).toBe(ALL_PATTERNS.length)
    })
  })

  describe('pattern coherence', () => {
    it('should have critical advantage for impossible grep tasks', () => {
      const impossible = getPatternsByGrepDifficulty('impossible')
      impossible.forEach((pattern) => {
        expect(pattern.example.searchAdvantage).toBe('critical')
      })
    })

    it('should have significant or moderate advantage for hard grep tasks', () => {
      const hard = getPatternsByGrepDifficulty('hard')
      hard.forEach((pattern) => {
        const advantage = pattern.example.searchAdvantage
        expect(['significant', 'moderate']).toContain(advantage)
      })
    })

    it('should have concrete task descriptions', () => {
      ALL_PATTERNS.forEach((pattern) => {
        expect(pattern.example.taskDescription).not.toContain('{')
        expect(pattern.example.taskDescription).not.toContain('}')
      })
    })

    it('should have success criteria with multiple checks', () => {
      ALL_PATTERNS.forEach((pattern) => {
        const criteriaCount = Object.keys(pattern.example.successCriteria).length
        expect(criteriaCount).toBeGreaterThanOrEqual(3)
      })
    })

    it('should have grep and search approaches that differ', () => {
      ALL_PATTERNS.forEach((pattern) => {
        expect(pattern.example.grepApproach).not.toBe(pattern.example.searchApproach)
        expect(pattern.example.grepApproach.toLowerCase()).not.toContain('semantic')
        expect(pattern.example.searchApproach.toLowerCase()).not.toContain('grep')
      })
    })
  })

  describe('pattern template placeholders', () => {
    it('should have placeholders in all pattern templates', () => {
      ALL_PATTERNS.forEach((pattern) => {
        expect(pattern.pattern).toMatch(/\{[^}]+\}/)
      })
    })

    it('should have descriptive placeholder names', () => {
      ALL_PATTERNS.forEach((pattern) => {
        const placeholders = pattern.pattern.match(/\{([^}]+)\}/g)
        expect(placeholders).not.toBeNull()
        placeholders?.forEach((placeholder) => {
          // Should have at least one character inside braces
          expect(placeholder.length).toBeGreaterThanOrEqual(3)
        })
      })
    })
  })
})
