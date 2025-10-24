/**
 * Simple in-memory caching utility with TTL support
 */

interface CacheEntry<T> {
  value: T
  expiresAt: number
}

interface CacheMetrics {
  hits: number
  misses: number
  sets: number
  evictions: number
}

/**
 * Simple in-memory cache with TTL support
 */
export class Cache<T = any> {
  private store: Map<string, CacheEntry<T>>
  private metrics: CacheMetrics
  private defaultTtlMs: number

  /**
   * Create a new cache instance
   * @param defaultTtlMs - Default TTL in milliseconds (default: 5 minutes)
   */
  constructor(defaultTtlMs: number = 5 * 60 * 1000) {
    this.store = new Map()
    this.metrics = {
      hits: 0,
      misses: 0,
      sets: 0,
      evictions: 0,
    }
    this.defaultTtlMs = defaultTtlMs
  }

  /**
   * Get a value from the cache
   * @param key - Cache key
   * @returns Cached value or null if not found or expired
   */
  get(key: string): T | null {
    const entry = this.store.get(key)

    if (!entry) {
      this.metrics.misses++
      return null
    }

    // Check if expired
    if (Date.now() > entry.expiresAt) {
      this.store.delete(key)
      this.metrics.evictions++
      this.metrics.misses++
      return null
    }

    this.metrics.hits++
    return entry.value
  }

  /**
   * Set a value in the cache
   * @param key - Cache key
   * @param value - Value to cache
   * @param ttlMs - Optional TTL in milliseconds (uses default if not provided)
   */
  set(key: string, value: T, ttlMs?: number): void {
    const ttl = ttlMs ?? this.defaultTtlMs
    const expiresAt = Date.now() + ttl

    this.store.set(key, {
      value,
      expiresAt,
    })

    this.metrics.sets++
  }

  /**
   * Delete a value from the cache
   * @param key - Cache key
   * @returns True if value was deleted, false if not found
   */
  delete(key: string): boolean {
    const deleted = this.store.delete(key)
    if (deleted) {
      this.metrics.evictions++
    }
    return deleted
  }

  /**
   * Clear all entries from the cache
   */
  clear(): void {
    this.store.clear()
    this.metrics.evictions += this.store.size
  }

  /**
   * Get cache size (number of entries)
   */
  size(): number {
    return this.store.size
  }

  /**
   * Get cache metrics
   * @returns Cache hit/miss/set statistics
   */
  getMetrics(): Readonly<CacheMetrics> {
    return { ...this.metrics }
  }

  /**
   * Get cache hit rate (0-1)
   * @returns Hit rate as decimal (0-1) or 0 if no requests yet
   */
  getHitRate(): number {
    const total = this.metrics.hits + this.metrics.misses
    if (total === 0) return 0
    return this.metrics.hits / total
  }

  /**
   * Clean up expired entries
   * @returns Number of entries removed
   */
  cleanup(): number {
    const now = Date.now()
    let removed = 0

    for (const [key, entry] of this.store.entries()) {
      if (now > entry.expiresAt) {
        this.store.delete(key)
        removed++
      }
    }

    this.metrics.evictions += removed
    return removed
  }

  /**
   * Reset cache metrics
   */
  resetMetrics(): void {
    this.metrics = {
      hits: 0,
      misses: 0,
      sets: 0,
      evictions: 0,
    }
  }
}

/**
 * Global cache instance for explain tool
 * Default TTL: 5 minutes
 */
export const explainCache = new Cache<string>(5 * 60 * 1000)

/**
 * Periodic cleanup interval (every 1 minute)
 */
setInterval(() => {
  explainCache.cleanup()
}, 60 * 1000)
