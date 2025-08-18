/**
 * Agent Service
 * 
 * Service layer for agent lifecycle management including spawning, monitoring, and termination.
 * Implements the Result pattern, caching, audit logging, and authorization.
 */

import { spawn, ChildProcess } from 'node:child_process';
import { EventEmitter } from 'node:events';
import fs from 'node:fs/promises';
import path from 'node:path';
import { 
  BaseService, 
  Result, 
  success, 
  failure, 
  ServiceError, 
  ServiceConfig,
  CacheProvider,
  AuditLogger,
} from './base.js';
import { getDatabase } from '../../db/connection.js';

export interface AgentConfig extends ServiceConfig {
  maxAgents?: number;
  defaultTimeout?: number;
  logRetentionDays?: number;
  resourceLimits?: {
    memory?: number; // MB
    cpu?: number;    // percentage
  };
}

export interface AgentInfo {
  id: string;
  name: string;
  type: string;
  status: 'starting' | 'running' | 'stopped' | 'failed' | 'terminated';
  pid?: number;
  createdAt: string;
  updatedAt: string;
  createdBy?: string;
  config?: Record<string, any>;
  metadata?: Record<string, any>;
  workingDirectory?: string;
  logPath?: string;
}

export interface AgentSpawnOptions {
  name: string;
  type: string;
  command: string;
  args?: string[];
  env?: Record<string, string>;
  workingDirectory?: string;
  config?: Record<string, any>;
  metadata?: Record<string, any>;
  timeout?: number;
}

export interface AgentMetrics {
  agentId: string;
  cpu: number;
  memory: number;
  uptime: number;
  messagesProcessed: number;
  errorsCount: number;
  lastActivity: string;
  timestamp: string;
}

export interface AgentMessage {
  id: string;
  agentId: string;
  type: 'info' | 'warn' | 'error' | 'debug';
  content: string;
  timestamp: string;
  metadata?: Record<string, any>;
}

export interface AgentLogFilter {
  agentId?: string;
  type?: string[];
  startTime?: string;
  endTime?: string;
  limit?: number;
  offset?: number;
}

export class AgentService extends BaseService {
  private maxAgents: number;
  private defaultTimeout: number;
  private logRetentionDays: number;
  private resourceLimits: { memory?: number; cpu?: number };
  private runningAgents = new Map<string, ChildProcess>();
  private agentMetrics = new Map<string, AgentMetrics>();
  private eventEmitter = new EventEmitter();

  constructor(
    config: AgentConfig = {},
    cache?: CacheProvider,
    auditLogger?: AuditLogger,
  ) {
    super(config, cache, auditLogger);
    
    this.maxAgents = config.maxAgents || 20;
    this.defaultTimeout = config.defaultTimeout || 3600000; // 1 hour
    this.logRetentionDays = config.logRetentionDays || 7;
    this.resourceLimits = config.resourceLimits || { memory: 2048, cpu: 80 };

    // Setup periodic cleanup and monitoring
    setInterval(() => this.cleanupLogs(), 24 * 60 * 60 * 1000); // Daily
    setInterval(() => this.updateMetrics(), 30000); // Every 30 seconds
  }

  /**
   * Validate agent configuration
   */
  private validateAgentOptions(options: AgentSpawnOptions): void {
    if (!options.name || options.name.length < 1 || options.name.length > 100) {
      throw new ServiceError(
        'Agent name must be between 1 and 100 characters',
        'INVALID_AGENT_NAME',
        400,
        undefined,
        this.correlationId,
      );
    }

    if (!/^[a-zA-Z0-9_-]+$/.test(options.name)) {
      throw new ServiceError(
        'Agent name can only contain letters, numbers, underscores, and hyphens',
        'INVALID_AGENT_NAME',
        400,
        undefined,
        this.correlationId,
      );
    }

    if (!options.command || options.command.length === 0) {
      throw new ServiceError(
        'Agent command is required',
        'INVALID_AGENT_COMMAND',
        400,
        undefined,
        this.correlationId,
      );
    }
  }

