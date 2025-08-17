#!/usr/bin/env node
/**
 * Database CLI Tool
 * 
 * Command-line interface for managing database migrations, seeds, and maintenance.
 * Usage: tsx src/db/cli.ts <command> [options]
 */

import { program } from 'commander';
import { 
  runMigrations, 
  migrationStatus, 
  resetMigrations,
  createMigrationRunner 
} from './migrations.js';
import { runSeeds } from '../../seeds/run_seeds.js';
import { initializeDatabase, closeDatabase, getDatabase } from './connection.js';

program
  .name('crewchief-db')
  .description('CrewChief Web UI Database Management CLI')
  .version('1.0.0');

// Migration commands
program
  .command('migrate')
  .description('Run pending database migrations')
  .action(async () => {
    try {
      console.log('🚀 Running database migrations...');
      await initializeDatabase();
      await runMigrations();
      console.log('✅ Migrations completed successfully');
    } catch (error) {
      console.error('❌ Migration failed:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

program
  .command('migrate:status')
  .description('Show migration status')
  .action(async () => {
    try {
      await initializeDatabase();
      await migrationStatus();
    } catch (error) {
      console.error('❌ Failed to get migration status:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

program
  .command('migrate:reset')
  .description('Reset all migrations (DANGEROUS - development only)')
  .option('--force', 'Force reset without confirmation')
  .action(async (options) => {
    if (process.env.NODE_ENV === 'production' && !options.force) {
      console.error('❌ Cannot reset migrations in production environment');
      process.exit(1);
    }

    if (!options.force) {
      console.warn('⚠️  This will destroy all web UI data and reset migrations');
      console.warn('⚠️  This action cannot be undone');
      
      // In a real CLI, you'd prompt for confirmation here
      console.log('Use --force flag to proceed');
      process.exit(1);
    }

    try {
      console.log('🔥 Resetting migrations...');
      await initializeDatabase();
      await resetMigrations();
      console.log('✅ Migrations reset completed');
    } catch (error) {
      console.error('❌ Reset failed:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

// Seed commands
program
  .command('seed')
  .description('Run seed data scripts for development')
  .action(async () => {
    try {
      console.log('🌱 Running seed data...');
      await initializeDatabase();
      await runSeeds();
      console.log('✅ Seed data completed successfully');
    } catch (error) {
      console.error('❌ Seed data failed:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

// Database utility commands
program
  .command('health')
  .description('Check database connection health')
  .action(async () => {
    try {
      console.log('🏥 Checking database health...');
      const db = await initializeDatabase();
      const isHealthy = await db.healthCheck();
      
      if (isHealthy) {
        console.log('✅ Database connection is healthy');
        
        // Show connection pool stats
        const stats = db.getPoolStats();
        console.log(`📊 Pool stats: ${stats.totalCount} total, ${stats.idleCount} idle, ${stats.waitingCount} waiting`);
      } else {
        console.error('❌ Database connection is unhealthy');
        process.exit(1);
      }
    } catch (error) {
      console.error('❌ Health check failed:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

program
  .command('setup')
  .description('Complete database setup (migrate + seed)')
  .action(async () => {
    try {
      console.log('🛠️  Setting up database...');
      await initializeDatabase();
      
      console.log('\n📈 Running migrations...');
      await runMigrations();
      
      console.log('\n🌱 Running seed data...');
      await runSeeds();
      
      console.log('\n✅ Database setup completed successfully!');
      console.log('\nYou can now start the web UI server with: npm run dev');
    } catch (error) {
      console.error('❌ Setup failed:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

program
  .command('cleanup')
  .description('Run database cleanup and maintenance tasks')
  .action(async () => {
    try {
      console.log('🧹 Running database cleanup...');
      await initializeDatabase();
      const db = getDatabase();
      
      // Run cleanup functions
      console.log('  • Cleaning up expired sessions...');
      await db.query('SELECT cleanup_expired_sessions()');
      
      console.log('  • Cleaning up old search history...');
      await db.query('SELECT cleanup_old_search_history()');
      
      console.log('  • Cleaning up old agent runs...');
      await db.query('SELECT cleanup_old_agent_runs()');
      
      console.log('  • Cleaning up old agent messages...');
      await db.query('SELECT cleanup_old_agent_messages()');
      
      console.log('  • Cleaning up worktree status cache...');
      await db.query('SELECT cleanup_worktree_status_cache()');
      
      console.log('  • Marking stale worktrees...');
      await db.query('SELECT mark_stale_worktrees()');
      
      console.log('✅ Database cleanup completed');
    } catch (error) {
      console.error('❌ Cleanup failed:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

program
  .command('stats')
  .description('Show database statistics')
  .action(async () => {
    try {
      console.log('📊 Database Statistics');
      console.log('=====================\n');
      
      await initializeDatabase();
      const db = getDatabase();
      
      // Table row counts
      const tables = [
        'web_sessions',
        'web_search_history', 
        'web_ui_preferences',
        'agent_runs',
        'agent_messages',
        'worktree_status'
      ];
      
      console.log('📈 Table Row Counts:');
      for (const table of tables) {
        const result = await db.query(`SELECT COUNT(*) as count FROM ${table}`);
        const count = result.rows[0]?.count || 0;
        console.log(`   ${table}: ${count}`);
      }
      
      // Active sessions
      console.log('\n🔐 Sessions:');
      const sessions = await db.query(`
        SELECT 
          COUNT(*) as total,
          COUNT(*) FILTER (WHERE is_active = true AND expires_at > NOW()) as active,
          COUNT(*) FILTER (WHERE expires_at <= NOW()) as expired
        FROM web_sessions
      `);
      
      if (sessions.rows.length > 0) {
        const s = sessions.rows[0];
        console.log(`   Total: ${s.total}, Active: ${s.active}, Expired: ${s.expired}`);
      }
      
      // Recent agent activity
      console.log('\n🤖 Agent Activity (Last 24h):');
      const agents = await db.query(`
        SELECT 
          agent_type,
          status,
          COUNT(*) as count
        FROM agent_runs 
        WHERE started_at > NOW() - INTERVAL '24 hours'
        GROUP BY agent_type, status
        ORDER BY agent_type, status
      `);
      
      if (agents.rows.length > 0) {
        agents.rows.forEach(row => {
          console.log(`   ${row.agent_type} ${row.status}: ${row.count}`);
        });
      } else {
        console.log('   No recent agent activity');
      }
      
      // Search activity
      console.log('\n🔍 Search Activity (Last 24h):');
      const searches = await db.query(`
        SELECT 
          COUNT(*) as total_searches,
          COUNT(DISTINCT session_id) as unique_users,
          ROUND(AVG(execution_time_ms), 0) as avg_time_ms
        FROM web_search_history 
        WHERE searched_at > NOW() - INTERVAL '24 hours'
      `);
      
      if (searches.rows.length > 0 && searches.rows[0].total_searches > 0) {
        const s = searches.rows[0];
        console.log(`   Total searches: ${s.total_searches}`);
        console.log(`   Unique users: ${s.unique_users}`);
        console.log(`   Average time: ${s.avg_time_ms}ms`);
      } else {
        console.log('   No recent search activity');
      }
      
      // Database size info
      console.log('\n💾 Storage:');
      const size = await db.query(`
        SELECT 
          schemaname,
          tablename,
          pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
        FROM pg_tables 
        WHERE tablename LIKE 'web_%' OR tablename LIKE 'agent_%' OR tablename LIKE 'worktree_status'
        ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
      `);
      
      size.rows.forEach(row => {
        console.log(`   ${row.tablename}: ${row.size}`);
      });
      
    } catch (error) {
      console.error('❌ Failed to get statistics:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

// Query command for development
program
  .command('query <sql>')
  .description('Execute a SQL query (development only)')
  .option('--limit <number>', 'Limit number of rows', '25')
  .action(async (sql, options) => {
    if (process.env.NODE_ENV === 'production') {
      console.error('❌ Query command not available in production');
      process.exit(1);
    }
    
    try {
      await initializeDatabase();
      const db = getDatabase();
      
      // Add LIMIT if it's a SELECT and doesn't already have one
      let query = sql.trim();
      if (query.toLowerCase().startsWith('select') && 
          !query.toLowerCase().includes('limit')) {
        query += ` LIMIT ${options.limit}`;
      }
      
      console.log(`🔍 Executing: ${query}\n`);
      const result = await db.query(query);
      
      if (result.rows.length > 0) {
        console.table(result.rows);
        console.log(`\n📊 ${result.rows.length} row(s) returned`);
      } else {
        console.log('📊 No rows returned');
      }
      
    } catch (error) {
      console.error('❌ Query failed:', error);
      process.exit(1);
    } finally {
      await closeDatabase();
    }
  });

// Parse command line arguments
program.parse();

// If no command provided, show help
if (!process.argv.slice(2).length) {
  program.outputHelp();
}