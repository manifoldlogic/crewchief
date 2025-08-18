import session from 'express-session';
import { createClient, RedisClientType } from 'redis';
import { v4 as uuidv4 } from 'uuid';

// Dynamic import for connect-redis due to ESM compatibility issues
let ConnectRedis: any;

// Session configuration
const SESSION_SECRET = process.env.SESSION_SECRET || 'dev-session-secret-change-in-production';
const SESSION_NAME = 'crewchief.sid';
const SESSION_MAX_AGE = 24 * 60 * 60 * 1000; // 24 hours in milliseconds
const REDIS_URL = process.env.REDIS_URL || 'redis://localhost:6379';

// Session data interface
export interface SessionData {
  userId?: number;
  userUuid?: string;
  email?: string;
  roles?: string[];
  permissions?: string[];
  loginTime?: Date;
  lastActivity?: Date;
  ipAddress?: string;
  userAgent?: string;
  isAuthenticated?: boolean;
  twoFactorVerified?: boolean;
  oauthProvider?: string;
  deviceFingerprint?: string;
  preferences?: Record<string, any>;
}

export interface SessionInfo {
  sessionId: string;
  userId?: number;
  data: SessionData;
  createdAt: Date;
  lastAccessed: Date;
  expiresAt: Date;
  ipAddress?: string;
  userAgent?: string;
}

export class SessionService {
  private redisClient: RedisClientType;
  private redisStore: any;
  private isConnected = false;

  constructor() {
    this.redisClient = createClient({
      url: REDIS_URL,
      socket: {
        connectTimeout: 10000,
        lazyConnect: true,
      },
    });

    this.setupRedisClient();
    this.initializeRedisStore();
  }

  // Initialize Redis store with dynamic import
  private async initializeRedisStore() {
    try {
      const connectRedisModule = await import('connect-redis');
      ConnectRedis = connectRedisModule.default;
      this.redisStore = new ConnectRedis({
        client: this.redisClient,
        prefix: 'crewchief:session:',
      });
    } catch (error) {
      console.error('Failed to initialize Redis store:', error);
      // Fall back to memory store for development
      this.redisStore = null;
    }
  }

  // Setup Redis client with error handling
  private setupRedisClient(): void {
    this.redisClient.on('error', (err) => {
      console.error('Redis Client Error:', err);
      this.isConnected = false;
    });

    this.redisClient.on('connect', () => {
      console.log('Redis Client connected');
      this.isConnected = true;
    });

    this.redisClient.on('disconnect', () => {
      console.log('Redis Client disconnected');
      this.isConnected = false;
    });

    this.redisClient.on('reconnecting', () => {
      console.log('Redis Client reconnecting...');
    });
  }

  // Connect to Redis
  async connect(): Promise<void> {
    try {
      if (!this.isConnected) {
        await this.redisClient.connect();
        this.isConnected = true;
        console.log('✅ Redis session store connected');
      }
    } catch (error) {
      console.error('❌ Failed to connect to Redis:', error);
      throw new Error('Session service unavailable');
    }
  }

  // Disconnect from Redis
  async disconnect(): Promise<void> {
    try {
      if (this.isConnected) {
        await this.redisClient.disconnect();
        this.isConnected = false;
        console.log('Redis session store disconnected');
      }
    } catch (error) {
      console.error('Error disconnecting from Redis:', error);
    }
  }

  // Get Express session middleware
  getSessionMiddleware() {
    return session({
      store: this.redisStore || undefined, // Fall back to memory store if Redis not available
      secret: SESSION_SECRET,
      name: SESSION_NAME,
      resave: false,
      saveUninitialized: false,
      rolling: true, // Reset expiration on activity
      cookie: {
        secure: process.env.NODE_ENV === 'production', // HTTPS only in production
        httpOnly: true, // Prevent XSS
        maxAge: SESSION_MAX_AGE,
        sameSite: process.env.NODE_ENV === 'production' ? 'strict' : 'lax',
      },
      genid: () => uuidv4(), // Generate secure session IDs
    });
  }

