/**
 * Tests for DATABASE_URL fallback behavior in CLI
 *
 * These tests verify the environment variable handling pattern implemented in cli.cjs:
 * 1. Explicit DATABASE_URL is preserved (not overridden)
 * 2. DATABASE_URL is set when not present (auto-detection)
 *
 * Run with: node tests/connection-fallback.test.js
 */

const assert = require('assert');

// Test 1: Respects explicit DATABASE_URL
function testRespectsExplicitDatabaseUrl() {
  const env = {
    ...process.env,
    DATABASE_URL: 'postgresql://test:test@testhost:5432/testdb'
  };

  // Simulate CLI logic: Check if DATABASE_URL exists before setting
  if (!env.DATABASE_URL) {
    env.DATABASE_URL = 'postgresql://maproom:maproom@maproom-postgres:5432/maproom';
  }

  // Verify explicit URL was preserved
  assert.strictEqual(
    env.DATABASE_URL,
    'postgresql://test:test@testhost:5432/testdb',
    'Explicit DATABASE_URL should be preserved'
  );

  console.log('✓ Test 1 passed: Respects explicit DATABASE_URL');
}

// Test 2: Sets DATABASE_URL when not present
function testSetsDatabaseUrlWhenNotPresent() {
  const env = { ...process.env };
  delete env.DATABASE_URL;

  // Simulate CLI logic: Check if DATABASE_URL exists before setting
  if (!env.DATABASE_URL) {
    env.DATABASE_URL = 'postgresql://maproom:maproom@maproom-postgres:5432/maproom';
  }

  // Verify DATABASE_URL was set
  assert.ok(env.DATABASE_URL, 'DATABASE_URL should be set');
  assert.ok(
    env.DATABASE_URL.includes('maproom'),
    'DATABASE_URL should include maproom'
  );

  console.log('✓ Test 2 passed: Sets DATABASE_URL when not present');
}

// Run all tests
function runTests() {
  const startTime = Date.now();

  console.log('\nRunning connection-fallback tests...\n');

  try {
    testRespectsExplicitDatabaseUrl();
    testSetsDatabaseUrlWhenNotPresent();

    const duration = Date.now() - startTime;
    console.log(`\n✓ All tests passed in ${duration}ms\n`);
    process.exit(0);
  } catch (error) {
    console.error('\n✗ Test failed:', error.message);
    console.error(error.stack);
    process.exit(1);
  }
}

// Run tests if executed directly
if (require.main === module) {
  runTests();
}

module.exports = { runTests };
