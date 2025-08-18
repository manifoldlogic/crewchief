/**
 * Tests for error logger utility
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { errorLogger, logError, logReactError, logAPIError, logNetworkError } from '../../../src/client/utils/errorLogger';
import type { AppError } from '../../../src/client/types/error';

describe('ErrorLogger', () => {
  let consoleSpy: any;
  let sessionStorageMock: { [key: string]: string };

  beforeEach(() => {
    // Mock console methods
    consoleSpy = {
      group: vi.spyOn(console, 'group').mockImplementation(() => {}),
      groupEnd: vi.spyOn(console, 'groupEnd').mockImplementation(() => {}),
      error: vi.spyOn(console, 'error').mockImplementation(() => {}),
    };

    // Mock sessionStorage
    sessionStorageMock = {};
    Object.defineProperty(window, 'sessionStorage', {
      value: {
        getItem: vi.fn((key: string) => sessionStorageMock[key] || null),
        setItem: vi.fn((key: string, value: string) => {
          sessionStorageMock[key] = value;
        }),
      },
      writable: true,
    });

    // Mock navigator
    Object.defineProperty(window, 'navigator', {
      value: {
        userAgent: 'Test User Agent',
        onLine: true,
      },
      writable: true,
    });

    // Mock window.location
    Object.defineProperty(window, 'location', {
      value: {
        href: 'http://localhost:5173/test-page',
      },
      writable: true,
    });

    // Clear any existing logs
    errorLogger.clearLogs();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Basic Logging', () => {
    it('logs an error with default severity', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error);

      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[MEDIUM] TEST_ERROR'),
        expect.any(String)
      );
      expect(consoleSpy.error).toHaveBeenCalledWith('Message:', 'Test error');
      expect(consoleSpy.groupEnd).toHaveBeenCalled();
    });

    it('logs an error with custom severity', () => {
      const error: AppError = {
        message: 'Critical error',
        code: 'CRITICAL_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error, 'critical');

      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[CRITICAL] CRITICAL_ERROR'),
        expect.any(String)
      );
    });

    it('logs an error without error code', () => {
      const error: AppError = {
        message: 'Generic error',
        timestamp: new Date(),
      };

      errorLogger.log(error);

      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[MEDIUM] ERROR'),
        expect.any(String)
      );
    });
  });

  describe('React Error Logging', () => {
    it('logs React errors correctly', () => {
      const error = new Error('React component error');
      const errorInfo = {
        componentStack: '\n    in ErrorComponent (at App.js:10)\n    in App (at index.js:5)',
      };

      logReactError(error, errorInfo);

      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[HIGH] REACT_ERROR'),
        expect.any(String)
      );
      expect(consoleSpy.error).toHaveBeenCalledWith('Message:', 'React component error');
    });

    it('includes context information in React error logs', () => {
      const error = new Error('React error with context');
      const errorInfo = { componentStack: 'mock stack' };
      const context = { userId: '123', page: 'dashboard' };

      logReactError(error, errorInfo, context);

      expect(consoleSpy.error).toHaveBeenCalledWith('Full Log Entry:', expect.objectContaining({
        userId: '123',
        page: 'dashboard',
      }));
    });
  });

  describe('API Error Logging', () => {
    it('logs API errors with endpoint information', () => {
      const error = new Error('API request failed');

      logAPIError(error, '/api/users', 'GET', 404);

      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[HIGH] API_ERROR'),
        expect.any(String)
      );
    });

    it('determines severity based on status code', () => {
      const error = new Error('Server error');

      // Test 5xx error (critical)
      logAPIError(error, '/api/test', 'POST', 500);
      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[CRITICAL]'),
        expect.any(String)
      );

      // Test 4xx error (high)
      logAPIError(error, '/api/test', 'POST', 404);
      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[HIGH]'),
        expect.any(String)
      );

      // Test no status (medium)
      logAPIError(error, '/api/test', 'POST');
      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[MEDIUM]'),
        expect.any(String)
      );
    });
  });

  describe('Network Error Logging', () => {
    it('logs network errors with offline status', () => {
      const error = new Error('Network connection failed');

      logNetworkError(error, true, false);

      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[HIGH] NETWORK_ERROR'),
        expect.any(String)
      );
    });

    it('logs timeout errors', () => {
      const error = new Error('Request timeout');

      logNetworkError(error, false, true);

      expect(consoleSpy.error).toHaveBeenCalledWith('Full Log Entry:', expect.objectContaining({
        error: expect.objectContaining({
          timeout: true,
        }),
      }));
    });
  });

  describe('Error Message Sanitization', () => {
    it('sanitizes sensitive information in production mode', () => {
      // Mock production environment
      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: false };

      const error: AppError = {
        message: 'Login failed: password=secret123 token=abc-xyz-token key=mykey',
        code: 'AUTH_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error);

      expect(consoleSpy.error).toHaveBeenCalledWith('Full Log Entry:', expect.objectContaining({
        error: expect.objectContaining({
          message: 'Login failed: password=*** token=*** key=***',
        }),
      }));

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });

    it('does not sanitize messages in development mode', () => {
      // Ensure development environment
      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: true };

      const error: AppError = {
        message: 'Debug error: password=secret123',
        code: 'DEBUG_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error);

      expect(consoleSpy.error).toHaveBeenCalledWith('Full Log Entry:', expect.objectContaining({
        error: expect.objectContaining({
          message: 'Debug error: password=secret123',
        }),
      }));

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });
  });

  describe('Log Management', () => {
    it('maintains a queue of recent logs', () => {
      const errors = Array.from({ length: 5 }, (_, i) => ({
        message: `Error ${i}`,
        code: `ERROR_${i}`,
        timestamp: new Date(),
      }));

      errors.forEach(error => errorLogger.log(error));

      const recentLogs = errorLogger.getRecentLogs();
      expect(recentLogs).toHaveLength(5);
      expect(recentLogs[0].error.message).toBe('Error 0');
      expect(recentLogs[4].error.message).toBe('Error 4');
    });

    it('limits queue size to prevent memory leaks', () => {
      // Log more than 100 errors
      const errors = Array.from({ length: 105 }, (_, i) => ({
        message: `Error ${i}`,
        code: `ERROR_${i}`,
        timestamp: new Date(),
      }));

      errors.forEach(error => errorLogger.log(error));

      const recentLogs = errorLogger.getRecentLogs(200);
      expect(recentLogs).toHaveLength(100); // Should be capped at 100
      expect(recentLogs[0].error.message).toBe('Error 5'); // First 5 should be removed
    });

    it('returns limited number of recent logs', () => {
      const errors = Array.from({ length: 10 }, (_, i) => ({
        message: `Error ${i}`,
        code: `ERROR_${i}`,
        timestamp: new Date(),
      }));

      errors.forEach(error => errorLogger.log(error));

      const recentLogs = errorLogger.getRecentLogs(3);
      expect(recentLogs).toHaveLength(3);
      expect(recentLogs[0].error.message).toBe('Error 7');
      expect(recentLogs[2].error.message).toBe('Error 9');
    });

    it('clears the log queue', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error);
      expect(errorLogger.getRecentLogs()).toHaveLength(1);

      errorLogger.clearLogs();
      expect(errorLogger.getRecentLogs()).toHaveLength(0);
    });
  });

  describe('Session Management', () => {
    it('creates a session ID if none exists', () => {
      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error);

      expect(window.sessionStorage.setItem).toHaveBeenCalledWith(
        'crewchief_session_id',
        expect.any(String)
      );
    });

    it('reuses existing session ID', () => {
      sessionStorageMock['crewchief_session_id'] = 'existing-session-id';

      const error: AppError = {
        message: 'Test error',
        code: 'TEST_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error);

      expect(consoleSpy.error).toHaveBeenCalledWith('Full Log Entry:', expect.objectContaining({
        sessionId: 'existing-session-id',
      }));
    });
  });

  describe('Environment Detection', () => {
    it('logs to development console in development mode', () => {
      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: true };

      const error: AppError = {
        message: 'Dev error',
        code: 'DEV_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error);

      expect(consoleSpy.group).toHaveBeenCalled();
      expect(consoleSpy.error).toHaveBeenCalledWith('Message:', 'Dev error');

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });

    it('logs to production in production mode', () => {
      const originalEnv = import.meta.env;
      (import.meta.env as any) = { ...originalEnv, DEV: false };

      const error: AppError = {
        message: 'Prod error',
        code: 'PROD_ERROR',
        timestamp: new Date(),
      };

      errorLogger.log(error);

      // Should still log to console in test environment, but with sanitized message
      expect(consoleSpy.error).toHaveBeenCalledWith('Production Error:', expect.objectContaining({
        message: 'Prod error',
        code: 'PROD_ERROR',
      }));

      // Restore environment
      (import.meta.env as any) = originalEnv;
    });
  });

  describe('Convenience Functions', () => {
    it('logError function works correctly', () => {
      const error: AppError = {
        message: 'Convenience test',
        code: 'CONVENIENCE_ERROR',
        timestamp: new Date(),
      };

      logError(error, 'high');

      expect(consoleSpy.group).toHaveBeenCalledWith(
        expect.stringContaining('[HIGH] CONVENIENCE_ERROR'),
        expect.any(String)
      );
    });

    it('all convenience functions are exported', () => {
      expect(typeof logError).toBe('function');
      expect(typeof logReactError).toBe('function');
      expect(typeof logAPIError).toBe('function');
      expect(typeof logNetworkError).toBe('function');
    });
  });
});