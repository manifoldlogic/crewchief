/**
 * WebSocket Context Module
 * 
 * Exports all WebSocket client functionality including:
 * - WebSocket context and provider
 * - React hooks for WebSocket operations
 * - TypeScript types
 * - Core client implementation
 */

// Core exports
export { WebSocketClient } from './client.js';
export { WebSocketProvider, useWebSocketContext } from './context.js';

// Hook exports
export {
  useWebSocket,
  useWebSocketConnection,
  useWebSocketMessaging,
  useWebSocketRooms,
  useWebSocketSubscriptions,
  useWebSocketDashboard,
  useWebSocketEvent,
  useWebSocketAuth,
  useWebSocketStats,
  useWebSocketAgents,
  useWebSocketWorktrees,
} from './hooks.js';

// Type exports
export type {
  // Configuration
  WebSocketConfig,
  UseWebSocketOptions,
  
  // Connection
  ConnectionState,
  ConnectionInfo,
  AuthToken,
  
  // Context
  WebSocketContextValue,
  WebSocketContextState,
  WebSocketContextActions,
  
  // Events
  WebSocketEventType,
  WebSocketEventHandler,
  WebSocketEventHandlers,
  
  // Messages
  BaseMessage,
  QueuedMessage,
  IncomingMessage,
  OutgoingMessage,
  
  // Data Types
  WorktreeUpdate,
  AgentStatusChange,
  RunProgress,
  MaproomIndexingStatus,
  DashboardStats,
  ActivityEvent,
  PerformanceMetrics,
  
  // Management
  RoomInfo,
  Subscription,
  
  // Errors
  WebSocketError,
  
  // Socket.IO
  AuthenticatedSocket,
} from './types.js';