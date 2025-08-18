/**
 * WebSocket Context
 * 
 * React context for WebSocket client integration with automatic
 * reconnection, authentication, and state management.
 */

import React, { createContext, useContext, useEffect, useReducer, useCallback, useRef } from 'react';
import { WebSocketClient } from './client.js';
import type {
  WebSocketContextValue,
  WebSocketContextState,
  ConnectionState,
  AuthToken,
  QueuedMessage,
  ConnectionInfo,
  WebSocketError,
  DashboardStats,
  ActivityEvent,
  AgentStatusChange,
  PerformanceMetrics,
  RoomInfo,
  Subscription,
  WebSocketEventType,
  WebSocketConfig,
} from './types.js';

// Initial State
const initialState: WebSocketContextState = {
  connectionState: 'disconnected',
  connectionInfo: null,
  error: null,
  isAuthenticated: false,
  authToken: null,
  reconnectAttempts: 0,
  nextReconnectDelay: 1000,
  messageQueue: [],
  queueSize: 0,
  joinedRooms: new Map(),
  activeSubscriptions: new Map(),
  dashboardStats: null,
  activities: [],
  agents: [],
  performance: null,
  totalMessagesSent: 0,
  totalMessagesReceived: 0,
  lastHeartbeat: null,
};

// Action Types
type WebSocketAction =
  | { type: 'SET_CONNECTION_STATE'; payload: ConnectionState }
  | { type: 'SET_CONNECTION_INFO'; payload: ConnectionInfo | null }
  | { type: 'SET_ERROR'; payload: WebSocketError | null }
  | { type: 'SET_AUTH_TOKEN'; payload: AuthToken | null }
  | { type: 'SET_RECONNECT_ATTEMPTS'; payload: number }
  | { type: 'SET_NEXT_RECONNECT_DELAY'; payload: number }
  | { type: 'SET_MESSAGE_QUEUE'; payload: QueuedMessage[] }
  | { type: 'SET_QUEUE_SIZE'; payload: number }
  | { type: 'ADD_ROOM'; payload: { name: string; info: RoomInfo } }
  | { type: 'REMOVE_ROOM'; payload: string }
  | { type: 'ADD_SUBSCRIPTION'; payload: { type: string; subscription: Subscription } }
  | { type: 'REMOVE_SUBSCRIPTION'; payload: string }
  | { type: 'SET_DASHBOARD_STATS'; payload: DashboardStats | null }
  | { type: 'ADD_ACTIVITY'; payload: ActivityEvent }
  | { type: 'CLEAR_ACTIVITIES' }
  | { type: 'SET_ACTIVITIES'; payload: ActivityEvent[] }
  | { type: 'UPDATE_AGENT'; payload: AgentStatusChange }
  | { type: 'SET_AGENTS'; payload: AgentStatusChange[] }
  | { type: 'SET_PERFORMANCE'; payload: PerformanceMetrics | null }
  | { type: 'INCREMENT_SENT_MESSAGES' }
  | { type: 'INCREMENT_RECEIVED_MESSAGES' }
  | { type: 'SET_LAST_HEARTBEAT'; payload: Date | null }
  | { type: 'RESET_STATE' };

