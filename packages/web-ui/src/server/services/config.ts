/**
 * Config Service
 * 
 * Service layer for configuration management including loading, saving, and validation.
 * Implements the Result pattern, caching, audit logging, and authorization.
 */

import fs from 'node:fs/promises';
import path from 'node:path';
import { z } from 'zod';
import { 
  BaseService, 
  Result, 
  success, 
  failure, 
  ServiceError, 
  ValidationError,
  ServiceConfig,
  CacheProvider,
  AuditLogger,
} from './base.js';
import { getDatabase } from '../../db/connection.js';

export interface ConfigServiceConfig extends ServiceConfig {
  configDirectory?: string;
  backupRetention?: number;
  validateOnLoad?: boolean;
  encryptSensitive?: boolean;
}

export interface ConfigInfo {
  id: string;
  name: string;
  type: 'user' | 'system' | 'environment';
  version: string;
  data: Record<string, any>;
  schema?: string;
  encrypted: boolean;
  createdAt: string;
  updatedAt: string;
  createdBy?: string;
  description?: string;
}

export interface ConfigUpdateOptions {
  version?: string;
  description?: string;
  createBackup?: boolean;
  validateSchema?: boolean;
}

export interface ConfigBackup {
  id: string;
  configId: string;
  version: string;
  data: Record<string, any>;
  createdAt: string;
  createdBy?: string;
  reason?: string;
}

// Schema definitions for different config types
const AgentConfigSchema = z.object({
  name: z.string().min(1),
  type: z.string(),
  command: z.string(),
  args: z.array(z.string()).optional(),
  env: z.record(z.string()).optional(),
  timeout: z.number().positive().optional(),
  retries: z.number().min(0).optional(),
});

const WorktreeConfigSchema = z.object({
  baseRepoPath: z.string(),
  worktreeBasePath: z.string(),
  maxWorktrees: z.number().positive(),
  autoCleanup: z.boolean(),
  cleanupAfterDays: z.number().positive(),
  copyPatterns: z.array(z.string()).optional(),
});

const MaproomConfigSchema = z.object({
  binaryPath: z.string().optional(),
  timeout: z.number().positive(),
  retries: z.number().min(0),
  retryDelay: z.number().positive(),
  cacheEnabled: z.boolean(),
  cacheTtl: z.number().positive(),
});

const DatabaseConfigSchema = z.object({
  host: z.string(),
  port: z.number().min(1).max(65535),
  database: z.string(),
  username: z.string(),
  password: z.string(),
  ssl: z.boolean().optional(),
  poolSize: z.number().positive().optional(),
});

const RedisConfigSchema = z.object({
  url: z.string().url(),
  enabled: z.boolean(),
  ttl: z.number().positive(),
  maxRetries: z.number().min(0).optional(),
});

const SystemConfigSchema = z.object({
  app: z.object({
    name: z.string(),
    version: z.string(),
    environment: z.enum(['development', 'staging', 'production']),
    port: z.number().min(1).max(65535),
    host: z.string(),
  }),
  security: z.object({
    jwtSecret: z.string().min(32),
    sessionSecret: z.string().min(32),
    corsOrigins: z.array(z.string()),
    rateLimit: z.object({
      windowMs: z.number().positive(),
      max: z.number().positive(),
    }),
  }),
  database: DatabaseConfigSchema,
  redis: RedisConfigSchema.optional(),
  agents: z.object({
    maxAgents: z.number().positive(),
    defaultTimeout: z.number().positive(),
    resourceLimits: z.object({
      memory: z.number().positive(),
      cpu: z.number().min(0).max(100),
    }),
  }),
  worktrees: WorktreeConfigSchema,
  maproom: MaproomConfigSchema,
});

const SCHEMA_REGISTRY = {
  system: SystemConfigSchema,
  agent: AgentConfigSchema,
  worktree: WorktreeConfigSchema,
  maproom: MaproomConfigSchema,
  database: DatabaseConfigSchema,
  redis: RedisConfigSchema,
};

export class ConfigService extends BaseService {
  private configDirectory: string;
  private backupRetention: number;
  private validateOnLoad: boolean;
  private encryptSensitive: boolean;

  constructor(
    config: ConfigServiceConfig = {},
    cache?: CacheProvider,
    auditLogger?: AuditLogger,
  ) {
    super(config, cache, auditLogger);
    
    this.configDirectory = config.configDirectory || path.join(process.cwd(), 'config');
    this.backupRetention = config.backupRetention || 10;
    this.validateOnLoad = config.validateOnLoad ?? true;
    this.encryptSensitive = config.encryptSensitive ?? true;
  }

