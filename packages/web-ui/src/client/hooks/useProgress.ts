import { useState, useEffect, useRef, useCallback } from 'react';
import { useWebSocket } from '../contexts/websocket/hooks';
import type { 
  RunProgress, 
  MaproomIndexingStatus, 
  AgentStatusChange 
} from '../contexts/websocket/types';

export interface ProgressItem {
  id: string;
  type: 'agent' | 'maproom' | 'run' | 'custom';
  label: string;
  progress: number;
  status: 'active' | 'completed' | 'error' | 'paused';
  startTime: Date;
  endTime?: Date;
  currentStep?: string;
  totalSteps?: number;
  metadata?: Record<string, any>;
}

export interface UseProgressReturn {
  /** All progress items */
  items: ProgressItem[];
  /** Add a custom progress item */
  addProgress: (item: Omit<ProgressItem, 'id' | 'startTime'>) => string;
  /** Update a progress item */
  updateProgress: (id: string, updates: Partial<ProgressItem>) => void;
  /** Remove a progress item */
  removeProgress: (id: string) => void;
  /** Clear all progress items */
  clearProgress: () => void;
  /** Get progress by type */
  getProgressByType: (type: ProgressItem['type']) => ProgressItem[];
  /** Get active progress items */
  getActiveProgress: () => ProgressItem[];
  /** Calculate overall progress across all items */
  getOverallProgress: () => number;
}

