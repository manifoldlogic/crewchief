/**
 * WebSocket Client
 * 
 * Core WebSocket client implementation with automatic reconnection,
 * message queuing, authentication, and proper cleanup.
 */

import { io, Socket } from 'socket.io-client';
import type {
  WebSocketConfig,
  ConnectionState,
  AuthToken,
  QueuedMessage,
  ConnectionInfo,
  WebSocketError,
  WebSocketEventHandler,
  WebSocketEventType,
  AuthenticatedSocket,
} from './types.js';

export class WebSocketClient {
  private socket: AuthenticatedSocket | null = null;
  private config: Required<WebSocketConfig>;
  private connectionState: ConnectionState = 'disconnected';
  private authToken: AuthToken | null = null;
  private messageQueue: QueuedMessage[] = [];
  private reconnectAttempts = 0;
  private reconnectTimeoutId: NodeJS.Timeout | null = null;
  private heartbeatIntervalId: NodeJS.Timeout | null = null;
  private isReconnecting = false;
  private connectionInfo: ConnectionInfo | null = null;
  private eventHandlers = new Map<string, Set<WebSocketEventHandler>>();
  private joinedRooms = new Set<string>();
  private activeSubscriptions = new Set<string>();
  
  // Metrics
  private totalMessagesSent = 0;
  private totalMessagesReceived = 0;
  private lastHeartbeat: Date | null = null;

  constructor(config: WebSocketConfig = {}) {
    this.config = {
      url: config.url || this.getDefaultUrl(),
      autoConnect: config.autoConnect ?? true,
      maxReconnectAttempts: config.maxReconnectAttempts ?? 10,
      reconnectBaseDelay: config.reconnectBaseDelay ?? 1000,
      maxReconnectDelay: config.maxReconnectDelay ?? 30000,
      heartbeatInterval: config.heartbeatInterval ?? 30000,
      messageQueueMaxSize: config.messageQueueMaxSize ?? 100,
      connectionTimeout: config.connectionTimeout ?? 10000,
    };
  }

  private getDefaultUrl(): string {
    if (typeof window !== 'undefined') {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      return `${protocol}//${host}`;
    }
    return 'ws://localhost:3457';
  }

  // Connection Management
  public async connect(): Promise<void> {
    if (this.connectionState === 'connected' || this.connectionState === 'connecting') {
      return;
    }

    this.setConnectionState('connecting');
    
    try {
      await this.createConnection();
      this.setConnectionState('connected');
      this.reconnectAttempts = 0;
      this.startHeartbeat();
      this.processMessageQueue();
      this.rejoinRoomsAndSubscriptions();
    } catch (error) {
      this.setConnectionState('error');
      throw error;
    }
  }

  public disconnect(): void {
    this.isReconnecting = false;
    this.clearReconnectTimeout();
    this.stopHeartbeat();
    
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
    }
    
