import { useEffect, useState, useRef, useCallback } from 'react';
import { useWebSocketDashboard, useWebSocketConnection, useWebSocketMessaging } from '../contexts/websocket/index.js';

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
  cpuUsage?: number;
  memoryUsage?: number;
  lastActive: string;
  currentTask?: string;
  worktreeId?: string;
  progress?: number;
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
  // Use the new WebSocket context hooks
  const connection = useWebSocketConnection();
  const dashboard = useWebSocketDashboard();
  const messaging = useWebSocketMessaging();

  const {
    autoConnect = true,
    maxReconnectAttempts = 5,
    reconnectDelay = 1000,
  } = options;

  // Legacy state for backward compatibility
  const [connecting, setConnecting] = useState(false);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout>();

  const connect = useCallback(async () => {
    if (connecting || connection.isConnected) return;

    try {
      setConnecting(true);
      await connection.connect();
      setConnecting(false);
    } catch (err) {
      setConnecting(false);
      throw err;
    }
  }, [connecting, connection.isConnected, connection.connect]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
    }
    connection.disconnect();
    setConnecting(false);
  }, [connection.disconnect]);

  const sendMessage = useCallback((type: string, data: any) => {
    if (connection.isConnected) {
      messaging.sendMessageSync(type, data);
    }
  }, [connection.isConnected, messaging.sendMessageSync]);

  const refreshStats = useCallback(() => {
    dashboard.refreshDashboardStats();
  }, [dashboard.refreshDashboardStats]);

  // Auto-connect on mount
  useEffect(() => {
    if (autoConnect && !connection.isConnected && connection.connectionState === 'disconnected') {
      connect();
    }
  }, [autoConnect, connection.isConnected, connection.connectionState, connect]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
    };
  }, []);

  return {
    // Connection state (legacy compatibility)
    connected: connection.isConnected,
    connecting,
    error: connection.error?.message || null,
    
    // Real-time data
    stats: dashboard.dashboardStats,
    activities: dashboard.activities,
    agents: dashboard.agents.map(agent => ({
      ...agent,
      // Map new agent type to legacy type for compatibility
      type: agent.type || 'unknown',
    })),
    performance: dashboard.performance,
    
    // Actions
    connect,
    disconnect,
    sendMessage,
    refreshStats,
    clearActivities: dashboard.clearActivities,
    filterActivities: dashboard.filterActivities,
    
    // Utils
    reconnectAttempts: connection.reconnectAttempts,
  };
}