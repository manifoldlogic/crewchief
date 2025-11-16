/**
 * Tests for time formatting utilities
 */

import { describe, it, expect } from 'vitest'
import { formatRelativeTime } from './time'

describe('formatRelativeTime', () => {
  describe('seconds range (< 60 seconds)', () => {
    it('should return "just now" for current time', () => {
      const now = new Date()
      expect(formatRelativeTime(now)).toBe('just now')
    })

    it('should return "just now" for 30 seconds ago', () => {
      const now = new Date()
      const thirtySecondsAgo = new Date(now.getTime() - 30 * 1000)
      expect(formatRelativeTime(thirtySecondsAgo)).toBe('just now')
    })

    it('should return "just now" for 59 seconds ago', () => {
      const now = new Date()
      const fiftyNineSecondsAgo = new Date(now.getTime() - 59 * 1000)
      expect(formatRelativeTime(fiftyNineSecondsAgo)).toBe('just now')
    })

    it('should return "just now" for future timestamps', () => {
      const now = new Date()
      const future = new Date(now.getTime() + 5000)
      expect(formatRelativeTime(future)).toBe('just now')
    })
  })

  describe('minutes range (1-59 minutes)', () => {
    it('should return "1 minute ago" for exactly 1 minute', () => {
      const now = new Date()
      const oneMinuteAgo = new Date(now.getTime() - 60 * 1000)
      expect(formatRelativeTime(oneMinuteAgo)).toBe('1 minute ago')
    })

    it('should return "5 minutes ago" for 5 minutes', () => {
      const now = new Date()
      const fiveMinutesAgo = new Date(now.getTime() - 5 * 60 * 1000)
      expect(formatRelativeTime(fiveMinutesAgo)).toBe('5 minutes ago')
    })

    it('should return "30 minutes ago" for 30 minutes', () => {
      const now = new Date()
      const thirtyMinutesAgo = new Date(now.getTime() - 30 * 60 * 1000)
      expect(formatRelativeTime(thirtyMinutesAgo)).toBe('30 minutes ago')
    })

    it('should return "59 minutes ago" for 59 minutes', () => {
      const now = new Date()
      const fiftyNineMinutesAgo = new Date(now.getTime() - 59 * 60 * 1000)
      expect(formatRelativeTime(fiftyNineMinutesAgo)).toBe('59 minutes ago')
    })
  })

  describe('hours range (1-23 hours)', () => {
    it('should return "1 hour ago" for exactly 1 hour', () => {
      const now = new Date()
      const oneHourAgo = new Date(now.getTime() - 60 * 60 * 1000)
      expect(formatRelativeTime(oneHourAgo)).toBe('1 hour ago')
    })

    it('should return "2 hours ago" for 2 hours', () => {
      const now = new Date()
      const twoHoursAgo = new Date(now.getTime() - 2 * 60 * 60 * 1000)
      expect(formatRelativeTime(twoHoursAgo)).toBe('2 hours ago')
    })

    it('should return "12 hours ago" for 12 hours', () => {
      const now = new Date()
      const twelveHoursAgo = new Date(now.getTime() - 12 * 60 * 60 * 1000)
      expect(formatRelativeTime(twelveHoursAgo)).toBe('12 hours ago')
    })

    it('should return "23 hours ago" for 23 hours', () => {
      const now = new Date()
      const twentyThreeHoursAgo = new Date(now.getTime() - 23 * 60 * 60 * 1000)
      expect(formatRelativeTime(twentyThreeHoursAgo)).toBe('23 hours ago')
    })
  })

  describe('days range (24+ hours)', () => {
    it('should return "1 day ago" for exactly 24 hours', () => {
      const now = new Date()
      const oneDayAgo = new Date(now.getTime() - 24 * 60 * 60 * 1000)
      expect(formatRelativeTime(oneDayAgo)).toBe('1 day ago')
    })

    it('should return "2 days ago" for 2 days', () => {
      const now = new Date()
      const twoDaysAgo = new Date(now.getTime() - 2 * 24 * 60 * 60 * 1000)
      expect(formatRelativeTime(twoDaysAgo)).toBe('2 days ago')
    })

    it('should return "7 days ago" for 7 days', () => {
      const now = new Date()
      const sevenDaysAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000)
      expect(formatRelativeTime(sevenDaysAgo)).toBe('7 days ago')
    })

    it('should return "30 days ago" for 30 days', () => {
      const now = new Date()
      const thirtyDaysAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000)
      expect(formatRelativeTime(thirtyDaysAgo)).toBe('30 days ago')
    })

    it('should return "365 days ago" for 1 year', () => {
      const now = new Date()
      const oneYearAgo = new Date(now.getTime() - 365 * 24 * 60 * 60 * 1000)
      expect(formatRelativeTime(oneYearAgo)).toBe('365 days ago')
    })
  })

  describe('edge cases', () => {
    it('should handle timestamps at boundary between seconds and minutes', () => {
      const now = new Date()
      // Exactly 60 seconds = 1 minute
      const boundary = new Date(now.getTime() - 60 * 1000)
      expect(formatRelativeTime(boundary)).toBe('1 minute ago')
    })

    it('should handle timestamps at boundary between minutes and hours', () => {
      const now = new Date()
      // Exactly 60 minutes = 1 hour
      const boundary = new Date(now.getTime() - 60 * 60 * 1000)
      expect(formatRelativeTime(boundary)).toBe('1 hour ago')
    })

    it('should handle timestamps at boundary between hours and days', () => {
      const now = new Date()
      // Exactly 24 hours = 1 day
      const boundary = new Date(now.getTime() - 24 * 60 * 60 * 1000)
      expect(formatRelativeTime(boundary)).toBe('1 day ago')
    })
  })

  describe('plural handling', () => {
    it('should use singular "minute" for 1 minute', () => {
      const now = new Date()
      const oneMinute = new Date(now.getTime() - 60 * 1000)
      expect(formatRelativeTime(oneMinute)).toBe('1 minute ago')
    })

    it('should use plural "minutes" for 2+ minutes', () => {
      const now = new Date()
      const twoMinutes = new Date(now.getTime() - 2 * 60 * 1000)
      expect(formatRelativeTime(twoMinutes)).toBe('2 minutes ago')
    })

    it('should use singular "hour" for 1 hour', () => {
      const now = new Date()
      const oneHour = new Date(now.getTime() - 60 * 60 * 1000)
      expect(formatRelativeTime(oneHour)).toBe('1 hour ago')
    })

    it('should use plural "hours" for 2+ hours', () => {
      const now = new Date()
      const twoHours = new Date(now.getTime() - 2 * 60 * 60 * 1000)
      expect(formatRelativeTime(twoHours)).toBe('2 hours ago')
    })

    it('should use singular "day" for 1 day', () => {
      const now = new Date()
      const oneDay = new Date(now.getTime() - 24 * 60 * 60 * 1000)
      expect(formatRelativeTime(oneDay)).toBe('1 day ago')
    })

    it('should use plural "days" for 2+ days', () => {
      const now = new Date()
      const twoDays = new Date(now.getTime() - 2 * 24 * 60 * 60 * 1000)
      expect(formatRelativeTime(twoDays)).toBe('2 days ago')
    })
  })
})
