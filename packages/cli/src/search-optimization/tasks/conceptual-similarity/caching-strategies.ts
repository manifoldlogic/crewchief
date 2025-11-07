/**
 * Task: Find Caching Strategies
 *
 * Find all caching implementations regardless of technology or approach.
 * This includes: in-memory caches (Map, WeakMap), memoization, computed properties,
 * file system caches, database query caches, and HTTP caches.
 *
 * Why grep struggles (30-60% success):
 * - Multiple keywords: "cache", "memoize", "store", "cached", "Map", "WeakMap"
 * - Implicit caching (no "cache" keyword): `if (this._value) return this._value`
 * - Different patterns: Map.get/set, memoization decorators, lazy initialization
 * - False positives: non-cache usage of Map, storage without caching intent
 *
 * Why semantic search succeeds (>70% success):
 * - Recognizes caching patterns: check-then-compute-then-store
 * - Understands memoization without explicit keywords
 * - Identifies lazy initialization as a form of caching
 * - Connects related concepts: cache, store, memoize, lazy
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_CACHING_STRATEGIES: SearchTask = {
  id: 'tier2-conceptual-caching',
  name: 'Find Caching Strategies',
  category: 'conceptual-similarity',
  difficulty: 'medium',

  description:
    'Find all caching implementations in the codebase. ' +
    'This includes: in-memory caches (Map, WeakMap, object properties), memoization, ' +
    'lazy initialization, computed values, and any form of result reuse. ' +
    'Identify the different caching strategies and what data or computations they cache.',

  internalNotes:
    'Grep struggles with diverse caching patterns: ' +
    '- Map-based: `const cache = new Map(); if (cache.has(key)) return cache.get(key)` ' +
    '- Property-based: `if (this._cached) return this._cached` ' +
    '- Memoization: `const memo = fn => { const cache = {}; return (x) => cache[x] ||= fn(x) }` ' +
    '- Lazy: `get value() { return this._value ??= computeValue() }` ' +
    'Semantic search recognizes the caching pattern regardless of implementation.',

  searchTarget: {
    type: 'pattern',
    // Looking for files with caching
    pattern: /cache|memoiz|Map\s*\(|WeakMap|cached|store.*get|lazy|compute.*once/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the caching strategies used in the codebase. ' +
      'For each strategy, identify: 1) The caching approach (Map, memoization, lazy init, etc.), ' +
      '2) Where it is implemented (files and functions), ' +
      '3) What data or computation is being cached, ' +
      '4) Cache invalidation strategy if any. ' +
      'Focus on actual caching mechanisms that reuse computed results.',
    validator: {
      type: 'explanation',
      // Must mention files with caching
      mentionsFiles: ['loader.ts'],
      // Must discuss caching concepts
      mentionsPattern:
        /(cache|caches|caching|memoiz|lazy|computed).*(?:strategy|pattern|mechanism|implementation|reuse)/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.35, // 35% - grep finds explicit "cache" but misses implicit patterns
  expectedSearchSuccess: 0.8, // 80% - search understands caching concepts

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /cache|memoiz|Map\s*\(|WeakMap|cached|store.*get|lazy|compute.*once/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['loader.ts'],
        mentionsPattern:
          /(cache|caches|caching|memoiz|lazy|computed).*(?:strategy|pattern|mechanism|implementation|reuse)/i,
      },
    },
  }),
}