// Reducer
function webSocketReducer(state: WebSocketContextState, action: WebSocketAction): WebSocketContextState {
  switch (action.type) {
    case 'SET_CONNECTION_STATE':
      return { 
        ...state, 
        connectionState: action.payload,
        isAuthenticated: action.payload === 'connected' ? state.isAuthenticated : false,
      };
    
    case 'SET_CONNECTION_INFO':
      return { 
        ...state, 
        connectionInfo: action.payload,
        isAuthenticated: action.payload?.authenticated || false,
      };
    
    case 'SET_ERROR':
      return { ...state, error: action.payload };
    
    case 'SET_AUTH_TOKEN':
      return { ...state, authToken: action.payload };
    
    case 'SET_RECONNECT_ATTEMPTS':
      return { ...state, reconnectAttempts: action.payload };
    
    case 'SET_NEXT_RECONNECT_DELAY':
      return { ...state, nextReconnectDelay: action.payload };
    
    case 'SET_MESSAGE_QUEUE':
      return { 
        ...state, 
        messageQueue: action.payload,
        queueSize: action.payload.length,
      };
    
    case 'SET_QUEUE_SIZE':
      return { ...state, queueSize: action.payload };
    
    case 'ADD_ROOM':
      const newRooms = new Map(state.joinedRooms);
      newRooms.set(action.payload.name, action.payload.info);
      return { ...state, joinedRooms: newRooms };
    
    case 'REMOVE_ROOM':
      const updatedRooms = new Map(state.joinedRooms);
      updatedRooms.delete(action.payload);
      return { ...state, joinedRooms: updatedRooms };
    
    case 'ADD_SUBSCRIPTION':
      const newSubscriptions = new Map(state.activeSubscriptions);
      newSubscriptions.set(action.payload.type, action.payload.subscription);
      return { ...state, activeSubscriptions: newSubscriptions };
    
    case 'REMOVE_SUBSCRIPTION':
      const updatedSubscriptions = new Map(state.activeSubscriptions);
      updatedSubscriptions.delete(action.payload);
      return { ...state, activeSubscriptions: updatedSubscriptions };
    
    case 'SET_DASHBOARD_STATS':
      return { ...state, dashboardStats: action.payload };
    
    case 'ADD_ACTIVITY':
      return { 
        ...state, 
        activities: [action.payload, ...state.activities.slice(0, 49)] // Keep last 50
      };
    
    case 'CLEAR_ACTIVITIES':
      return { ...state, activities: [] };
    
    case 'SET_ACTIVITIES':
      return { ...state, activities: action.payload };
    
    case 'UPDATE_AGENT':
      const updatedAgents = state.agents.filter(a => a.id !== action.payload.id);
      return { 
        ...state, 
        agents: [action.payload, ...updatedAgents]
      };
    
    case 'SET_AGENTS':
      return { ...state, agents: action.payload };
    
    case 'SET_PERFORMANCE':
      return { ...state, performance: action.payload };
    
    case 'INCREMENT_SENT_MESSAGES':
      return { ...state, totalMessagesSent: state.totalMessagesSent + 1 };
    
    case 'INCREMENT_RECEIVED_MESSAGES':
      return { ...state, totalMessagesReceived: state.totalMessagesReceived + 1 };
    
    case 'SET_LAST_HEARTBEAT':
      return { ...state, lastHeartbeat: action.payload };
    
    case 'RESET_STATE':
      return { ...initialState };
    
    default:
      return state;
  }
}

// Context
const WebSocketContext = createContext<WebSocketContextValue | null>(null);

// Provider Props
interface WebSocketProviderProps {
  children: React.ReactNode;
  config?: WebSocketConfig;
  authToken?: AuthToken | null;
}

