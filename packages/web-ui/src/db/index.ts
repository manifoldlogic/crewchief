/**
 * Database Module Exports
 * 
 * Main entry point for all database-related functionality.
 * Provides convenient access to connection, query building, migrations, and utilities.
 */

// Connection and configuration
export {
  DatabaseConnection,
  getDatabase,
  initializeDatabase,
  closeDatabase,
  createDatabaseConfig,
  type DatabaseConfig,
  type QueryOptions as ConnectionQueryOptions,
} from './connection.js';

// Query building and utilities
export {
  QueryBuilder,
  SessionQuery,
  SearchHistoryQuery,
  AgentRunsQuery,
  WorktreeStatusQuery,
  executeRawQuery,
  executeProcedure,
  buildInsertQuery,
  buildUpdateQuery,
  mapToEntity,
  mapArrayToEntities,
  executePaginatedQuery,
  type PaginationOptions,
  type SortOptions,
  type FilterCondition,
  type QueryOptions,
  type PaginatedResult,
} from './query-builder.js';

// Migration system
export {
  MigrationRunner,
  runMigrations,
  migrationStatus,
  resetMigrations,
  createMigrationRunner,
  type Migration,
  type MigrationRecord,
  type MigrationResult,
} from './migrations.js';

// Re-export common types for convenience
export type {
  QueryResult,
  PoolClient,
} from 'pg';

/**
 * Database initialization helper for applications
 */
export async function setupDatabase(): Promise<void> {
  // Initialize connection
  await initializeDatabase();
  
  // Run pending migrations
  await runMigrations();
  
  console.log('✅ Database setup completed');
}

/**
 * Database health check for monitoring
 */
export async function checkDatabaseHealth(): Promise<{
  healthy: boolean;
  poolStats: {
    totalCount: number;
    idleCount: number;
    waitingCount: number;
  };
  error?: string;
}> {
  try {
    const db = getDatabase();
    const healthy = await db.healthCheck();
    const poolStats = db.getPoolStats();
    
    return {
      healthy,
      poolStats,
    };
  } catch (error) {
    return {
      healthy: false,
      poolStats: { totalCount: 0, idleCount: 0, waitingCount: 0 },
      error: error instanceof Error ? error.message : String(error),
    };
  }
}