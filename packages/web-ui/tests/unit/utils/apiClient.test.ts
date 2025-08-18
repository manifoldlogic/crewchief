/**
 * Tests for API client utility
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { APIClient, apiClient, get, post, put, del, patch } from '../../../src/client/utils/apiClient';

// Mock the error logger
vi.mock('../../../src/client/utils/errorLogger', () => ({
  logAPIError: vi.fn(),
  logNetworkError: vi.fn(),
}));

// Helper to create a proper response mock
const createMockResponse = (data: any, options: {
  status?: number;
  statusText?: string;
  ok?: boolean;
  contentType?: string;
} = {}) => {
  const {
    status = 200,
    statusText = 'OK',
    ok = status >= 200 && status < 300,
    contentType = 'application/json'
  } = options;
  
  return {
    ok,
    status,
    statusText,
    headers: new Headers({ 'content-type': contentType }),
    json: async () => data,
    text: async () => typeof data === 'string' ? data : JSON.stringify(data),
    blob: async () => new Blob([typeof data === 'string' ? data : JSON.stringify(data)]),
  };
};

describe('APIClient', () => {
  let fetchMock: any;

  beforeEach(() => {
    fetchMock = vi.fn();
    global.fetch = fetchMock;
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Constructor', () => {
    it('creates client with default base URL', () => {
      const client = new APIClient();
      expect(client).toBeInstanceOf(APIClient);
    });

    it('creates client with custom base URL', () => {
      const client = new APIClient('https://api.example.com');
      expect(client).toBeInstanceOf(APIClient);
    });
  });

  describe('URL Building', () => {
    it('builds URLs correctly with default base URL', async () => {
      fetchMock.mockResolvedValue(createMockResponse({ success: true }));

      const client = new APIClient();
      await client.get('users');

      expect(fetchMock).toHaveBeenCalledWith(
        '/api/users',
        expect.any(Object)
      );
    });

    it('builds URLs correctly with custom base URL', async () => {
      fetchMock.mockResolvedValue(createMockResponse({ success: true }));

      const client = new APIClient('https://api.example.com/v1');
      await client.get('users');

      expect(fetchMock).toHaveBeenCalledWith(
        'https://api.example.com/v1/users',
        expect.any(Object)
      );
    });

    it('handles leading slashes in endpoints', async () => {
      fetchMock.mockResolvedValue(createMockResponse({ success: true }));

      const client = new APIClient();
      await client.get('/users');

      expect(fetchMock).toHaveBeenCalledWith(
        '/api/users',
        expect.any(Object)
      );
    });

    it('handles trailing slashes in base URL', async () => {
      fetchMock.mockResolvedValue(createMockResponse({ success: true }));

      const client = new APIClient('/api/');
      await client.get('users');

      expect(fetchMock).toHaveBeenCalledWith(
        '/api/users',
        expect.any(Object)
      );
    });
  });

  describe('HTTP Methods', () => {
    beforeEach(() => {
      fetchMock.mockResolvedValue(createMockResponse({ success: true }));
    });

    it('makes GET requests correctly', async () => {
      const client = new APIClient();
      const response = await client.get('users');

      expect(fetchMock).toHaveBeenCalledWith(
        '/api/users',
        expect.objectContaining({
          method: 'GET',
        })
      );
      expect(response.data).toEqual({ success: true });
      expect(response.status).toBe(200);
    });

    it('makes POST requests with data', async () => {
      const client = new APIClient();
      const data = { name: 'John Doe' };
      
      await client.post('users', data);

      expect(fetchMock).toHaveBeenCalledWith(
        '/api/users',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify(data),
        })
      );
    });

    it('makes PUT requests with data', async () => {
      const client = new APIClient();
      const data = { id: 1, name: 'Jane Doe' };
      
      await client.put('users/1', data);

      expect(fetchMock).toHaveBeenCalledWith(
        '/api/users/1',
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify(data),
        })
      );
    });

    it('makes DELETE requests', async () => {
      const client = new APIClient();
      
      await client.delete('users/1');

      expect(fetchMock).toHaveBeenCalledWith(
        '/api/users/1',
        expect.objectContaining({
          method: 'DELETE',
        })
      );
    });

    it('makes PATCH requests with data', async () => {
      const client = new APIClient();
      const data = { name: 'Updated Name' };
      
      await client.patch('users/1', data);

      expect(fetchMock).toHaveBeenCalledWith(
        '/api/users/1',
        expect.objectContaining({
          method: 'PATCH',
          body: JSON.stringify(data),
        })
      );
    });
  });

  describe('Request Configuration', () => {
    it('sets default headers', async () => {
      fetchMock.mockResolvedValue(createMockResponse({}));

      const client = new APIClient();
      await client.get('test');

      const [, config] = fetchMock.mock.calls[0];
      const headers = new Headers(config.headers);
      
      expect(headers.get('Content-Type')).toBe('application/json');
      expect(headers.get('Accept')).toBe('application/json');
    });

    it('allows custom headers', async () => {
      fetchMock.mockResolvedValue(createMockResponse({}));

      const client = new APIClient();
      await client.get('test', {
        headers: {
          'Authorization': 'Bearer token123',
          'X-Custom': 'custom-value',
        },
      });

      const [, config] = fetchMock.mock.calls[0];
      const headers = new Headers(config.headers);
      
      expect(headers.get('Authorization')).toBe('Bearer token123');
      expect(headers.get('X-Custom')).toBe('custom-value');
      expect(headers.get('Content-Type')).toBe('application/json');
    });

    it('overrides default headers', async () => {
      fetchMock.mockResolvedValue(createMockResponse({}));

      const client = new APIClient();
      await client.get('test', {
        headers: {
          'Content-Type': 'text/plain',
        },
      });

      const [, config] = fetchMock.mock.calls[0];
      const headers = new Headers(config.headers);
      
      expect(headers.get('Content-Type')).toBe('text/plain');
    });
  });

  describe('Error Handling', () => {
    it('throws API error for non-ok responses', async () => {
      fetchMock.mockResolvedValue(createMockResponse(
        { error: 'User not found' },
        { status: 404, statusText: 'Not Found', ok: false }
      ));

      const client = new APIClient();

      await expect(client.get('users/999')).rejects.toThrow('User not found');
    });

    it('creates API error with status information', async () => {
      fetchMock.mockResolvedValue(createMockResponse(
        { message: 'Database error' },
        { status: 500, statusText: 'Internal Server Error', ok: false }
      ));

      const client = new APIClient();

      await expect(async () => {
        await client.get('users');
      }).rejects.toMatchObject({
        status: 500,
        statusText: 'Internal Server Error',
        endpoint: 'users',
        method: 'GET',
        code: 'API_ERROR',
      });
    });

    it('handles network errors', async () => {
      fetchMock.mockRejectedValue(new Error('Network error'));

      const client = new APIClient();

      await expect(client.get('users')).rejects.toThrow('Network error');
    });

    it('logs API errors', async () => {
      const { logAPIError } = await import('../../../src/client/utils/errorLogger');
      
      fetchMock.mockResolvedValue(createMockResponse(
        { error: 'Not found' },
        { status: 404, statusText: 'Not Found', ok: false }
      ));

      const client = new APIClient();

      try {
        await client.get('users/999');
      } catch {
        // Expected to throw
      }

      expect(logAPIError).toHaveBeenCalledWith(
        expect.any(Error),
        'users/999',
        'GET',
        404
      );
    });

    it('logs network errors', async () => {
      const { logNetworkError } = await import('../../../src/client/utils/errorLogger');
      
      fetchMock.mockRejectedValue(new Error('Failed to fetch'));

      const client = new APIClient();

      await expect(client.get('users')).rejects.toThrow('Failed to fetch');

      expect(logNetworkError).toHaveBeenCalledWith(
        expect.any(Error),
        false,
        false
      );
    });
  });

  describe('Timeout Handling', () => {
    it('applies default timeout', async () => {
      fetchMock.mockImplementation((url, config) => {
        return Promise.resolve(createMockResponse({}));
      });

      const client = new APIClient();
      await client.get('test');

      expect(fetchMock).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          signal: expect.any(AbortSignal),
        })
      );
    });

    it('applies custom timeout', async () => {
      fetchMock.mockImplementation(() => {
        return Promise.reject(new DOMException('The operation was aborted', 'AbortError'));
      });

      const client = new APIClient();

      await expect(client.get('test', { timeout: 10 })).rejects.toThrow('AbortError');
    });
  });

  describe('Retry Logic', () => {
    it('retries on server errors', async () => {
      fetchMock
        .mockResolvedValueOnce(createMockResponse(
          { error: 'Server error' },
          { status: 500, statusText: 'Internal Server Error', ok: false }
        ))
        .mockResolvedValueOnce(createMockResponse({ success: true }));

      const client = new APIClient();
      const response = await client.get('test', { retries: 1, retryDelay: 10 });

      expect(fetchMock).toHaveBeenCalledTimes(2);
      expect(response.data).toEqual({ success: true });
    });

    it('retries on network errors', async () => {
      fetchMock
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce(createMockResponse({ success: true }));

      const client = new APIClient();
      
      await expect(async () => {
        const response = await client.get('test', { retries: 1, retryDelay: 10 });
        expect(fetchMock).toHaveBeenCalledTimes(2);
        expect(response.data).toEqual({ success: true });
      }).not.toThrow();
    });

    it('does not retry on client errors', async () => {
      fetchMock.mockResolvedValue(createMockResponse(
        { error: 'Bad request' },
        { status: 400, statusText: 'Bad Request', ok: false }
      ));

      const client = new APIClient();

      await expect(client.get('test', { retries: 3 })).rejects.toThrow();
      expect(fetchMock).toHaveBeenCalledTimes(1);
    });

    it('retries on specific 4xx errors', async () => {
      fetchMock
        .mockResolvedValueOnce(createMockResponse(
          { error: 'Rate limited' },
          { status: 429, statusText: 'Too Many Requests', ok: false }
        ))
        .mockResolvedValueOnce(createMockResponse({ success: true }));

      const client = new APIClient();
      const response = await client.get('test', { retries: 1, retryDelay: 10 });

      expect(fetchMock).toHaveBeenCalledTimes(2);
      expect(response.data).toEqual({ success: true });
    });

    it('implements exponential backoff', async () => {
      const delays: number[] = [];
      const originalSetTimeout = global.setTimeout;
      
      global.setTimeout = ((fn: any, delay: number) => {
        delays.push(delay);
        return originalSetTimeout(fn, 1); // Speed up test
      }) as any;

      fetchMock
        .mockRejectedValueOnce(new Error('Network error'))
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce(createMockResponse({ success: true }));

      const client = new APIClient();
      
      await expect(async () => {
        await client.get('test', { retries: 2, retryDelay: 100 });
        // Should have exponential backoff: 100ms, 200ms
        expect(delays).toEqual([100, 200]);
      }).not.toThrow();

      global.setTimeout = originalSetTimeout;
    });
  });

  describe('Response Parsing', () => {
    it('parses JSON responses', async () => {
      fetchMock.mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => ({ message: 'success' }),
        headers: new Headers({ 'content-type': 'application/json' }),
        statusText: 'OK',
      });

      const client = new APIClient();
      const response = await client.get('test');

      expect(response.data).toEqual({ message: 'success' });
    });

    it('parses text responses', async () => {
      fetchMock.mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => 'plain text response',
        headers: new Headers({ 'content-type': 'text/plain' }),
        statusText: 'OK',
      });

      const client = new APIClient();
      const response = await client.get('test');

      expect(response.data).toBe('plain text response');
    });

    it('handles blob responses', async () => {
      const mockBlob = new Blob(['binary data']);
      
      fetchMock.mockResolvedValue({
        ok: true,
        status: 200,
        blob: async () => mockBlob,
        headers: new Headers({ 'content-type': 'application/octet-stream' }),
        statusText: 'OK',
      });

      const client = new APIClient();
      const response = await client.get('test');

      expect(response.data).toBe(mockBlob);
    });
  });

  describe('Convenience Functions', () => {
    beforeEach(() => {
      fetchMock.mockResolvedValue(createMockResponse({ success: true }));
    });

    it('get function works', async () => {
      const response = await get('test');
      expect(response.data).toEqual({ success: true });
    });

    it('post function works', async () => {
      const response = await post('test', { data: 'value' });
      expect(response.data).toEqual({ success: true });
    });

    it('put function works', async () => {
      const response = await put('test', { data: 'value' });
      expect(response.data).toEqual({ success: true });
    });

    it('del function works', async () => {
      const response = await del('test');
      expect(response.data).toEqual({ success: true });
    });

    it('patch function works', async () => {
      const response = await patch('test', { data: 'value' });
      expect(response.data).toEqual({ success: true });
    });
  });

  describe('Singleton Instance', () => {
    it('exports singleton apiClient instance', () => {
      expect(apiClient).toBeInstanceOf(APIClient);
    });

    it('convenience functions use singleton instance', async () => {
      fetchMock.mockResolvedValue(createMockResponse({ from: 'singleton' }));

      const response = await get('test');
      
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/test',
        expect.any(Object)
      );
      expect(response.data).toEqual({ from: 'singleton' });
    });
  });
});