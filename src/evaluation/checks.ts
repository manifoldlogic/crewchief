import fs from 'node:fs';
import path from 'node:path';
import { runCommand } from '../utils/exec';

export interface CheckResult {
  name: string;
  passed: boolean;
  details?: string;
}

export interface EvaluationSummary {
  results: CheckResult[];
  score: number; // 0..1
}

export async function runDefaultChecks(worktreePath: string, runDir: string): Promise<EvaluationSummary> {
  const results: CheckResult[] = [];

  // Check 1: pnpm available in environment
  try {
    const res = await runCommand('pnpm', ['--version'], { cwd: worktreePath });
    results.push({ name: 'env:pnpm', passed: res.exitCode === 0, details: res.stdout.trim() });
  } catch (err) {
    results.push({ name: 'env:pnpm', passed: false, details: String(err) });
  }

  // Check 2: events.log contains at least one JSONL event
  try {
    const eventsPath = path.join(runDir, 'events.log');
    const exists = fs.existsSync(eventsPath);
    const size = exists ? fs.statSync(eventsPath).size : 0;
    results.push({ name: 'agent:events', passed: exists && size > 0, details: exists ? `${size} bytes` : 'missing' });
  } catch (err) {
    results.push({ name: 'agent:events', passed: false, details: String(err) });
  }

  const passed = results.filter((r) => r.passed).length;
  const score = results.length ? passed / results.length : 0;
  return { results, score };
}


