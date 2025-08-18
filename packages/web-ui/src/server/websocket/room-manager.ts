export interface RoomMember {
  socketId: string;
  joinedAt: Date;
  type: string; // 'general', 'worktree', 'agent', etc.
}

export interface Room {
  name: string;
  members: Map<string, RoomMember>;
  createdAt: Date;
  lastActivity: Date;
  metadata?: any;
}

export class RoomManager {
  private rooms = new Map<string, Room>();
  private socketRooms = new Map<string, Set<string>>(); // socketId -> Set of room names

  constructor() {
    // Clean up empty rooms periodically
    setInterval(() => {
      this.cleanupEmptyRooms();
    }, 300000); // 5 minutes
  }

  public validateRoomName(roomName: string): string | null {
    // Remove any invalid characters and normalize
    const cleaned = roomName.trim().replace(/[^a-zA-Z0-9_-]/g, '');
    
    if (cleaned.length === 0) {
      return null;
    }

    if (cleaned.length > 100) {
      return null;
    }

    return cleaned;
  }

  public createRoom(roomName: string, metadata?: any): boolean {
    const validName = this.validateRoomName(roomName);
    if (!validName) {
      return false;
    }

    if (this.rooms.has(validName)) {
      return false; // Room already exists
    }

    const room: Room = {
      name: validName,
      members: new Map(),
      createdAt: new Date(),
      lastActivity: new Date(),
      metadata,
    };

    this.rooms.set(validName, room);
    console.log(`📺 Room created: ${validName}`);
    return true;
  }

  public addToRoom(roomName: string, socketId: string, type = 'general'): boolean {
    const validName = this.validateRoomName(roomName);
    if (!validName) {
      return false;
    }

    // Create room if it doesn't exist
    if (!this.rooms.has(validName)) {
      this.createRoom(validName);
    }

    const room = this.rooms.get(validName)!;
    
    // Add member to room
    room.members.set(socketId, {
      socketId,
      joinedAt: new Date(),
      type,
    });
    room.lastActivity = new Date();

    // Track socket's rooms
    if (!this.socketRooms.has(socketId)) {
      this.socketRooms.set(socketId, new Set());
    }
    this.socketRooms.get(socketId)!.add(validName);

    return true;
  }

  public removeFromRoom(roomName: string, socketId: string): boolean {
    const room = this.rooms.get(roomName);
    if (!room) {
      return false;
    }

    room.members.delete(socketId);
    room.lastActivity = new Date();

    // Update socket's rooms
    const socketRoomSet = this.socketRooms.get(socketId);
    if (socketRoomSet) {
      socketRoomSet.delete(roomName);
      if (socketRoomSet.size === 0) {
        this.socketRooms.delete(socketId);
      }
    }

    return true;
  }

  public removeSocketFromAllRooms(socketId: string): void {
    const socketRoomSet = this.socketRooms.get(socketId);
    if (!socketRoomSet) {
      return;
    }

    for (const roomName of socketRoomSet) {
      this.removeFromRoom(roomName, socketId);
    }
  }

  public getRoomSize(roomName: string): number {
    const room = this.rooms.get(roomName);
    return room ? room.members.size : 0;
  }

  public getRoomMembers(roomName: string): RoomMember[] {
    const room = this.rooms.get(roomName);
    return room ? Array.from(room.members.values()) : [];
  }

  public getSocketRooms(socketId: string): string[] {
    const socketRoomSet = this.socketRooms.get(socketId);
    return socketRoomSet ? Array.from(socketRoomSet) : [];
  }

  public getAllRooms(): Room[] {
    return Array.from(this.rooms.values());
  }

  public getRoomInfo(roomName: string): Room | undefined {
    return this.rooms.get(roomName);
  }

  public isSocketInRoom(socketId: string, roomName: string): boolean {
    const room = this.rooms.get(roomName);
    return room ? room.members.has(socketId) : false;
  }

  public getRoomStats(): {
    totalRooms: number;
    totalMembers: number;
    averageMembersPerRoom: number;
    roomsByType: Record<string, number>;
  } {
    let totalMembers = 0;
    const roomsByType: Record<string, number> = {};

    for (const room of this.rooms.values()) {
      totalMembers += room.members.size;
      
      for (const member of room.members.values()) {
        roomsByType[member.type] = (roomsByType[member.type] || 0) + 1;
      }
    }

    return {
      totalRooms: this.rooms.size,
      totalMembers,
      averageMembersPerRoom: this.rooms.size > 0 ? totalMembers / this.rooms.size : 0,
      roomsByType,
    };
  }

  private cleanupEmptyRooms(): void {
    const emptyRooms: string[] = [];
    
    for (const [roomName, room] of this.rooms) {
      if (room.members.size === 0) {
        emptyRooms.push(roomName);
      }
    }

    for (const roomName of emptyRooms) {
      this.rooms.delete(roomName);
      console.log(`📺 Cleaned up empty room: ${roomName}`);
    }
  }

  // Helper methods for specific room types
  public createWorktreeRoom(worktreeId: string): string {
    const roomName = `worktree:${worktreeId}`;
    this.createRoom(roomName, { type: 'worktree', worktreeId });
    return roomName;
  }

  public createAgentRoom(agentId: string): string {
    const roomName = `agent:${agentId}`;
    this.createRoom(roomName, { type: 'agent', agentId });
    return roomName;
  }

  public createRunRoom(runId: string): string {
    const roomName = `run:${runId}`;
    this.createRoom(roomName, { type: 'run', runId });
    return roomName;
  }

  public getWorktreeRooms(): Room[] {
    return this.getAllRooms().filter(room => 
      room.metadata?.type === 'worktree'
    );
  }

  public getAgentRooms(): Room[] {
    return this.getAllRooms().filter(room => 
      room.metadata?.type === 'agent'
    );
  }

  public getRunRooms(): Room[] {
    return this.getAllRooms().filter(room => 
      room.metadata?.type === 'run'
    );
  }

  public broadcastToWorktreeRoom(worktreeId: string, event: string, data: any, io: any): void {
    const roomName = `worktree:${worktreeId}`;
    io.to(roomName).emit(event, data);
  }

  public broadcastToAgentRoom(agentId: string, event: string, data: any, io: any): void {
    const roomName = `agent:${agentId}`;
    io.to(roomName).emit(event, data);
  }

  public broadcastToRunRoom(runId: string, event: string, data: any, io: any): void {
    const roomName = `run:${runId}`;
    io.to(roomName).emit(event, data);
  }
}