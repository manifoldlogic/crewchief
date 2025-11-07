/**
 * Tests for task library
 */

import { describe, it, expect } from 'vitest'
import {
  ALL_TASKS,
  getTasksByCategory,
  getTasksByDifficulty,
  getTaskById,
  getRandomTask,
  getRandomTasks,
} from '../../src/search-optimization/tasks/index.js'

describe('task library', () => {
  it('should have at least 10 tasks', () => {
    expect(ALL_TASKS.length).toBeGreaterThanOrEqual(10)
  })

  it('should have tasks across all categories', () => {
    const categories = new Set(ALL_TASKS.map((task) => task.category))
    expect(categories.size).toBeGreaterThanOrEqual(4)
    expect(categories.has('finding-implementation')).toBe(true)
    expect(categories.has('understanding-architecture')).toBe(true)
    expect(categories.has('locating-errors')).toBe(true)
  })

  it('should have tasks with different difficulties', () => {
    const difficulties = new Set(ALL_TASKS.map((task) => task.difficulty))
    expect(difficulties.has('easy')).toBe(true)
    expect(difficulties.has('medium')).toBe(true)
  })

  it('should have valid task structure', () => {
    for (const task of ALL_TASKS) {
      expect(task.id).toBeTruthy()
      expect(task.name).toBeTruthy()
      expect(task.description).toBeTruthy()
      expect(task.searchTarget).toBeTruthy()
      expect(task.followUpTask).toBeTruthy()
      expect(task.difficulty).toBeTruthy()
      expect(task.category).toBeTruthy()
      expect(task.successValidator).toBeTruthy()
      expect(typeof task.successValidator).toBe('function')
    }
  })

  it('should have unique task IDs', () => {
    const ids = ALL_TASKS.map((task) => task.id)
    const uniqueIds = new Set(ids)
    expect(ids.length).toBe(uniqueIds.size)
  })

  describe('getTasksByCategory', () => {
    it('should filter tasks by category', () => {
      const implTasks = getTasksByCategory('finding-implementation')
      expect(implTasks.length).toBeGreaterThan(0)
      expect(implTasks.every((task) => task.category === 'finding-implementation')).toBe(true)
    })

    it('should return empty array for unknown category', () => {
      const tasks = getTasksByCategory('unknown-category')
      expect(tasks).toEqual([])
    })
  })

  describe('getTasksByDifficulty', () => {
    it('should filter tasks by difficulty', () => {
      const easyTasks = getTasksByDifficulty('easy')
      expect(easyTasks.length).toBeGreaterThan(0)
      expect(easyTasks.every((task) => task.difficulty === 'easy')).toBe(true)
    })

    it('should return medium tasks', () => {
      const mediumTasks = getTasksByDifficulty('medium')
      expect(mediumTasks.length).toBeGreaterThan(0)
      expect(mediumTasks.every((task) => task.difficulty === 'medium')).toBe(true)
    })
  })

  describe('getTaskById', () => {
    it('should find task by ID', () => {
      const firstTask = ALL_TASKS[0]
      const found = getTaskById(firstTask.id)
      expect(found).toEqual(firstTask)
    })

    it('should return undefined for unknown ID', () => {
      const found = getTaskById('unknown-id')
      expect(found).toBeUndefined()
    })
  })

  describe('getRandomTask', () => {
    it('should return a task', () => {
      const task = getRandomTask()
      expect(task).toBeTruthy()
      expect(ALL_TASKS).toContain(task)
    })
  })

  describe('getRandomTasks', () => {
    it('should return requested number of tasks', () => {
      const tasks = getRandomTasks(5)
      expect(tasks.length).toBe(5)
    })

    it('should return all tasks if count exceeds total', () => {
      const tasks = getRandomTasks(1000)
      expect(tasks.length).toBe(ALL_TASKS.length)
    })

    it('should return different tasks', () => {
      const tasks = getRandomTasks(5)
      const ids = new Set(tasks.map((task) => task.id))
      expect(ids.size).toBe(5)
    })
  })

  describe('task validators', () => {
    it('should have functioning validators', () => {
      const task = ALL_TASKS[0]

      const mockOutput = {
        searchResults: [
          {
            query: 'test',
            results: [],
          },
        ],
        workResult: {
          success: false,
        },
        searchCount: 5,
        toolCallCount: 20,
        durationSeconds: 120,
      }

      const score = task.successValidator(mockOutput)

      expect(score.searchQuality).toBeGreaterThanOrEqual(0)
      expect(score.searchQuality).toBeLessThanOrEqual(1)
      expect(score.taskCompletion).toBeGreaterThanOrEqual(0)
      expect(score.taskCompletion).toBeLessThanOrEqual(1)
      expect(score.efficiency).toBeGreaterThanOrEqual(0)
      expect(score.efficiency).toBeLessThanOrEqual(1)
      expect(score.total).toBeGreaterThanOrEqual(0)
      expect(score.total).toBeLessThanOrEqual(1)
      expect(score.details).toBeTruthy()
    })
  })
})
