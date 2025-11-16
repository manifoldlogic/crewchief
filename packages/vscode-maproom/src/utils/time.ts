/**
 * Time formatting utilities
 *
 * Provides human-readable relative time formatting for UI display.
 */

/**
 * Format a timestamp as relative time from now
 *
 * Formats timestamps in a human-readable way:
 * - "just now" for times less than 60 seconds ago
 * - "X minutes ago" for times less than 1 hour ago
 * - "X hours ago" for times less than 24 hours ago
 * - "X days ago" for times 24+ hours ago
 *
 * @param timestamp - Date object to format
 * @returns Human-readable relative time string
 *
 * @example
 * ```typescript
 * const now = new Date()
 * const thirtySecondsAgo = new Date(now.getTime() - 30000)
 * formatRelativeTime(thirtySecondsAgo) // "just now"
 *
 * const fiveMinutesAgo = new Date(now.getTime() - 5 * 60 * 1000)
 * formatRelativeTime(fiveMinutesAgo) // "5 minutes ago"
 *
 * const twoHoursAgo = new Date(now.getTime() - 2 * 60 * 60 * 1000)
 * formatRelativeTime(twoHoursAgo) // "2 hours ago"
 *
 * const threeDaysAgo = new Date(now.getTime() - 3 * 24 * 60 * 60 * 1000)
 * formatRelativeTime(threeDaysAgo) // "3 days ago"
 * ```
 */
export function formatRelativeTime(timestamp: Date): string {
  const now = new Date()
  const diffMs = now.getTime() - timestamp.getTime()

  // Handle future timestamps gracefully
  if (diffMs < 0) {
    return 'just now'
  }

  const seconds = Math.floor(diffMs / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  // Less than 60 seconds
  if (seconds < 60) {
    return 'just now'
  }

  // Less than 1 hour (60 minutes)
  if (minutes < 60) {
    return `${minutes} minute${minutes === 1 ? '' : 's'} ago`
  }

  // Less than 24 hours
  if (hours < 24) {
    return `${hours} hour${hours === 1 ? '' : 's'} ago`
  }

  // 24+ hours
  return `${days} day${days === 1 ? '' : 's'} ago`
}
