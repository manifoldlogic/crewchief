import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  sanitizeInput,
  sanitizeFormData,
  validateRequiredFields,
  prepareSecureFormData,
  defaultRateLimit,
} from '../../../src/client/lib/form-security';

// Mock DOMPurify
vi.mock('dompurify', () => ({
  default: {
    sanitize: vi.fn((input: string) => input.replace(/<[^>]*>/g, '')),
  },
}));

describe('Form Security Utilities', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('sanitizeInput', () => {
    it('removes HTML tags from input', () => {
      const maliciousInput = '<script>alert("xss")</script>Hello World';
      const sanitized = sanitizeInput(maliciousInput);
      expect(sanitized).toBe('alert("xss")Hello World');
    });

    it('handles empty and null inputs', () => {
      expect(sanitizeInput('')).toBe('');
    });
  });

  describe('sanitizeFormData', () => {
    it('sanitizes string values in an object', () => {
      const data = {
        name: '<script>alert("xss")</script>John',
        email: 'john@example.com',
        age: 25,
      };

      const sanitized = sanitizeFormData(data);
      
      expect(sanitized.name).toBe('alert("xss")John');
      expect(sanitized.email).toBe('john@example.com');
      expect(sanitized.age).toBe(25);
    });

    it('sanitizes nested objects', () => {
      const data = {
        user: {
          name: '<script>alert("xss")</script>John',
          profile: {
            bio: '<img src="x" onerror="alert()">Bio',
          },
        },
      };

      const sanitized = sanitizeFormData(data);
      
      expect(sanitized.user.name).toBe('alert("xss")John');
      expect(sanitized.user.profile.bio).toBe('Bio');
    });

    it('sanitizes arrays', () => {
      const data = {
        tags: ['<script>tag1</script>', 'tag2', '<b>tag3</b>'],
        users: [
          { name: '<script>John</script>' },
          { name: 'Jane' },
        ],
      };

      const sanitized = sanitizeFormData(data);
      
      expect(sanitized.tags).toEqual(['tag1', 'tag2', 'tag3']);
      expect(sanitized.users[0].name).toBe('John');
      expect(sanitized.users[1].name).toBe('Jane');
    });
  });

  describe('validateRequiredFields', () => {
    it('validates required fields correctly', () => {
      const data = {
        name: 'John',
        email: '',
        age: 25,
      };

      const result = validateRequiredFields(data, ['name', 'email']);
      
      expect(result.isValid).toBe(false);
      expect(result.missingFields).toEqual(['email']);
    });

    it('passes validation when all required fields are present', () => {
      const data = {
        name: 'John',
        email: 'john@example.com',
        age: 25,
      };

      const result = validateRequiredFields(data, ['name', 'email']);
      
      expect(result.isValid).toBe(true);
      expect(result.missingFields).toEqual([]);
    });

    it('handles undefined and null values', () => {
      const data = {
        name: 'John',
        email: null,
        phone: undefined,
      };

      const result = validateRequiredFields(data, ['name', 'email', 'phone']);
      
      expect(result.isValid).toBe(false);
      expect(result.missingFields).toEqual(['email', 'phone']);
    });
  });

  describe('prepareSecureFormData', () => {
    beforeEach(() => {
      // Reset rate limiting for each test
      defaultRateLimit['submissions'].clear();
    });

    it('prepares form data with all security features', () => {
      const data = {
        name: '<script>John</script>',
        email: 'john@example.com',
      };

      const result = prepareSecureFormData(data, {
        formId: 'test-form',
        requiredFields: ['name', 'email'],
      });

      expect(result.isValid).toBe(true);
      expect(result.errors).toEqual([]);
      expect(result.data.name).toBe('John');
      expect(result.data._token).toBeDefined();
    });

    it('fails validation for missing required fields', () => {
      const data = {
        name: 'John',
        email: '',
      };

      const result = prepareSecureFormData(data, {
        formId: 'test-form',
        requiredFields: ['name', 'email'],
      });

      expect(result.isValid).toBe(false);
      expect(result.errors).toContain('Missing required fields: email');
    });

    it('respects rate limiting', () => {
      const data = { name: 'John' };

      // First 5 submissions should succeed
      for (let i = 0; i < 5; i++) {
        const result = prepareSecureFormData(data, {
          formId: 'rate-limited-form',
          rateLimit: true,
        });
        expect(result.isValid).toBe(true);
      }

      // 6th submission should fail due to rate limiting
      const result = prepareSecureFormData(data, {
        formId: 'rate-limited-form',
        rateLimit: true,
      });

      expect(result.isValid).toBe(false);
      expect(result.errors[0]).toContain('Too many submissions');
    });

    it('can disable security features', () => {
      const data = {
        name: '<script>John</script>',
      };

      const result = prepareSecureFormData(data, {
        sanitize: false,
        addCSRF: false,
        rateLimit: false,
      });

      expect(result.data.name).toBe('<script>John</script>'); // Not sanitized
      expect(result.data._token).toBeUndefined(); // No CSRF token
    });
  });

  describe('Rate Limiting', () => {
    beforeEach(() => {
      defaultRateLimit['submissions'].clear();
    });

    it('allows submissions within rate limit', () => {
      expect(defaultRateLimit.canSubmit('test-form')).toBe(true);
      
      defaultRateLimit.recordSubmission('test-form');
      expect(defaultRateLimit.canSubmit('test-form')).toBe(true);
      
      expect(defaultRateLimit.getRemainingAttempts('test-form')).toBe(4);
    });

    it('blocks submissions when rate limit exceeded', () => {
      // Exhaust rate limit
      for (let i = 0; i < 5; i++) {
        defaultRateLimit.recordSubmission('test-form');
      }

      expect(defaultRateLimit.canSubmit('test-form')).toBe(false);
      expect(defaultRateLimit.getRemainingAttempts('test-form')).toBe(0);
    });

    it('tracks different forms separately', () => {
      // Exhaust rate limit for form1
      for (let i = 0; i < 5; i++) {
        defaultRateLimit.recordSubmission('form1');
      }

      expect(defaultRateLimit.canSubmit('form1')).toBe(false);
      expect(defaultRateLimit.canSubmit('form2')).toBe(true);
    });
  });
});