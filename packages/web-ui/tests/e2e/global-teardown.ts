import { FullConfig } from '@playwright/test';
import { closeTestDatabase } from '../utils/database.js';

async function globalTeardown(config: FullConfig) {
  console.log('Tearing down E2E test environment...');

  try {
    // Close database connections
    await closeTestDatabase();
    console.log('✅ Database connections closed');

    console.log('🧹 E2E test environment cleaned up');
  } catch (error) {
    console.error('❌ Failed to teardown E2E test environment:', error);
    // Don't throw here as it might mask test failures
  }
}

export default globalTeardown;