import type { Server as SocketIOServer } from 'socket.io';
import type { Pool } from 'pg';
import type { AuthenticatedSocket } from './server.js';
import type { RoomManager } from './room-manager.js';

export interface SubscriptionFilter {
  worktreeId?: string;
  agentId?: string;
  runId?: string;
  userId?: string;
}

export class EventHandlers {
  private io: SocketIOServer;
  private db: Pool;
  private roomManager: RoomManager;
  private subscriptions = new Map<string, Map<string, SubscriptionFilter>>(); // socketId -> subscriptionType -> filters

  constructor(io: SocketIOServer, db: Pool, roomManager: RoomManager) {
    this.io = io;
    this.db = db;
    this.roomManager = roomManager;
  }

  public handleSubscription(socket: AuthenticatedSocket, data: { type: string; filters?: SubscriptionFilter }): void {
    const { type, filters = {} } = data;

    if (!this.subscriptions.has(socket.id)) {
      this.subscriptions.set(socket.id, new Map());
    }

    this.subscriptions.get(socket.id)!.set(type, filters);

    // Join appropriate rooms based on subscription type
    switch (type) {
      case 'worktree-updates':
        if (filters.worktreeId) {
          const roomName = this.roomManager.createWorktreeRoom(filters.worktreeId);
          socket.join(roomName);
          socket.joinedRooms.add(roomName);
        }
        break;

      case 'agent-updates':
        if (filters.agentId) {
          const roomName = this.roomManager.createAgentRoom(filters.agentId);
          socket.join(roomName);
          socket.joinedRooms.add(roomName);
        }
        break;

      case 'run-updates':
        if (filters.runId) {
          const roomName = this.roomManager.createRunRoom(filters.runId);
          socket.join(roomName);
          socket.joinedRooms.add(roomName);
        }
        break;

      case 'maproom-updates':
        socket.join('maproom:updates');
        socket.joinedRooms.add('maproom:updates');
        break;

      case 'config-updates':
        socket.join('config:updates');
        socket.joinedRooms.add('config:updates');
        break;

      case 'system-updates':
        socket.join('system:updates');
        socket.joinedRooms.add('system:updates');
        break;
    }

    socket.emit('subscription-confirmed', {
      type,
      filters,
      timestamp: new Date().toISOString(),
    });

    console.log(`📡 Socket ${socket.id} subscribed to ${type} with filters:`, filters);
  }

  public handleUnsubscription(socket: AuthenticatedSocket, data: { type: string }): void {
    const { type } = data;

    if (!this.subscriptions.has(socket.id)) {
      return;
    }

    const socketSubscriptions = this.subscriptions.get(socket.id)!;
    const filters = socketSubscriptions.get(type);

    if (!filters) {
      return;
    }

    socketSubscriptions.delete(type);

    // Leave appropriate rooms
    switch (type) {
      case 'worktree-updates':
        if (filters.worktreeId) {
          const roomName = `worktree:${filters.worktreeId}`;
          socket.leave(roomName);
          socket.joinedRooms.delete(roomName);
        }
        break;

      case 'agent-updates':
        if (filters.agentId) {
          const roomName = `agent:${filters.agentId}`;
          socket.leave(roomName);
          socket.joinedRooms.delete(roomName);
        }
        break;

      case 'run-updates':
        if (filters.runId) {
          const roomName = `run:${filters.runId}`;
          socket.leave(roomName);
          socket.joinedRooms.delete(roomName);
        }
        break;

      case 'maproom-updates':
        socket.leave('maproom:updates');
        socket.joinedRooms.delete('maproom:updates');
        break;

      case 'config-updates':
        socket.leave('config:updates');
        socket.joinedRooms.delete('config:updates');
        break;

      case 'system-updates':
        socket.leave('system:updates');
        socket.joinedRooms.delete('system:updates');
        break;
    }

    socket.emit('unsubscription-confirmed', {
      type,
      timestamp: new Date().toISOString(),
    });

    console.log(`📡 Socket ${socket.id} unsubscribed from ${type}`);
  }

  // Entity change event emitters
  public emitWorktreeUpdate(worktreeId: string, event: string, data: any): void {
    const roomName = `worktree:${worktreeId}`;
    this.io.to(roomName).emit('worktree-update', {
      worktreeId,
      event,
      data,
      timestamp: new Date().toISOString(),
    });
  }

  public emitAgentStatusChange(agentId: string, status: any): void {
    const roomName = `agent:${agentId}`;
    this.io.to(roomName).emit('agent-status-change', {
      agentId,
      status,
      timestamp: new Date().toISOString(),
    });
  }