  /**
   * Store agent info in database
   */
  private async storeAgentInfo(agent: AgentInfo): Promise<void> {
    const db = getDatabase();
    
    await db.query(`
      INSERT INTO agent_runs (
        id, name, type, status, pid, created_at, updated_at,
        created_by, config, metadata, working_directory, log_path
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
      ON CONFLICT (id) DO UPDATE SET
        status = EXCLUDED.status,
        pid = EXCLUDED.pid,
        updated_at = EXCLUDED.updated_at,
        config = EXCLUDED.config,
        metadata = EXCLUDED.metadata
    `, [
      agent.id,
      agent.name,
      agent.type,
      agent.status,
      agent.pid,
      agent.createdAt,
      agent.updatedAt,
      agent.createdBy,
      JSON.stringify(agent.config || {}),
      JSON.stringify(agent.metadata || {}),
      agent.workingDirectory,
      agent.logPath,
    ]);
  }

  /**
   * Get agent info from database
   */
  private async getAgentInfoFromDb(id: string): Promise<AgentInfo | null> {
    const db = getDatabase();
    
    const result = await db.query(`
      SELECT * FROM agent_runs WHERE id = $1
    `, [id]);

    if (result.rows.length === 0) {
      return null;
    }

    const row = result.rows[0];
    return {
      id: row.id,
      name: row.name,
      type: row.type,
      status: row.status,
      pid: row.pid,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      createdBy: row.created_by,
      config: row.config || {},
      metadata: row.metadata || {},
      workingDirectory: row.working_directory,
      logPath: row.log_path,
    };
  }

  /**
   * Store agent message in database
   */
  private async storeAgentMessage(message: AgentMessage): Promise<void> {
    const db = getDatabase();
    
    await db.query(`
      INSERT INTO agent_messages (
        id, agent_id, type, content, timestamp, metadata
      ) VALUES ($1, $2, $3, $4, $5, $6)
    `, [
      message.id,
      message.agentId,
      message.type,
      message.content,
      message.timestamp,
      JSON.stringify(message.metadata || {}),
    ]);
  }

  /**
   * Generate unique agent ID
   */
  private generateAgentId(name: string): string {
    const timestamp = Date.now();
    const random = Math.random().toString(36).substr(2, 6);
    return `${name}_${timestamp}_${random}`;
  }

  /**
   * Create log file for agent
   */
  private async createLogFile(agentId: string): Promise<string> {
    const logDir = path.join(process.cwd(), 'logs', 'agents');
    await fs.mkdir(logDir, { recursive: true });
    
    const logFile = path.join(logDir, `${agentId}.log`);
    await fs.writeFile(logFile, ''); // Create empty log file
    
    return logFile;
  }