// Provider Component
export function WebSocketProvider({ children, config = {}, authToken }: WebSocketProviderProps) {
  const [state, dispatch] = useReducer(webSocketReducer, initialState);
  const clientRef = useRef<WebSocketClient | null>(null);
  const isInitializedRef = useRef(false);

  // Initialize client
  useEffect(() => {
    if (!isInitializedRef.current) {
      clientRef.current = new WebSocketClient({
        autoConnect: false, // We'll handle connection manually
        ...config,
      });
      
      setupEventHandlers();
      isInitializedRef.current = true;
    }
  }, []);

  // Handle auth token changes
  useEffect(() => {
    if (clientRef.current && authToken !== state.authToken) {
      dispatch({ type: 'SET_AUTH_TOKEN', payload: authToken });
      clientRef.current.setAuthToken(authToken);
    }
  }, [authToken, state.authToken]);

  // Auto-connect if configured
  useEffect(() => {
    if (config.autoConnect !== false && clientRef.current && !state.connectionState || state.connectionState === 'disconnected') {
      connect();
    }
  }, [config.autoConnect]);

  // Setup event handlers
  const setupEventHandlers = useCallback(() => {
    if (!clientRef.current) return;

    const client = clientRef.current;

    // Connection events
    client.on('connectionStateChange', (connectionState: ConnectionState) => {
      dispatch({ type: 'SET_CONNECTION_STATE', payload: connectionState });
    });

    client.on('connectionInfo', (info: ConnectionInfo) => {
      dispatch({ type: 'SET_CONNECTION_INFO', payload: info });
    });

    client.on('error', (error: WebSocketError) => {
      dispatch({ type: 'SET_ERROR', payload: error });
    });

    client.on('disconnect', () => {
      dispatch({ type: 'SET_CONNECTION_INFO', payload: null });
    });

    client.on('reconnectAttempt', (attempts: number) => {
      dispatch({ type: 'SET_RECONNECT_ATTEMPTS', payload: attempts });
    });

    // Room events
    client.on('roomJoined', (data: { room: string; memberCount: number }) => {
      dispatch({ 
        type: 'ADD_ROOM', 
        payload: { 
          name: data.room, 
          info: {
            name: data.room,
            type: 'general',
            memberCount: data.memberCount,
            joinedAt: new Date(),
          }
        }
      });
    });

    client.on('roomLeft', (data: { room: string }) => {
      dispatch({ type: 'REMOVE_ROOM', payload: data.room });
    });

    // Subscription events
    client.on('subscriptionConfirmed', (data: { type: string; filters?: any }) => {
      dispatch({
        type: 'ADD_SUBSCRIPTION',
        payload: {
          type: data.type,
          subscription: {
            type: data.type as WebSocketEventType,
            filters: data.filters,
            active: true,
            subscribedAt: new Date(),
          }
        }
      });
    });

    client.on('unsubscriptionConfirmed', (data: { type: string }) => {
      dispatch({ type: 'REMOVE_SUBSCRIPTION', payload: data.type });
    });

    // Real-time data events
    client.on('dashboard-stats-update', (stats: DashboardStats) => {
      dispatch({ type: 'SET_DASHBOARD_STATS', payload: stats });
    });

    client.on('activity-event', (activity: ActivityEvent) => {
      dispatch({ type: 'ADD_ACTIVITY', payload: activity });
    });

    client.on('agent-status-update', (agent: AgentStatusChange) => {
      dispatch({ type: 'UPDATE_AGENT', payload: agent });
    });

    client.on('agent-status-change', (agent: AgentStatusChange) => {
      dispatch({ type: 'UPDATE_AGENT', payload: agent });
    });

    client.on('performance-metrics', (metrics: PerformanceMetrics) => {
      dispatch({ type: 'SET_PERFORMANCE', payload: metrics });
    });

    // Queue events
    client.on('messageQueued', (data: { queueSize: number }) => {
      dispatch({ type: 'SET_QUEUE_SIZE', payload: data.queueSize });
    });

    client.on('queueCleared', () => {
      dispatch({ type: 'SET_MESSAGE_QUEUE', payload: [] });
    });

    // Heartbeat
    client.on('heartbeat', () => {
      dispatch({ type: 'SET_LAST_HEARTBEAT', payload: new Date() });
    });

    // Auth refresh
    client.on('authRefreshRequested', () => {
      // This would trigger a refresh in the auth context
      // For now, we'll just emit a custom event
      window.dispatchEvent(new CustomEvent('websocket-auth-refresh-requested'));
    });
  }, []);

  // Actions
  const connect = useCallback(async () => {
    if (!clientRef.current) return;
    
    try {
      await clientRef.current.connect();
      dispatch({ type: 'SET_ERROR', payload: null });
    } catch (error) {
      dispatch({ 
        type: 'SET_ERROR', 
        payload: {
          code: 'CONNECTION_FAILED',
          message: error instanceof Error ? error.message : 'Connection failed',
          timestamp: new Date(),
        }
      });
      throw error;
    }
  }, []);

  const disconnect = useCallback(() => {
    if (clientRef.current) {
      clientRef.current.disconnect();
      dispatch({ type: 'RESET_STATE' });
    }
  }, []);

  const reconnect = useCallback(async () => {
    if (!clientRef.current) return;
    
    try {
      await clientRef.current.reconnect();
      dispatch({ type: 'SET_ERROR', payload: null });
      dispatch({ type: 'SET_RECONNECT_ATTEMPTS', payload: 0 });
    } catch (error) {
      dispatch({ 
        type: 'SET_ERROR', 
        payload: {
          code: 'RECONNECTION_FAILED',
          message: error instanceof Error ? error.message : 'Reconnection failed',
          timestamp: new Date(),
        }
      });
      throw error;
    }
  }, []);

  const setAuthToken = useCallback((token: AuthToken | null) => {
    dispatch({ type: 'SET_AUTH_TOKEN', payload: token });
    if (clientRef.current) {
      clientRef.current.setAuthToken(token);
    }
  }, []);

  const refreshAuth = useCallback(async () => {
    if (clientRef.current) {
      await clientRef.current.refreshAuth();
    }
  }, []);

  const sendMessage = useCallback(async (event: string, data: any, room?: string) => {
    if (!clientRef.current) {
      throw new Error('WebSocket client not initialized');
    }
    
    await clientRef.current.sendMessage(event, data, room);
    dispatch({ type: 'INCREMENT_SENT_MESSAGES' });
  }, []);

  const sendMessageSync = useCallback((event: string, data: any, room?: string) => {
    if (!clientRef.current) {
      throw new Error('WebSocket client not initialized');
    }
    
    clientRef.current.sendMessageSync(event, data, room);
    if (clientRef.current.isConnected) {
      dispatch({ type: 'INCREMENT_SENT_MESSAGES' });
    }
  }, []);

  const joinRoom = useCallback(async (room: string, type = 'general') => {
    if (!clientRef.current) {
      throw new Error('WebSocket client not initialized');
    }
    
    await clientRef.current.joinRoom(room, type);
  }, []);

  const leaveRoom = useCallback(async (room: string) => {
    if (!clientRef.current) {
      throw new Error('WebSocket client not initialized');
    }
    
    await clientRef.current.leaveRoom(room);
  }, []);

  const subscribe = useCallback(async (type: WebSocketEventType, filters?: Record<string, any>) => {
    if (!clientRef.current) {
      throw new Error('WebSocket client not initialized');
    }
    
    await clientRef.current.subscribe(type, filters);
  }, []);

  const unsubscribe = useCallback(async (type: WebSocketEventType) => {
    if (!clientRef.current) {
      throw new Error('WebSocket client not initialized');
    }
    
    await clientRef.current.unsubscribe(type);
  }, []);

  const clearMessageQueue = useCallback(() => {
    if (clientRef.current) {
      clientRef.current.clearMessageQueue();
    }
  }, []);

  const retryFailedMessages = useCallback(async () => {
    if (clientRef.current) {
      await clientRef.current.retryFailedMessages();
    }
  }, []);

  const clearActivities = useCallback(() => {
    dispatch({ type: 'CLEAR_ACTIVITIES' });
  }, []);

  const refreshDashboardStats = useCallback(() => {
    if (clientRef.current && clientRef.current.isConnected) {
      clientRef.current.sendMessageSync('request-dashboard-refresh', {});
    }
  }, []);

  const isConnected = useCallback(() => {
    return clientRef.current?.isConnected || false;
  }, []);

  const getConnectionStats = useCallback(() => {
    return clientRef.current?.stats || {
      connected: false,
      reconnectAttempts: 0,
      queueSize: 0,
      joinedRooms: 0,
      subscriptions: 0,
    };
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (clientRef.current) {
        clientRef.current.disconnect();
      }
    };
  }, []);

  // Context value
  const contextValue: WebSocketContextValue = {
    // State
    ...state,
    
    // Actions
    connect,
    disconnect,
    reconnect,
    setAuthToken,
    refreshAuth,
    sendMessage,
    sendMessageSync,
    joinRoom,
    leaveRoom,
    subscribe,
    unsubscribe,
    clearMessageQueue,
    retryFailedMessages,
    clearActivities,
    refreshDashboardStats,
    isConnected,
    getConnectionStats,
  };

  return (
    <WebSocketContext.Provider value={contextValue}>
      {children}
    </WebSocketContext.Provider>
  );
}

// Hook
export function useWebSocketContext(): WebSocketContextValue {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error('useWebSocketContext must be used within a WebSocketProvider');
  }
  return context;
}