  // Create session data
  async createSession(sessionId: string, data: SessionData): Promise<void> {
    try {
      const sessionKey = `crewchief:session:${sessionId}`;
      const sessionInfo: SessionInfo = {
        sessionId,
        userId: data.userId,
        data: {
          ...data,
          isAuthenticated: true,
          loginTime: new Date(),
          lastActivity: new Date(),
        },
        createdAt: new Date(),
        lastAccessed: new Date(),
        expiresAt: new Date(Date.now() + SESSION_MAX_AGE),
        ipAddress: data.ipAddress,
        userAgent: data.userAgent,
      };

      await this.redisClient.setEx(
        sessionKey,
        Math.floor(SESSION_MAX_AGE / 1000), // TTL in seconds
        JSON.stringify(sessionInfo)
      );

      // Track user sessions for management
      if (data.userId) {
        const userSessionsKey = `crewchief:user_sessions:${data.userId}`;
        await this.redisClient.sAdd(userSessionsKey, sessionId);
        await this.redisClient.expire(userSessionsKey, Math.floor(SESSION_MAX_AGE / 1000));
      }

    } catch (error) {
      console.error('Error creating session:', error);
      throw new Error('Failed to create session');
    }
  }

  // Get session data
  async getSession(sessionId: string): Promise<SessionInfo | null> {
    try {
      const sessionKey = `crewchief:session:${sessionId}`;
      const sessionData = await this.redisClient.get(sessionKey);
      
      if (!sessionData) {
        return null;
      }

      const session = JSON.parse(sessionData) as SessionInfo;
      
      // Update last accessed time
      session.lastAccessed = new Date();
      session.data.lastActivity = new Date();
      
      await this.redisClient.setEx(
        sessionKey,
        Math.floor(SESSION_MAX_AGE / 1000),
        JSON.stringify(session)
      );

      return session;

    } catch (error) {
      console.error('Error getting session:', error);
      return null;
    }
  }

  // Update session data
  async updateSession(sessionId: string, updates: Partial<SessionData>): Promise<void> {
    try {
      const session = await this.getSession(sessionId);
      if (!session) {
        throw new Error('Session not found');
      }

      session.data = {
        ...session.data,
        ...updates,
        lastActivity: new Date(),
      };
      session.lastAccessed = new Date();

      const sessionKey = `crewchief:session:${sessionId}`;
      await this.redisClient.setEx(
        sessionKey,
        Math.floor(SESSION_MAX_AGE / 1000),
        JSON.stringify(session)
      );

    } catch (error) {
      console.error('Error updating session:', error);
      throw new Error('Failed to update session');
    }
  }

  // Destroy session
  async destroySession(sessionId: string): Promise<void> {
    try {
      const session = await this.getSession(sessionId);
      
      const sessionKey = `crewchief:session:${sessionId}`;
      await this.redisClient.del(sessionKey);

      // Remove from user sessions tracking
      if (session?.userId) {
        const userSessionsKey = `crewchief:user_sessions:${session.userId}`;
        await this.redisClient.sRem(userSessionsKey, sessionId);
      }

    } catch (error) {
      console.error('Error destroying session:', error);
      throw new Error('Failed to destroy session');
    }
  }

  // Destroy all user sessions (for logout all)
  async destroyAllUserSessions(userId: number): Promise<number> {
    try {
      const userSessionsKey = `crewchief:user_sessions:${userId}`;
      const sessionIds = await this.redisClient.sMembers(userSessionsKey);

      let destroyedCount = 0;
      
      for (const sessionId of sessionIds) {
        try {
          await this.destroySession(sessionId);
          destroyedCount++;
        } catch (error) {
          console.error(`Error destroying session ${sessionId}:`, error);
        }
      }

      // Clear the user sessions set
      await this.redisClient.del(userSessionsKey);
      
      return destroyedCount;

    } catch (error) {
      console.error('Error destroying all user sessions:', error);
      throw new Error('Failed to destroy user sessions');
    }
  }

