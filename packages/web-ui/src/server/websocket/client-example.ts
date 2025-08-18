/**
 * Example WebSocket Client Implementation
 * 
 * This file provides example code for connecting to the CrewChief WebSocket server
 * from a web client. It demonstrates:
 * - Authentication
 * - Auto-reconnection
 * - Room management
 * - Event subscription
 * - Heartbeat/ping-pong
 */

import { io, Socket } from 'socket.io-client';

export interface ClientConfig {
  url: string;
  token?: string;
  autoReconnect: boolean;
  reconnectionAttempts: number;
  reconnectionDelay: number;
  heartbeatInterval: number;
}

export class CrewChiefWebSocketClient {
  private socket: Socket | null = null;
  private config: ClientConfig;
  private heartbeatInterval: NodeJS.Timeout | null = null;
  private reconnectAttempts = 0;
  private isConnected = false;
  private subscriptions = new Set<string>();
  private joinedRooms = new Set<string>();

  constructor(config: Partial<ClientConfig> = {}) {
    this.config = {
      url: 'http://localhost:3456',
      autoReconnect: true,
      reconnectionAttempts: 5,
      reconnectionDelay: 1000,
      heartbeatInterval: 30000, // 30 seconds
      ...config,
    };
  }

  public async connect(): Promise<void> {
    if (this.socket?.connected) {
      console.log('Already connected');
      return;
    }

    console.log('Connecting to WebSocket server...');

    this.socket = io(this.config.url, {
      auth: {
        token: this.config.token,
      },
      transports: ['websocket', 'polling'],
      timeout: 10000,
      forceNew: true,
    });

    return new Promise((resolve, reject) => {
      if (!this.socket) {
        reject(new Error('Failed to create socket'));
        return;
      }

      // Connection event handlers
      this.socket.on('connect', () => {
        console.log('✅ Connected to WebSocket server');
        this.isConnected = true;
        this.reconnectAttempts = 0;
        this.startHeartbeat();
        
        // Rejoin rooms and resubscribe to events
        this.rejoinRoomsAndSubscriptions();
        
        resolve();
      });

      this.socket.on('connect_error', (error) => {
        console.error('❌ Connection error:', error.message);
        this.isConnected = false;
        
        if (this.config.autoReconnect && this.reconnectAttempts < this.config.reconnectionAttempts) {
          this.reconnectAttempts++;
          console.log(`🔄 Reconnecting... (${this.reconnectAttempts}/${this.config.reconnectionAttempts})`);
          
          setTimeout(() => {
            this.connect().catch(() => {
              // Will be handled by the next retry or final rejection
            });
          }, this.config.reconnectionDelay * this.reconnectAttempts);
        } else {
          reject(error);
        }
      });

      this.socket.on('disconnect', (reason) => {
        console.log('🔌 Disconnected:', reason);
        this.isConnected = false;
        this.stopHeartbeat();
        
        if (this.config.autoReconnect && reason !== 'io client disconnect') {
          this.connect().catch(console.error);
        }
      });

      // Server event handlers
      this.setupEventHandlers();
    });
  }

  private setupEventHandlers(): void {
    if (!this.socket) return;

    // Connection acknowledgment
    this.socket.on('connected', (data) => {
      console.log('🎉 Connection acknowledged:', data);
    });

    // Heartbeat/ping-pong
    this.socket.on('heartbeat', (data) => {
      // Server heartbeat received, no action needed
    });

    this.socket.on('pong', (data) => {
      console.log('🏓 Pong received:', data.timestamp);
    });

    // Room events
    this.socket.on('room-joined', (data) => {
      console.log(`📺 Joined room: ${data.room} (${data.memberCount} members)`);
    });

    this.socket.on('room-left', (data) => {
      console.log(`📺 Left room: ${data.room} (${data.memberCount} members)`);
    });

    // Message events
    this.socket.on('broadcast-message', (data) => {
      console.log('📢 Broadcast message:', data);
    });

    this.socket.on('room-message', (data) => {
      console.log(`📺 Room message in ${data.room}:`, data.message);
    });

    this.socket.on('private-message', (data) => {
      console.log('💬 Private message:', data);
    });

    // Subscription confirmations
    this.socket.on('subscription-confirmed', (data) => {
      console.log(`📡 Subscribed to ${data.type}:`, data.filters);
    });

    this.socket.on('unsubscription-confirmed', (data) => {
      console.log(`📡 Unsubscribed from ${data.type}`);
    });

    // Real-time updates
    this.socket.on('worktree-update', (data) => {
      console.log('🌳 Worktree update:', data);
    });

    this.socket.on('agent-status-change', (data) => {
      console.log('🤖 Agent status change:', data);
    });

    this.socket.on('run-progress', (data) => {
      console.log('🏃 Run progress:', data);
    });

    this.socket.on('maproom-indexing-status', (data) => {
      console.log('🗺️ Maproom indexing status:', data);
    });

    this.socket.on('config-change', (data) => {
      console.log('⚙️ Config change:', data);
    });

    this.socket.on('system-update', (data) => {
      console.log('🖥️ System update:', data);
    });

    this.socket.on('global-update', (data) => {
      console.log('🌐 Global update:', data);
    });

    // Error handling
    this.socket.on('error', (error) => {
      console.error('❌ Socket error:', error);
    });

    // Server shutdown
    this.socket.on('server-shutdown', (data) => {
      console.warn('🛑 Server shutting down:', data.message);
    });
  }

