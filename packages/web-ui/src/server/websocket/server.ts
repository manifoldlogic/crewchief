import { Server as SocketIOServer } from 'socket.io';
import { Server as HttpServer } from 'http';
import jwt from 'jsonwebtoken';
import type { Pool } from 'pg';
import { EventEmitter } from 'events';
import { ConnectionManager } from './connection-manager.js';
import { MessageHandler } from './message-handler.js';
import { RoomManager } from './room-manager.js';
import { EventHandlers } from './event-handlers.js';

export interface WebSocketConfig {
  cors: {
    origin: string | string[] | boolean;
    credentials: boolean;
  };
  connectionTimeout: number;
  heartbeatInterval: number;
  maxConnections: number;
  maxMessageSize: number;
  rateLimitWindow: number;
  rateLimitMaxRequests: number;
}

export interface AuthenticatedSocket extends SocketIOServer['sockets']['sockets'][0] {
  userId?: string;
  userName?: string;
  isAuthenticated: boolean;
  lastHeartbeat: Date;
  messageCount: number;
  joinedRooms: Set<string>;
}

export class WebSocketServer extends EventEmitter {
  private io: SocketIOServer;
  private connectionManager: ConnectionManager;
  private messageHandler: MessageHandler;
  private roomManager: RoomManager;
  private eventHandlers: EventHandlers;
  private heartbeatInterval: NodeJS.Timeout | null = null;
  private config: WebSocketConfig;
  private db: Pool;

  constructor(httpServer: HttpServer, db: Pool, config: Partial<WebSocketConfig> = {}) {
    super();
    this.db = db;
    
    // Default configuration
    this.config = {
      cors: {
        origin: process.env.NODE_ENV === 'development' 
          ? ['http://localhost:3000', 'http://localhost:3456', 'http://localhost:5173']
          : process.env.ALLOWED_ORIGINS?.split(',') || false,
        credentials: true,
      },
      connectionTimeout: 30000, // 30 seconds
      heartbeatInterval: 15000, // 15 seconds
      maxConnections: 1000,
      maxMessageSize: 1024 * 1024, // 1MB
      rateLimitWindow: 60000, // 1 minute
      rateLimitMaxRequests: 100,
      ...config,
    };

    // Initialize Socket.IO server
    this.io = new SocketIOServer(httpServer, {
      cors: this.config.cors,
      pingTimeout: this.config.connectionTimeout,
      pingInterval: this.config.heartbeatInterval,
      maxHttpBufferSize: this.config.maxMessageSize,
      transports: ['websocket', 'polling'],
      allowEIO3: true,
    });

    // Initialize managers
    this.connectionManager = new ConnectionManager(this.config.maxConnections);
    this.messageHandler = new MessageHandler(this.config);
    this.roomManager = new RoomManager();
    this.eventHandlers = new EventHandlers(this.io, this.db, this.roomManager);

    this.setupMiddleware();
    this.setupEventHandlers();
    this.startHeartbeat();

    console.log('🔌 WebSocket server initialized');
  }

  private setupMiddleware(): void {
    // Authentication middleware
    this.io.use(async (socket: AuthenticatedSocket, next) => {
      try {
        const token = socket.handshake.auth.token || socket.handshake.headers.authorization?.replace('Bearer ', '');
        
        if (!token) {
          // Allow anonymous connections but mark them as unauthenticated
          socket.isAuthenticated = false;
          socket.userId = undefined;
          socket.userName = 'Anonymous';
          socket.lastHeartbeat = new Date();
          socket.messageCount = 0;
          socket.joinedRooms = new Set();
          return next();
        }

        // Verify JWT token
        const jwtSecret = process.env.JWT_SECRET || 'development-secret-key';
        const decoded = jwt.verify(token, jwtSecret) as any;
        
        socket.isAuthenticated = true;
        socket.userId = decoded.userId || decoded.id;
        socket.userName = decoded.username || decoded.name || 'User';
        socket.lastHeartbeat = new Date();
        socket.messageCount = 0;
        socket.joinedRooms = new Set();

        console.log(`✅ WebSocket authenticated: ${socket.userName} (${socket.userId})`);
        next();
      } catch (error) {
        console.error('WebSocket authentication failed:', error);
        // Allow connection but mark as unauthenticated
        socket.isAuthenticated = false;
        socket.userId = undefined;
        socket.userName = 'Anonymous';
        socket.lastHeartbeat = new Date();
        socket.messageCount = 0;
        socket.joinedRooms = new Set();
        next();
      }
    });

    // Rate limiting middleware
    this.io.use((socket: AuthenticatedSocket, next) => {
      const clientIp = socket.handshake.address;
      
      if (!this.messageHandler.checkRateLimit(clientIp)) {
        console.warn(`Rate limit exceeded for IP: ${clientIp}`);
        next(new Error('Rate limit exceeded'));
        return;
      }
      
      next();
    });

    // Connection limit middleware
    this.io.use((socket, next) => {
      if (!this.connectionManager.canAcceptConnection()) {
        console.warn('Maximum connection limit reached');
        next(new Error('Server is at maximum capacity'));
        return;
      }
      
      next();
    });

    // Origin validation middleware
    this.io.use((socket, next) => {
      const origin = socket.handshake.headers.origin;
      
      if (process.env.NODE_ENV === 'production' && origin) {
        const allowedOrigins = this.config.cors.origin;
        if (Array.isArray(allowedOrigins) && !allowedOrigins.includes(origin)) {
          console.warn(`Invalid origin: ${origin}`);
          next(new Error('Invalid origin'));
          return;
        }
      }
      
      next();
    });
  }

