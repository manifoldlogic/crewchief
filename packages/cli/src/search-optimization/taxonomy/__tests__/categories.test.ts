import { describe, it, expect } from 'vitest'
import {
  RELATIONSHIP_DISCOVERY,
  CONCEPTUAL_SIMILARITY,
  AMBIGUITY_RESOLUTION,
  NEGATIVE_SPACE,
  CROSS_CUTTING_CONCERNS,
  ARCHITECTURAL_UNDERSTANDING,
  ALL_CATEGORIES,
  getCategoryByName,
  getCategoriesByGrepDifficulty,
  getCategoriesBySearchAdvantage,
} from '../categories.js'

describe('TaskCategory', () => {
  describe('category definitions', () => {
    it('should define RELATIONSHIP_DISCOVERY category', () => {
      expect(RELATIONSHIP_DISCOVERY.name).toBe('relationship-discovery')
      expect(RELATIONSHIP_DISCOVERY.grepDifficulty).toBe('impossible')
      expect(RELATIONSHIP_DISCOVERY.searchAdvantage).toBe('critical')
      expect(RELATIONSHIP_DISCOVERY.realWorldFrequency).toBe('common')
      expect(RELATIONSHIP_DISCOVERY.exampleScenarios).toHaveLength(3)
    })

    it('should define CONCEPTUAL_SIMILARITY category', () => {
      expect(CONCEPTUAL_SIMILARITY.name).toBe('conceptual-similarity')
      expect(CONCEPTUAL_SIMILARITY.grepDifficulty).toBe('hard')
      expect(CONCEPTUAL_SIMILARITY.searchAdvantage).toBe('significant')
      expect(CONCEPTUAL_SIMILARITY.realWorldFrequency).toBe('common')
      expect(CONCEPTUAL_SIMILARITY.exampleScenarios).toHaveLength(3)
    })

    it('should define AMBIGUITY_RESOLUTION category', () => {
      expect(AMBIGUITY_RESOLUTION.name).toBe('ambiguity-resolution')
      expect(AMBIGUITY_RESOLUTION.grepDifficulty).toBe('hard')
      expect(AMBIGUITY_RESOLUTION.searchAdvantage).toBe('significant')
      expect(AMBIGUITY_RESOLUTION.realWorldFrequency).toBe('occasional')
      expect(AMBIGUITY_RESOLUTION.exampleScenarios).toHaveLength(3)
    })

    it('should define NEGATIVE_SPACE category', () => {
      expect(NEGATIVE_SPACE.name).toBe('negative-space')
      expect(NEGATIVE_SPACE.grepDifficulty).toBe('impossible')
      expect(NEGATIVE_SPACE.searchAdvantage).toBe('critical')
      expect(NEGATIVE_SPACE.realWorldFrequency).toBe('occasional')
      expect(NEGATIVE_SPACE.exampleScenarios).toHaveLength(3)
    })

    it('should define CROSS_CUTTING_CONCERNS category', () => {
      expect(CROSS_CUTTING_CONCERNS.name).toBe('cross-cutting-concerns')
      expect(CROSS_CUTTING_CONCERNS.grepDifficulty).toBe('hard')
      expect(CROSS_CUTTING_CONCERNS.searchAdvantage).toBe('moderate')
      expect(CROSS_CUTTING_CONCERNS.realWorldFrequency).toBe('common')
      expect(CROSS_CUTTING_CONCERNS.exampleScenarios).toHaveLength(3)
    })

    it('should define ARCHITECTURAL_UNDERSTANDING category', () => {
      expect(ARCHITECTURAL_UNDERSTANDING.name).toBe('architectural-understanding')
      expect(ARCHITECTURAL_UNDERSTANDING.grepDifficulty).toBe('impossible')
      expect(ARCHITECTURAL_UNDERSTANDING.searchAdvantage).toBe('critical')
      expect(ARCHITECTURAL_UNDERSTANDING.realWorldFrequency).toBe('common')
      expect(ARCHITECTURAL_UNDERSTANDING.exampleScenarios).toHaveLength(3)
    })
  })

  describe('category structure validation', () => {
    it('should have all required fields for each category', () => {
      ALL_CATEGORIES.forEach((category) => {
        expect(category).toHaveProperty('name')
        expect(category).toHaveProperty('description')
        expect(category).toHaveProperty('grepDifficulty')
        expect(category).toHaveProperty('searchAdvantage')
        expect(category).toHaveProperty('realWorldFrequency')
        expect(category).toHaveProperty('exampleScenarios')

        // Validate types
        expect(typeof category.name).toBe('string')
        expect(typeof category.description).toBe('string')
        expect(Array.isArray(category.exampleScenarios)).toBe(true)
        expect(category.exampleScenarios.length).toBeGreaterThan(0)
      })
    })

    it('should have unique category names', () => {
      const names = ALL_CATEGORIES.map((c) => c.name)
      const uniqueNames = new Set(names)
      expect(uniqueNames.size).toBe(names.length)
    })

    it('should have valid grepDifficulty values', () => {
      const validDifficulties = ['impossible', 'hard', 'possible', 'easy']
      ALL_CATEGORIES.forEach((category) => {
        expect(validDifficulties).toContain(category.grepDifficulty)
      })
    })

    it('should have valid searchAdvantage values', () => {
      const validAdvantages = ['critical', 'significant', 'moderate', 'none']
      ALL_CATEGORIES.forEach((category) => {
        expect(validAdvantages).toContain(category.searchAdvantage)
      })
    })

    it('should have valid realWorldFrequency values', () => {
      const validFrequencies = ['common', 'occasional', 'rare']
      ALL_CATEGORIES.forEach((category) => {
        expect(validFrequencies).toContain(category.realWorldFrequency)
      })
    })
  })

  describe('ALL_CATEGORIES', () => {
    it('should contain exactly 6 categories', () => {
      expect(ALL_CATEGORIES).toHaveLength(6)
    })

    it('should include all defined categories', () => {
      expect(ALL_CATEGORIES).toContain(RELATIONSHIP_DISCOVERY)
      expect(ALL_CATEGORIES).toContain(CONCEPTUAL_SIMILARITY)
      expect(ALL_CATEGORIES).toContain(AMBIGUITY_RESOLUTION)
      expect(ALL_CATEGORIES).toContain(NEGATIVE_SPACE)
      expect(ALL_CATEGORIES).toContain(CROSS_CUTTING_CONCERNS)
      expect(ALL_CATEGORIES).toContain(ARCHITECTURAL_UNDERSTANDING)
    })

    it('should be ordered by grep difficulty (impossible first)', () => {
      const impossibleCategories = ALL_CATEGORIES.slice(0, 3)
      const hardCategories = ALL_CATEGORIES.slice(3)

      impossibleCategories.forEach((category) => {
        expect(category.grepDifficulty).toBe('impossible')
      })

      hardCategories.forEach((category) => {
        expect(category.grepDifficulty).toBe('hard')
      })
    })
  })

  describe('getCategoryByName', () => {
    it('should find category by exact name', () => {
      const category = getCategoryByName('relationship-discovery')
      expect(category).toBeDefined()
      expect(category?.name).toBe('relationship-discovery')
    })

    it('should return undefined for non-existent category', () => {
      const category = getCategoryByName('non-existent-category')
      expect(category).toBeUndefined()
    })

    it('should find all categories by their names', () => {
      const names = [
        'relationship-discovery',
        'conceptual-similarity',
        'ambiguity-resolution',
        'negative-space',
        'cross-cutting-concerns',
        'architectural-understanding',
      ]

      names.forEach((name) => {
        const category = getCategoryByName(name)
        expect(category).toBeDefined()
        expect(category?.name).toBe(name)
      })
    })
  })

  describe('getCategoriesByGrepDifficulty', () => {
    it('should find all impossible categories', () => {
      const impossible = getCategoriesByGrepDifficulty('impossible')
      expect(impossible).toHaveLength(3)
      impossible.forEach((category) => {
        expect(category.grepDifficulty).toBe('impossible')
      })
    })

    it('should find all hard categories', () => {
      const hard = getCategoriesByGrepDifficulty('hard')
      expect(hard).toHaveLength(3)
      hard.forEach((category) => {
        expect(category.grepDifficulty).toBe('hard')
      })
    })

    it('should return empty array for difficulties not in taxonomy', () => {
      const possible = getCategoriesByGrepDifficulty('possible')
      expect(possible).toHaveLength(0)

      const easy = getCategoriesByGrepDifficulty('easy')
      expect(easy).toHaveLength(0)
    })
  })

  describe('getCategoriesBySearchAdvantage', () => {
    it('should find all critical advantage categories', () => {
      const critical = getCategoriesBySearchAdvantage('critical')
      expect(critical).toHaveLength(3)
      critical.forEach((category) => {
        expect(category.searchAdvantage).toBe('critical')
      })
    })

    it('should find all significant advantage categories', () => {
      const significant = getCategoriesBySearchAdvantage('significant')
      expect(significant).toHaveLength(2)
      significant.forEach((category) => {
        expect(category.searchAdvantage).toBe('significant')
      })
    })

    it('should find all moderate advantage categories', () => {
      const moderate = getCategoriesBySearchAdvantage('moderate')
      expect(moderate).toHaveLength(1)
      moderate.forEach((category) => {
        expect(category.searchAdvantage).toBe('moderate')
      })
    })

    it('should return empty array for advantages not in taxonomy', () => {
      const none = getCategoriesBySearchAdvantage('none')
      expect(none).toHaveLength(0)
    })
  })

  describe('category coherence', () => {
    it('should have critical search advantage for impossible grep tasks', () => {
      const impossible = getCategoriesByGrepDifficulty('impossible')
      impossible.forEach((category) => {
        expect(category.searchAdvantage).toBe('critical')
      })
    })

    it('should have non-empty descriptions for all categories', () => {
      ALL_CATEGORIES.forEach((category) => {
        expect(category.description.length).toBeGreaterThan(20)
      })
    })

    it('should have concrete example scenarios for all categories', () => {
      ALL_CATEGORIES.forEach((category) => {
        expect(category.exampleScenarios.length).toBeGreaterThanOrEqual(2)
        category.exampleScenarios.forEach((scenario) => {
          expect(scenario.length).toBeGreaterThan(10)
          expect(typeof scenario).toBe('string')
        })
      })
    })
  })
})
