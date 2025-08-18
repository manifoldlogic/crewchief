import { useState, useCallback, useRef, useEffect } from 'react';

export interface SearchFilters {
  languages: string[];
  fileTypes: string[];
  paths: string[];
  dateRange: {
    start?: string;
    end?: string;
  };
  relevanceThreshold: number;
  maxResults: number;
}

export interface SearchResult {
  id: string;
  file_path: string;
  line_start: number;
  line_end: number;
  content: string;
  relevance_score: number;
  language?: string;
  chunk_type?: string;
  context?: {
    before?: string;
    after?: string;
  };
}

export interface SearchResponse {
  query: string;
  results: SearchResult[];
  totalCount: number;
  executionTimeMs: number;
  filters: SearchFilters;
  cached: boolean;
}

export interface SearchError {
  message: string;
  code?: string;
  details?: any;
}

export interface UseMaproomSearchOptions {
  debounceMs?: number;
  maxRetries?: number;
  retryDelayMs?: number;
  cacheResults?: boolean;
}

export interface UseMaproomSearchResult {
  // State
  results: SearchResult[];
  loading: boolean;
  error: SearchError | null;
  executionTime: number;
  totalCount: number;
  query: string;
  filters: SearchFilters;
  
  // Actions
  search: (query: string, filters?: Partial<SearchFilters>) => Promise<void>;
  clearResults: () => void;
  clearError: () => void;
  retry: () => Promise<void>;
  
  // Status
  hasSearched: boolean;
  isRetrying: boolean;
}

const DEFAULT_FILTERS: SearchFilters = {
  languages: [],
  fileTypes: [],
  paths: [],
  dateRange: {},
  relevanceThreshold: 0.0,
  maxResults: 100,
};

export const useMaproomSearch = (
  options: UseMaproomSearchOptions = {}
): UseMaproomSearchResult => {
  const {
    debounceMs = 300,
    maxRetries = 2,
    retryDelayMs = 1000,
    cacheResults = true,
  } = options;

  // State
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<SearchError | null>(null);
  const [executionTime, setExecutionTime] = useState(0);
  const [totalCount, setTotalCount] = useState(0);
  const [query, setQuery] = useState('');
  const [filters, setFilters] = useState<SearchFilters>(DEFAULT_FILTERS);
  const [hasSearched, setHasSearched] = useState(false);
  const [isRetrying, setIsRetrying] = useState(false);

  // Refs
  const abortControllerRef = useRef<AbortController | null>(null);
  const debounceTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const cacheRef = useRef<Map<string, SearchResponse>>(new Map());
  const lastSearchRef = useRef<{ query: string; filters: SearchFilters } | null>(null);

  // Cache key generation
  const getCacheKey = useCallback((searchQuery: string, searchFilters: SearchFilters): string => {
    return JSON.stringify({ query: searchQuery, filters: searchFilters });
  }, []);

  // API call function
  const performSearch = useCallback(async (
    searchQuery: string, 
    searchFilters: SearchFilters,
    retryCount = 0
  ): Promise<SearchResponse> => {
    const cacheKey = getCacheKey(searchQuery, searchFilters);
    
    // Check cache first
    if (cacheResults && cacheRef.current.has(cacheKey)) {
      const cachedResult = cacheRef.current.get(cacheKey)!;
      return { ...cachedResult, cached: true };
    }

    // Create abort controller for this request
    abortControllerRef.current = new AbortController();

    try {
      const startTime = Date.now();
      
      // Build request payload
      const requestBody = {
        query: searchQuery,
        filters: {
          ...(searchFilters.languages.length > 0 && { languages: searchFilters.languages }),
          ...(searchFilters.fileTypes.length > 0 && { fileTypes: searchFilters.fileTypes }),
          ...(searchFilters.paths.length > 0 && { paths: searchFilters.paths }),
          ...(searchFilters.dateRange.start && { dateRange: searchFilters.dateRange }),
          ...(searchFilters.relevanceThreshold > 0 && { relevanceThreshold: searchFilters.relevanceThreshold }),
          maxResults: searchFilters.maxResults,
        },
      };

      const response = await fetch('/api/maproom/search', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(requestBody),
        signal: abortControllerRef.current.signal,
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(errorData.message || `Search failed: ${response.status} ${response.statusText}`);
      }

      const data = await response.json();
      
      const searchResponse: SearchResponse = {
        query: searchQuery,
        results: data.results || [],
        totalCount: data.totalCount || data.results?.length || 0,
        executionTimeMs: Date.now() - startTime,
        filters: searchFilters,
        cached: false,
      };

      // Cache the result
      if (cacheResults) {
        cacheRef.current.set(cacheKey, searchResponse);
        
        // Limit cache size to prevent memory issues
        if (cacheRef.current.size > 50) {
          const firstKey = cacheRef.current.keys().next().value;
          cacheRef.current.delete(firstKey);
        }
      }

      return searchResponse;
    } catch (error: any) {
      // Don't retry on abort
      if (error.name === 'AbortError') {
        throw error;
      }

      // Retry logic
      if (retryCount < maxRetries) {
        await new Promise(resolve => setTimeout(resolve, retryDelayMs * (retryCount + 1)));
        return performSearch(searchQuery, searchFilters, retryCount + 1);
      }

      throw error;
    }
  }, [getCacheKey, cacheResults, maxRetries, retryDelayMs]);

  // Main search function
  const search = useCallback(async (
    searchQuery: string, 
    searchFilters: Partial<SearchFilters> = {}
  ): Promise<void> => {
    if (!searchQuery.trim()) {
      setResults([]);
      setError(null);
      setHasSearched(false);
      return;
    }

    // Cancel any existing request
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }

    // Clear debounce timeout
    if (debounceTimeoutRef.current) {
      clearTimeout(debounceTimeoutRef.current);
    }

    const finalFilters = { ...DEFAULT_FILTERS, ...filters, ...searchFilters };
    
    setQuery(searchQuery);
    setFilters(finalFilters);
    setLoading(true);
    setError(null);
    setIsRetrying(false);

    // Store for retry
    lastSearchRef.current = { query: searchQuery, filters: finalFilters };

    try {
      const response = await performSearch(searchQuery, finalFilters);
      
      setResults(response.results);
      setTotalCount(response.totalCount);
      setExecutionTime(response.executionTimeMs);
      setHasSearched(true);
    } catch (error: any) {
      if (error.name !== 'AbortError') {
        const searchError: SearchError = {
          message: error.message || 'Search failed',
          code: error.code,
          details: error,
        };
        setError(searchError);
        setResults([]);
        setTotalCount(0);
        setHasSearched(true);
      }
    } finally {
      setLoading(false);
      setIsRetrying(false);
    }
  }, [filters, performSearch]);

  // Retry last search
  const retry = useCallback(async (): Promise<void> => {
    if (!lastSearchRef.current) return;
    
    setIsRetrying(true);
    await search(lastSearchRef.current.query, lastSearchRef.current.filters);
  }, [search]);

  // Clear functions
  const clearResults = useCallback(() => {
    setResults([]);
    setTotalCount(0);
    setExecutionTime(0);
    setQuery('');
    setHasSearched(false);
    setError(null);
    lastSearchRef.current = null;
  }, []);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }
      if (debounceTimeoutRef.current) {
        clearTimeout(debounceTimeoutRef.current);
      }
    };
  }, []);

  return {
    // State
    results,
    loading,
    error,
    executionTime,
    totalCount,
    query,
    filters,
    
    // Actions
    search,
    clearResults,
    clearError,
    retry,
    
    // Status
    hasSearched,
    isRetrying,
  };
};