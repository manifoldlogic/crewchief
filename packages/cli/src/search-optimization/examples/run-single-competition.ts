/**
 * Single Competition Example
 *
 * Demonstrates running a minimal competition with 2 variants on 1 task.
 * This is a good starting point to verify your setup works.
 *
 * Cost: ~$0.50-1.00
 * Time: ~2-5 minutes
 *
 * Usage:
 *   tsx src/search-optimization/examples/run-single-competition.ts
 */

import { runCompetition } from '../competition-runner.js'
import { TASK_FIND_CLI_ENTRY } from '../tasks/config.js'
import type { Variant } from '../types.js'

// Choose a simple task for testing

console.log('🏁 Running Single Competition Example\n')
console.log('This will spawn 2 agents with different tool descriptions')
console.log('and compare their performance on a simple task.\n')
console.log('Task: Find CLI Entry Point')
console.log('Expected cost: $0.50-1.00')
console.log('Expected time: 2-5 minutes\n')

// Define two variants to test
const variants: Variant[] = [
  {
    id: 'baseline',
    name: 'Baseline Description',
    searchToolDescription: `Semantic code search - BEST FOR: finding functions/classes by concept, understanding code relationships, exploring unfamiliar codebases.

Use this tool when searching for functionality rather than exact text matches.

Examples: "authentication flow", "error handling", "database connection"`,
  },
  {
    id: 'enhanced',
    name: 'Enhanced with Query Guidance',
    searchToolDescription: `Semantic code search - BEST FOR: finding functions/classes by concept, understanding code relationships, exploring unfamiliar codebases.

QUERY FORMULATION:
- Extract 2-3 core technical terms from your question
- Remove question words (how, what, where, when, why)
- Prefer code-like terminology over natural language

Examples:
  "How does checkout work?" → "checkout payment"
  "Where is authentication handled?" → "authentication"
  "Find error handling logic" → "error handler"

Use this tool when searching for functionality rather than exact text matches.`,
  },
]

// Ask for confirmation
async function confirm(): Promise<boolean> {
  console.log('⚠️  This will make API calls costing ~$0.50-1.00')
  console.log('Continue? (y/n): ')

  return new Promise((resolve) => {
    process.stdin.once('data', (data) => {
      const answer = data.toString().trim().toLowerCase()
      resolve(answer === 'y' || answer === 'yes')
    })
  })
}

async function main() {
  // Check for confirmation in CI or non-interactive mode
  if (process.env.CI || !process.stdin.isTTY) {
    console.log('Running in non-interactive mode, skipping confirmation...\n')
  } else {
    const confirmed = await confirm()
    if (!confirmed) {
      console.log('Aborted.')
      process.exit(0)
    }
  }

  console.log('\n' + '='.repeat(60))
  console.log('Starting competition...\n')

  const startTime = Date.now()

  try {
    const result = await runCompetition({
      task: TASK_FIND_CLI_ENTRY,
      variants,
      parallelExecution: false, // Run sequentially for easier debugging
      timeout: 180, // 3 minutes per agent
      baseDir: '.crewchief/competitions',
    })

    const durationSeconds = (Date.now() - startTime) / 1000

    console.log('\n' + '='.repeat(60))
    console.log('🎉 Competition Complete!\n')
    console.log('Duration:', durationSeconds.toFixed(1), 'seconds')
    console.log('Winner:', result.winner.variantName)
    console.log('Winner Score:', (result.winner.score * 100).toFixed(1) + '%')
    console.log('\nDetailed Report:\n')
    console.log(result.report)

    console.log('\n' + '='.repeat(60))
    console.log('Next Steps:\n')
    console.log('1. Review the full report at:')
    console.log(`   .crewchief/competitions/${result.competitionId}/report.txt`)
    console.log('\n2. Examine individual agent runs:')
    console.log(`   .crewchief/competitions/${result.competitionId}/run-*/agent-result.json`)
    console.log('\n3. Try running more competitions:')
    console.log('   - Modify variants to test different descriptions')
    console.log('   - Try different tasks from the task library')
    console.log('   - Run full validation: pnpm search-optimization:validate-full')
    console.log('\n4. Read the documentation:')
    console.log('   docs/search-optimization/competition-framework.md')
  } catch (error) {
    console.error('\n❌ Competition failed:', error)
    console.error('\nTroubleshooting:')
    console.error('1. Run the setup check: tsx src/search-optimization/examples/test-setup.ts')
    console.error('2. Verify environment variables are set')
    console.error('3. Check PostgreSQL is running')
    console.error('4. Verify Anthropic API key is valid')
    process.exit(1)
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error('Fatal error:', error)
    process.exit(1)
  })
}