  private startHeartbeat(): void {
    this.heartbeatInterval = setInterval(() => {
      if (this.socket?.connected) {
        this.socket.emit('ping');
      }
    }, this.config.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  private async rejoinRoomsAndSubscriptions(): Promise<void> {
    // Rejoin rooms
    for (const room of this.joinedRooms) {
      await this.joinRoom(room);
    }

    // Resubscribe to events
    for (const subscription of this.subscriptions) {
      const [type, filtersJson] = subscription.split('|');
      const filters = filtersJson ? JSON.parse(filtersJson) : {};
      await this.subscribe(type, filters);
    }
  }

  // Public API methods

  public async joinRoom(room: string, type = 'general'): Promise<void> {
    if (!this.socket?.connected) {
      throw new Error('Not connected');
    }

    return new Promise((resolve, reject) => {
      this.socket!.emit('join-room', { room, type });
      
      const timeout = setTimeout(() => {
        reject(new Error('Join room timeout'));
      }, 5000);

      this.socket!.once('room-joined', (data) => {
        clearTimeout(timeout);
        if (data.room === room) {
          this.joinedRooms.add(room);
          resolve();
        }
      });

      this.socket!.once('error', (error) => {
        clearTimeout(timeout);
        reject(error);
      });
    });
  }

  public async leaveRoom(room: string): Promise<void> {
    if (!this.socket?.connected) {
      throw new Error('Not connected');
    }

    return new Promise((resolve, reject) => {
      this.socket!.emit('leave-room', { room });
      
      const timeout = setTimeout(() => {
        reject(new Error('Leave room timeout'));
      }, 5000);

      this.socket!.once('room-left', (data) => {
        clearTimeout(timeout);
        if (data.room === room) {
          this.joinedRooms.delete(room);
          resolve();
        }
      });

      this.socket!.once('error', (error) => {
        clearTimeout(timeout);
        reject(error);
      });
    });
  }

  public async subscribe(type: string, filters: any = {}): Promise<void> {
    if (!this.socket?.connected) {
      throw new Error('Not connected');
    }

    return new Promise((resolve, reject) => {
      this.socket!.emit('subscribe', { type, filters });
      
      const timeout = setTimeout(() => {
        reject(new Error('Subscribe timeout'));
      }, 5000);

      this.socket!.once('subscription-confirmed', (data) => {
        clearTimeout(timeout);
        if (data.type === type) {
          this.subscriptions.add(`${type}|${JSON.stringify(filters)}`);
          resolve();
        }
      });

      this.socket!.once('error', (error) => {
        clearTimeout(timeout);
        reject(error);
      });
    });
  }

  public async unsubscribe(type: string): Promise<void> {
    if (!this.socket?.connected) {
      throw new Error('Not connected');
    }

    return new Promise((resolve, reject) => {
      this.socket!.emit('unsubscribe', { type });
      
      const timeout = setTimeout(() => {
        reject(new Error('Unsubscribe timeout'));
      }, 5000);

      this.socket!.once('unsubscription-confirmed', (data) => {
        clearTimeout(timeout);
        if (data.type === type) {
          // Remove all subscriptions with this type
          for (const sub of this.subscriptions) {
            if (sub.startsWith(`${type}|`)) {
              this.subscriptions.delete(sub);
            }
          }
          resolve();
        }
      });

      this.socket!.once('error', (error) => {
        clearTimeout(timeout);
        reject(error);
      });
    });
  }

  public sendMessage(type: string, data: any): void {
    if (!this.socket?.connected) {
      throw new Error('Not connected');
    }

    this.socket.emit('message', { type, ...data });
  }

  public sendRoomMessage(room: string, message: string): void {
    this.sendMessage('room-message', { room, message });
  }

  public sendPrivateMessage(targetUserId: string, message: string): void {
    this.sendMessage('private-message', { targetUserId, message });
  }

  public sendBroadcast(payload: any): void {
    this.sendMessage('broadcast', { payload });
  }

  public updateStatus(status: any): void {
    this.sendMessage('status-update', { status });
  }

  public disconnect(): void {
    console.log('Disconnecting from WebSocket server...');
    
    this.stopHeartbeat();
    
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
    }
    
    this.isConnected = false;
    this.joinedRooms.clear();
    this.subscriptions.clear();
  }

  public get connected(): boolean {
    return this.isConnected && this.socket?.connected === true;
  }

  public getStats(): {
    connected: boolean;
    reconnectAttempts: number;
    joinedRooms: number;
    subscriptions: number;
  } {
    return {
      connected: this.connected,
      reconnectAttempts: this.reconnectAttempts,
      joinedRooms: this.joinedRooms.size,
      subscriptions: this.subscriptions.size,
    };
  }
}

// Example usage:
/*
const client = new CrewChiefWebSocketClient({
  url: 'http://localhost:3456',
  token: 'your-jwt-token',
});

// Connect
await client.connect();

// Join worktree room
await client.joinRoom('worktree:my-project');

// Subscribe to agent updates
await client.subscribe('agent-updates', { agentId: 'agent-123' });

// Send messages
client.sendRoomMessage('worktree:my-project', 'Hello from client!');

// Clean up
client.disconnect();
*/