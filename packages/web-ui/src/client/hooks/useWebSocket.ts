import { useEffect, useState, useRef, useCallback } from 'react';
import { CrewChiefWebSocketClient } from '../../server/websocket/client-example.js';

export interface DashboardStats {
  totalWorktrees: number;
  activeAgents: number;
  indexedFiles: number;
  systemHealth: 'healthy' | 'warning' | 'critical';
  apiResponseTime: number;
  diskUsage: number;
  lastUpdated: string;
}

export interface ActivityEvent {
  id: string;
  type: 'worktree_update' | 'agent_status_change' | 'run_progress' | 'maproom_indexing_status' | 'config_change' | 'system_update';
  timestamp: string;
  title: string;
  description: string;
  severity: 'info' | 'success' | 'warning' | 'error';
  metadata?: Record<string, any>;
}

export interface AgentStatus {
  id: string;
  type: string;
  name: string;
  status: 'idle' | 'running' | 'error' | 'stopped';
  cpuUsage: number;
  memoryUsage: number;
  lastActive: string;
  currentTask?: string;
  worktreeId?: string;
}

export interface PerformanceMetrics {
  apiResponseTime: number;
  websocketConnected: boolean;
  dbQueryTime: number;
  timestamp: string;
}

export interface UseWebSocketOptions {
  autoConnect?: boolean;
  maxReconnectAttempts?: number;
  reconnectDelay?: number;
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const {
    autoConnect = true,
    maxReconnectAttempts = 5,
    reconnectDelay = 1000,
  } = options;

  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [activities, setActivities] = useState<ActivityEvent[]>([]);
  const [agents, setAgents] = useState<AgentStatus[]>([]);
  const [performance, setPerformance] = useState<PerformanceMetrics | null>(null);

  const clientRef = useRef<CrewChiefWebSocketClient | null>(null);
  const reconnectAttempts = useRef(0);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout>();

  const connect = useCallback(async () => {
    if (connecting || connected) return;

    try {
      setConnecting(true);
      setError(null);

      const client = new CrewChiefWebSocketClient({
        url: `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}`,
        autoReconnect: false, // We handle reconnection ourselves
        reconnectionAttempts: 1,
      });

      // Set up event listeners before connecting
      const originalSocket = (client as any).socket;
      if (originalSocket) {
        // Dashboard-specific event handlers
        originalSocket.on('dashboard-stats-update', (data: DashboardStats) => {
          setStats(data);
        });

        originalSocket.on('activity-event', (event: ActivityEvent) => {
          setActivities(prev => [event, ...prev.slice(0, 49)]); // Keep last 50 events
        });

        originalSocket.on('agent-status-update', (agentData: AgentStatus) => {
          setAgents(prev => {
            const updated = prev.filter(a => a.id !== agentData.id);
            return [agentData, ...updated];
          });
        });

        originalSocket.on('performance-metrics', (metrics: PerformanceMetrics) => {
          setPerformance(metrics);
        });

        originalSocket.on('connect', () => {
          setConnected(true);
          setConnecting(false);
          setError(null);
          reconnectAttempts.current = 0;
        });

        originalSocket.on('disconnect', () => {
          setConnected(false);
          handleReconnect();
        });

        originalSocket.on('connect_error', (err: Error) => {
          setError(err.message);
          setConnecting(false);
          handleReconnect();
        });
      }

      await client.connect();
      
      // Subscribe to dashboard events
      await client.subscribe('dashboard-updates');
      await client.subscribe('agent-updates');
      await client.subscribe('activity-events');
      await client.subscribe('performance-metrics');

      clientRef.current = client;
      
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Connection failed');
      setConnecting(false);
      handleReconnect();
    }
  }, [connecting, connected]);

  const handleReconnect = useCallback(() => {
    if (reconnectAttempts.current >= maxReconnectAttempts) {
      setError(`Failed to reconnect after ${maxReconnectAttempts} attempts`);
      return;
    }

    reconnectAttempts.current++;
    const delay = reconnectDelay * Math.pow(2, reconnectAttempts.current - 1); // Exponential backoff

    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
    }

    reconnectTimeoutRef.current = setTimeout(() => {
      connect();
    }, delay);
  }, [connect, maxReconnectAttempts, reconnectDelay]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
    }

    if (clientRef.current) {
      clientRef.current.disconnect();
      clientRef.current = null;
    }

    setConnected(false);
    setConnecting(false);
    setError(null);
    reconnectAttempts.current = 0;
  }, []);

  const sendMessage = useCallback((type: string, data: any) => {
    if (clientRef.current && connected) {
      clientRef.current.sendMessage(type, data);
    }
  }, [connected]);

  const refreshStats = useCallback(() => {
    sendMessage('request-dashboard-refresh', {});
  }, [sendMessage]);

  const clearActivities = useCallback(() => {
    setActivities([]);
  }, []);

  const filterActivities = useCallback((type?: string, severity?: string) => {
    return activities.filter(activity => {
      if (type && activity.type !== type) return false;
      if (severity && activity.severity !== severity) return false;
      return true;
    });
  }, [activities]);

  // Auto-connect on mount
  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, [autoConnect, connect, disconnect]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
    };
  }, []);

  return {
    // Connection state
    connected,
    connecting,
    error,
    
    // Real-time data
    stats,
    activities,
    agents,
    performance,
    
    // Actions
    connect,
    disconnect,
    sendMessage,
    refreshStats,
    clearActivities,
    filterActivities,
    
    // Utils
    reconnectAttempts: reconnectAttempts.current,
  };
}