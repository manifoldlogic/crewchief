import { Pool, PoolClient } from 'pg';
import { readFileSync } from 'fs';
import { join } from 'path';

let testPool: Pool | null = null;

/**
 * Database configuration for tests
 */
export const TEST_DATABASE_CONFIG = {
  host: process.env.PGHOST || 'localhost',
  port: parseInt(process.env.PGPORT || '5432', 10),
  user: process.env.PGUSER || 'test',
  password: process.env.PGPASSWORD || 'test',
  database: process.env.PGDATABASE || 'crewchief_test',
  max: 5,
  idleTimeoutMillis: 1000,
  connectionTimeoutMillis: 5000,
};

/**
 * Get or create a database connection pool for tests
 */
export async function getTestDatabase(): Promise<Pool> {
  if (!testPool) {
    testPool = new Pool(TEST_DATABASE_CONFIG);
    
    // Test the connection
    try {
      const client = await testPool.connect();
      client.release();
    } catch (error) {
      console.error('Failed to connect to test database:', error);
      throw new Error(
        `Cannot connect to test database. Make sure PostgreSQL is running and the database "${TEST_DATABASE_CONFIG.database}" exists.`
      );
    }
  }
  
  return testPool;
}

/**
 * Close the test database connection pool
 */
export async function closeTestDatabase(): Promise<void> {
  if (testPool) {
    await testPool.end();
    testPool = null;
  }
}

/**
 * Run database migrations for tests
 */
export async function runTestMigrations(): Promise<void> {
  const pool = await getTestDatabase();
  const client = await pool.connect();
  
  try {
    // Read and execute migration files
    const migrationsDir = join(__dirname, '../../migrations');
    const migrationFiles = [
      '0001_web_sessions.sql',
      '0002_web_search_history.sql',
      '0003_web_ui_preferences.sql',
      '0004_agent_runs.sql',
      '0005_agent_messages.sql',
      '0006_worktree_status.sql',
    ];
    
    for (const file of migrationFiles) {
      try {
        const migrationSQL = readFileSync(join(migrationsDir, file), 'utf-8');
        await client.query(migrationSQL);
      } catch (error) {
        console.warn(`Migration ${file} failed or already applied:`, error);
      }
    }
  } finally {
    client.release();
  }
}

/**
 * Clean up all test data from the database
 */
export async function cleanTestDatabase(): Promise<void> {
  const pool = await getTestDatabase();
  const client = await pool.connect();
  
  try {
    // Drop all tables in reverse order to handle dependencies
    const dropQueries = [
      'DROP TABLE IF EXISTS worktree_status CASCADE',
      'DROP TABLE IF EXISTS agent_messages CASCADE',
      'DROP TABLE IF EXISTS agent_runs CASCADE',
      'DROP TABLE IF EXISTS web_ui_preferences CASCADE',
      'DROP TABLE IF EXISTS web_search_history CASCADE',
      'DROP TABLE IF EXISTS web_sessions CASCADE',
    ];
    
    for (const query of dropQueries) {
      await client.query(query);
    }
  } finally {
    client.release();
  }
}

/**
 * Truncate all tables for a clean state between tests
 */
export async function truncateTestTables(): Promise<void> {
  const pool = await getTestDatabase();
  const client = await pool.connect();
  
  try {
    await client.query('BEGIN');
    
    // Disable foreign key checks temporarily
    await client.query('SET session_replication_role = replica');
    
    // Truncate all tables
    const truncateQueries = [
      'TRUNCATE TABLE worktree_status RESTART IDENTITY CASCADE',
      'TRUNCATE TABLE agent_messages RESTART IDENTITY CASCADE',
      'TRUNCATE TABLE agent_runs RESTART IDENTITY CASCADE',
      'TRUNCATE TABLE web_ui_preferences RESTART IDENTITY CASCADE',
      'TRUNCATE TABLE web_search_history RESTART IDENTITY CASCADE',
      'TRUNCATE TABLE web_sessions RESTART IDENTITY CASCADE',
    ];
    
    for (const query of truncateQueries) {
      try {
        await client.query(query);
      } catch (error) {
        // Table might not exist, continue
        console.warn(`Truncate failed for query: ${query}`, error);
      }
    }
    
    // Re-enable foreign key checks
    await client.query('SET session_replication_role = DEFAULT');
    
    await client.query('COMMIT');
  } catch (error) {
    await client.query('ROLLBACK');
    throw error;
  } finally {
    client.release();
  }
}

/**
 * Setup database for tests (run migrations)
 */
export async function setupTestDatabase(): Promise<void> {
  await runTestMigrations();
}

/**
 * Teardown database after tests
 */
export async function teardownTestDatabase(): Promise<void> {
  await truncateTestTables();
}

/**
 * Execute a query with the test database
 */
export async function queryTestDatabase(
  text: string,
  params?: any[]
): Promise<any> {
  const pool = await getTestDatabase();
  const client = await pool.connect();
  
  try {
    const result = await client.query(text, params);
    return result;
  } finally {
    client.release();
  }
}

/**
 * Get a client for transaction-based tests
 */
export async function getTestDatabaseClient(): Promise<PoolClient> {
  const pool = await getTestDatabase();
  return pool.connect();
}