  /**
   * Validate configuration data against schema
   */
  private validateConfigData(data: any, schemaName: string): { valid: boolean; errors?: string[] } {
    const schema = SCHEMA_REGISTRY[schemaName as keyof typeof SCHEMA_REGISTRY];
    if (!schema) {
      return { valid: false, errors: [`Unknown schema: ${schemaName}`] };
    }

    try {
      schema.parse(data);
      return { valid: true };
    } catch (error) {
      if (error instanceof z.ZodError) {
        return {
          valid: false,
          errors: error.errors.map(err => `${err.path.join('.')}: ${err.message}`),
        };
      }
      return { valid: false, errors: [error.message] };
    }
  }

  /**
   * Get config file path
   */
  private getConfigFilePath(name: string): string {
    return path.join(this.configDirectory, `${name}.json`);
  }

  /**
   * Store config info in database
   */
  private async storeConfigInfo(config: ConfigInfo): Promise<void> {
    const db = getDatabase();
    
    await db.query(`
      INSERT INTO system_config (
        id, name, type, version, data, schema, encrypted, created_at, updated_at,
        created_by, description
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
      ON CONFLICT (id) DO UPDATE SET
        type = EXCLUDED.type,
        version = EXCLUDED.version,
        data = EXCLUDED.data,
        schema = EXCLUDED.schema,
        encrypted = EXCLUDED.encrypted,
        updated_at = EXCLUDED.updated_at,
        description = EXCLUDED.description
    `, [
      config.id,
      config.name,
      config.type,
      config.version,
      JSON.stringify(config.data),
      config.schema,
      config.encrypted,
      config.createdAt,
      config.updatedAt,
      config.createdBy,
      config.description,
    ]);
  }

  /**
   * Get config info from database
   */
  private async getConfigInfoFromDb(id: string): Promise<ConfigInfo | null> {
    const db = getDatabase();
    
    const result = await db.query(`
      SELECT * FROM system_config WHERE id = $1
    `, [id]);

    if (result.rows.length === 0) {
      return null;
    }

    const row = result.rows[0];
    let data = row.data;
    
    // Decrypt if necessary
    if (row.encrypted && this.config.security?.encryptionKey) {
      try {
        data = JSON.parse(this.decrypt(JSON.stringify(data)));
      } catch (error) {
        console.warn('Failed to decrypt config data:', error);
      }
    }

    return {
      id: row.id,
      name: row.name,
      type: row.type,
      version: row.version,
      data,
      schema: row.schema,
      encrypted: row.encrypted,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      createdBy: row.created_by,
      description: row.description,
    };
  }

  /**
   * Store config backup
   */
  private async storeConfigBackup(backup: ConfigBackup): Promise<void> {
    const db = getDatabase();
    
    await db.query(`
      INSERT INTO config_backups (
        id, config_id, version, data, created_at, created_by, reason
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
    `, [
      backup.id,
      backup.configId,
      backup.version,
      JSON.stringify(backup.data),
      backup.createdAt,
      backup.createdBy,
      backup.reason,
    ]);
  }

  /**
   * Determine if data contains sensitive information
   */
  private containsSensitiveData(data: any): boolean {
    const sensitiveKeys = ['password', 'secret', 'key', 'token', 'credential'];
    const dataStr = JSON.stringify(data).toLowerCase();
    return sensitiveKeys.some(key => dataStr.includes(key));
  }

  /**
   * Generate next version number
   */
  private generateNextVersion(currentVersion?: string): string {
    if (!currentVersion) {
      return '1.0.0';
    }

    const [major, minor, patch] = currentVersion.split('.').map(Number);
    return `${major}.${minor}.${patch + 1}`;
  }

