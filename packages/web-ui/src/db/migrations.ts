/**
 * Database Migration Runner
 * 
 * Handles running and tracking database migrations for the CrewChief web UI.
 * Integrates with the existing Maproom database structure.
 */

import { readdir, readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { DatabaseConnection, getDatabase } from './connection.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export interface Migration {
  id: string;
  filename: string;
  sql: string;
  checksum: string;
}

export interface MigrationRecord {
  id: string;
  filename: string;
  checksum: string;
  executed_at: Date;
  execution_time_ms: number;
  success: boolean;
  error_message?: string;
}

export interface MigrationResult {
  migration: Migration;
  executed: boolean;
  success: boolean;
  executionTime: number;
  error?: Error;
}

export class MigrationRunner {
  private db: DatabaseConnection;
  private migrationsPath: string;

  constructor(db: DatabaseConnection, migrationsPath?: string) {
    this.db = db;
    this.migrationsPath = migrationsPath || join(__dirname, '../../migrations');
  }

  /**
   * Initialize the migrations table if it doesn't exist
   */
  async initializeMigrationsTable(): Promise<void> {
    const sql = `
      CREATE TABLE IF NOT EXISTS web_migrations (
        id TEXT PRIMARY KEY,
        filename TEXT NOT NULL,
        checksum TEXT NOT NULL,
        executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        execution_time_ms INTEGER NOT NULL,
        success BOOLEAN NOT NULL DEFAULT true,
        error_message TEXT,
        UNIQUE(filename)
      );
      
      CREATE INDEX IF NOT EXISTS idx_web_migrations_executed_at 
      ON web_migrations(executed_at DESC);
      
      CREATE INDEX IF NOT EXISTS idx_web_migrations_success 
      ON web_migrations(success, executed_at DESC);
    `;

    await this.db.query(sql);
  }

  /**
   * Load all migration files from the migrations directory
   */
  async loadMigrations(): Promise<Migration[]> {
    try {
      const files = await readdir(this.migrationsPath);
      const migrationFiles = files
        .filter(file => file.endsWith('.sql'))
        .sort(); // Ensure consistent ordering

      const migrations: Migration[] = [];

      for (const filename of migrationFiles) {
        const filepath = join(this.migrationsPath, filename);
        const sql = await readFile(filepath, 'utf-8');
        const checksum = await this.calculateChecksum(sql);
        
        // Extract ID from filename (e.g., "0001_web_sessions.sql" -> "0001_web_sessions")
        const id = filename.replace('.sql', '');

        migrations.push({
          id,
          filename,
          sql,
          checksum,
        });
      }

      return migrations;
    } catch (error) {
      console.error('Failed to load migration files:', error);
      throw new Error(`Could not load migrations from ${this.migrationsPath}`);
    }
  }

  /**
   * Get executed migrations from the database
   */
  async getExecutedMigrations(): Promise<MigrationRecord[]> {
    const { rows } = await this.db.query<MigrationRecord>(`
      SELECT id, filename, checksum, executed_at, execution_time_ms, success, error_message
      FROM web_migrations
      ORDER BY executed_at ASC
    `);

    return rows;
  }

  /**
   * Get pending migrations that need to be executed
   */
  async getPendingMigrations(): Promise<Migration[]> {
    const allMigrations = await this.loadMigrations();
    const executedMigrations = await this.getExecutedMigrations();
    const executedIds = new Set(executedMigrations.map(m => m.id));

    return allMigrations.filter(migration => !executedIds.has(migration.id));
  }