    this.setConnectionState('disconnected');
    this.connectionInfo = null;
    this.joinedRooms.clear();
    this.activeSubscriptions.clear();
  }

  public async reconnect(): Promise<void> {
    this.disconnect();
    await this.connect();
  }

  private createConnection(): Promise<void> {
    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        reject(new Error('Connection timeout'));
      }, this.config.connectionTimeout);

      const auth: any = {};
      if (this.authToken?.accessToken) {
        auth.token = this.authToken.accessToken;
      }

      this.socket = io(this.config.url, {
        auth,
        transports: ['websocket', 'polling'],
        timeout: this.config.connectionTimeout,
        forceNew: true,
        autoConnect: false,
      }) as AuthenticatedSocket;

      this.setupSocketEventHandlers(resolve, reject, timeoutId);
      this.socket.connect();
    });
  }

  private setupSocketEventHandlers(
    resolve: () => void,
    reject: (error: Error) => void,
    timeoutId: NodeJS.Timeout
  ): void {
    if (!this.socket) return;

    // Connection events
    this.socket.on('connect', () => {
      clearTimeout(timeoutId);
      this.emit('connect');
      resolve();
    });

    this.socket.on('connect_error', (error: Error) => {
      clearTimeout(timeoutId);
      this.handleConnectionError(error);
      reject(error);
    });

    this.socket.on('disconnect', (reason: string) => {
      this.handleDisconnection(reason);
    });

    // Server events
    this.socket.on('connected', (data: ConnectionInfo) => {
      this.connectionInfo = {
        ...data,
        connectedAt: new Date(),
      };
      this.emit('connectionInfo', this.connectionInfo);
    });

    this.socket.on('heartbeat', () => {
      this.lastHeartbeat = new Date();
    });

    this.socket.on('pong', () => {
      this.lastHeartbeat = new Date();
    });

    // Room events
    this.socket.on('room-joined', (data: { room: string; memberCount: number }) => {
      this.joinedRooms.add(data.room);
      this.emit('roomJoined', data);
    });

    this.socket.on('room-left', (data: { room: string; memberCount: number }) => {
      this.joinedRooms.delete(data.room);
      this.emit('roomLeft', data);
    });

    // Subscription events
    this.socket.on('subscription-confirmed', (data: { type: string; filters?: any }) => {
      this.activeSubscriptions.add(data.type);
      this.emit('subscriptionConfirmed', data);
    });

    this.socket.on('unsubscription-confirmed', (data: { type: string }) => {
      this.activeSubscriptions.delete(data.type);
      this.emit('unsubscriptionConfirmed', data);
    });

    // Real-time events
    this.setupRealTimeEventHandlers();

    // Error handling
    this.socket.on('error', (error: any) => {
      this.handleError({
        code: 'SOCKET_ERROR',
        message: error.message || 'Socket error occurred',
        timestamp: new Date(),
        context: { error },
      });
    });

    this.socket.on('server-shutdown', (data: { message: string; timestamp: string }) => {
      this.emit('serverShutdown', data);
    });
  }

  private setupRealTimeEventHandlers(): void {
    if (!this.socket) return;

    const eventTypes: WebSocketEventType[] = [
      'worktree-update',
      'agent-status-change',
      'run-progress',
      'maproom-indexing-status',
      'config-change',
      'system-update',
      'dashboard-stats-update',
      'activity-event',
      'performance-metrics',
      'agent-status-update',
      'global-update',
    ];

    eventTypes.forEach((eventType) => {
      this.socket!.on(eventType, (data: any) => {
        this.totalMessagesReceived++;
        this.emit(eventType, data);
      });
    });
  }

  // Authentication
  public setAuthToken(token: AuthToken | null): void {
    this.authToken = token;
    
    if (this.socket && token?.accessToken) {
      this.socket.auth = { token: token.accessToken };
      
      // If connected, we might want to re-authenticate
      if (this.connectionState === 'connected') {
        this.socket.emit('authenticate', { token: token.accessToken });
      }
    }
  }

  public async refreshAuth(): Promise<void> {
    // This would typically call an auth service to refresh tokens
    // For now, we'll emit an event that the auth context can handle
    this.emit('authRefreshRequested');
  }

  // Messaging
  public async sendMessage(event: string, data: any, room?: string): Promise<void> {
    const message: QueuedMessage = {
      id: this.generateMessageId(),
      timestamp: new Date().toISOString(),
      type: 'outgoing',
      event,
      data,
      retry: 0,
      maxRetries: 3,
    };

    if (room) {
      (message as any).room = room;
    }

    if (this.connectionState === 'connected' && this.socket) {
      try {
        await this.sendMessageDirect(message);
        this.totalMessagesSent++;
      } catch (error) {
        this.queueMessage(message);
        throw error;
      }
    } else {
      this.queueMessage(message);
    }
  }

  public sendMessageSync(event: string, data: any, room?: string): void {
    if (this.connectionState === 'connected' && this.socket) {
      const messageData = room ? { ...data, room } : data;
      this.socket.emit(event, messageData);
      this.totalMessagesSent++;
    } else {
      const message: QueuedMessage = {
        id: this.generateMessageId(),
        timestamp: new Date().toISOString(),
        type: 'outgoing',
        event,
        data,
        retry: 0,
        maxRetries: 3,
      };
      
      if (room) {
        (message as any).room = room;
      }
      
      this.queueMessage(message);
    }
  }

  private async sendMessageDirect(message: QueuedMessage): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!this.socket) {
        reject(new Error('Not connected'));
        return;
      }

      const timeout = setTimeout(() => {
        reject(new Error('Message timeout'));
      }, 5000);

      const messageData = (message as any).room 
        ? { ...message.data, room: (message as any).room }
        : message.data;

      this.socket.emit(message.event, messageData, (response?: any) => {
        clearTimeout(timeout);
        if (response?.error) {
          reject(new Error(response.error));
        } else {
          resolve();
        }
      });
    });
  }

  private queueMessage(message: QueuedMessage): void {
    if (this.messageQueue.length >= this.config.messageQueueMaxSize) {
      // Remove oldest message
      this.messageQueue.shift();
    }
    
    this.messageQueue.push(message);
    this.emit('messageQueued', { queueSize: this.messageQueue.length });
  }

  private async processMessageQueue(): Promise<void> {
    if (this.connectionState !== 'connected' || !this.socket) {
      return;
    }

    const messagesToProcess = [...this.messageQueue];
    this.messageQueue = [];

    for (const message of messagesToProcess) {
      try {
        await this.sendMessageDirect(message);
        this.totalMessagesSent++;
      } catch (error) {
        message.retry = (message.retry || 0) + 1;
        
        if (message.retry < (message.maxRetries || 3)) {
          this.queueMessage(message);
        } else {
          this.emit('messageDropped', { message, error });
        }
      }
    }
  }

  // Room Management
  public async joinRoom(room: string, type = 'general'): Promise<void> {
    if (!this.socket) {
      throw new Error('Not connected');
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Join room timeout'));
      }, 5000);

      this.socket!.emit('join-room', { room, type });
      
      const onRoomJoined = (data: { room: string }) => {
        if (data.room === room) {
          clearTimeout(timeout);
          this.socket!.off('room-joined', onRoomJoined);
          this.socket!.off('error', onError);
          resolve();
        }
      };

      const onError = (error: any) => {
        clearTimeout(timeout);
        this.socket!.off('room-joined', onRoomJoined);
        this.socket!.off('error', onError);
        reject(error);
      };

      this.socket!.on('room-joined', onRoomJoined);
      this.socket!.on('error', onError);
    });
  }

  public async leaveRoom(room: string): Promise<void> {
    if (!this.socket) {
      throw new Error('Not connected');
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Leave room timeout'));
      }, 5000);

      this.socket!.emit('leave-room', { room });
      
      const onRoomLeft = (data: { room: string }) => {
        if (data.room === room) {
          clearTimeout(timeout);
          this.socket!.off('room-left', onRoomLeft);
          this.socket!.off('error', onError);
          resolve();
        }
      };

      const onError = (error: any) => {
        clearTimeout(timeout);
        this.socket!.off('room-left', onRoomLeft);
        this.socket!.off('error', onError);
        reject(error);
      };

      this.socket!.on('room-left', onRoomLeft);
      this.socket!.on('error', onError);
    });
  }

  // Subscription Management
  public async subscribe(type: WebSocketEventType, filters?: Record<string, any>): Promise<void> {
    if (!this.socket) {
      throw new Error('Not connected');
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Subscribe timeout'));
      }, 5000);

      this.socket!.emit('subscribe', { type, filters });
      
      const onSubscriptionConfirmed = (data: { type: string }) => {
        if (data.type === type) {
          clearTimeout(timeout);
          this.socket!.off('subscription-confirmed', onSubscriptionConfirmed);
          this.socket!.off('error', onError);
          resolve();
        }
      };

      const onError = (error: any) => {
        clearTimeout(timeout);
        this.socket!.off('subscription-confirmed', onSubscriptionConfirmed);
        this.socket!.off('error', onError);
        reject(error);
      };

      this.socket!.on('subscription-confirmed', onSubscriptionConfirmed);
      this.socket!.on('error', onError);
    });
  }

  public async unsubscribe(type: WebSocketEventType): Promise<void> {
    if (!this.socket) {
      throw new Error('Not connected');
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Unsubscribe timeout'));
      }, 5000);

      this.socket!.emit('unsubscribe', { type });
      
      const onUnsubscriptionConfirmed = (data: { type: string }) => {
        if (data.type === type) {
          clearTimeout(timeout);
          this.socket!.off('unsubscription-confirmed', onUnsubscriptionConfirmed);
          this.socket!.off('error', onError);
          resolve();
        }
      };

      const onError = (error: any) => {
        clearTimeout(timeout);
        this.socket!.off('unsubscription-confirmed', onUnsubscriptionConfirmed);
        this.socket!.off('error', onError);
        reject(error);
      };

      this.socket!.on('unsubscription-confirmed', onUnsubscriptionConfirmed);
      this.socket!.on('error', onError);
    });
  }

  // Event Management
  public on(event: string, handler: WebSocketEventHandler): void {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, new Set());
    }
    this.eventHandlers.get(event)!.add(handler);
  }

  public off(event: string, handler: WebSocketEventHandler): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      handlers.delete(handler);
      if (handlers.size === 0) {
        this.eventHandlers.delete(event);
      }
    }
  }

  private emit(event: string, data?: any): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      handlers.forEach(handler => {
        try {
          handler(data);
        } catch (error) {
          console.error(`Error in WebSocket event handler for ${event}:`, error);
        }
      });
    }
  }

  // Reconnection Logic
  private handleConnectionError(error: Error): void {
    this.setConnectionState('error');
    this.handleError({
      code: 'CONNECTION_ERROR',
      message: error.message,
      timestamp: new Date(),
      context: { error },
    });
    
    if (!this.isReconnecting) {
      this.scheduleReconnect();
    }
  }

  private handleDisconnection(reason: string): void {
    this.setConnectionState('disconnected');
    this.stopHeartbeat();
    this.connectionInfo = null;
    this.emit('disconnect', reason);

    if (reason !== 'io client disconnect' && !this.isReconnecting) {
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      this.handleError({
        code: 'MAX_RECONNECT_ATTEMPTS',
        message: `Failed to reconnect after ${this.config.maxReconnectAttempts} attempts`,
        timestamp: new Date(),
      });
      return;
    }

    this.isReconnecting = true;
    this.setConnectionState('reconnecting');
    
    const delay = Math.min(
      this.config.reconnectBaseDelay * Math.pow(2, this.reconnectAttempts),
      this.config.maxReconnectDelay
    );

    this.reconnectTimeoutId = setTimeout(async () => {
      this.reconnectAttempts++;
      this.emit('reconnectAttempt', this.reconnectAttempts);
      
      try {
        await this.connect();
        this.isReconnecting = false;
      } catch (error) {
        this.scheduleReconnect();
      }
    }, delay);
  }

  private clearReconnectTimeout(): void {
    if (this.reconnectTimeoutId) {
      clearTimeout(this.reconnectTimeoutId);
      this.reconnectTimeoutId = null;
    }
  }

  // Heartbeat
  private startHeartbeat(): void {
    this.heartbeatIntervalId = setInterval(() => {
      if (this.socket?.connected) {
        this.socket.emit('ping');
      }
    }, this.config.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatIntervalId) {
      clearInterval(this.heartbeatIntervalId);
      this.heartbeatIntervalId = null;
    }
  }

  // State Management
  private setConnectionState(state: ConnectionState): void {
    if (this.connectionState !== state) {
      this.connectionState = state;
      this.emit('connectionStateChange', state);
    }
  }

  private handleError(error: WebSocketError): void {
    this.emit('error', error);
  }

  // Utilities
  private generateMessageId(): string {
    return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  private async rejoinRoomsAndSubscriptions(): Promise<void> {
    // Rejoin rooms
    const roomsToRejoin = Array.from(this.joinedRooms);
    this.joinedRooms.clear();
    
    for (const room of roomsToRejoin) {
      try {
        await this.joinRoom(room);
      } catch (error) {
        console.warn(`Failed to rejoin room ${room}:`, error);
      }
    }

    // Resubscribe to events
    const subscriptionsToRestore = Array.from(this.activeSubscriptions);
    this.activeSubscriptions.clear();
    
    for (const subscription of subscriptionsToRestore) {
      try {
        await this.subscribe(subscription as WebSocketEventType);
      } catch (error) {
        console.warn(`Failed to resubscribe to ${subscription}:`, error);
      }
    }
  }

  // Public Getters
  public get state(): ConnectionState {
    return this.connectionState;
  }

  public get isConnected(): boolean {
    return this.connectionState === 'connected' && this.socket?.connected === true;
  }

  public get isAuthenticated(): boolean {
    return this.connectionInfo?.authenticated === true;
  }

  public get queueSize(): number {
    return this.messageQueue.length;
  }

  public get stats() {
    return {
      connectionState: this.connectionState,
      isConnected: this.isConnected,
      isAuthenticated: this.isAuthenticated,
      reconnectAttempts: this.reconnectAttempts,
      queueSize: this.queueSize,
      joinedRooms: this.joinedRooms.size,
      subscriptions: this.activeSubscriptions.size,
      totalMessagesSent: this.totalMessagesSent,
      totalMessagesReceived: this.totalMessagesReceived,
      lastHeartbeat: this.lastHeartbeat,
      connectionInfo: this.connectionInfo,
    };
  }

  // Cleanup
  public clearMessageQueue(): void {
    this.messageQueue = [];
    this.emit('queueCleared');
  }

  public async retryFailedMessages(): Promise<void> {
    await this.processMessageQueue();
  }
}