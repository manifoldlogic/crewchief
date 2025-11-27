/**
 * Tests for WatchEvent type definitions and type guards
 */

import { describe, it, expect } from 'vitest'
import { isWatchEvent, validateWatchEvent, type BranchSwitchedEvent } from './events'

describe('isWatchEvent', () => {
  describe('branch_switched event', () => {
    it('should validate valid branch_switched event', () => {
      const event: BranchSwitchedEvent = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(true)
    })

    it('should validate branch_switched event with worktree_created true', () => {
      const event: BranchSwitchedEvent = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-new',
        old_worktree_id: 1,
        new_worktree_id: 99,
        worktree_created: true,
      }

      expect(isWatchEvent(event)).toBe(true)
    })

    it('should reject branch_switched event missing timestamp', () => {
      const event = {
        type: 'branch_switched',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event missing repo', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event missing old_branch', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event missing new_branch', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event missing old_worktree_id', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event missing new_worktree_id', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event missing worktree_created', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event with wrong timestamp type', () => {
      const event = {
        type: 'branch_switched',
        timestamp: 12345,
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event with wrong repo type', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 123,
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event with wrong old_worktree_id type', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: '1',
        new_worktree_id: 42,
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event with wrong new_worktree_id type', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: '42',
        worktree_created: false,
      }

      expect(isWatchEvent(event)).toBe(false)
    })

    it('should reject branch_switched event with wrong worktree_created type', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: 'false',
      }

      expect(isWatchEvent(event)).toBe(false)
    })
  })

  describe('validateWatchEvent', () => {
    it('should return valid branch_switched event', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        repo: 'crewchief',
        old_branch: 'main',
        new_branch: 'feature-auth',
        old_worktree_id: 1,
        new_worktree_id: 42,
        worktree_created: false,
      }

      const result = validateWatchEvent(event)
      expect(result.type).toBe('branch_switched')
    })

    it('should throw for invalid branch_switched event', () => {
      const event = {
        type: 'branch_switched',
        timestamp: '2025-01-16T10:30:00Z',
        // missing required fields
      }

      expect(() => validateWatchEvent(event)).toThrow(TypeError)
    })
  })

  describe('edge cases', () => {
    it('should reject null', () => {
      expect(isWatchEvent(null)).toBe(false)
    })

    it('should reject undefined', () => {
      expect(isWatchEvent(undefined)).toBe(false)
    })

    it('should reject non-object', () => {
      expect(isWatchEvent('string')).toBe(false)
      expect(isWatchEvent(123)).toBe(false)
      expect(isWatchEvent(true)).toBe(false)
    })

    it('should reject object without type', () => {
      expect(isWatchEvent({ foo: 'bar' })).toBe(false)
    })

    it('should reject unknown event type', () => {
      expect(isWatchEvent({ type: 'unknown_event' })).toBe(false)
    })
  })
})