export const useProgress = (): UseProgressReturn => {
  const [items, setItems] = useState<ProgressItem[]>([]);
  const { 
    connectionState, 
    agents, 
    subscribe, 
    unsubscribe 
  } = useWebSocket();

  // Subscribe to WebSocket events
  useEffect(() => {
    if (connectionState === 'connected') {
      subscribe('run-progress');
      subscribe('maproom-indexing-status');
      subscribe('agent-status-change');
    }

    return () => {
      unsubscribe('run-progress');
      unsubscribe('maproom-indexing-status');
      unsubscribe('agent-status-change');
    };
  }, [connectionState, subscribe, unsubscribe]);

  // Handle run progress updates
  useEffect(() => {
    const handleRunProgress = (data: RunProgress) => {
      setItems(prev => {
        const existingIndex = prev.findIndex(item => 
          item.id === data.id && item.type === 'run'
        );

        const progressItem: ProgressItem = {
          id: data.id,
          type: 'run',
          label: `Agent Run (${data.agentId})`,
          progress: data.progress,
          status: data.status === 'running' ? 'active' : 
                   data.status === 'completed' ? 'completed' : 
                   data.status === 'failed' ? 'error' : 'paused',
          startTime: new Date(data.startTime),
          endTime: data.endTime ? new Date(data.endTime) : undefined,
          currentStep: data.currentStep,
          totalSteps: data.totalSteps,
          metadata: {
            agentId: data.agentId,
            error: data.error,
          },
        };

        if (existingIndex >= 0) {
          const updated = [...prev];
          updated[existingIndex] = progressItem;
          return updated;
        } else {
          return [...prev, progressItem];
        }
      });
    };

    // Note: In a real implementation, you'd subscribe to WebSocket events here
    // For now, we'll handle updates through the WebSocket context
  }, []);

  // Handle maproom indexing status
  useEffect(() => {
    const handleMaproomStatus = (data: MaproomIndexingStatus) => {
      setItems(prev => {
        const existingIndex = prev.findIndex(item => 
          item.id === data.id && item.type === 'maproom'
        );

        const progressItem: ProgressItem = {
          id: data.id,
          type: 'maproom',
          label: 'Maproom Indexing',
          progress: data.progress,
          status: data.status === 'scanning' || data.status === 'indexing' ? 'active' : 
                   data.status === 'completed' ? 'completed' : 
                   data.status === 'error' ? 'error' : 'paused',
          startTime: new Date(), // Would be provided by the actual event
          currentStep: data.currentFile,
          totalSteps: data.totalFiles,
          metadata: {
            filesProcessed: data.filesProcessed,
            totalFiles: data.totalFiles,
            currentFile: data.currentFile,
            error: data.error,
          },
        };

        if (existingIndex >= 0) {
          const updated = [...prev];
          updated[existingIndex] = progressItem;
          return updated;
        } else {
          return [...prev, progressItem];
        }
      });
    };

    // Note: In a real implementation, you'd subscribe to WebSocket events here
  }, []);

  // Handle agent status changes
  useEffect(() => {
    agents.forEach(agent => {
      setItems(prev => {
        const existingIndex = prev.findIndex(item => 
          item.id === agent.id && item.type === 'agent'
        );

        // Only track agents that are actively running tasks
        if (agent.status !== 'running' || !agent.currentTask) {
          if (existingIndex >= 0) {
            // Remove inactive agents
            return prev.filter((_, index) => index !== existingIndex);
          }
          return prev;
        }

        const progressItem: ProgressItem = {
          id: agent.id,
          type: 'agent',
          label: `${agent.name} (${agent.type})`,
          progress: agent.progress || 0,
          status: 'active',
          startTime: new Date(agent.lastActive),
          currentStep: agent.currentTask,
          metadata: {
            agentType: agent.type,
            worktreeId: agent.worktreeId,
            cpuUsage: agent.cpuUsage,
            memoryUsage: agent.memoryUsage,
          },
        };

        if (existingIndex >= 0) {
          const updated = [...prev];
          updated[existingIndex] = progressItem;
          return updated;
        } else {
          return [...prev, progressItem];
        }
      });
    });
  }, [agents]);

  const addProgress = useCallback((item: Omit<ProgressItem, 'id' | 'startTime'>): string => {
    const id = `progress-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    const newItem: ProgressItem = {
      ...item,
      id,
      startTime: new Date(),
    };

    setItems(prev => [...prev, newItem]);
    return id;
  }, []);

  const updateProgress = useCallback((id: string, updates: Partial<ProgressItem>) => {
    setItems(prev => prev.map(item => 
      item.id === id ? { ...item, ...updates } : item
    ));
  }, []);

  const removeProgress = useCallback((id: string) => {
    setItems(prev => prev.filter(item => item.id !== id));
  }, []);

  const clearProgress = useCallback(() => {
    setItems([]);
  }, []);

  const getProgressByType = useCallback((type: ProgressItem['type']) => {
    return items.filter(item => item.type === type);
  }, [items]);

  const getActiveProgress = useCallback(() => {
    return items.filter(item => item.status === 'active');
  }, [items]);

  const getOverallProgress = useCallback(() => {
    const activeItems = getActiveProgress();
    if (activeItems.length === 0) return 100;

    const totalProgress = activeItems.reduce((sum, item) => sum + item.progress, 0);
    return totalProgress / activeItems.length;
  }, [getActiveProgress]);

  return {
    items,
    addProgress,
    updateProgress,
    removeProgress,
    clearProgress,
    getProgressByType,
    getActiveProgress,
    getOverallProgress,
  };
};

// Hook for managing real-time performance metrics
export interface PerformanceMetrics {
  memoryUsage: number;
  cpuUsage: number;
  networkActivity: {
    bytesReceived: number;
    bytesSent: number;
    requestsPerSecond: number;
  };
  renderPerformance: {
    fps: number;
    frameTime: number;
    memoryLeaks: boolean;
  };
  websocketHealth: {
    connected: boolean;
    latency: number;
    messagesPerSecond: number;
  };
}

export interface UsePerformanceMonitorReturn {
  metrics: PerformanceMetrics;
  isMonitoring: boolean;
  startMonitoring: () => void;
  stopMonitoring: () => void;
  resetMetrics: () => void;
}

export const usePerformanceMonitor = (): UsePerformanceMonitorReturn => {
  const [metrics, setMetrics] = useState<PerformanceMetrics>({
    memoryUsage: 0,
    cpuUsage: 0,
    networkActivity: {
      bytesReceived: 0,
      bytesSent: 0,
      requestsPerSecond: 0,
    },
    renderPerformance: {
      fps: 60,
      frameTime: 16,
      memoryLeaks: false,
    },
    websocketHealth: {
      connected: false,
      latency: 0,
      messagesPerSecond: 0,
    },
  });
  
  const [isMonitoring, setIsMonitoring] = useState(false);
  const intervalRef = useRef<NodeJS.Timeout>();
  const frameCountRef = useRef(0);
  const lastFrameTimeRef = useRef(performance.now());
  const networkStatsRef = useRef({ received: 0, sent: 0, requests: 0 });

  const { connectionState, getConnectionStats } = useWebSocket();

  // Monitor render performance
  useEffect(() => {
    if (!isMonitoring) return;

    let frameId: number;
    
    const measureFrame = () => {
      const now = performance.now();
      const frameTime = now - lastFrameTimeRef.current;
      lastFrameTimeRef.current = now;
      frameCountRef.current++;

      // Update FPS every second
      if (frameCountRef.current % 60 === 0) {
        const fps = Math.round(1000 / frameTime);
        setMetrics(prev => ({
          ...prev,
          renderPerformance: {
            ...prev.renderPerformance,
            fps: Math.min(fps, 60),
            frameTime,
          },
        }));
      }

      frameId = requestAnimationFrame(measureFrame);
    };

    frameId = requestAnimationFrame(measureFrame);

    return () => {
      if (frameId) {
        cancelAnimationFrame(frameId);
      }
    };
  }, [isMonitoring]);

  // Monitor system metrics
  useEffect(() => {
    if (!isMonitoring) return;

    intervalRef.current = setInterval(() => {
      // Memory usage (approximate)
      const memoryInfo = (performance as any).memory;
      const memoryUsage = memoryInfo ? 
        (memoryInfo.usedJSHeapSize / memoryInfo.totalJSHeapSize) * 100 : 0;

      // WebSocket health
      const wsStats = getConnectionStats();
      
      setMetrics(prev => ({
        ...prev,
        memoryUsage,
        websocketHealth: {
          connected: connectionState === 'connected',
          latency: 0, // Would be calculated from ping/pong
          messagesPerSecond: 0, // Would be tracked from message count
        },
      }));

      // Check for memory leaks (simplified)
      if (memoryInfo && memoryInfo.usedJSHeapSize > memoryInfo.totalJSHeapSize * 0.9) {
        setMetrics(prev => ({
          ...prev,
          renderPerformance: {
            ...prev.renderPerformance,
            memoryLeaks: true,
          },
        }));
      }
    }, 1000);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [isMonitoring, connectionState, getConnectionStats]);

  const startMonitoring = useCallback(() => {
    setIsMonitoring(true);
  }, []);

  const stopMonitoring = useCallback(() => {
    setIsMonitoring(false);
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
    }
  }, []);

  const resetMetrics = useCallback(() => {
    frameCountRef.current = 0;
    lastFrameTimeRef.current = performance.now();
    networkStatsRef.current = { received: 0, sent: 0, requests: 0 };
    
    setMetrics({
      memoryUsage: 0,
      cpuUsage: 0,
      networkActivity: {
        bytesReceived: 0,
        bytesSent: 0,
        requestsPerSecond: 0,
      },
      renderPerformance: {
        fps: 60,
        frameTime: 16,
        memoryLeaks: false,
      },
      websocketHealth: {
        connected: false,
        latency: 0,
        messagesPerSecond: 0,
      },
    });
  }, []);

  return {
    metrics,
    isMonitoring,
    startMonitoring,
    stopMonitoring,
    resetMetrics,
  };
};

// Hook for tracking operation history
export interface OperationHistoryItem {
  id: string;
  type: string;
  label: string;
  startTime: Date;
  endTime?: Date;
  status: 'completed' | 'error' | 'cancelled';
  duration?: number;
  metadata?: Record<string, any>;
}

export interface UseOperationHistoryReturn {
  history: OperationHistoryItem[];
  addOperation: (operation: Omit<OperationHistoryItem, 'id'>) => void;
  getOperationsByType: (type: string) => OperationHistoryItem[];
  getRecentOperations: (limit?: number) => OperationHistoryItem[];
  clearHistory: () => void;
  exportHistory: () => string;
}

export const useOperationHistory = (maxItems = 100): UseOperationHistoryReturn => {
  const [history, setHistory] = useState<OperationHistoryItem[]>([]);

  const addOperation = useCallback((operation: Omit<OperationHistoryItem, 'id'>) => {
    const newOperation: OperationHistoryItem = {
      ...operation,
      id: `op-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      duration: operation.endTime ? 
        operation.endTime.getTime() - operation.startTime.getTime() : undefined,
    };

    setHistory(prev => {
      const updated = [newOperation, ...prev];
      return updated.slice(0, maxItems); // Keep only recent items
    });
  }, [maxItems]);

  const getOperationsByType = useCallback((type: string) => {
    return history.filter(op => op.type === type);
  }, [history]);

  const getRecentOperations = useCallback((limit = 10) => {
    return history.slice(0, limit);
  }, [history]);

  const clearHistory = useCallback(() => {
    setHistory([]);
  }, []);

  const exportHistory = useCallback(() => {
    return JSON.stringify(history, null, 2);
  }, [history]);

  return {
    history,
    addOperation,
    getOperationsByType,
    getRecentOperations,
    clearHistory,
    exportHistory,
  };
};

export default useProgress;