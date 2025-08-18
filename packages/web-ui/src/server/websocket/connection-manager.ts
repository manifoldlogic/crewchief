import type { AuthenticatedSocket } from './server.js';

export class ConnectionManager {
  private connections = new Map<string, AuthenticatedSocket>();
  private userConnections = new Map<string, Set<string>>(); // userId -> Set of socketIds
  private maxConnections: number;

  constructor(maxConnections: number) {
    this.maxConnections = maxConnections;
  }

  public addConnection(socketId: string, socket: AuthenticatedSocket): boolean {
    if (this.connections.size >= this.maxConnections) {
      return false;
    }

    this.connections.set(socketId, socket);

    // Track user connections if authenticated
    if (socket.isAuthenticated && socket.userId) {
      if (!this.userConnections.has(socket.userId)) {
        this.userConnections.set(socket.userId, new Set());
      }
      this.userConnections.get(socket.userId)!.add(socketId);
    }

    return true;
  }

  public removeConnection(socketId: string): void {
    const socket = this.connections.get(socketId);
    if (socket) {
      // Remove from user connections
      if (socket.isAuthenticated && socket.userId) {
        const userSockets = this.userConnections.get(socket.userId);
        if (userSockets) {
          userSockets.delete(socketId);
          if (userSockets.size === 0) {
            this.userConnections.delete(socket.userId);
          }
        }
      }

      this.connections.delete(socketId);
    }
  }

  public getConnection(socketId: string): AuthenticatedSocket | undefined {
    return this.connections.get(socketId);
  }

  public getAllConnections(): Map<string, AuthenticatedSocket> {
    return new Map(this.connections);
  }

  public getUserConnections(userId: string): AuthenticatedSocket[] {
    const socketIds = this.userConnections.get(userId);
    if (!socketIds) {
      return [];
    }

    const sockets: AuthenticatedSocket[] = [];
    for (const socketId of socketIds) {
      const socket = this.connections.get(socketId);
      if (socket) {
        sockets.push(socket);
      }
    }

    return sockets;
  }

  public getConnectionCount(): number {
    return this.connections.size;
  }

  public canAcceptConnection(): boolean {
    return this.connections.size < this.maxConnections;
  }

  public getConnectedUsers(): Array<{ id: string; name: string; socketId: string }> {
    const users: Array<{ id: string; name: string; socketId: string }> = [];
    
    for (const [socketId, socket] of this.connections) {
      if (socket.isAuthenticated && socket.userId) {
        users.push({
          id: socket.userId,
          name: socket.userName || 'Unknown',
          socketId,
        });
      }
    }

    return users;
  }

  public getConnectionStats(): {
    total: number;
    authenticated: number;
    anonymous: number;
    uniqueUsers: number;
  } {
    let authenticated = 0;
    let anonymous = 0;

    for (const socket of this.connections.values()) {
      if (socket.isAuthenticated) {
        authenticated++;
      } else {
        anonymous++;
      }
    }

    return {
      total: this.connections.size,
      authenticated,
      anonymous,
      uniqueUsers: this.userConnections.size,
    };
  }

  public isUserConnected(userId: string): boolean {
    return this.userConnections.has(userId) && this.userConnections.get(userId)!.size > 0;
  }

  public getUserConnectionCount(userId: string): number {
    const userSockets = this.userConnections.get(userId);
    return userSockets ? userSockets.size : 0;
  }

  public disconnectUser(userId: string, reason = 'Disconnected by server'): void {
    const sockets = this.getUserConnections(userId);
    for (const socket of sockets) {
      socket.disconnect(true);
    }
  }

  public cleanupStaleConnections(maxAge: number): number {
    const now = new Date();
    let cleaned = 0;

    for (const [socketId, socket] of this.connections) {
      const age = now.getTime() - socket.lastHeartbeat.getTime();
      if (age > maxAge) {
        socket.disconnect(true);
        cleaned++;
      }
    }

    return cleaned;
  }
}