  public emitRunProgress(runId: string, progress: any): void {
    const roomName = `run:${runId}`;
    this.io.to(roomName).emit('run-progress', {
      runId,
      progress,
      timestamp: new Date().toISOString(),
    });
  }

  public emitMaproomIndexingStatus(status: any): void {
    this.io.to('maproom:updates').emit('maproom-indexing-status', {
      status,
      timestamp: new Date().toISOString(),
    });
  }

  public emitConfigChange(configKey: string, newValue: any, oldValue?: any): void {
    this.io.to('config:updates').emit('config-change', {
      key: configKey,
      newValue,
      oldValue,
      timestamp: new Date().toISOString(),
    });
  }

  public emitSystemUpdate(type: string, data: any): void {
    this.io.to('system:updates').emit('system-update', {
      type,
      data,
      timestamp: new Date().toISOString(),
    });
  }

  // Specific event handlers for different entity types
  public handleWorktreeCreated(worktreeId: string, worktreeData: any): void {
    this.emitWorktreeUpdate(worktreeId, 'created', worktreeData);
    this.io.emit('global-update', {
      type: 'worktree-created',
      worktreeId,
      data: worktreeData,
      timestamp: new Date().toISOString(),
    });
  }

  public handleWorktreeDeleted(worktreeId: string): void {
    this.emitWorktreeUpdate(worktreeId, 'deleted', { worktreeId });
    this.io.emit('global-update', {
      type: 'worktree-deleted',
      worktreeId,
      timestamp: new Date().toISOString(),
    });
  }

  public handleAgentStarted(agentId: string, agentData: any): void {
    this.emitAgentStatusChange(agentId, { status: 'started', ...agentData });
    this.io.emit('global-update', {
      type: 'agent-started',
      agentId,
      data: agentData,
      timestamp: new Date().toISOString(),
    });
  }

  public handleAgentStopped(agentId: string, reason?: string): void {
    this.emitAgentStatusChange(agentId, { status: 'stopped', reason });
    this.io.emit('global-update', {
      type: 'agent-stopped',
      agentId,
      reason,
      timestamp: new Date().toISOString(),
    });
  }

  public handleRunStarted(runId: string, runData: any): void {
    this.emitRunProgress(runId, { status: 'started', ...runData });
    this.io.emit('global-update', {
      type: 'run-started',
      runId,
      data: runData,
      timestamp: new Date().toISOString(),
    });
  }

  public handleRunCompleted(runId: string, result: any): void {
    this.emitRunProgress(runId, { status: 'completed', result });
    this.io.emit('global-update', {
      type: 'run-completed',
      runId,
      result,
      timestamp: new Date().toISOString(),
    });
  }

  public handleRunFailed(runId: string, error: any): void {
    this.emitRunProgress(runId, { status: 'failed', error });
    this.io.emit('global-update', {
      type: 'run-failed',
      runId,
      error,
      timestamp: new Date().toISOString(),
    });
  }

  public handleMaproomScanStarted(scanData: any): void {
    this.emitMaproomIndexingStatus({ 
      type: 'scan-started', 
      ...scanData 
    });
  }

  public handleMaproomScanProgress(progress: any): void {
    this.emitMaproomIndexingStatus({ 
      type: 'scan-progress', 
      ...progress 
    });
  }

  public handleMaproomScanCompleted(result: any): void {
    this.emitMaproomIndexingStatus({ 
      type: 'scan-completed', 
      ...result 
    });
  }

  // Cleanup subscriptions when socket disconnects
  public cleanupSocketSubscriptions(socketId: string): void {
    this.subscriptions.delete(socketId);
  }

  // Get subscription stats
  public getSubscriptionStats(): {
    totalSubscriptions: number;
    subscriptionTypes: Record<string, number>;
    activeSubscribers: number;
  } {
    let totalSubscriptions = 0;
    const subscriptionTypes: Record<string, number> = {};

    for (const socketSubscriptions of this.subscriptions.values()) {
      totalSubscriptions += socketSubscriptions.size;
      
      for (const type of socketSubscriptions.keys()) {
        subscriptionTypes[type] = (subscriptionTypes[type] || 0) + 1;
      }
    }

    return {
      totalSubscriptions,
      subscriptionTypes,
      activeSubscribers: this.subscriptions.size,
    };
  }

  // Broadcast to all subscribers of a specific type
  public broadcastToSubscribers(subscriptionType: string, event: string, data: any): void {
    for (const [socketId, socketSubscriptions] of this.subscriptions) {
      if (socketSubscriptions.has(subscriptionType)) {
        const socket = this.io.sockets.sockets.get(socketId);
        if (socket) {
          socket.emit(event, data);
        }
      }
    }
  }
}