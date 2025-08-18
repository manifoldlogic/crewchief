/**
 * WebSocket Hooks
 * 
 * React hooks for WebSocket functionality with automatic
 * subscription management and optimized re-renders.
 */

import { useEffect, useCallback, useRef, useMemo } from 'react';
import { useWebSocketContext } from './context.js';
import type {
  UseWebSocketOptions,
  WebSocketEventType,
  WebSocketEventHandler,
  DashboardStats,
  ActivityEvent,
  AgentStatusChange,
  PerformanceMetrics,
  WebSocketContextValue,
} from './types.js';

/**
 * Main WebSocket hook - provides access to WebSocket context
 */
export function useWebSocket(options: UseWebSocketOptions = {}) {
  const context = useWebSocketContext();
  const {
    autoConnect = true,
    requireAuth = false,
    subscriptions = [],
    rooms = [],
  } = options;

  const hasInitialized = useRef(false);

  // Auto-connect if enabled
  useEffect(() => {
    if (autoConnect && !hasInitialized.current && context.connectionState === 'disconnected') {
      if (!requireAuth || context.isAuthenticated || context.authToken) {
        context.connect().catch(console.error);
        hasInitialized.current = true;
      }
    }
  }, [autoConnect, requireAuth, context.isAuthenticated, context.authToken, context.connectionState]);

  // Auto-subscribe to specified event types
  useEffect(() => {
    if (context.isConnected && subscriptions.length > 0) {
      subscriptions.forEach(async (subscription) => {
        try {
          if (!context.activeSubscriptions.has(subscription)) {
            await context.subscribe(subscription);
          }
        } catch (error) {
          console.warn(`Failed to subscribe to ${subscription}:`, error);
        }
      });
    }
  }, [context.isConnected, subscriptions, context.activeSubscriptions]);

  // Auto-join specified rooms
  useEffect(() => {
    if (context.isConnected && rooms.length > 0) {
      rooms.forEach(async (room) => {
        try {
          if (!context.joinedRooms.has(room)) {
            await context.joinRoom(room);
          }
        } catch (error) {
          console.warn(`Failed to join room ${room}:`, error);
        }
      });
    }
  }, [context.isConnected, rooms, context.joinedRooms]);

  return context;
}

/**
 * Hook for connection state with optimized re-renders
 */
export function useWebSocketConnection() {
  const { connectionState, isAuthenticated, error, reconnectAttempts, isConnected, connect, disconnect, reconnect } = useWebSocketContext();

  return {
    connectionState,
    isConnected: isConnected(),
    isAuthenticated,
    error,
    reconnectAttempts,
    connect,
    disconnect,
    reconnect,
  };
}

/**
 * Hook for messaging with queue management
 */
export function useWebSocketMessaging() {
  const { 
    sendMessage, 
    sendMessageSync, 
    messageQueue, 
    queueSize, 
    clearMessageQueue, 
    retryFailedMessages,
    isConnected,
  } = useWebSocketContext();

  const send = useCallback(async (event: string, data: any, room?: string) => {
    return await sendMessage(event, data, room);
  }, [sendMessage]);

  const sendSync = useCallback((event: string, data: any, room?: string) => {
    sendMessageSync(event, data, room);
  }, [sendMessageSync]);

  return {
    sendMessage: send,
    sendMessageSync: sendSync,
    messageQueue,
    queueSize,
    clearMessageQueue,
    retryFailedMessages,
    isConnected: isConnected(),
  };
}

/**
 * Hook for room management
 */
export function useWebSocketRooms() {
  const { joinedRooms, joinRoom, leaveRoom, isConnected } = useWebSocketContext();

  const join = useCallback(async (room: string, type = 'general') => {
    return await joinRoom(room, type);
  }, [joinRoom]);

  const leave = useCallback(async (room: string) => {
    return await leaveRoom(room);
  }, [leaveRoom]);

  const getRoomsArray = useMemo(() => {
    return Array.from(joinedRooms.entries()).map(([name, info]) => ({ name, ...info }));
  }, [joinedRooms]);

  return {
    joinedRooms: getRoomsArray,
    joinRoom: join,
    leaveRoom: leave,
    isConnected: isConnected(),
  };
}

/**
 * Hook for subscription management
 */
export function useWebSocketSubscriptions() {
  const { activeSubscriptions, subscribe, unsubscribe, isConnected } = useWebSocketContext();

  const sub = useCallback(async (type: WebSocketEventType, filters?: Record<string, any>) => {
    return await subscribe(type, filters);
  }, [subscribe]);

  const unsub = useCallback(async (type: WebSocketEventType) => {
    return await unsubscribe(type);
  }, [unsubscribe]);

  const getSubscriptionsArray = useMemo(() => {
    return Array.from(activeSubscriptions.entries()).map(([type, subscription]) => ({ type, ...subscription }));
  }, [activeSubscriptions]);

  return {
    activeSubscriptions: getSubscriptionsArray,
    subscribe: sub,
    unsubscribe: unsub,
    isConnected: isConnected(),
  };
}

/**
 * Hook for dashboard data with optimized updates
 */
