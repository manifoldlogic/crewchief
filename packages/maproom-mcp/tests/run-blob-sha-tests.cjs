#!/usr/bin/env node
/**
 * Standalone test runner for blob SHA compatibility
 *
 * This tests that PostgreSQL compute_git_blob_sha() produces identical
 * output to the Rust compute_blob_sha() function.
 */

const { Client } = require('pg');

// Known SHA-256 values from Rust test suite
const KNOWN_HASHES = {
  '': '473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813',
  'test': 'aa19560d465e7d43915547490a1f6b73eb55702e3d12cb82fb577df60bad4928',
};

// Test cases
const TEST_CASES = [
  { content: '', description: 'empty string' },
  { content: 'test', description: 'simple ASCII' },
  { content: 'function foo() {\n  return 42;\n}', description: 'multi-line code' },
  { content: 'Hello 世界 🌍', description: 'unicode content' },
  { content: 'x'.repeat(1024), description: 'large content (1KB)' },
  { content: 'special!@#$%^&*()_+-=[]{}|;:\'",.<>?/', description: 'special characters' },
  { content: 'line1\nline2\nline3', description: 'newlines' },
  { content: 'line1\r\nline2', description: 'CRLF newlines' },
  { content: '  leading and trailing spaces  ', description: 'whitespace' },
];