  /**
   * Validate migration checksums
   */
  async validateMigrations(): Promise<{ valid: boolean; errors: string[] }> {
    const allMigrations = await this.loadMigrations();
    const executedMigrations = await this.getExecutedMigrations();
    const errors: string[] = [];

    for (const executed of executedMigrations) {
      const current = allMigrations.find(m => m.id === executed.id);
      
      if (!current) {
        errors.push(`Migration ${executed.id} was executed but file no longer exists`);
        continue;
      }

      if (current.checksum !== executed.checksum) {
        errors.push(`Migration ${executed.id} checksum mismatch - file may have been modified after execution`);
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * Execute a single migration
   */
  async executeMigration(migration: Migration): Promise<MigrationResult> {
    const startTime = Date.now();
    
    try {
      console.log(`Executing migration: ${migration.id}`);
      
      await this.db.transaction(async (client) => {
        // Execute the migration SQL
        await client.query(migration.sql);
        
        // Record the migration
        await client.query(`
          INSERT INTO web_migrations (id, filename, checksum, execution_time_ms, success)
          VALUES ($1, $2, $3, $4, $5)
        `, [
          migration.id,
          migration.filename,
          migration.checksum,
          Date.now() - startTime,
          true,
        ]);
      });

      const executionTime = Date.now() - startTime;
      console.log(`Migration ${migration.id} completed in ${executionTime}ms`);

      return {
        migration,
        executed: true,
        success: true,
        executionTime,
      };

    } catch (error) {
      const executionTime = Date.now() - startTime;
      console.error(`Migration ${migration.id} failed:`, error);

      // Record the failed migration
      try {
        await this.db.query(`
          INSERT INTO web_migrations (id, filename, checksum, execution_time_ms, success, error_message)
          VALUES ($1, $2, $3, $4, $5, $6)
        `, [
          migration.id,
          migration.filename,
          migration.checksum,
          executionTime,
          false,
          error instanceof Error ? error.message : String(error),
        ]);
      } catch (recordError) {
        console.error('Failed to record migration failure:', recordError);
      }

      return {
        migration,
        executed: false,
        success: false,
        executionTime,
        error: error instanceof Error ? error : new Error(String(error)),
      };
    }
  }

  /**
   * Run all pending migrations
   */
  async runPendingMigrations(): Promise<MigrationResult[]> {
    await this.initializeMigrationsTable();
    
    const validation = await this.validateMigrations();
    if (!validation.valid) {
      throw new Error(`Migration validation failed: ${validation.errors.join(', ')}`);
    }

    const pendingMigrations = await this.getPendingMigrations();
    
    if (pendingMigrations.length === 0) {
      console.log('No pending migrations to execute');
      return [];
    }

    console.log(`Found ${pendingMigrations.length} pending migrations`);
    const results: MigrationResult[] = [];

    for (const migration of pendingMigrations) {
      const result = await this.executeMigration(migration);
      results.push(result);

      if (!result.success) {
        console.error(`Migration ${migration.id} failed, stopping execution`);
        break;
      }
    }

    const successful = results.filter(r => r.success).length;
    const failed = results.filter(r => !r.success).length;
    
    console.log(`Migration summary: ${successful} successful, ${failed} failed`);

    return results;
  }

  /**
   * Get migration status
   */
  async getMigrationStatus(): Promise<{
    total: number;
    executed: number;
    pending: number;
    failed: number;
    lastMigration?: MigrationRecord;
  }> {
    const allMigrations = await this.loadMigrations();
    const executedMigrations = await this.getExecutedMigrations();
    const pendingMigrations = await this.getPendingMigrations();
    
    const failed = executedMigrations.filter(m => !m.success).length;
    const lastMigration = executedMigrations[executedMigrations.length - 1];

    return {
      total: allMigrations.length,
      executed: executedMigrations.length,
      pending: pendingMigrations.length,
      failed,
      lastMigration,
    };
  }

  /**
   * Reset migrations (dangerous - for development only)
   */
  async resetMigrations(): Promise<void> {
    if (process.env.NODE_ENV === 'production') {
      throw new Error('Cannot reset migrations in production environment');
    }

    console.warn('DANGEROUS: Resetting all migrations');
    
    await this.db.transaction(async (client) => {
      // Drop all web UI tables
      await client.query(`
        DROP TABLE IF EXISTS web_sessions CASCADE;
        DROP TABLE IF EXISTS web_search_history CASCADE;
        DROP TABLE IF EXISTS web_ui_preferences CASCADE;
        DROP TABLE IF EXISTS agent_runs CASCADE;
        DROP TABLE IF EXISTS agent_messages CASCADE;
        DROP TABLE IF EXISTS worktree_status CASCADE;
        DROP TABLE IF EXISTS web_migrations CASCADE;
        
        -- Drop custom types
        DROP TYPE IF EXISTS agent_status CASCADE;
        DROP TYPE IF EXISTS agent_type CASCADE;
        DROP TYPE IF EXISTS message_type CASCADE;
        DROP TYPE IF EXISTS message_priority CASCADE;
        DROP TYPE IF EXISTS worktree_state CASCADE;
        DROP TYPE IF EXISTS git_file_status CASCADE;
        DROP TYPE IF EXISTS web_preference_key CASCADE;
      `);
    });

    console.log('All web UI migrations have been reset');
  }

  /**
   * Calculate checksum for migration content
   */
  private async calculateChecksum(content: string): Promise<string> {
    const crypto = await import('crypto');
    return crypto.createHash('sha256').update(content, 'utf-8').digest('hex');
  }
}

/**
 * CLI-style migration runner functions
 */

export async function runMigrations(): Promise<void> {
  const db = getDatabase();
  const runner = new MigrationRunner(db);
  
  try {
    await runner.runPendingMigrations();
  } catch (error) {
    console.error('Migration failed:', error);
    process.exit(1);
  }
}

export async function migrationStatus(): Promise<void> {
  const db = getDatabase();
  const runner = new MigrationRunner(db);
  
  const status = await runner.getMigrationStatus();
  
  console.log('Migration Status:');
  console.log(`  Total migrations: ${status.total}`);
  console.log(`  Executed: ${status.executed}`);
  console.log(`  Pending: ${status.pending}`);
  console.log(`  Failed: ${status.failed}`);
  
  if (status.lastMigration) {
    console.log(`  Last migration: ${status.lastMigration.id} (${status.lastMigration.executed_at})`);
  }
  
  if (status.pending > 0) {
    console.log(`\nRun 'npm run migrate' to execute pending migrations`);
  }
}

export async function resetMigrations(): Promise<void> {
  const db = getDatabase();
  const runner = new MigrationRunner(db);
  
  await runner.resetMigrations();
}

/**
 * Create a new migration runner instance
 */
export function createMigrationRunner(
  db?: DatabaseConnection,
  migrationsPath?: string
): MigrationRunner {
  return new MigrationRunner(db || getDatabase(), migrationsPath);
}