# Ticket: COMPFIX-1004: Security Controls

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Add security validations and controls to protect against path traversal, command injection, resource exhaustion, and sensitive data exposure in competition runner operations. This ticket implements the security controls identified in the security review to reduce risk from 4/10 to 2/10.

## Background

The competition framework handles variant IDs, file paths, subprocess execution, and sensitive credentials without proper validation or sanitization. Security review (`planning/security-review.md`) identified four HIGH/MEDIUM priority vulnerabilities:

1. **Path traversal** - Malicious variant IDs could create files outside competition directory
2. **Command injection** - String interpolation in execSync enables shell injection
3. **Resource exhaustion** - Uncontrolled parallel operations could exhaust system resources
4. **Sensitive data exposure** - Database credentials logged in plaintext

While the overall risk is LOW (2/10) due to trusted execution environment, these controls are **required before production deployment** per security sign-off (lines 451-456).

**Reference:** Section "Security Controls" in `planning/security-review.md` (lines 56-290)

## Acceptance Criteria

- [ ] Variant ID validation function rejects path traversal attempts (`..`, `/`, `\`)
- [ ] Variant IDs restricted to alphanumeric, dash, and underscore only
- [ ] Variant IDs limited to 64 characters maximum
- [ ] Resource limits enforced: MAX_VARIANTS=50, MAX_PARALLEL_AGENTS=10, MAX_TIMEOUT=600000
- [ ] Database URL sanitization function redacts credentials in logs (replace `://user:pass@` with `://***:***@`)
- [ ] All `execSync` calls replaced with `spawn` + args array (command injection protection)
- [ ] No `shell: true` option used in any subprocess calls
- [ ] Sensitive environment variables never logged directly
- [ ] Unit tests verify: path traversal rejected, resource limits enforced, URLs sanitized, spawn used correctly

## Technical Requirements

### 1. Variant ID Validation

```typescript
// packages/cli/src/search-optimization/security/validators.ts

export function validateVariantId(id: string): void {
  // Reject path traversal attempts
  if (id.includes('..') || id.includes('/') || id.includes('\\')) {
    throw new Error('Invalid variant ID: path traversal detected')
  }

  // Enforce allowed characters
  if (!/^[a-zA-Z0-9_-]+$/.test(id)) {
    throw new Error('Invalid variant ID: only alphanumeric, dash, underscore allowed')
  }

  // Enforce max length
  if (id.length > 64) {
    throw new Error('Invalid variant ID: max 64 characters')
  }

  // Additional check: no consecutive dashes/underscores (optional)
  if (/[-_]{2,}/.test(id)) {
    throw new Error('Invalid variant ID: no consecutive dashes or underscores')
  }
}
```

**Usage in variant loading:**
```typescript
async function loadVariants(variantIds: string[]): Promise<Variant[]> {
  const variants: Variant[] = []

  for (const id of variantIds) {
    // Validate BEFORE using in any file operations
    validateVariantId(id)

    const variant = await loadVariantById(id)
    variants.push(variant)
  }

  return variants
}
```

### 2. Resource Limits

```typescript
// packages/cli/src/search-optimization/security/limits.ts

export const SECURITY_LIMITS = {
  MAX_VARIANTS: 50,
  MAX_PARALLEL_AGENTS: 10,
  MAX_TIMEOUT: 600_000,      // 10 minutes
  MIN_TIMEOUT: 30_000,       // 30 seconds
  MAX_COMPETITION_AGE: 86400_000  // 24 hours
} as const

export function validateCompetitionConfig(config: CompetitionConfig): void {
  // Limit number of variants
  if (config.variants.length > SECURITY_LIMITS.MAX_VARIANTS) {
    throw new Error(
      `Too many variants: ${config.variants.length} exceeds maximum of ${SECURITY_LIMITS.MAX_VARIANTS}`
    )
  }

  // Limit parallel execution
  if (config.parallelAgents && config.parallelAgents > SECURITY_LIMITS.MAX_PARALLEL_AGENTS) {
    throw new Error(
      `Too many parallel agents: ${config.parallelAgents} exceeds maximum of ${SECURITY_LIMITS.MAX_PARALLEL_AGENTS}`
    )
  }

  // Validate timeout range
  if (config.timeout) {
    if (config.timeout < SECURITY_LIMITS.MIN_TIMEOUT) {
      throw new Error(`Timeout too short: minimum ${SECURITY_LIMITS.MIN_TIMEOUT}ms`)
    }
    if (config.timeout > SECURITY_LIMITS.MAX_TIMEOUT) {
      throw new Error(`Timeout too long: maximum ${SECURITY_LIMITS.MAX_TIMEOUT}ms`)
    }
  }
}

export async function runAgentsInParallel(
  envs: VariantEnvironment[],
  task: Task
): Promise<ParticipantResult[]> {
  const results: ParticipantResult[] = []

  // Process in batches to respect MAX_PARALLEL_AGENTS
  for (let i = 0; i < envs.length; i += SECURITY_LIMITS.MAX_PARALLEL_AGENTS) {
    const batch = envs.slice(i, i + SECURITY_LIMITS.MAX_PARALLEL_AGENTS)

    console.log(`Running batch ${Math.floor(i / SECURITY_LIMITS.MAX_PARALLEL_AGENTS) + 1} of ${Math.ceil(envs.length / SECURITY_LIMITS.MAX_PARALLEL_AGENTS)}`)

    const batchResults = await Promise.all(
      batch.map(env => runVariantAgent(env, task))
    )

    results.push(...batchResults)
  }

  return results
}
```

### 3. Sensitive Data Sanitization

```typescript
// packages/cli/src/search-optimization/security/sanitize.ts

export function sanitizeDbUrl(url: string): string {
  // postgresql://user:password@host:port/db
  //            ^^^^^^^^^^^^^ redact this part
  return url.replace(/:\/\/([^:]+):([^@]+)@/, '://***:***@')
}

export function sanitizeEnvironment(env: Record<string, string>): Record<string, string> {
  const sanitized = { ...env }

  // Redact sensitive variables
  const sensitiveKeys = [
    'MAPROOM_DATABASE_URL',
    'DATABASE_URL',
    'ANTHROPIC_API_KEY',
    'OPENAI_API_KEY',
    'PASSWORD',
    'SECRET'
  ]

  for (const key of Object.keys(sanitized)) {
    // Check if key matches any sensitive pattern
    if (sensitiveKeys.some(pattern => key.includes(pattern))) {
      if (key.includes('URL')) {
        sanitized[key] = sanitizeDbUrl(sanitized[key])
      } else {
        sanitized[key] = '***'
      }
    }
  }

  return sanitized
}

export function sanitizeAgentResult(result: ParticipantResult): ParticipantResult {
  return {
    ...result,
    environment: sanitizeEnvironment(result.environment || {})
  }
}
```

**Usage in logging:**
```typescript
// BEFORE (unsafe)
console.log('Competition config:', config)
console.log('Database URL:', process.env.MAPROOM_DATABASE_URL)

// AFTER (safe)
console.log('Competition config:', {
  ...config,
  environment: sanitizeEnvironment(process.env)
})
console.log('Database URL:', sanitizeDbUrl(process.env.MAPROOM_DATABASE_URL || ''))
```

### 4. Command Injection Protection

**CRITICAL:** Replace ALL `execSync` with string interpolation to use `spawn` with args array.

**Audit all files for execSync usage:**
```bash
grep -r "execSync" packages/cli/src/search-optimization/
```

**Replace pattern:**
```typescript
// BEFORE (vulnerable)
import { execSync } from 'child_process'

const output = execSync(`crewchief-maproom scan --repo ${repo} --worktree ${worktree}`)

// AFTER (safe)
import { spawn } from 'child_process'

async function execMaproom(args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    const proc = spawn('crewchief-maproom', args, {
      stdio: 'pipe',
      shell: false  // CRITICAL: never use shell
    })

    let stdout = ''
    let stderr = ''

    proc.stdout.on('data', (data) => { stdout += data.toString() })
    proc.stderr.on('data', (data) => { stderr += data.toString() })

    proc.on('close', (code) => {
      if (code === 0) {
        resolve(stdout)
      } else {
        reject(new Error(`Command failed with code ${code}: ${stderr}`))
      }
    })
  })
}

// Usage
const output = await execMaproom(['scan', '--repo', repo, '--worktree', worktree])
```

**Files to audit and fix:**
- `scan-orchestrator.ts` (should already use spawn from COMPFIX-1002)
- `pre-flight-validator.ts` (uses execSync for maproom status)
- `competition-runner.ts` (check for any direct execSync calls)
- Any other files in search-optimization directory

### 5. Integration into Competition Runner

```typescript
// packages/cli/src/search-optimization/competition-runner.ts

import { validateVariantId } from './security/validators'
import { validateCompetitionConfig, runAgentsInParallel } from './security/limits'
import { sanitizeDbUrl } from './security/sanitize'

export async function runCompetition(config: CompetitionConfig): Promise<CompetitionResult> {
  // Validate config FIRST (before any operations)
  validateCompetitionConfig(config)

  // Validate all variant IDs BEFORE using them
  for (const variantId of config.variants) {
    validateVariantId(variantId)
  }

  console.log('🏁 Starting competition with pre-flight validation')

  // Rest of competition logic...

  // Use sanitized logging
  if (!dbValid) {
    throw new Error(`
❌ Database connection failed

Current value: ${sanitizeDbUrl(process.env.MAPROOM_DATABASE_URL || 'not set')}

Troubleshooting:
- Verify PostgreSQL is running
- Check MAPROOM_DATABASE_URL environment variable
    `.trim())
  }

  // Use batched parallel execution (respects MAX_PARALLEL_AGENTS)
  if (config.parallelExecution) {
    const results = await runAgentsInParallel(variantEnvs, config.task)
    participants.push(...results)
  }

  // Sanitize results before saving
  const sanitizedParticipants = participants.map(sanitizeAgentResult)

  // Generate report with sanitized data
  const report = generateCompetitionReport({
    ...data,
    participants: sanitizedParticipants
  })
}
```

## Implementation Notes

### Testing Strategy

Create `packages/cli/src/search-optimization/security/validators.test.ts`:

```typescript
describe('Security Validators', () => {
  describe('validateVariantId', () => {
    it('accepts valid variant IDs', () => {
      expect(() => validateVariantId('variant-a-detailed')).not.toThrow()
      expect(() => validateVariantId('VARIANT_CONTROL')).not.toThrow()
      expect(() => validateVariantId('variant-123')).not.toThrow()
    })

    it('rejects path traversal attempts', () => {
      expect(() => validateVariantId('../etc/passwd')).toThrow('path traversal')
      expect(() => validateVariantId('variant/../etc')).toThrow('path traversal')
      expect(() => validateVariantId('..\\windows\\system32')).toThrow('path traversal')
    })

    it('rejects invalid characters', () => {
      expect(() => validateVariantId('variant@email.com')).toThrow('only alphanumeric')
      expect(() => validateVariantId('variant$money')).toThrow('only alphanumeric')
      expect(() => validateVariantId('variant<script>')).toThrow('only alphanumeric')
    })

    it('rejects too-long IDs', () => {
      const longId = 'a'.repeat(65)
      expect(() => validateVariantId(longId)).toThrow('max 64 characters')
    })
  })
})
```

Create `packages/cli/src/search-optimization/security/limits.test.ts`:

```typescript
describe('Resource Limits', () => {
  describe('validateCompetitionConfig', () => {
    it('accepts valid configs', () => {
      expect(() => validateCompetitionConfig({
        variants: Array(10).fill('variant-a'),
        task: TASK_TEST,
        timeout: 180000
      })).not.toThrow()
    })

    it('rejects too many variants', () => {
      expect(() => validateCompetitionConfig({
        variants: Array(51).fill('variant-a'),
        task: TASK_TEST
      })).toThrow('Too many variants')
    })

    it('rejects too many parallel agents', () => {
      expect(() => validateCompetitionConfig({
        variants: ['a', 'b'],
        parallelAgents: 11,
        task: TASK_TEST
      })).toThrow('Too many parallel agents')
    })

    it('rejects invalid timeout', () => {
      expect(() => validateCompetitionConfig({
        variants: ['a'],
        timeout: 1000000,  // 16+ minutes
        task: TASK_TEST
      })).toThrow('Timeout too long')
    })
  })

  describe('runAgentsInParallel', () => {
    it('processes agents in batches', async () => {
      const envs = Array(25).fill(null).map((_, i) => ({
        variant: { id: `v${i}`, name: `Variant ${i}` },
        worktreePath: `/tmp/v${i}`,
        worktreeName: `v${i}`
      }))

      const mockRun = jest.fn().mockResolvedValue({ success: true })

      // Should process in 3 batches (10 + 10 + 5)
      await runAgentsInParallel(envs, TASK_TEST, mockRun)

      expect(mockRun).toHaveBeenCalledTimes(25)
      // Verify batching occurred (implementation detail)
    })
  })
})
```

Create `packages/cli/src/search-optimization/security/sanitize.test.ts`:

```typescript
describe('Sensitive Data Sanitization', () => {
  describe('sanitizeDbUrl', () => {
    it('redacts credentials from PostgreSQL URL', () => {
      const url = 'postgresql://maproom:secret123@localhost:5432/maproom'
      const sanitized = sanitizeDbUrl(url)
      expect(sanitized).toBe('postgresql://***:***@localhost:5432/maproom')
      expect(sanitized).not.toContain('secret123')
    })

    it('handles URLs without credentials', () => {
      const url = 'postgresql://localhost:5432/maproom'
      const sanitized = sanitizeDbUrl(url)
      expect(sanitized).toBe('postgresql://localhost:5432/maproom')
    })
  })

  describe('sanitizeEnvironment', () => {
    it('redacts sensitive environment variables', () => {
      const env = {
        MAPROOM_DATABASE_URL: 'postgresql://user:pass@localhost/db',
        ANTHROPIC_API_KEY: 'sk-ant-1234567890',
        NODE_ENV: 'development',
        PATH: '/usr/bin'
      }

      const sanitized = sanitizeEnvironment(env)

      expect(sanitized.MAPROOM_DATABASE_URL).toBe('postgresql://***:***@localhost/db')
      expect(sanitized.ANTHROPIC_API_KEY).toBe('***')
      expect(sanitized.NODE_ENV).toBe('development')  // Not sensitive
      expect(sanitized.PATH).toBe('/usr/bin')         // Not sensitive
    })
  })
})
```

### Security Review Checklist

Before completing this ticket, verify:

- [ ] All variant IDs validated before file operations
- [ ] No `execSync` with string interpolation remains
- [ ] All `spawn` calls use `shell: false`
- [ ] Database URLs sanitized in all logs and error messages
- [ ] Resource limits enforced in competition runner
- [ ] Unit tests cover all security controls
- [ ] Security review sign-off conditions met (planning/security-review.md lines 451-469)

## Dependencies

- **Prerequisite tickets:** None (independent security enhancements)

- **External dependencies:** None (uses Node.js built-ins)

- **Blocks:** All Phase 2 tickets should have security controls in place

## Risk Assessment

- **Risk**: Validation too strict (blocks legitimate variant IDs)
  - **Mitigation**: Allow alphanumeric, dash, underscore (covers 99% of use cases)
  - **Fallback**: Add allowlist for special cases if needed

- **Risk**: Resource limits too conservative (blocks valid competitions)
  - **Mitigation**: Limits based on actual usage patterns (MAX_VARIANTS=50 >> current max of 12)
  - **Future**: Make limits configurable via environment variables

- **Risk**: Sanitization breaks debugging
  - **Mitigation**: Only sanitize in logs/reports, not in memory
  - **Fallback**: Add `--debug-unsafe` flag to show full credentials (dev only)

- **Risk**: Missed execSync calls (incomplete audit)
  - **Mitigation**: Grep audit + unit tests verifying spawn usage
  - **CI Check**: Add linter rule to prevent future execSync usage

## Files/Packages Affected

**New files:**
- `packages/cli/src/search-optimization/security/validators.ts`
- `packages/cli/src/search-optimization/security/validators.test.ts`
- `packages/cli/src/search-optimization/security/limits.ts`
- `packages/cli/src/search-optimization/security/limits.test.ts`
- `packages/cli/src/search-optimization/security/sanitize.ts`
- `packages/cli/src/search-optimization/security/sanitize.test.ts`

**Modified files:**
- `packages/cli/src/search-optimization/competition-runner.ts` (add validation calls)
- `packages/cli/src/search-optimization/pre-flight-validator.ts` (replace execSync with spawn)
- Any other files using execSync in search-optimization directory

**No breaking changes** - all security controls are additive
