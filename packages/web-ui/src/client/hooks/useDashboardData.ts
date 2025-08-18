import { useState, useCallback, useRef } from 'react';
import { apiClient } from '../utils/apiClient';
import { useToast } from '../components/ui/use-toast';

interface DashboardData {
  health: any | null;
  worktreeStats: any | null;
  agentStats: any | null;
  indexStats: any | null;
}

interface UseDashboardDataOptions {
  retryAttempts?: number;
  retryDelay?: number;
  onError?: (error: Error) => void;
  onSuccess?: (data: DashboardData) => void;
}

export function useDashboardData(options: UseDashboardDataOptions = {}) {
  const {
    retryAttempts = 3,
    retryDelay = 1000,
    onError,
    onSuccess,
  } = options;

  const [data, setData] = useState<DashboardData>({
    health: null,
    worktreeStats: null,
    agentStats: null,
    indexStats: null,
  });
  
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastFetch, setLastFetch] = useState<Date | null>(null);
  
  const retryTimeoutRef = useRef<NodeJS.Timeout>();
  const abortControllerRef = useRef<AbortController>();
  const { toast } = useToast();

  const fetchWithRetry = useCallback(async (
    fetchFn: () => Promise<any>,
    attemptNumber = 1
  ): Promise<any> => {
    try {
      return await fetchFn();
    } catch (error) {
      if (attemptNumber >= retryAttempts) {
        throw error;
      }

      // Exponential backoff
      const delay = retryDelay * Math.pow(2, attemptNumber - 1);
      
      await new Promise(resolve => {
        retryTimeoutRef.current = setTimeout(resolve, delay);
      });

      return fetchWithRetry(fetchFn, attemptNumber + 1);
    }
  }, [retryAttempts, retryDelay]);

  const fetchDashboardData = useCallback(async (showLoadingState = true) => {
    // Cancel any existing request
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }

    // Clear any pending retry
    if (retryTimeoutRef.current) {
      clearTimeout(retryTimeoutRef.current);
    }

    try {
      if (showLoadingState) {
        setLoading(true);
      }
      setError(null);

      // Create new abort controller for this request
      abortControllerRef.current = new AbortController();
      const signal = abortControllerRef.current.signal;

      // Define fetch functions with abort signal
      const healthFetch = () => apiClient.get('/health', { signal });
      const worktreeStatsFetch = () => apiClient.get('/worktrees/stats', { signal });
      const agentStatsFetch = () => apiClient.get('/agents/runs/stats', { signal });

      // Fetch all endpoints in parallel with retry logic
      const [healthResponse, worktreeStatsResponse, agentStatsResponse] = await Promise.allSettled([
        fetchWithRetry(healthFetch),
        fetchWithRetry(worktreeStatsFetch),
        fetchWithRetry(agentStatsFetch),
      ]);

      // Check if request was aborted
      if (signal.aborted) {
        return;
      }

      const newData: DashboardData = {
        health: healthResponse.status === 'fulfilled' ? healthResponse.value.data : null,
        worktreeStats: worktreeStatsResponse.status === 'fulfilled' ? worktreeStatsResponse.value.data : null,
        agentStats: agentStatsResponse.status === 'fulfilled' ? agentStatsResponse.value.data : null,
        indexStats: null, // Would come from WebSocket or separate endpoint
      };

      setData(newData);
      setLastFetch(new Date());
      onSuccess?.(newData);

      // Handle partial failures
      const failedEndpoints = [];
      if (healthResponse.status === 'rejected') {
        failedEndpoints.push({ name: 'Health', error: healthResponse.reason });
      }
      if (worktreeStatsResponse.status === 'rejected') {
        failedEndpoints.push({ name: 'Worktrees', error: worktreeStatsResponse.reason });
      }
      if (agentStatsResponse.status === 'rejected') {
        failedEndpoints.push({ name: 'Agents', error: agentStatsResponse.reason });
      }

      if (failedEndpoints.length > 0) {
        const errorMessage = `Partial data load failed: ${failedEndpoints.map(e => e.name).join(', ')}`;
        setError(errorMessage);
        
        toast({
          title: 'Partial Data Load',
          description: errorMessage,
          variant: 'destructive',
        });

        // Log detailed errors for debugging
        failedEndpoints.forEach(({ name, error }) => {
          console.error(`Failed to fetch ${name} stats:`, error);
        });
      }

    } catch (err) {
      // Don't show error if request was aborted
      if (abortControllerRef.current?.signal.aborted) {
        return;
      }

      const errorMessage = err instanceof Error ? err.message : 'Failed to load dashboard data';
      setError(errorMessage);
      onError?.(err instanceof Error ? err : new Error(errorMessage));

      toast({
        title: 'Data Load Failed',
        description: errorMessage,
        variant: 'destructive',
      });

      console.error('Dashboard data fetch error:', err);
    } finally {
      if (showLoadingState) {
        setLoading(false);
      }
    }
  }, [fetchWithRetry, onSuccess, onError, toast]);

  const refreshData = useCallback(() => {
    return fetchDashboardData(false);
  }, [fetchDashboardData]);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const isStale = useCallback((maxAgeMs = 5 * 60 * 1000) => {
    if (!lastFetch) return true;
    return Date.now() - lastFetch.getTime() > maxAgeMs;
  }, [lastFetch]);

  // Cleanup on unmount
  const cleanup = useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }
    if (retryTimeoutRef.current) {
      clearTimeout(retryTimeoutRef.current);
    }
  }, []);

  return {
    data,
    loading,
    error,
    lastFetch,
    fetchDashboardData,
    refreshData,
    clearError,
    isStale,
    cleanup,
  };
}