  private setupEventHandlers(): void {
    this.io.on('connection', (socket: AuthenticatedSocket) => {
      console.log(`🔌 WebSocket connected: ${socket.id} (${socket.userName})`);
      
      // Add to connection manager
      this.connectionManager.addConnection(socket.id, socket);

      // Send initial connection acknowledgment
      socket.emit('connected', {
        id: socket.id,
        authenticated: socket.isAuthenticated,
        user: socket.isAuthenticated ? {
          id: socket.userId,
          name: socket.userName,
        } : null,
        serverTime: new Date().toISOString(),
      });

      // Handle heartbeat/ping-pong
      socket.on('ping', () => {
        socket.lastHeartbeat = new Date();
        socket.emit('pong', { timestamp: new Date().toISOString() });
      });

      // Handle room joining
      socket.on('join-room', (data: { room: string; type?: string }) => {
        try {
          const roomName = this.roomManager.validateRoomName(data.room);
          if (roomName) {
            socket.join(roomName);
            socket.joinedRooms.add(roomName);
            this.roomManager.addToRoom(roomName, socket.id, data.type || 'general');
            
            socket.emit('room-joined', { 
              room: roomName, 
              memberCount: this.roomManager.getRoomSize(roomName) 
            });
            
            console.log(`📺 Socket ${socket.id} joined room: ${roomName}`);
          } else {
            socket.emit('error', { message: 'Invalid room name' });
          }
        } catch (error) {
          console.error('Error joining room:', error);
          socket.emit('error', { message: 'Failed to join room' });
        }
      });

      // Handle room leaving
      socket.on('leave-room', (data: { room: string }) => {
        try {
          const roomName = data.room;
          socket.leave(roomName);
          socket.joinedRooms.delete(roomName);
          this.roomManager.removeFromRoom(roomName, socket.id);
          
          socket.emit('room-left', { 
            room: roomName, 
            memberCount: this.roomManager.getRoomSize(roomName) 
          });
          
          console.log(`📺 Socket ${socket.id} left room: ${roomName}`);
        } catch (error) {
          console.error('Error leaving room:', error);
          socket.emit('error', { message: 'Failed to leave room' });
        }
      });

      // Handle custom message events
      socket.on('message', (data: any) => {
        if (!this.messageHandler.validateMessage(data, socket)) {
          return;
        }

        // Handle different message types
        this.messageHandler.handleMessage(data, socket, this.io);
      });

      // Handle subscription to data updates
      socket.on('subscribe', (data: { type: string; filters?: any }) => {
        this.eventHandlers.handleSubscription(socket, data);
      });

      socket.on('unsubscribe', (data: { type: string }) => {
        this.eventHandlers.handleUnsubscription(socket, data);
      });

      // Handle disconnection
      socket.on('disconnect', (reason) => {
        console.log(`🔌 WebSocket disconnected: ${socket.id} (${reason})`);
        
        // Clean up
        this.connectionManager.removeConnection(socket.id);
        
        // Leave all rooms
        for (const room of socket.joinedRooms) {
          this.roomManager.removeFromRoom(room, socket.id);
        }
        
        this.emit('user-disconnected', {
          socketId: socket.id,
          userId: socket.userId,
          userName: socket.userName,
          reason,
        });
      });

      // Handle errors
      socket.on('error', (error) => {
        console.error(`WebSocket error for ${socket.id}:`, error);
      });

      this.emit('user-connected', {
        socketId: socket.id,
        userId: socket.userId,
        userName: socket.userName,
        authenticated: socket.isAuthenticated,
      });
    });
  }

  private startHeartbeat(): void {
    this.heartbeatInterval = setInterval(() => {
      const now = new Date();
      const connections = this.connectionManager.getAllConnections();
      
      for (const [socketId, socket] of connections) {
        const timeSinceLastHeartbeat = now.getTime() - socket.lastHeartbeat.getTime();
        
        if (timeSinceLastHeartbeat > this.config.connectionTimeout) {
          console.warn(`Disconnecting stale connection: ${socketId}`);
          socket.disconnect(true);
        }
      }
      
      // Emit heartbeat to all connected clients
      this.io.emit('heartbeat', { timestamp: now.toISOString() });
    }, this.config.heartbeatInterval);
  }

  // Public methods for broadcasting updates
  public broadcastToRoom(room: string, event: string, data: any): void {
    this.io.to(room).emit(event, data);
  }

  public broadcastToAll(event: string, data: any): void {
    this.io.emit(event, data);
  }

  public broadcastToUser(userId: string, event: string, data: any): void {
    const connections = this.connectionManager.getUserConnections(userId);
    connections.forEach(socket => {
      socket.emit(event, data);
    });
  }

  public getConnectionCount(): number {
    return this.connectionManager.getConnectionCount();
  }

  public getConnectedUsers(): Array<{ id: string; name: string; socketId: string }> {
    return this.connectionManager.getConnectedUsers();
  }

  public getRoomMembers(room: string): number {
    return this.roomManager.getRoomSize(room);
  }

  public async shutdown(): Promise<void> {
    console.log('🔌 Shutting down WebSocket server...');
    
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
    }
    
    // Notify all clients of shutdown
    this.io.emit('server-shutdown', { 
      message: 'Server is shutting down', 
      timestamp: new Date().toISOString() 
    });
    
    // Close all connections
    this.io.close();
    
    console.log('🔌 WebSocket server shut down');
  }
}