/**
 * Security validators for competition framework
 * Protects against path traversal and malicious variant IDs
 */

export function validateVariantId(id: string): void {
  // Reject path traversal attempts
  if (id.includes('..') || id.includes('/') || id.includes('\\')) {
    throw new Error('Invalid variant ID: path traversal detected')
  }

  // Enforce allowed characters
  if (!/^[a-zA-Z0-9_-]+$/.test(id)) {
    throw new Error('Invalid variant ID: only alphanumeric, dash, underscore allowed')
  }

  // Enforce max length
  if (id.length > 64) {
    throw new Error('Invalid variant ID: max 64 characters')
  }

  // Additional check: no consecutive dashes/underscores
  if (/[-_]{2,}/.test(id)) {
    throw new Error('Invalid variant ID: no consecutive dashes or underscores')
  }
}