  /**
   * Spawn a new agent
   */
  async spawnAgent(
    options: AgentSpawnOptions,
    userId?: string,
  ): Promise<Result<AgentInfo>> {
    try {
      this.checkAuthorization(userId, 'write');
      this.validateAgentOptions(options);

      // Check max agents limit
      const activeAgents = await this.listAgents(userId, { status: ['running', 'starting'] });
      if (activeAgents.success && activeAgents.data.length >= this.maxAgents) {
        return failure(
          new ServiceError(
            `Maximum number of agents (${this.maxAgents}) reached`,
            'MAX_AGENTS_EXCEEDED',
            429,
            undefined,
            this.correlationId,
          ),
          this.correlationId,
        );
      }

      const agentId = this.generateAgentId(options.name);
      const logPath = await this.createLogFile(agentId);
      const workingDirectory = options.workingDirectory || process.cwd();

      // Create agent info
      const agentInfo: AgentInfo = {
        id: agentId,
        name: options.name,
        type: options.type,
        status: 'starting',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        createdBy: userId,
        config: options.config,
        metadata: options.metadata,
        workingDirectory,
        logPath,
      };

      await this.storeAgentInfo(agentInfo);

      // Prepare spawn environment
      const env = {
        ...process.env,
        ...options.env,
        AGENT_ID: agentId,
        AGENT_NAME: options.name,
        AGENT_LOG_PATH: logPath,
      };

      // Spawn the process
      const child = spawn(options.command, options.args || [], {
        cwd: workingDirectory,
        env,
        stdio: ['pipe', 'pipe', 'pipe'],
        detached: false,
      });

      // Store process reference
      this.runningAgents.set(agentId, child);

      // Update agent info with PID
      agentInfo.pid = child.pid;
      agentInfo.status = 'running';
      agentInfo.updatedAt = new Date().toISOString();
      await this.storeAgentInfo(agentInfo);

      // Setup process event handlers
      this.setupProcessHandlers(agentId, child, logPath);

      // Setup timeout if specified
      const timeout = options.timeout || this.defaultTimeout;
      if (timeout > 0) {
        setTimeout(() => {
          if (this.runningAgents.has(agentId)) {
            this.terminateAgent(agentId, userId, 'timeout');
          }
        }, timeout);
      }

      // Clear cache
      await this.clearCachePattern('agents:*');

      await this.auditLog('agent', 'spawn_agent', true, {
        userId,
        resource: agentId,
        metadata: { 
          name: options.name,
          type: options.type,
          command: options.command,
          pid: child.pid,
        },
      });

      // Emit event
      this.eventEmitter.emit('agent:spawned', agentInfo);

      return success(agentInfo, this.correlationId);
    } catch (error) {
      await this.auditLog('agent', 'spawn_agent', false, {
        userId,
        error: error.message,
        metadata: { options },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to spawn agent: ${error.message}`,
          'AGENT_SPAWN_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Setup process event handlers
   */
  private setupProcessHandlers(agentId: string, child: ChildProcess, logPath: string): void {
    const logStream = require('fs').createWriteStream(logPath, { flags: 'a' });

    // Handle stdout
    child.stdout?.on('data', (data) => {
      const content = data.toString();
      logStream.write(`[STDOUT] ${new Date().toISOString()} ${content}`);
      
      this.storeAgentMessage({
        id: `${agentId}_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
        agentId,
        type: 'info',
        content: content.trim(),
        timestamp: new Date().toISOString(),
      });
    });

