/**
 * Seed Data Runner
 * 
 * Executes all seed files to populate the database with development data.
 * This script should be run after migrations to set up a complete development environment.
 */

import { readFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { getDatabase, initializeDatabase } from '../src/db/connection.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

interface SeedFile {
  name: string;
  path: string;
  description: string;
}

const SEED_FILES: SeedFile[] = [
  {
    name: '001_sample_sessions.sql',
    path: join(__dirname, '001_sample_sessions.sql'),
    description: 'Create sample user sessions for development'
  },
  {
    name: '002_sample_search_history.sql',
    path: join(__dirname, '002_sample_search_history.sql'),
    description: 'Add realistic search history data'
  },
  {
    name: '003_sample_preferences.sql',
    path: join(__dirname, '003_sample_preferences.sql'),
    description: 'Set up user preferences with various scopes'
  },
  {
    name: '004_sample_agent_data.sql',
    path: join(__dirname, '004_sample_agent_data.sql'),
    description: 'Create agent runs and message history'
  }
];

async function runSeedFile(seedFile: SeedFile): Promise<void> {
  console.log(`📄 Running seed: ${seedFile.name}`);
  console.log(`   ${seedFile.description}`);
  
  try {
    const sql = await readFile(seedFile.path, 'utf-8');
    const db = getDatabase();
    
    // Execute the seed file
    await db.query(sql);
    
    console.log(`✅ Completed: ${seedFile.name}`);
  } catch (error) {
    console.error(`❌ Failed: ${seedFile.name}`);
    console.error(`   Error: ${error instanceof Error ? error.message : String(error)}`);
    throw error;
  }
}

async function clearExistingData(): Promise<void> {
  console.log('🧹 Clearing existing seed data...');
  
  const db = getDatabase();
  
  // Clear data in reverse dependency order
  await db.query(`
    -- Clear web UI data (keeping maproom data intact)
    DELETE FROM agent_messages WHERE sender_agent_id LIKE '%dev%' OR sender_agent_id = 'orchestrator';
    DELETE FROM agent_runs WHERE agent_id LIKE '%dev%' OR agent_id LIKE 'claude-%' OR agent_id LIKE 'gemini-%';
    DELETE FROM worktree_status; -- Will be repopulated by the application
    DELETE FROM web_ui_preferences WHERE session_id IN (
      SELECT session_id FROM web_sessions WHERE auth_token LIKE 'dev_token_%' OR auth_token LIKE 'expired_token_%'
    );
    DELETE FROM web_search_history WHERE session_id IN (
      SELECT session_id FROM web_sessions WHERE auth_token LIKE 'dev_token_%' OR auth_token LIKE 'expired_token_%'
    );
    DELETE FROM web_sessions WHERE auth_token LIKE 'dev_token_%' OR auth_token LIKE 'expired_token_%';
  `);
  
  console.log('✅ Existing seed data cleared');
}

async function validateSeedData(): Promise<void> {
  console.log('🔍 Validating seed data...');
  
  const db = getDatabase();
  
  // Check that data was created
  const checks = [
    {
      name: 'Web Sessions',
      query: "SELECT COUNT(*) as count FROM web_sessions WHERE auth_token LIKE 'dev_token_%'",
      expected: 3
    },
    {
      name: 'Search History',
      query: "SELECT COUNT(*) as count FROM web_search_history wsh JOIN web_sessions ws ON wsh.session_id = ws.session_id WHERE ws.auth_token LIKE 'dev_token_%'",
      expected: 8
    },
    {
      name: 'User Preferences',
      query: "SELECT COUNT(*) as count FROM web_ui_preferences wup JOIN web_sessions ws ON wup.session_id = ws.session_id WHERE ws.auth_token LIKE 'dev_token_%'",
      expected: 30
    },
    {
      name: 'Agent Runs',
      query: "SELECT COUNT(*) as count FROM agent_runs WHERE agent_id LIKE 'claude-%' OR agent_id LIKE 'gemini-%'",
      expected: 5
    },
    {
      name: 'Agent Messages',
      query: "SELECT COUNT(*) as count FROM agent_messages WHERE sender_agent_id IN ('orchestrator', 'claude-001', 'claude-002', 'gemini-001', 'gemini-002')",
      expected: 7
    }
  ];
  
  for (const check of checks) {
    const result = await db.query<{ count: string }>(check.query);
    const count = parseInt(result.rows[0]?.count || '0');
    
    if (count >= check.expected) {
      console.log(`✅ ${check.name}: ${count} records created`);
    } else {
      console.warn(`⚠️  ${check.name}: Only ${count} records created (expected at least ${check.expected})`);
    }
  }
}

async function showDataSummary(): Promise<void> {
  console.log('\n📊 Seed Data Summary:');
  
  const db = getDatabase();
  
  // Show session summary
  const sessions = await db.query(`
    SELECT 
      user_id,
      COUNT(*) as session_count,
      MIN(expires_at) as expires_soon,
      session_data->>'theme' as theme
    FROM web_sessions 
    WHERE auth_token LIKE 'dev_token_%' OR auth_token LIKE 'expired_token_%'
    GROUP BY user_id, session_data->>'theme'
    ORDER BY user_id
  `);
  
  console.log('\n🔐 Sessions:');
  sessions.rows.forEach(row => {
    const status = new Date(row.expires_soon) > new Date() ? 'active' : 'expired';
    console.log(`   ${row.user_id || 'anonymous'} (${row.theme} theme): ${row.session_count} session(s) - ${status}`);
  });
  
  // Show agent run summary
  const runs = await db.query(`
    SELECT 
      agent_type,
      status,
      COUNT(*) as count,
      ROUND(AVG(evaluation_score), 2) as avg_score
    FROM agent_runs 
    WHERE agent_id LIKE 'claude-%' OR agent_id LIKE 'gemini-%'
    GROUP BY agent_type, status
    ORDER BY agent_type, status
  `);
  
  console.log('\n🤖 Agent Runs:');
  runs.rows.forEach(row => {
    const score = row.avg_score ? ` (avg score: ${row.avg_score})` : '';
    console.log(`   ${row.agent_type} ${row.status}: ${row.count} run(s)${score}`);
  });
  
  // Show search activity
  const searches = await db.query(`
    SELECT 
      search_type,
      COUNT(*) as count,
      ROUND(AVG(execution_time_ms), 0) as avg_time_ms,
      ROUND(AVG(result_count), 0) as avg_results
    FROM web_search_history wsh
    JOIN web_sessions ws ON wsh.session_id = ws.session_id
    WHERE ws.auth_token LIKE 'dev_token_%'
    GROUP BY search_type
    ORDER BY count DESC
  `);
  
  console.log('\n🔍 Search Activity:');
  searches.rows.forEach(row => {
    console.log(`   ${row.search_type}: ${row.count} searches (avg: ${row.avg_time_ms}ms, ${row.avg_results} results)`);
  });
}

async function main(): Promise<void> {
  console.log('🌱 CrewChief Web UI - Seed Data Runner');
  console.log('=====================================\n');
  
  try {
    // Initialize database connection
    console.log('🔌 Connecting to database...');
    await initializeDatabase();
    
    // Clear existing seed data
    await clearExistingData();
    
    // Run all seed files
    console.log('\n📊 Running seed files...');
    for (const seedFile of SEED_FILES) {
      await runSeedFile(seedFile);
    }
    
    // Validate the seeded data
    console.log('');
    await validateSeedData();
    
    // Show summary
    await showDataSummary();
    
    console.log('\n🎉 Seed data creation completed successfully!');
    console.log('\nYou can now:');
    console.log('  • Start the web UI server: npm run dev');
    console.log('  • Login with dev tokens for testing');
    console.log('  • Explore sample data in the interface');
    console.log('\nDev tokens:');
    console.log('  • dev_user_1: check web_sessions table for full token');
    console.log('  • dev_user_2: check web_sessions table for full token');
    console.log('  • anonymous: check web_sessions table for full token');
    
  } catch (error) {
    console.error('\n💥 Seed data creation failed:');
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  } finally {
    // Close database connection
    const db = getDatabase();
    await db.close();
  }
}

// Run if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { main as runSeeds, SEED_FILES };