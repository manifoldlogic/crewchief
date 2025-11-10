/**
 * Test Setup Script
 *
 * Quick verification that your environment is configured correctly
 * for running the AGENTOPT competition framework.
 *
 * Usage:
 *   tsx src/search-optimization/examples/test-setup.ts
 */

import { execSync } from 'child_process'

interface SetupCheck {
  name: string
  check: () => boolean | Promise<boolean>
  fix?: string
}

const checks: SetupCheck[] = [
  {
    name: 'Node.js version >= 18.0.0',
    check: () => {
      const version = process.version
      const major = parseInt(version.slice(1).split('.')[0])
      return major >= 18
    },
    fix: 'Install Node.js 18 or higher from https://nodejs.org',
  },
  {
    name: 'ANTHROPIC_API_KEY environment variable',
    check: () => {
      const key = process.env.ANTHROPIC_API_KEY
      return !!key && key.startsWith('sk-ant-')
    },
    fix: 'Set ANTHROPIC_API_KEY: export ANTHROPIC_API_KEY="sk-ant-..."',
  },
  {
    name: 'MAPROOM_DATABASE_URL environment variable',
    check: () => {
      const url = process.env.MAPROOM_DATABASE_URL
      return !!url && url.startsWith('postgresql://')
    },
    fix: 'Set MAPROOM_DATABASE_URL: export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"',
  },
  {
    name: 'PostgreSQL connection',
    check: async () => {
      try {
        const url = process.env.MAPROOM_DATABASE_URL
        if (!url) return false

        execSync(`psql "${url}" -c "SELECT 1;" > /dev/null 2>&1`)
        return true
      } catch {
        return false
      }
    },
    fix: 'Start PostgreSQL: cd packages/maproom-mcp && docker compose -f config/docker-compose.yml up -d',
  },
  {
    name: 'pgvector extension',
    check: async () => {
      try {
        const url = process.env.MAPROOM_DATABASE_URL
        if (!url) return false

        const output = execSync(`psql "${url}" -c "SELECT * FROM pg_extension WHERE extname='vector';" -t`, {
          encoding: 'utf8',
        })
        return output.trim().length > 0
      } catch {
        return false
      }
    },
    fix: 'Install pgvector: psql $MAPROOM_DATABASE_URL -c "CREATE EXTENSION IF NOT EXISTS vector;"',
  },
  {
    name: 'Anthropic API access',
    check: async () => {
      try {
        const key = process.env.ANTHROPIC_API_KEY
        if (!key) return false

        const response = await fetch('https://api.anthropic.com/v1/messages', {
          method: 'POST',
          headers: {
            'content-type': 'application/json',
            'x-api-key': key,
            'anthropic-version': '2023-06-01',
          },
          body: JSON.stringify({
            model: 'claude-3-5-sonnet-latest',
            max_tokens: 10,
            messages: [{ role: 'user', content: 'test' }],
          }),
        })

        return response.ok || response.status === 400 // 400 is ok, means API key works
      } catch {
        return false
      }
    },
    fix: 'Verify API key at https://console.anthropic.com/settings/keys',
  },
  {
    name: '@anthropic-ai/claude-agent-sdk installed',
    check: async () => {
      try {
        await import('@anthropic-ai/claude-agent-sdk')
        return true
      } catch {
        return false
      }
    },
    fix: 'Install SDK: pnpm install',
  },
]

async function runChecks(): Promise<void> {
  console.log('🔍 Checking AGENTOPT Competition Framework Setup\n')
  console.log('='.repeat(60))

  let allPassed = true
  const failures: Array<{ name: string; fix: string }> = []

  for (const check of checks) {
    process.stdout.write(`Checking: ${check.name}... `)

    try {
      const result = await check.check()

      if (result) {
        console.log('✅ PASS')
      } else {
        console.log('❌ FAIL')
        allPassed = false
        if (check.fix) {
          failures.push({ name: check.name, fix: check.fix })
        }
      }
    } catch (error) {
      console.log('❌ ERROR')
      console.log(`   Error: ${error instanceof Error ? error.message : String(error)}`)
      allPassed = false
      if (check.fix) {
        failures.push({ name: check.name, fix: check.fix })
      }
    }
  }

  console.log('='.repeat(60))

  if (allPassed) {
    console.log('\n✅ All checks passed! Your environment is ready.')
    console.log('\nNext steps:')
    console.log('  1. Run a test competition:')
    console.log('     tsx src/search-optimization/examples/run-single-competition.ts')
    console.log('  2. Read the documentation:')
    console.log('     docs/search-optimization/competition-framework.md')
    console.log('  3. Run full validation (expensive!):')
    console.log('     pnpm search-optimization:validate-full')
  } else {
    console.log('\n❌ Some checks failed. Please fix the following:\n')

    failures.forEach(({ name, fix }, i) => {
      console.log(`${i + 1}. ${name}`)
      console.log(`   Fix: ${fix}\n`)
    })

    console.log('After fixing, run this script again to verify.')
    process.exit(1)
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  runChecks().catch((error) => {
    console.error('Fatal error:', error)
    process.exit(1)
  })
}

export { runChecks, checks }