  /**
   * Load configuration
   */
  async loadConfig(
    name: string,
    userId?: string,
  ): Promise<Result<ConfigInfo>> {
    try {
      this.checkAuthorization(userId, 'read');

      const cacheKey = `config:${name}`;
      
      const config = await this.withCache(
        cacheKey,
        async () => {
          // Try database first
          let configInfo = await this.getConfigInfoFromDb(name);
          
          // If not in database, try loading from file
          if (!configInfo) {
            const filePath = this.getConfigFilePath(name);
            try {
              const fileContent = await fs.readFile(filePath, 'utf-8');
              const data = JSON.parse(fileContent);
              
              configInfo = {
                id: name,
                name,
                type: 'system',
                version: '1.0.0',
                data,
                encrypted: false,
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                description: `Loaded from file: ${filePath}`,
              };

              // Store in database
              await this.storeConfigInfo(configInfo);
            } catch (fileError) {
              throw new ServiceError(
                `Configuration '${name}' not found`,
                'CONFIG_NOT_FOUND',
                404,
                undefined,
                this.correlationId,
              );
            }
          }

          // Validate if enabled
          if (this.validateOnLoad && configInfo.schema) {
            const validation = this.validateConfigData(configInfo.data, configInfo.schema);
            if (!validation.valid) {
              throw new ValidationError(
                `Configuration validation failed: ${validation.errors?.join(', ')}`,
                { errors: validation.errors },
                this.correlationId,
              );
            }
          }

          return configInfo;
        },
        300, // 5 minutes cache
      );

      await this.auditLog('config', 'load_config', true, {
        userId,
        resource: name,
        metadata: { 
          version: config.version,
          type: config.type,
          hasSchema: !!config.schema,
        },
      });

      return success(config, this.correlationId);
    } catch (error) {
      await this.auditLog('config', 'load_config', false, {
        userId,
        resource: name,
        error: error.message,
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to load configuration: ${error.message}`,
          'CONFIG_LOAD_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Save configuration
   */
  async saveConfig(
    name: string,
    data: Record<string, any>,
    options: ConfigUpdateOptions = {},
    userId?: string,
  ): Promise<Result<ConfigInfo>> {
    try {
      this.checkAuthorization(userId, 'write');

      // Validate schema if provided
      const schemaName = options.validateSchema !== false ? name : undefined;
      if (schemaName && SCHEMA_REGISTRY[schemaName as keyof typeof SCHEMA_REGISTRY]) {
        const validation = this.validateConfigData(data, schemaName);
        if (!validation.valid) {
          return failure(
            new ValidationError(
              `Configuration validation failed: ${validation.errors?.join(', ')}`,
              { errors: validation.errors },
              this.correlationId,
            ),
            this.correlationId,
          );
        }
      }

      // Get existing config for versioning
      const existingConfig = await this.getConfigInfoFromDb(name);
      
      // Create backup if requested
      if (options.createBackup !== false && existingConfig) {
        const backup: ConfigBackup = {
          id: `${name}_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
          configId: name,
          version: existingConfig.version,
          data: existingConfig.data,
          createdAt: new Date().toISOString(),
          createdBy: userId,
          reason: 'automatic_backup',
        };
        await this.storeConfigBackup(backup);
      }

      // Determine if encryption is needed
      const needsEncryption = this.encryptSensitive && this.containsSensitiveData(data);
      
      // Prepare encrypted data if needed
      let processedData = data;
      if (needsEncryption && this.config.security?.encryptionKey) {
        processedData = JSON.parse(this.encrypt(JSON.stringify(data)));
      }

      const configInfo: ConfigInfo = {
        id: name,
        name,
        type: existingConfig?.type || 'user',
        version: options.version || this.generateNextVersion(existingConfig?.version),
        data: processedData,
        schema: schemaName,
        encrypted: needsEncryption,
        createdAt: existingConfig?.createdAt || new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        createdBy: existingConfig?.createdBy || userId,
        description: options.description || existingConfig?.description,
      };

      // Store in database
      await this.storeConfigInfo(configInfo);

      // Also save to file for persistence
      const filePath = this.getConfigFilePath(name);
      await fs.mkdir(this.configDirectory, { recursive: true });
      await fs.writeFile(filePath, JSON.stringify(data, null, 2));

      // Clear cache
      await this.clearCachePattern(`config:${name}`);

      await this.auditLog('config', 'save_config', true, {
        userId,
        resource: name,
        metadata: { 
          version: configInfo.version,
          encrypted: needsEncryption,
          hasSchema: !!schemaName,
          backupCreated: options.createBackup !== false && !!existingConfig,
        },
      });

      // Return config with original (unencrypted) data
      return success({
        ...configInfo,
        data,
      }, this.correlationId);
    } catch (error) {
      await this.auditLog('config', 'save_config', false, {
        userId,
        resource: name,
        error: error.message,
        metadata: { options },
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to save configuration: ${error.message}`,
          'CONFIG_SAVE_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * List all configurations
   */
  async listConfigs(userId?: string): Promise<Result<ConfigInfo[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const cacheKey = 'config:list';
      
      const configs = await this.withCache(
        cacheKey,
        async () => {
          const db = getDatabase();
          const result = await db.query(`
            SELECT id, name, type, version, schema, encrypted, created_at, updated_at,
                   created_by, description
            FROM system_config 
            ORDER BY name
          `);

          return result.rows.map(row => ({
            id: row.id,
            name: row.name,
            type: row.type,
            version: row.version,
            data: {}, // Don't include data in list view
            schema: row.schema,
            encrypted: row.encrypted,
            createdAt: row.created_at,
            updatedAt: row.updated_at,
            createdBy: row.created_by,
            description: row.description,
          }));
        },
        60, // 1 minute cache
      );

      await this.auditLog('config', 'list_configs', true, {
        userId,
        metadata: { count: configs.length },
      });

      return success(configs, this.correlationId);
    } catch (error) {
      await this.auditLog('config', 'list_configs', false, {
        userId,
        error: error.message,
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to list configurations: ${error.message}`,
          'CONFIG_LIST_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Delete configuration
   */
  async deleteConfig(name: string, userId?: string): Promise<Result<void>> {
    try {
      this.checkAuthorization(userId, 'write');

      const existingConfig = await this.getConfigInfoFromDb(name);
      if (!existingConfig) {
        return failure(
          new ServiceError(
            `Configuration '${name}' not found`,
            'CONFIG_NOT_FOUND',
            404,
            undefined,
            this.correlationId,
          ),
          this.correlationId,
        );
      }

      // Create final backup
      const backup: ConfigBackup = {
        id: `${name}_final_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
        configId: name,
        version: existingConfig.version,
        data: existingConfig.data,
        createdAt: new Date().toISOString(),
        createdBy: userId,
        reason: 'deletion_backup',
      };
      await this.storeConfigBackup(backup);

      // Delete from database
      const db = getDatabase();
      await db.query('DELETE FROM system_config WHERE id = $1', [name]);

      // Delete file if it exists
      const filePath = this.getConfigFilePath(name);
      try {
        await fs.unlink(filePath);
      } catch {
        // File might not exist, ignore
      }

      // Clear cache
      await this.clearCachePattern(`config:${name}`);

      await this.auditLog('config', 'delete_config', true, {
        userId,
        resource: name,
        metadata: { 
          version: existingConfig.version,
          backupId: backup.id,
        },
      });

      return success(undefined, this.correlationId);
    } catch (error) {
      await this.auditLog('config', 'delete_config', false, {
        userId,
        resource: name,
        error: error.message,
      });

      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to delete configuration: ${error.message}`,
          'CONFIG_DELETE_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Validate configuration data
   */
  async validateConfig(
    data: Record<string, any>,
    schemaName: string,
    userId?: string,
  ): Promise<Result<{ valid: boolean; errors?: string[] }>> {
    try {
      this.checkAuthorization(userId, 'read');

      const validation = this.validateConfigData(data, schemaName);

      await this.auditLog('config', 'validate_config', true, {
        userId,
        metadata: { 
          schemaName,
          valid: validation.valid,
          errorCount: validation.errors?.length || 0,
        },
      });

      return success(validation, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to validate configuration: ${error.message}`,
          'CONFIG_VALIDATE_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Get configuration backups
   */
  async getConfigBackups(configId: string, userId?: string): Promise<Result<ConfigBackup[]>> {
    try {
      this.checkAuthorization(userId, 'read');

      const db = getDatabase();
      const result = await db.query(`
        SELECT * FROM config_backups 
        WHERE config_id = $1 
        ORDER BY created_at DESC 
        LIMIT $2
      `, [configId, this.backupRetention]);

      const backups = result.rows.map(row => ({
        id: row.id,
        configId: row.config_id,
        version: row.version,
        data: row.data,
        createdAt: row.created_at,
        createdBy: row.created_by,
        reason: row.reason,
      }));

      return success(backups, this.correlationId);
    } catch (error) {
      if (error instanceof ServiceError) {
        return failure(error, this.correlationId);
      }

      return failure(
        new ServiceError(
          `Failed to get configuration backups: ${error.message}`,
          'CONFIG_BACKUPS_FAILED',
          500,
          { originalError: error.message },
          this.correlationId,
        ),
        this.correlationId,
      );
    }
  }

  /**
   * Health check for the Config service
   */
  async healthCheck(): Promise<{ healthy: boolean; details?: any }> {
    try {
      // Check if config directory is accessible
      await fs.access(this.configDirectory);
      
      // Check database connectivity
      const db = getDatabase();
      await db.query('SELECT 1');

      return {
        healthy: true,
        details: {
          configDirectory: this.configDirectory,
          backupRetention: this.backupRetention,
          validateOnLoad: this.validateOnLoad,
          encryptSensitive: this.encryptSensitive,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    } catch (error) {
      return {
        healthy: false,
        details: {
          error: error.message,
          configDirectory: this.configDirectory,
          cacheAvailable: this.cache.isAvailable(),
        },
      };
    }
  }
}

// Export factory function for dependency injection
export function createConfigService(
  config?: ConfigServiceConfig,
  cache?: CacheProvider,
  auditLogger?: AuditLogger,
): ConfigService {
  return new ConfigService(config, cache, auditLogger);
}