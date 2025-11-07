/**
 * Task: Find Cache Operations
 *
 * Find cache read vs write vs invalidation operations.
 * "Cache" is ambiguous: must distinguish get, set, delete, clear, and invalidate operations.
 *
 * Why grep struggles (30-60% success):
 * - "cache" appears in all operations: read, write, invalidate
 * - Must distinguish operation types from keyword alone
 * - Different APIs: Map.get/set/delete, cache.read/write, cache.invalidate
 * - Context needed: is this storing or retrieving?
 *
 * Why semantic search succeeds (>70% success):
 * - Recognizes operation intent: read vs write vs invalidate
 * - Understands cache lifecycle: populate, use, invalidate
 * - Identifies patterns: if-not-cached-then-compute (read), cache.set (write)
 * - Distinguishes operation types from surrounding code
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_CACHE_OPERATIONS: SearchTask = {
  id: 'tier2-ambiguity-cache-ops',
  name: 'Find Cache Operations',
  category: 'ambiguity-resolution',
  difficulty: 'medium',

  description:
    'Find all cache operations in the codebase, distinguishing between: ' +
    '1) Cache reads (get, retrieve, lookup), ' +
    '2) Cache writes (set, store, populate), ' +
    '3) Cache invalidation (delete, clear, evict, invalidate). ' +
    'For each type of operation, identify where it occurs and what data is cached.',

  internalNotes:
    'Grep struggles with "cache" ambiguity to distinguish cache operation types: ' +
    '- Read: `cache.get(key)`, `if (cache.has(key)) return cache.get(key)` ' +
    '- Write: `cache.set(key, value)`, `cache[key] = value` ' +
    '- Invalidate: `cache.delete(key)`, `cache.clear()`, `cache = new Map()` ' +
    '- All contain "cache" keyword, but intent differs ' +
    'Semantic search understands the operation type from context.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with cache operations
    pattern: /cache.*get|cache.*set|cache.*delete|cache.*clear|cache.*invalidate|Map.*get|Map.*set|Map.*delete/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the cache operations found in the codebase. ' +
      'Organize by operation type: ' +
      '1) Cache reads - where data is retrieved from cache, ' +
      '2) Cache writes - where data is stored to cache, ' +
      '3) Cache invalidation - where cache is cleared or entries removed. ' +
      'For each operation type, identify the files and explain what is being cached.',
    validator: {
      type: 'explanation',
      // Must mention files with cache operations
      mentionsFiles: ['loader.ts'],
      // Must discuss different operation types
      mentionsPattern: /(cache|caching).*(?:read|write|set|get|invalidate|delete|clear|operation|retrieve|store)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.35, // 35% - grep finds "cache" but can't distinguish operation types
  expectedSearchSuccess: 0.75, // 75% - search understands operation context

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /cache.*get|cache.*set|cache.*delete|cache.*clear|cache.*invalidate|Map.*get|Map.*set|Map.*delete/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['loader.ts'],
        mentionsPattern: /(cache|caching).*(?:read|write|set|get|invalidate|delete|clear|operation|retrieve|store)/i,
      },
    },
  }),
}
