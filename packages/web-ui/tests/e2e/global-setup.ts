import { chromium, FullConfig } from '@playwright/test';
import { setupTestDatabase } from '../utils/database.js';

async function globalSetup(config: FullConfig) {
  console.log('Setting up E2E test environment...');

  try {
    // Setup test database
    await setupTestDatabase();
    console.log('✅ Test database initialized');

    // Start a browser instance for auth setup if needed
    const browser = await chromium.launch();
    const context = await browser.newContext();
    const page = await context.newPage();

    // Pre-warm the application by visiting it
    try {
      await page.goto('http://localhost:3000', { timeout: 30000 });
      console.log('✅ Application is responding');
    } catch (error) {
      console.warn('⚠️ Application not yet available, tests will wait for it');
    }

    await context.close();
    await browser.close();

    console.log('🚀 E2E test environment ready');
  } catch (error) {
    console.error('❌ Failed to setup E2E test environment:', error);
    throw error;
  }
}

export default globalSetup;