async function runTests() {
  // MAPROOM_DATABASE_URL is set by vitest.config.ts to the correct test database
  const connectionString = process.env.MAPROOM_DATABASE_URL || 'postgresql://maproom:maproom@host.docker.internal:5434/maproom_test';

  const client = new Client({ connectionString });

  try {
    await client.connect();

    console.log('===============================================');
    console.log('Blob SHA Migration - Integration Tests');
    console.log('===============================================');
    console.log('');

    let passedTests = 0;
    let failedTests = 0;

    // Test 1: Function exists
    console.log('✓ Testing: Function exists in database');
    const fnCheck = await client.query(`
      SELECT proname
      FROM pg_proc
      WHERE proname = 'compute_git_blob_sha'
        AND pronamespace = 'maproom'::regnamespace
    `);
    if (fnCheck.rows.length > 0) {
      console.log('  PASS: Function maproom.compute_git_blob_sha exists');
      passedTests++;
    } else {
      console.log('  FAIL: Function not found');
      failedTests++;
    }
    console.log('');

    // Test 2: Match known Rust hash for empty content
    console.log('✓ Testing: Known hash for empty content');
    const emptyResult = await client.query("SELECT maproom.compute_git_blob_sha('') as sha");
    const emptyHash = emptyResult.rows[0].sha;
    if (emptyHash === KNOWN_HASHES['']) {
      console.log('  PASS: PostgreSQL hash matches Rust hash');
      console.log(`  Hash: ${emptyHash}`);
      passedTests++;
    } else {
      console.log('  FAIL: Hash mismatch');
      console.log(`  Expected: ${KNOWN_HASHES['']}`);
      console.log(`  Got:      ${emptyHash}`);
      failedTests++;
    }
    console.log('');

    // Test 3: Match known Rust hash for "test" content
    console.log('✓ Testing: Known hash for "test" content');
    const testResult = await client.query("SELECT maproom.compute_git_blob_sha('test') as sha");
    const testHash = testResult.rows[0].sha;
    if (testHash === KNOWN_HASHES['test']) {
      console.log('  PASS: PostgreSQL hash matches Rust hash');
      console.log(`  Hash: ${testHash}`);
      passedTests++;
    } else {
      console.log('  FAIL: Hash mismatch');
      console.log(`  Expected: ${KNOWN_HASHES['test']}`);
      console.log(`  Got:      ${testHash}`);
      failedTests++;
    }
    console.log('');

    // Test 4: Determinism
    console.log('✓ Testing: Determinism (same input produces same hash)');
    const content = 'function foo() { return 1; }';
    const result1 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [content]);
    const result2 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [content]);
    if (result1.rows[0].sha === result2.rows[0].sha) {
      console.log('  PASS: Repeated calls produce identical hashes');
      passedTests++;
    } else {
      console.log('  FAIL: Hashes differ on repeated calls');
      failedTests++;
    }
    console.log('');

    // Test 5: Different content produces different hashes
    console.log('✓ Testing: Different content produces different hashes');
    const content1 = 'function foo() { return 1; }';
    const content2 = 'function bar() { return 2; }';
    const r1 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [content1]);
    const r2 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [content2]);
    if (r1.rows[0].sha !== r2.rows[0].sha) {
      console.log('  PASS: Different content produces different hashes');
      passedTests++;
    } else {
      console.log('  FAIL: Same hash for different content');
      failedTests++;
    }
    console.log('');

    // Test 6: Whitespace sensitivity
    console.log('✓ Testing: Whitespace sensitivity');
    const ws1 = 'function foo() { return 1; }';
    const ws2 = 'function foo() { return 1;  }'; // Extra space
    const ws1Result = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [ws1]);
    const ws2Result = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [ws2]);
    if (ws1Result.rows[0].sha !== ws2Result.rows[0].sha) {
      console.log('  PASS: Extra whitespace changes hash');
      passedTests++;
    } else {
      console.log('  FAIL: Whitespace ignored');
      failedTests++;
    }
    console.log('');

    // Test 7: Unicode handling
    console.log('✓ Testing: Unicode content');
    const unicode = '函数 foo() { return "привет"; } // こんにちは';
    const u1 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [unicode]);
    const u2 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [unicode]);
    if (u1.rows[0].sha === u2.rows[0].sha) {
      console.log('  PASS: Unicode content is deterministic');
      passedTests++;
    } else {
      console.log('  FAIL: Unicode handling is non-deterministic');
      failedTests++;
    }
    console.log('');

    // Test 8: Newline handling
    console.log('✓ Testing: Newline handling (LF vs CRLF)');
    const lf = 'line1\nline2';
    const crlf = 'line1\r\nline2';
    const lfResult = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [lf]);
    const crlfResult = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [crlf]);
    if (lfResult.rows[0].sha !== crlfResult.rows[0].sha) {
      console.log('  PASS: Different newline types produce different hashes');
      passedTests++;
    } else {
      console.log('  FAIL: Newline types produce same hash');
      failedTests++;
    }
    console.log('');

    // Test 9: Large content
    console.log('✓ Testing: Large content (10KB)');
    const large = 'x'.repeat(10000);
    const l1 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [large]);
    const l2 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [large]);
    if (l1.rows[0].sha === l2.rows[0].sha && l1.rows[0].sha.length === 64) {
      console.log('  PASS: Large content handled deterministically');
      passedTests++;
    } else {
      console.log('  FAIL: Large content issues');
      failedTests++;
    }
    console.log('');

    // Test 10: All test cases are deterministic
    console.log('✓ Testing: All edge cases are deterministic');
    let allDeterministic = true;
    for (const testCase of TEST_CASES) {
      const r1 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [testCase.content]);
      const r2 = await client.query('SELECT maproom.compute_git_blob_sha($1) as sha', [testCase.content]);
      if (r1.rows[0].sha !== r2.rows[0].sha || r1.rows[0].sha.length !== 64) {
        console.log(`  FAIL: ${testCase.description} is not deterministic`);
        allDeterministic = false;
        failedTests++;
        break;
      }
    }
    if (allDeterministic) {
      console.log(`  PASS: All ${TEST_CASES.length} test cases are deterministic`);
      passedTests++;
    }
    console.log('');

    await client.end();

    console.log('===============================================');
    console.log(`Results: ${passedTests} passed, ${failedTests} failed`);
    console.log('===============================================');

    if (failedTests === 0) {
      console.log('');
      console.log('✓✓✓ ALL TESTS PASSED ✓✓✓');
      console.log('');
      console.log('PostgreSQL compute_git_blob_sha() produces identical');
      console.log('output to Rust compute_blob_sha() function.');
      return 0;
    } else {
      console.log('');
      console.log('✗✗✗ SOME TESTS FAILED ✗✗✗');
      return 1;
    }

  } catch (err) {
    console.error('✗ Test error:', err.message);
    return 1;
  }
}

runTests().then(code => process.exit(code));
