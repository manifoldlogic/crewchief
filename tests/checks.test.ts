import { describe, it, expect } from 'vitest';
import { runDefaultChecks } from '../src/evaluation/checks';
import fs from 'node:fs';
import path from 'node:path';

describe('evaluation checks', () => {
  it('returns results and score', async () => {
    const tmp = fs.mkdtempSync(path.join(process.cwd(), '.tmp-test-'));
    const runDir = fs.mkdtempSync(path.join(process.cwd(), '.tmp-run-'));
    const summary = await runDefaultChecks(tmp, runDir);
    expect(summary.results.length).toBeGreaterThan(0);
    expect(summary.score).toBeGreaterThanOrEqual(0);
    fs.rmSync(tmp, { recursive: true, force: true });
    fs.rmSync(runDir, { recursive: true, force: true });
  });
});


