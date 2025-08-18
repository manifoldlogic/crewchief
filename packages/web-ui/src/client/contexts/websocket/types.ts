/**
 * WebSocket Client Types
 * 
 * TypeScript types for the CrewChief WebSocket client integration.
 * Includes event types, message types, and connection state management.
 */

import { type Socket } from 'socket.io-client';

// Connection States
export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'error' | 'reconnecting';

// Authentication Types
export interface AuthToken {
  accessToken: string;
  refreshToken?: string;
  expiresAt?: Date;
}

// Connection Configuration
export interface WebSocketConfig {
  url?: string;
  autoConnect?: boolean;
  maxReconnectAttempts?: number;
  reconnectBaseDelay?: number;
  maxReconnectDelay?: number;
  heartbeatInterval?: number;
  messageQueueMaxSize?: number;
  connectionTimeout?: number;
}

// Connection Info
export interface ConnectionInfo {
  id?: string;
  authenticated: boolean;
  user?: {
    id: string;
    name: string;
    email?: string;
  };
  serverTime?: string;
  connectedAt?: Date;
}

// Event Types
export type WebSocketEventType = 
  | 'worktree-update'
  | 'agent-status-change' 
  | 'run-progress'
  | 'maproom-indexing-status'
  | 'config-change'
  | 'system-update'
  | 'dashboard-stats-update'
  | 'activity-event'
  | 'performance-metrics'
  | 'agent-status-update'
  | 'global-update';

// Message Types
export interface BaseMessage {
  id: string;
  timestamp: string;
  type: string;
}

export interface QueuedMessage extends BaseMessage {
  event: string;
  data: any;
  retry?: number;
  maxRetries?: number;
}

export interface IncomingMessage extends BaseMessage {
  event: WebSocketEventType;
  data: any;
  room?: string;
}

export interface OutgoingMessage extends BaseMessage {
  event: string;
  data: any;
  room?: string;
}

// Specific Event Data Types
export interface WorktreeUpdate {
  id: string;
  name: string;
  status: 'active' | 'inactive' | 'merging' | 'error';
  lastModified: string;
  agentId?: string;
  changes?: Array<{
    type: 'added' | 'modified' | 'deleted';
    path: string;
  }>;
}

export interface AgentStatusChange {
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

export interface RunProgress {
  id: string;
  agentId: string;
  status: 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';
  progress: number;
  startTime: string;
  endTime?: string;
  currentStep?: string;
  totalSteps?: number;
  error?: string;
}

export interface MaproomIndexingStatus {
  id: string;
  status: 'idle' | 'scanning' | 'indexing' | 'completed' | 'error';
  progress: number;
  filesProcessed: number;
  totalFiles: number;
  currentFile?: string;
  error?: string;
}

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
  type: WebSocketEventType;
  timestamp: string;
  title: string;
  description: string;
  severity: 'info' | 'success' | 'warning' | 'error';
  metadata?: Record<string, any>;
}

export interface PerformanceMetrics {
  apiResponseTime: number;
  websocketConnected: boolean;
  dbQueryTime: number;
  timestamp: string;
}

// Room Management
export interface RoomInfo {
  name: string;
  type: string;
  memberCount: number;
  joinedAt: Date;
}

// Subscription Management
export interface Subscription {
  type: WebSocketEventType;
  filters?: Record<string, any>;
  active: boolean;
  subscribedAt: Date;
}

// Error Types
export interface WebSocketError {
  code: string;
  message: string;
  timestamp: Date;
  context?: Record<string, any>;
}

// Context State
export interface WebSocketContextState {
  // Connection
  connectionState: ConnectionState;
  connectionInfo: ConnectionInfo | null;
  error: WebSocketError | null;
  
  // Authentication
  isAuthenticated: boolean;
  authToken: AuthToken | null;
  
  // Reconnection
  reconnectAttempts: number;
  nextReconnectDelay: number;
  
  // Messaging
  messageQueue: QueuedMessage[];
  queueSize: number;
  
  // Rooms & Subscriptions
  joinedRooms: Map<string, RoomInfo>;
  activeSubscriptions: Map<string, Subscription>;
  
  // Real-time Data
  dashboardStats: DashboardStats | null;
  activities: ActivityEvent[];
  agents: AgentStatusChange[];
  performance: PerformanceMetrics | null;
  
  // Metrics
  totalMessagesSent: number;
  totalMessagesReceived: number;
  lastHeartbeat: Date | null;
}

// Context Actions
export interface WebSocketContextActions {
  // Connection Management
  connect: () => Promise<void>;
  disconnect: () => void;
  reconnect: () => Promise<void>;
  
  // Authentication
  setAuthToken: (token: AuthToken | null) => void;
  refreshAuth: () => Promise<void>;
  
  // Messaging
  sendMessage: (event: string, data: any, room?: string) => Promise<void>;
  sendMessageSync: (event: string, data: any, room?: string) => void;
  
  // Room Management
  joinRoom: (room: string, type?: string) => Promise<void>;
  leaveRoom: (room: string) => Promise<void>;
  
  // Subscription Management
  subscribe: (type: WebSocketEventType, filters?: Record<string, any>) => Promise<void>;
  unsubscribe: (type: WebSocketEventType) => Promise<void>;
  
  // Queue Management
  clearMessageQueue: () => void;
  retryFailedMessages: () => Promise<void>;
  
  // Data Management
  clearActivities: () => void;
  refreshDashboardStats: () => void;
  
  // Utilities
  isConnected: () => boolean;
  getConnectionStats: () => {
    connected: boolean;
    reconnectAttempts: number;
    queueSize: number;
    joinedRooms: number;
    subscriptions: number;
  };
}

// Context Value
export interface WebSocketContextValue extends WebSocketContextState, WebSocketContextActions {}

// Hook Options
export interface UseWebSocketOptions {
  autoConnect?: boolean;
  requireAuth?: boolean;
  subscriptions?: WebSocketEventType[];
  rooms?: string[];
}

// Event Handlers
export type WebSocketEventHandler<T = any> = (data: T) => void;

export interface WebSocketEventHandlers {
  onConnect?: () => void;
  onDisconnect?: (reason: string) => void;
  onError?: (error: WebSocketError) => void;
  onReconnect?: (attemptNumber: number) => void;
  onMessage?: (message: IncomingMessage) => void;
  onWorktreeUpdate?: WebSocketEventHandler<WorktreeUpdate>;
  onAgentStatusChange?: WebSocketEventHandler<AgentStatusChange>;
  onRunProgress?: WebSocketEventHandler<RunProgress>;
  onMaproomIndexingStatus?: WebSocketEventHandler<MaproomIndexingStatus>;
  onDashboardStatsUpdate?: WebSocketEventHandler<DashboardStats>;
  onActivityEvent?: WebSocketEventHandler<ActivityEvent>;
  onPerformanceMetrics?: WebSocketEventHandler<PerformanceMetrics>;
}

// Socket.IO specific types
export interface AuthenticatedSocket extends Socket {
  userId?: string;
  userName?: string;
  isAuthenticated: boolean;
}