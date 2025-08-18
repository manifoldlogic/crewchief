import type { Server as SocketIOServer } from 'socket.io';
import type { AuthenticatedSocket, WebSocketConfig } from './server.js';

interface RateLimitEntry {
  count: number;
  firstRequest: number;
}

export class MessageHandler {
  private rateLimitMap = new Map<string, RateLimitEntry>();
  private config: WebSocketConfig;

  constructor(config: WebSocketConfig) {
    this.config = config;
    
    // Clean up rate limit map periodically
    setInterval(() => {
      this.cleanupRateLimit();
    }, this.config.rateLimitWindow);
  }

  public checkRateLimit(clientIp: string): boolean {
    const now = Date.now();
    const entry = this.rateLimitMap.get(clientIp);

    if (!entry) {
      this.rateLimitMap.set(clientIp, {
        count: 1,
        firstRequest: now,
      });
      return true;
    }

    // Reset if window has passed
    if (now - entry.firstRequest > this.config.rateLimitWindow) {
      this.rateLimitMap.set(clientIp, {
        count: 1,
        firstRequest: now,
      });
      return true;
    }

    // Check if under limit
    if (entry.count < this.config.rateLimitMaxRequests) {
      entry.count++;
      return true;
    }

    return false;
  }

  public validateMessage(data: any, socket: AuthenticatedSocket): boolean {
    // Check message size
    const messageSize = JSON.stringify(data).length;
    if (messageSize > this.config.maxMessageSize) {
      socket.emit('error', { 
        message: 'Message too large',
        maxSize: this.config.maxMessageSize,
        actualSize: messageSize,
      });
      return false;
    }

    // Check rate limit for this socket
    socket.messageCount++;
    if (socket.messageCount > this.config.rateLimitMaxRequests) {
      socket.emit('error', { 
        message: 'Message rate limit exceeded',
        limit: this.config.rateLimitMaxRequests,
      });
      return false;
    }

    // Reset message count periodically
    if (!socket.data?.lastReset || Date.now() - socket.data.lastReset > this.config.rateLimitWindow) {
      socket.messageCount = 1;
      if (!socket.data) socket.data = {};
      socket.data.lastReset = Date.now();
    }

    // Basic structure validation
    if (typeof data !== 'object' || !data.type) {
      socket.emit('error', { message: 'Invalid message format' });
      return false;
    }

    return true;
  }

  public handleMessage(data: any, socket: AuthenticatedSocket, io: SocketIOServer): void {
    try {
      switch (data.type) {
        case 'echo':
          this.handleEcho(data, socket);
          break;
        
        case 'broadcast':
          this.handleBroadcast(data, socket, io);
          break;
        
        case 'room-message':
          this.handleRoomMessage(data, socket, io);
          break;
        
        case 'private-message':
          this.handlePrivateMessage(data, socket, io);
          break;
        
        case 'status-update':
          this.handleStatusUpdate(data, socket, io);
          break;
        
        default:
          socket.emit('error', { message: `Unknown message type: ${data.type}` });
      }
    } catch (error) {
      console.error('Error handling message:', error);
      socket.emit('error', { message: 'Failed to process message' });
    }
  }

  private handleEcho(data: any, socket: AuthenticatedSocket): void {
    socket.emit('echo-response', {
      ...data,
      timestamp: new Date().toISOString(),
      socketId: socket.id,
    });
  }

  private handleBroadcast(data: any, socket: AuthenticatedSocket, io: SocketIOServer): void {
    // Only authenticated users can broadcast
    if (!socket.isAuthenticated) {
      socket.emit('error', { message: 'Authentication required for broadcast' });
      return;
    }

    const broadcastData = {
      ...data.payload,
      from: {
        id: socket.userId,
        name: socket.userName,
      },
      timestamp: new Date().toISOString(),
    };

    io.emit('broadcast-message', broadcastData);
    
    // Send confirmation to sender
    socket.emit('message-sent', {
      messageId: data.id || Math.random().toString(36),
      type: 'broadcast',
      timestamp: new Date().toISOString(),
    });
  }

  private handleRoomMessage(data: any, socket: AuthenticatedSocket, io: SocketIOServer): void {
    const { room, message } = data;
    
    if (!room || !message) {
      socket.emit('error', { message: 'Room and message are required' });
      return;
    }

    // Check if socket is in the room
    if (!socket.rooms.has(room)) {
      socket.emit('error', { message: 'Not a member of this room' });
      return;
    }

    const messageData = {
      message,
      room,
      from: {
        id: socket.userId,
        name: socket.userName,
        authenticated: socket.isAuthenticated,
      },
      timestamp: new Date().toISOString(),
    };

    io.to(room).emit('room-message', messageData);
    
    // Send confirmation to sender
    socket.emit('message-sent', {
      messageId: data.id || Math.random().toString(36),
      type: 'room',
      room,
      timestamp: new Date().toISOString(),
    });
  }

  private handlePrivateMessage(data: any, socket: AuthenticatedSocket, io: SocketIOServer): void {
    const { targetUserId, message } = data;
    
    if (!socket.isAuthenticated) {
      socket.emit('error', { message: 'Authentication required for private messages' });
      return;
    }

    if (!targetUserId || !message) {
      socket.emit('error', { message: 'Target user ID and message are required' });
      return;
    }

    const messageData = {
      message,
      from: {
        id: socket.userId,
        name: socket.userName,
      },
      timestamp: new Date().toISOString(),
    };

    // Find target user's sockets
    const targetSockets = Array.from(io.sockets.sockets.values()).filter(
      s => (s as AuthenticatedSocket).userId === targetUserId
    );

    if (targetSockets.length === 0) {
      socket.emit('error', { message: 'Target user not connected' });
      return;
    }

    // Send to all target user's connections
    targetSockets.forEach(targetSocket => {
      targetSocket.emit('private-message', messageData);
    });

    // Send confirmation to sender
    socket.emit('message-sent', {
      messageId: data.id || Math.random().toString(36),
      type: 'private',
      targetUserId,
      timestamp: new Date().toISOString(),
    });
  }

  private handleStatusUpdate(data: any, socket: AuthenticatedSocket, io: SocketIOServer): void {
    if (!socket.isAuthenticated) {
      socket.emit('error', { message: 'Authentication required for status updates' });
      return;
    }

    const statusData = {
      userId: socket.userId,
      userName: socket.userName,
      status: data.status,
      timestamp: new Date().toISOString(),
    };

    // Broadcast status update to all connected clients
    io.emit('user-status-update', statusData);
    
    // Send confirmation to sender
    socket.emit('message-sent', {
      messageId: data.id || Math.random().toString(36),
      type: 'status',
      timestamp: new Date().toISOString(),
    });
  }

  private cleanupRateLimit(): void {
    const now = Date.now();
    for (const [key, entry] of this.rateLimitMap.entries()) {
      if (now - entry.firstRequest > this.config.rateLimitWindow * 2) {
        this.rateLimitMap.delete(key);
      }
    }
  }

  public getRateLimitStats(): { totalEntries: number; activeEntries: number } {
    const now = Date.now();
    let activeEntries = 0;

    for (const entry of this.rateLimitMap.values()) {
      if (now - entry.firstRequest <= this.config.rateLimitWindow) {
        activeEntries++;
      }
    }

    return {
      totalEntries: this.rateLimitMap.size,
      activeEntries,
    };
  }
}