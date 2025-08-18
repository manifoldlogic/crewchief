/**
 * Database Connection Pool and Configuration
 * 
 * This module provides a PostgreSQL connection pool for the CrewChief web UI.
 * It connects to the same database used by Maproom for seamless integration.
 */

import { Pool, PoolConfig, PoolClient, QueryConfig, QueryResult } from 'pg';

export interface DatabaseConfig {
  host: string;
  port: number;
  database: string;
  user: string;
  password: string;
  ssl?: boolean | object;
  connectionTimeoutMillis?: number;
  idleTimeoutMillis?: number;
  max?: number; // Maximum number of clients in pool
  min?: number; // Minimum number of clients in pool
}

export interface QueryOptions {
  timeout?: number;
  retries?: number;
  retryDelay?: number;
}

export class DatabaseConnection {
  private pool: Pool;
  private config: DatabaseConfig;
  private isConnected: boolean = false;

  constructor(config: DatabaseConfig) {
    this.config = config;
    
    const poolConfig: PoolConfig = {
      host: config.host,
      port: config.port,
      database: config.database,
      user: config.user,
      password: config.password,
      ssl: config.ssl,
      connectionTimeoutMillis: config.connectionTimeoutMillis || 5000,
      idleTimeoutMillis: config.idleTimeoutMillis || 30000,
      max: config.max || 20, // Maximum pool size
      min: config.min || 5,  // Minimum pool size
      statement_timeout: 30000, // 30 second statement timeout
      query_timeout: 30000,     // 30 second query timeout
    };

    this.pool = new Pool(poolConfig);
    this.setupEventHandlers();
  }

  private setupEventHandlers(): void {
    this.pool.on('connect', (client: PoolClient) => {
      console.log('Database client connected');
      this.isConnected = true;
    });

    this.pool.on('error', (err: Error) => {
      console.error('Database pool error:', err);
      this.isConnected = false;
    });

    this.pool.on('remove', () => {
      console.log('Database client removed from pool');
    });
  }

  /**
   * Initialize the database connection and verify connectivity
   */
  async initialize(): Promise<void> {
    try {
      const client = await this.pool.connect();
      
      // Test the connection
      await client.query('SELECT NOW()');
      
      // Verify Maproom schema exists
      const { rows } = await client.query(`
        SELECT schema_name 
        FROM information_schema.schemata 
        WHERE schema_name = 'maproom'
      `);
      
      if (rows.length === 0) {
        throw new Error('Maproom schema not found in database');
      }
      
      client.release();
      this.isConnected = true;
      console.log('Database connection initialized successfully');
    } catch (error) {
      this.isConnected = false;
      console.error('Failed to initialize database connection:', error);
      throw error;
    }
  }

  /**
   * Execute a query with optional parameters
   */
  async query<T = any>(
    text: string, 
    params?: any[], 
    options: QueryOptions = {}
  ): Promise<QueryResult<T>> {
    const { timeout = 30000, retries = 2, retryDelay = 1000 } = options;
    
    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        const queryConfig: QueryConfig = {
          text,
          values: params,
          statement_timeout: timeout,
        };
        
        return await this.pool.query<T>(queryConfig);
      } catch (error) {
        if (attempt === retries) {
          console.error('Database query failed after retries:', error);
          throw error;
        }
        
        console.warn(`Database query attempt ${attempt + 1} failed, retrying:`, error);
        await new Promise(resolve => setTimeout(resolve, retryDelay));
      }
    }
    
    throw new Error('Query failed after all retries');
  }

  /**
   * Execute a query within a transaction
   */
  async transaction<T>(
    callback: (client: PoolClient) => Promise<T>
  ): Promise<T> {
    const client = await this.pool.connect();
    
    try {
      await client.query('BEGIN');
      const result = await callback(client);
      await client.query('COMMIT');
      return result;
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  /**
   * Get a client from the pool for multiple operations
   */
  async getClient(): Promise<PoolClient> {
    return await this.pool.connect();
  }

  /**
   * Check if the database connection is healthy
   */
  async healthCheck(): Promise<boolean> {
    try {
      const result = await this.query('SELECT 1 as health');
      return result.rows.length > 0 && result.rows[0].health === 1;
    } catch (error) {
      console.error('Database health check failed:', error);
      return false;
    }
  }

  /**
   * Get connection pool statistics
   */
  getPoolStats(): {
    totalCount: number;
    idleCount: number;
    waitingCount: number;
  } {
    return {
      totalCount: this.pool.totalCount,
      idleCount: this.pool.idleCount,
      waitingCount: this.pool.waitingCount,
    };
  }

  /**
   * Close all connections in the pool
   */
  async close(): Promise<void> {
    await this.pool.end();
    this.isConnected = false;
    console.log('Database connection pool closed');
  }

  /**
   * Check if connected
   */
  get connected(): boolean {
    return this.isConnected;
  }

  /**
   * Get the underlying connection pool
   */
  getPool(): Pool {
    return this.pool;
  }
}

/**
 * Database configuration factory
 */
export function createDatabaseConfig(): DatabaseConfig {
  return {
    host: process.env.CREWCHIEF_DB_HOST || 'localhost',
    port: parseInt(process.env.CREWCHIEF_DB_PORT || '5432'),
    database: process.env.CREWCHIEF_DB_NAME || 'crewchief',
    user: process.env.CREWCHIEF_DB_USER || 'postgres',
    password: process.env.CREWCHIEF_DB_PASSWORD || '',
    ssl: process.env.CREWCHIEF_DB_SSL === 'true' ? true : false,
    connectionTimeoutMillis: 5000,
    idleTimeoutMillis: 30000,
    max: parseInt(process.env.CREWCHIEF_DB_POOL_MAX || '20'),
    min: parseInt(process.env.CREWCHIEF_DB_POOL_MIN || '5'),
  };
}

// Singleton database instance
let dbInstance: DatabaseConnection | null = null;

/**
 * Get the singleton database instance
 */
export function getDatabase(): DatabaseConnection {
  if (!dbInstance) {
    const config = createDatabaseConfig();
    dbInstance = new DatabaseConnection(config);
  }
  return dbInstance;
}

/**
 * Initialize the database connection (call once at startup)
 */
export async function initializeDatabase(): Promise<DatabaseConnection> {
  const db = getDatabase();
  await db.initialize();
  return db;
}

/**
 * Close the database connection (call at shutdown)
 */
export async function closeDatabase(): Promise<void> {
  if (dbInstance) {
    await dbInstance.close();
    dbInstance = null;
  }
}