    // Handle stderr
    child.stderr?.on('data', (data) => {
      const content = data.toString();
      logStream.write(`[STDERR] ${new Date().toISOString()} ${content}`);
      
      this.storeAgentMessage({
        id: `${agentId}_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
        agentId,
        type: 'error',
        content: content.trim(),
        timestamp: new Date().toISOString(),
      });
    });

    // Handle process exit
    child.on('exit', async (code, signal) => {
      logStream.write(`[EXIT] ${new Date().toISOString()} Process exited with code ${code}, signal ${signal}\n`);
      logStream.end();

      this.runningAgents.delete(agentId);
      this.agentMetrics.delete(agentId);

      // Update agent status
      const agent = await this.getAgentInfoFromDb(agentId);
      if (agent) {
        agent.status = code === 0 ? 'stopped' : 'failed';
        agent.updatedAt = new Date().toISOString();
        await this.storeAgentInfo(agent);

        // Emit event
        this.eventEmitter.emit('agent:exited', { agentId, code, signal });
      }
    });

    // Handle process error
    child.on('error', async (error) => {
      logStream.write(`[ERROR] ${new Date().toISOString()} Process error: ${error.message}\n`);
      
      this.runningAgents.delete(agentId);
      this.agentMetrics.delete(agentId);

      // Update agent status
      const agent = await this.getAgentInfoFromDb(agentId);
      if (agent) {
        agent.status = 'failed';
        agent.updatedAt = new Date().toISOString();
        await this.storeAgentInfo(agent);

        // Emit event
        this.eventEmitter.emit('agent:error', { agentId, error });
      }
    });
  }

  /**
   * List agents with optional filtering
   */
  async listAgents(
    userId?: string,
    filters: { status?: string[]; type?: string; createdBy?: string } = {},
  ): Promise<Result<AgentInfo[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const cacheKey = `agents:list:${JSON.stringify(filters)}`;
      
      const agents = await this.withCache(
        cacheKey,
        async () => {
          const db = getDatabase();
          let query = 'SELECT * FROM agent_runs WHERE 1=1';
          const params: any[] = [];
          let paramIndex = 1;

          if (filters.status && filters.status.length > 0) {
            query += ` AND status = ANY($${paramIndex})`;
            params.push(filters.status);
            paramIndex++;
          }

          if (filters.type) {
            query += ` AND type = $${paramIndex}`;
            params.push(filters.type);
            paramIndex++;
          }

          if (filters.createdBy) {
            query += ` AND created_by = $${paramIndex}`;
            params.push(filters.createdBy);
            paramIndex++;
          }

          query += ' ORDER BY created_at DESC';

          const result = await db.query(query, params);

          return result.rows.map(row => ({
            id: row.id,
            name: row.name,
            type: row.type,
            status: row.status,
            pid: row.pid,
            createdAt: row.created_at,
            updatedAt: row.updated_at,
            createdBy: row.created_by,
            config: row.config || {},
            metadata: row.metadata || {},
            workingDirectory: row.working_directory,
            logPath: row.log_path,
          }));
        },
        60, // 1 minute cache
      );

      await this.auditLog('agent', 'list_agents', true, {
        userId,
        metadata: { filters, count: agents.length },
      });

      return success(agents, this.correlationId);
    } catch (error) {
      await this.auditLog('agent', 'list_agents', false, {
        userId,
        error: error.message,
        metadata: { filters },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to list agents: ${error.message}`,
          'AGENT_LIST_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get specific agent info
   */
  async getAgent(id: string, userId?: string): Promise<Result<AgentInfo | null>> {
    try {
      this.checkAuthorization(userId, 'read');

      const cacheKey = `agents:info:${id}`;
      
      const agent = await this.withCache(
        cacheKey,
        async () => {
          return await this.getAgentInfoFromDb(id);
        },
        60, // 1 minute cache
      );

      return success(agent, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get agent info: ${error.message}`,
          'AGENT_GET_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Terminate an agent
   */
  async terminateAgent(
    id: string, 
    userId?: string, 
    reason = 'manual',
  ): Promise<Result<void>> {
    try {
      this.checkAuthorization(userId, 'write');

      const agent = await this.getAgentInfoFromDb(id);
      if (!agent) {
        return failure(
          new ServiceError(
            `Agent '${id}' not found`,
            'AGENT_NOT_FOUND',
            404,
            undefined,
            this.correlationId,
          ),
          this.correlationId,
        );
      }

      const process = this.runningAgents.get(id);
      if (process) {
        process.kill('SIGTERM');
        
        // Give it time to gracefully shutdown, then force kill
        setTimeout(() => {
          if (this.runningAgents.has(id)) {
            process.kill('SIGKILL');
          }
        }, 5000);
      }

      // Update status
      agent.status = 'terminated';
      agent.updatedAt = new Date().toISOString();
      await this.storeAgentInfo(agent);

      // Clear cache
      await this.clearCachePattern('agents:*');

      await this.auditLog('agent', 'terminate_agent', true, {
        userId,
        resource: id,
        metadata: { reason, pid: agent.pid },
      });

      // Emit event
      this.eventEmitter.emit('agent:terminated', { agentId: id, reason, userId });

      return success(undefined, this.correlationId);
    } catch (error) {
      await this.auditLog('agent', 'terminate_agent', false, {
        userId,
        resource: id,
        error: error.message,
        metadata: { reason },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to terminate agent: ${error.message}`,
          'AGENT_TERMINATE_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get agent metrics
   */
  async getAgentMetrics(id: string, userId?: string): Promise<Result<AgentMetrics | null>> {
    try {
      this.checkAuthorization(userId, 'read');

      const metrics = this.agentMetrics.get(id);
      if (!metrics) {
        return success(null, this.correlationId);
      }

      return success(metrics, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get agent metrics: ${error.message}`,
          'AGENT_METRICS_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get agent logs
   */
  async getAgentLogs(
    filters: AgentLogFilter,
    userId?: string,
  ): Promise<Result<AgentMessage[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const db = getDatabase();
      let query = 'SELECT * FROM agent_messages WHERE 1=1';
      const params: any[] = [];
      let paramIndex = 1;

      if (filters.agentId) {
        query += ` AND agent_id = $${paramIndex}`;
        params.push(filters.agentId);
        paramIndex++;
      }

      if (filters.type && filters.type.length > 0) {
        query += ` AND type = ANY($${paramIndex})`;
        params.push(filters.type);
        paramIndex++;
      }

      if (filters.startTime) {
        query += ` AND timestamp >= $${paramIndex}`;
        params.push(filters.startTime);
        paramIndex++;
      }

      if (filters.endTime) {
        query += ` AND timestamp <= $${paramIndex}`;
        params.push(filters.endTime);
        paramIndex++;
      }

      query += ' ORDER BY timestamp DESC';

      if (filters.limit) {
        query += ` LIMIT $${paramIndex}`;
        params.push(filters.limit);
        paramIndex++;
      }

      if (filters.offset) {
        query += ` OFFSET $${paramIndex}`;
        params.push(filters.offset);
        paramIndex++;
      }

      const result = await db.query(query, params);

      const logs = result.rows.map(row => ({
        id: row.id,
        agentId: row.agent_id,
        type: row.type,
        content: row.content,
        timestamp: row.timestamp,
        metadata: row.metadata || {},
      }));

      return success(logs, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get agent logs: ${error.message}`,
          'AGENT_LOGS_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Subscribe to agent events
   */
  subscribe(event: string, callback: (data: any) => void): void {
    this.eventEmitter.on(event, callback);
  }

  /**
   * Unsubscribe from agent events
   */
  unsubscribe(event: string, callback: (data: any) => void): void {
    this.eventEmitter.off(event, callback);
  }

  /**
   * Update metrics for all running agents
   */
  private async updateMetrics(): Promise<void> {
    for (const [agentId, process] of this.runningAgents.entries()) {
      try {
        // Get process statistics (this would need a proper implementation)
        // For now, we'll generate mock metrics
        const metrics: AgentMetrics = {
          agentId,
          cpu: Math.random() * 100,
          memory: Math.random() * 1024,
          uptime: Date.now() - parseInt(agentId.split('_')[1]),
          messagesProcessed: Math.floor(Math.random() * 1000),
          errorsCount: Math.floor(Math.random() * 10),
          lastActivity: new Date().toISOString(),
          timestamp: new Date().toISOString(),
        };

        this.agentMetrics.set(agentId, metrics);
      } catch (error) {
        console.warn(`Failed to update metrics for agent ${agentId}:`, error);
      }
    }
  }

  /**
   * Cleanup old logs
   */
  private async cleanupLogs(): Promise<void> {
    try {
      const cutoffDate = new Date();
      cutoffDate.setDate(cutoffDate.getDate() - this.logRetentionDays);

      const db = getDatabase();
      await db.query(`
        DELETE FROM agent_messages 
        WHERE timestamp < $1
      `, [cutoffDate.toISOString()]);

      // Also cleanup log files
      const logDir = path.join(process.cwd(), 'logs', 'agents');
      try {
        const files = await fs.readdir(logDir);
        for (const file of files) {
          const filePath = path.join(logDir, file);
          const stats = await fs.stat(filePath);
          if (stats.mtime < cutoffDate) {
            await fs.unlink(filePath);
          }
        }
      } catch (error) {
        console.warn('Failed to cleanup log files:', error);
      }
    } catch (error) {
      console.warn('Agent log cleanup failed:', error);
    }
  }

  /**
   * Health check for the Agent service
   */
  async healthCheck(): Promise<{ healthy: boolean; details?: any }> {
    try {
      const runningCount = this.runningAgents.size;
      const maxAgents = this.maxAgents;
      const resourceUsage = Array.from(this.agentMetrics.values()).reduce(
        (acc, metrics) => ({
          cpu: acc.cpu + metrics.cpu,
          memory: acc.memory + metrics.memory,
        }),
        { cpu: 0, memory: 0 },
      );

      return {
        healthy: true,
        details: {
          runningAgents: runningCount,
          maxAgents,
          resourceUsage,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    } catch (error) {
      return {
        healthy: false,
        details: {
          error: error.message,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    }
  }
}

// Export factory function for dependency injection
export function createAgentService(
  config?: AgentConfig,
  cache?: CacheProvider,
  auditLogger?: AuditLogger,
): AgentService {
  return new AgentService(config, cache, auditLogger);
}