export function useWebSocketDashboard() {
  const { 
    dashboardStats, 
    activities, 
    agents, 
    performance, 
    clearActivities, 
    refreshDashboardStats,
    subscribe,
    isConnected,
  } = useWebSocketContext();

  // Auto-subscribe to dashboard events
  useEffect(() => {
    if (isConnected()) {
      const subscribeToEvents = async () => {
        try {
          await subscribe('dashboard-stats-update');
          await subscribe('activity-event');
          await subscribe('agent-status-update');
          await subscribe('performance-metrics');
        } catch (error) {
          console.warn('Failed to subscribe to dashboard events:', error);
        }
      };
      
      subscribeToEvents();
    }
  }, [isConnected(), subscribe]);

  const filterActivities = useCallback((type?: string, severity?: string) => {
    return activities.filter(activity => {
      if (type && activity.type !== type) return false;
      if (severity && activity.severity !== severity) return false;
      return true;
    });
  }, [activities]);

  const getAgentById = useCallback((id: string) => {
    return agents.find(agent => agent.id === id);
  }, [agents]);

  const getAgentsByStatus = useCallback((status: AgentStatusChange['status']) => {
    return agents.filter(agent => agent.status === status);
  }, [agents]);

  return {
    dashboardStats,
    activities,
    agents,
    performance,
    clearActivities,
    refreshDashboardStats,
    filterActivities,
    getAgentById,
    getAgentsByStatus,
    isConnected: isConnected(),
  };
}

/**
 * Hook for specific event listening with automatic cleanup
 */
export function useWebSocketEvent<T = any>(
  eventType: WebSocketEventType,
  handler: WebSocketEventHandler<T>,
  filters?: Record<string, any>
) {
  const { subscribe, unsubscribe, isConnected } = useWebSocketContext();
  const handlerRef = useRef(handler);
  const hasSubscribed = useRef(false);

  // Update handler ref when handler changes
  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  // Subscribe to event when connected
  useEffect(() => {
    if (isConnected() && !hasSubscribed.current) {
      subscribe(eventType, filters).catch(console.error);
      hasSubscribed.current = true;
    }
  }, [isConnected(), eventType, filters, subscribe]);

  // Setup event listener (this would require extending the client to support external handlers)
  useEffect(() => {
    // Note: This would need to be implemented in the client to support external event handlers
    // For now, components should use the context data directly
    
    return () => {
      // Cleanup if needed
    };
  }, [eventType]);
}

/**
 * Hook for authentication integration
 */
export function useWebSocketAuth() {
  const { 
    isAuthenticated, 
    authToken, 
    setAuthToken, 
    refreshAuth, 
    connectionState,
    connect,
    disconnect,
  } = useWebSocketContext();

  const updateAuth = useCallback((token: { accessToken: string; refreshToken?: string; expiresAt?: Date } | null) => {
    setAuthToken(token);
    
    // Reconnect if we're disconnected and now have a token
    if (token && connectionState === 'disconnected') {
      connect().catch(console.error);
    }
    
    // Disconnect if token is removed
    if (!token && connectionState === 'connected') {
      disconnect();
    }
  }, [setAuthToken, connectionState, connect, disconnect]);

  const refresh = useCallback(async () => {
    try {
      await refreshAuth();
    } catch (error) {
      console.error('Failed to refresh WebSocket auth:', error);
      throw error;
    }
  }, [refreshAuth]);

  return {
    isAuthenticated,
    authToken,
    setAuthToken: updateAuth,
    refreshAuth: refresh,
    connectionState,
  };
}

/**
 * Hook for performance metrics and connection stats
 */
export function useWebSocketStats() {
  const { 
    totalMessagesSent,
    totalMessagesReceived,
    lastHeartbeat,
    reconnectAttempts,
    queueSize,
    getConnectionStats,
    performance,
  } = useWebSocketContext();

  const stats = useMemo(() => ({
    totalMessagesSent,
    totalMessagesReceived,
    lastHeartbeat,
    reconnectAttempts,
    queueSize,
    performance,
    ...getConnectionStats(),
  }), [
    totalMessagesSent,
    totalMessagesReceived,
    lastHeartbeat,
    reconnectAttempts,
    queueSize,
    performance,
    getConnectionStats,
  ]);

  return stats;
}

/**
 * Hook for optimized agent monitoring
 */
export function useWebSocketAgents(agentIds?: string[]) {
  const { agents, subscribe, isConnected } = useWebSocketContext();

  // Auto-subscribe to agent updates
  useEffect(() => {
    if (isConnected()) {
      subscribe('agent-status-change').catch(console.error);
    }
  }, [isConnected(), subscribe]);

  const filteredAgents = useMemo(() => {
    if (!agentIds || agentIds.length === 0) {
      return agents;
    }
    return agents.filter(agent => agentIds.includes(agent.id));
  }, [agents, agentIds]);

  const runningAgents = useMemo(() => {
    return filteredAgents.filter(agent => agent.status === 'running');
  }, [filteredAgents]);

  const idleAgents = useMemo(() => {
    return filteredAgents.filter(agent => agent.status === 'idle');
  }, [filteredAgents]);

  const errorAgents = useMemo(() => {
    return filteredAgents.filter(agent => agent.status === 'error');
  }, [filteredAgents]);

  return {
    agents: filteredAgents,
    runningAgents,
    idleAgents,
    errorAgents,
    isConnected: isConnected(),
  };
}

/**
 * Hook for real-time worktree monitoring
 */
export function useWebSocketWorktrees(worktreeIds?: string[]) {
  const { subscribe, isConnected, sendMessage } = useWebSocketContext();

  // Auto-subscribe to worktree updates
  useEffect(() => {
    if (isConnected()) {
      const filters = worktreeIds ? { worktreeIds } : undefined;
      subscribe('worktree-update', filters).catch(console.error);
    }
  }, [isConnected(), subscribe, worktreeIds]);

  const requestWorktreeUpdate = useCallback(async (worktreeId: string) => {
    try {
      await sendMessage('request-worktree-update', { worktreeId });
    } catch (error) {
      console.error('Failed to request worktree update:', error);
      throw error;
    }
  }, [sendMessage]);

  return {
    requestWorktreeUpdate,
    isConnected: isConnected(),
  };
}