  // Get all user sessions
  async getUserSessions(userId: number): Promise<SessionInfo[]> {
    try {
      const userSessionsKey = `crewchief:user_sessions:${userId}`;
      const sessionIds = await this.redisClient.sMembers(userSessionsKey);

      const sessions: SessionInfo[] = [];
      
      for (const sessionId of sessionIds) {
        try {
          const session = await this.getSession(sessionId);
          if (session) {
            sessions.push(session);
          }
        } catch (error) {
          console.error(`Error getting session ${sessionId}:`, error);
        }
      }

      // Sort by last accessed time (most recent first)
      sessions.sort((a, b) => 
        new Date(b.lastAccessed).getTime() - new Date(a.lastAccessed).getTime()
      );

      return sessions;

    } catch (error) {
      console.error('Error getting user sessions:', error);
      return [];
    }
  }

  // Clean up expired sessions
  async cleanupExpiredSessions(): Promise<number> {
    try {
      // This is handled automatically by Redis TTL, but we can implement
      // additional cleanup for orphaned user session tracking
      
      // Get all user session keys
      const userSessionKeys = await this.redisClient.keys('crewchief:user_sessions:*');
      let cleanedCount = 0;

      for (const userSessionKey of userSessionKeys) {
        const sessionIds = await this.redisClient.sMembers(userSessionKey);
        const validSessionIds: string[] = [];

        for (const sessionId of sessionIds) {
          const sessionKey = `crewchief:session:${sessionId}`;
          const exists = await this.redisClient.exists(sessionKey);
          
          if (exists) {
            validSessionIds.push(sessionId);
          } else {
            cleanedCount++;
          }
        }

        // Update the user sessions set with only valid sessions
        if (validSessionIds.length > 0) {
          await this.redisClient.del(userSessionKey);
          await this.redisClient.sAdd(userSessionKey, validSessionIds);
          await this.redisClient.expire(userSessionKey, Math.floor(SESSION_MAX_AGE / 1000));
        } else {
          await this.redisClient.del(userSessionKey);
        }
      }

      return cleanedCount;

    } catch (error) {
      console.error('Error cleaning up expired sessions:', error);
      return 0;
    }
  }

  // Get session statistics
  async getSessionStatistics(): Promise<{
    totalActiveSessions: number;
    sessionsByUser: Record<number, number>;
    recentActivity: Array<{
      userId: number;
      sessionCount: number;
      lastActivity: Date;
    }>;
  }> {
    try {
      // Get all session keys
      const sessionKeys = await this.redisClient.keys('crewchief:session:*');
      const sessionsByUser: Record<number, number> = {};
      const userLastActivity: Record<number, Date> = {};

      for (const sessionKey of sessionKeys) {
        try {
          const sessionData = await this.redisClient.get(sessionKey);
          if (sessionData) {
            const session = JSON.parse(sessionData) as SessionInfo;
            if (session.userId) {
              sessionsByUser[session.userId] = (sessionsByUser[session.userId] || 0) + 1;
              
              const lastActivity = new Date(session.lastAccessed);
              if (!userLastActivity[session.userId] || lastActivity > userLastActivity[session.userId]) {
                userLastActivity[session.userId] = lastActivity;
              }
            }
          }
        } catch (error) {
          console.error(`Error processing session ${sessionKey}:`, error);
        }
      }

      const recentActivity = Object.entries(sessionsByUser).map(([userId, sessionCount]) => ({
        userId: parseInt(userId),
        sessionCount,
        lastActivity: userLastActivity[parseInt(userId)],
      })).sort((a, b) => b.lastActivity.getTime() - a.lastActivity.getTime());

      return {
        totalActiveSessions: sessionKeys.length,
        sessionsByUser,
        recentActivity,
      };

    } catch (error) {
      console.error('Error getting session statistics:', error);
      return {
        totalActiveSessions: 0,
        sessionsByUser: {},
        recentActivity: [],
      };
    }
  }

  // Health check
  async healthCheck(): Promise<boolean> {
    try {
      await this.redisClient.ping();
      return true;
    } catch (error) {
      console.error('Redis health check failed:', error);
      return false;
    }
  }

  // Get Redis info
  async getRedisInfo(): Promise<Record<string, string>> {
    try {
      const info = await this.redisClient.info();
      const infoObj: Record<string, string> = {};
      
      info.split('\r\n').forEach(line => {
        if (line.includes(':') && !line.startsWith('#')) {
          const [key, value] = line.split(':');
          infoObj[key] = value;
        }
      });

      return infoObj;
    } catch (error) {
      console.error('Error getting Redis info:', error);
      return {};
    }
  }
}