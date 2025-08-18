/**
 * API client with error handling and interceptors
 */

import type { APIError } from '@types/error';
import { logAPIError, logNetworkError } from './errorLogger';

export interface RequestConfig extends RequestInit {
  timeout?: number;
  retries?: number;
  retryDelay?: number;
}

export interface APIResponse<T = unknown> {
  data: T;
  status: number;
  statusText: string;
  headers: Headers;
}

export class APIClient {
  private baseURL: string;
  private defaultTimeout = 10000; // 10 seconds
  private defaultRetries = 3;
  private defaultRetryDelay = 1000; // 1 second

  constructor(baseURL = '/api') {
    this.baseURL = baseURL;
  }

  /**
   * Makes an HTTP request with error handling and retries
   */
  async request<T = unknown>(
    endpoint: string,
    config: RequestConfig = {}
  ): Promise<APIResponse<T>> {
    const {
      timeout = this.defaultTimeout,
      retries = this.defaultRetries,
      retryDelay = this.defaultRetryDelay,
      ...fetchConfig
    } = config;

    const url = this.buildURL(endpoint);
    const fullConfig = this.buildRequestConfig(fetchConfig);

    let lastError: Error | null = null;

    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        const response = await this.fetchWithTimeout(url, fullConfig, timeout);
        
        if (!response.ok) {
          throw await this.createAPIError(response, endpoint, fullConfig.method);
        }

        const data = await this.parseResponse<T>(response);
        
        return {
          data,
          status: response.status,
          statusText: response.statusText,
          headers: response.headers,
        };
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        // Log the error
        if (this.isNetworkError(lastError)) {
          logNetworkError(
            lastError,
            !navigator.onLine,
            lastError.name === 'AbortError'
          );
        } else if (this.isAPIError(lastError)) {
          logAPIError(
            lastError,
            endpoint,
            fullConfig.method,
            (lastError as APIError).status
          );
        }

        // Don't retry on certain errors
        if (!this.shouldRetry(lastError, attempt, retries)) {
          throw lastError;
        }

        // Wait before retrying
        if (attempt < retries) {
          await this.delay(retryDelay * Math.pow(2, attempt)); // Exponential backoff
        }
      }
    }

    throw lastError;
  }

  /**
   * GET request
   */
  async get<T = unknown>(endpoint: string, config?: RequestConfig): Promise<APIResponse<T>> {
    return this.request<T>(endpoint, { ...config, method: 'GET' });
  }

  /**
   * POST request
   */
  async post<T = unknown>(
    endpoint: string,
    data?: unknown,
    config?: RequestConfig
  ): Promise<APIResponse<T>> {
    return this.request<T>(endpoint, {
      ...config,
      method: 'POST',
      body: data ? JSON.stringify(data) : undefined,
    });
  }

  /**
   * PUT request
   */
  async put<T = unknown>(
    endpoint: string,
    data?: unknown,
    config?: RequestConfig
  ): Promise<APIResponse<T>> {
    return this.request<T>(endpoint, {
      ...config,
      method: 'PUT',
      body: data ? JSON.stringify(data) : undefined,
    });
  }

  /**
   * DELETE request
   */
  async delete<T = unknown>(endpoint: string, config?: RequestConfig): Promise<APIResponse<T>> {
    return this.request<T>(endpoint, { ...config, method: 'DELETE' });
  }

  /**
   * PATCH request
   */
  async patch<T = unknown>(
    endpoint: string,
    data?: unknown,
    config?: RequestConfig
  ): Promise<APIResponse<T>> {
    return this.request<T>(endpoint, {
      ...config,
      method: 'PATCH',
      body: data ? JSON.stringify(data) : undefined,
    });
  }

  /**
   * Builds the full URL
   */
  private buildURL(endpoint: string): string {
    const cleanEndpoint = endpoint.startsWith('/') ? endpoint.slice(1) : endpoint;
    const cleanBaseURL = this.baseURL.endsWith('/') ? this.baseURL.slice(0, -1) : this.baseURL;
    return `${cleanBaseURL}/${cleanEndpoint}`;
  }

  /**
   * Builds the request configuration with defaults
   */
  private buildRequestConfig(config: RequestInit): RequestInit {
    const headers = new Headers({
      'Content-Type': 'application/json',
      Accept: 'application/json',
      ...Object.fromEntries(new Headers(config.headers).entries()),
    });

    return {
      ...config,
      headers,
    };
  }

  /**
   * Fetch with timeout
   */
  private async fetchWithTimeout(
    url: string,
    config: RequestInit,
    timeout: number
  ): Promise<Response> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);

    try {
      const response = await fetch(url, {
        ...config,
        signal: controller.signal,
      });
      return response;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Creates an API error from a response
   */
  private async createAPIError(
    response: Response,
    endpoint: string,
    method?: string
  ): Promise<APIError> {
    let message = `HTTP ${response.status}: ${response.statusText}`;
    
    try {
      const errorData = await response.json();
      if (errorData.message) {
        message = errorData.message;
      } else if (errorData.error) {
        message = errorData.error;
      }
    } catch {
      // If we can't parse the error response, use the default message
    }

    const error = new Error(message) as APIError;
    error.status = response.status;
    error.statusText = response.statusText;
    error.endpoint = endpoint;
    error.method = method;
    error.code = 'API_ERROR';
    error.timestamp = new Date();

    return error;
  }

  /**
   * Parses the response based on content type
   */
  private async parseResponse<T>(response: Response): Promise<T> {
    const contentType = response.headers.get('content-type');
    
    if (contentType && contentType.includes('application/json')) {
      return response.json();
    }
    
    if (contentType && contentType.includes('text/')) {
      return response.text() as unknown as T;
    }
    
    return response.blob() as unknown as T;
  }

  /**
   * Checks if an error is a network error
   */
  private isNetworkError(error: Error): boolean {
    return (
      error.name === 'TypeError' ||
      error.name === 'AbortError' ||
      error.message.includes('fetch') ||
      error.message.includes('network') ||
      error.message.includes('Failed to fetch')
    );
  }

  /**
   * Checks if an error is an API error
   */
  private isAPIError(error: Error): boolean {
    return 'status' in error && 'endpoint' in error;
  }

  /**
   * Determines if a request should be retried
   */
  private shouldRetry(error: Error, attempt: number, maxRetries: number): boolean {
    if (attempt >= maxRetries) return false;

    // Don't retry on client errors (4xx) except for specific cases
    if (this.isAPIError(error)) {
      const apiError = error as APIError;
      if (apiError.status && apiError.status >= 400 && apiError.status < 500) {
        // Retry on specific 4xx errors
        return apiError.status === 408 || apiError.status === 429;
      }
    }

    // Retry on network errors and 5xx errors
    return this.isNetworkError(error) || (this.isAPIError(error) && (error as APIError).status! >= 500);
  }

  /**
   * Delays execution
   */
  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// Export singleton instance
export const apiClient = new APIClient();

// Export convenience methods
export const get = <T = unknown>(endpoint: string, config?: RequestConfig) =>
  apiClient.get<T>(endpoint, config);

export const post = <T = unknown>(endpoint: string, data?: unknown, config?: RequestConfig) =>
  apiClient.post<T>(endpoint, data, config);

export const put = <T = unknown>(endpoint: string, data?: unknown, config?: RequestConfig) =>
  apiClient.put<T>(endpoint, data, config);

export const del = <T = unknown>(endpoint: string, config?: RequestConfig) =>
  apiClient.delete<T>(endpoint, config);

export const patch = <T = unknown>(endpoint: string, data?: unknown, config?: RequestConfig) =>
  apiClient.patch<T>(endpoint, data, config);