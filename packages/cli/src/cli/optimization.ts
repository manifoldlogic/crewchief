import { Command } from 'commander'
import {
  generateLeaderboardReport,
  compareRunResults,
  promoteToProduction,
  generateProductionReport,
  exportLearnings,
  rollbackProduction,
  getLeaderboardEntry,
  deployVariant,
  getCurrentProduction,
} from '../search-optimization/tracking/index.js'
import { logger } from '../utils/logger.js'

export function registerOptimizationCommands(program: Command): void {
  // Internal tool - no description to keep it less discoverable
  const optimization = new Command('optimization').alias('opt')

  // Leaderboard command
  optimization
    .command('leaderboard')
    .description('View top 10 variants across all optimization runs')
    .option('--json', 'Output as JSON instead of formatted report')
    .option('--rank <number>', 'Show specific rank (1-10)', parseInt)
    .action(async (opts: { json?: boolean; rank?: number }) => {
      try {
        if (opts.rank !== undefined) {
          // Show specific entry
          const entry = await getLeaderboardEntry(opts.rank)
          if (!entry) {
            logger.error(`No variant found at rank ${opts.rank}`)
            process.exitCode = 1
            return
          }

          if (opts.json) {
            console.log(JSON.stringify(entry, null, 2))
          } else {
            console.log(`\nRank ${entry.rank}: ${entry.name}`)
            console.log(`Variant ID: ${entry.variantId}`)
            console.log(`Composite Score: ${(entry.compositeScore * 100).toFixed(1)}%`)
            console.log(
              `Tier Scores: T1=${(entry.tierScores.tier1 * 100).toFixed(1)}% T2=${(entry.tierScores.tier2 * 100).toFixed(1)}% T3=${(entry.tierScores.tier3 * 100).toFixed(1)}%`,
            )
            console.log(`Run ID: ${entry.runId}`)
            console.log(`Generation: ${entry.generation}`)
            console.log(`Converged: ${entry.converged ? 'Yes' : 'No'}`)
            console.log(`Timestamp: ${entry.timestamp}`)
          }
        } else {
          // Show full leaderboard
          const report = await generateLeaderboardReport()
          if (opts.json) {
            // Parse the report to extract structured data (or load leaderboard directly)
            const { loadLeaderboard } = await import('../search-optimization/tracking/leaderboard.js')
            const leaderboard = await loadLeaderboard()
            console.log(JSON.stringify(leaderboard, null, 2))
          } else {
            console.log(report)
          }
        }
      } catch (error) {
        logger.error(`Failed to load leaderboard: ${error instanceof Error ? error.message : String(error)}`)
        process.exitCode = 1
      }
    })

  // Compare runs command
  optimization
    .command('compare')
    .description('Compare two optimization runs side-by-side')
    .argument('<runId1>', 'First run ID')
    .argument('<runId2>', 'Second run ID')
    .option('--json', 'Output as JSON')
    .action(async (runId1: string, runId2: string, opts: { json?: boolean }) => {
      try {
        const comparison = await compareRunResults(runId1, runId2)

        if (opts.json) {
          console.log(JSON.stringify(comparison, null, 2))
        } else {
          console.log(comparison)
        }
      } catch (error) {
        logger.error(`Failed to compare runs: ${error instanceof Error ? error.message : String(error)}`)
        process.exitCode = 1
      }
    })

  // Promote to production command
  optimization
    .command('promote')
    .description('Promote a variant to production')
    .argument('<variantId>', 'Variant ID to promote')
    .option('--reason <text>', 'Reason for promotion')
    .option('--deployed-by <name>', 'Name of person deploying')
    .action(async (variantId: string, opts: { reason?: string; deployedBy?: string }) => {
      try {
        // Load the variant first
        const { loadVariant } = await import('../search-optimization/genetic-iterator.js')
        const variant = await loadVariant(variantId)

        await promoteToProduction(variant, opts.reason, opts.deployedBy)

        logger.info(`✓ Variant ${variantId} promoted to production`)
        logger.info(`  Reason: ${opts.reason || 'Not specified'}`)
        logger.info(`  Deployed by: ${opts.deployedBy || 'Not specified'}`)

        // Show updated production status
        const report = await generateProductionReport()
        console.log('\n' + report)
      } catch (error) {
        logger.error(`Failed to promote variant: ${error instanceof Error ? error.message : String(error)}`)
        process.exitCode = 1
      }
    })

  // Production status command
  optimization
    .command('production')
    .description('View current production variant and deployment history')
    .option('--history', 'Show full deployment history')
    .option('--json', 'Output as JSON')
    .action(async (opts: { history?: boolean; json?: boolean }) => {
      try {
        if (opts.json) {
          const { getCurrentProduction, getProductionHistory } = await import(
            '../search-optimization/tracking/production.js'
          )
          const current = await getCurrentProduction()
          const history = await getProductionHistory()
          console.log(JSON.stringify({ current, history }, null, 2))
        } else {
          const report = await generateProductionReport()
          console.log(report)
        }
      } catch (error) {
        logger.error(`Failed to load production status: ${error instanceof Error ? error.message : String(error)}`)
        process.exitCode = 1
      }
    })

  // Rollback command
  optimization
    .command('rollback')
    .description('Rollback production to previous variant')
    .option('--reason <text>', 'Reason for rollback')
    .option('--deployed-by <name>', 'Name of person performing rollback')
    .action(async (opts: { reason?: string; deployedBy?: string }) => {
      try {
        const result = await rollbackProduction(opts.reason, opts.deployedBy)

        logger.info(`✓ Rolled back to variant ${result.currentVariantId}`)
        logger.info(`  Reason: ${opts.reason || 'Not specified'}`)

        // Show updated production status
        const report = await generateProductionReport()
        console.log('\n' + report)
      } catch (error) {
        logger.error(`Failed to rollback: ${error instanceof Error ? error.message : String(error)}`)
        process.exitCode = 1
      }
    })

  // Export learnings command
  optimization
    .command('learnings')
    .description('Export insights and learnings from a completed run')
    .argument('<runId>', 'Run ID to analyze')
    .option('--json', 'Output as JSON')
    .action(async (runId: string, opts: { json?: boolean }) => {
      try {
        const learnings = await exportLearnings(runId)

        if (opts.json) {
          console.log(JSON.stringify(learnings, null, 2))
        } else {
          console.log(learnings)
        }
      } catch (error) {
        logger.error(`Failed to export learnings: ${error instanceof Error ? error.message : String(error)}`)
        process.exitCode = 1
      }
    })

  // List runs command
  optimization
    .command('runs')
    .description('List all optimization runs')
    .option('--limit <number>', 'Limit number of runs shown', parseInt, 10)
    .option('--status <status>', 'Filter by status (running|completed|failed)')
    .option('--json', 'Output as JSON')
    .action(async (opts: { limit?: number; status?: string; json?: boolean }) => {
      try {
        const { listRuns } = await import('../search-optimization/tracking/run-registry.js')
        let runs = await listRuns()

        // Filter by status if specified
        if (opts.status) {
          runs = runs.filter((r) => r.status === opts.status)
        }

        // Limit results
        if (opts.limit) {
          runs = runs.slice(0, opts.limit)
        }

        if (opts.json) {
          console.log(JSON.stringify(runs, null, 2))
        } else {
          if (runs.length === 0) {
            console.log('No optimization runs found')
            return
          }

          console.log(`\nOptimization Runs (showing ${runs.length}):\n`)
          for (const run of runs) {
            console.log(`${run.runId}`)
            console.log(`  Status: ${run.status}`)
            console.log(`  Started: ${run.startedAt}`)
            if (run.completedAt) {
              console.log(`  Completed: ${run.completedAt}`)
            }
            if (run.bestVariant) {
              console.log(`  Best Variant: ${run.bestVariant.id} (${(run.bestVariant.score * 100).toFixed(1)}%)`)
            }
            console.log(`  Converged: ${run.convergenceReached ? 'Yes' : 'No'}`)
            console.log('')
          }
        }
      } catch (error) {
        logger.error(`Failed to list runs: ${error instanceof Error ? error.message : String(error)}`)
        process.exitCode = 1
      }
    })

  // Deploy variant command
  optimization
    .command('deploy')
    .description('Deploy a variant to the live MCP server')
    .argument('[variantId]', 'Variant ID to deploy')
    .option('--production', 'Deploy current production variant')
    .option('--dry-run', 'Preview changes without applying')
    .option('--skip-build', 'Skip rebuild step')
    .option('--auto-restart', 'Automatically restart server if running')
    .action(
      async (
        variantId: string | undefined,
        opts: {
          production?: boolean
          dryRun?: boolean
          skipBuild?: boolean
          autoRestart?: boolean
        },
      ) => {
        try {
          let targetVariantId = variantId

          // If --production flag, use current production variant
          if (opts.production) {
            const current = await getCurrentProduction()
            if (!current) {
              logger.error('No production variant currently deployed')
              logger.info('Use "crewchief optimization promote <variantId>" to set a production variant first')
              process.exitCode = 1
              return
            }
            targetVariantId = current.currentVariantId
            logger.info(`Deploying current production variant: ${targetVariantId}`)
          }

          if (!targetVariantId) {
            logger.error('Variant ID is required')
            logger.info('Usage: crewchief optimization deploy <variantId>')
            logger.info('   or: crewchief optimization deploy --production')
            process.exitCode = 1
            return
          }

          // Show deployment plan
          console.log('\n=== DEPLOYMENT PLAN ===')
          console.log(`Variant ID: ${targetVariantId}`)
          console.log(`Dry Run: ${opts.dryRun ? 'Yes' : 'No'}`)
          console.log(`Skip Build: ${opts.skipBuild ? 'Yes' : 'No'}`)
          console.log(`Auto Restart: ${opts.autoRestart ? 'Yes' : 'No'}`)
          console.log('=======================\n')

          // Execute deployment
          const result = await deployVariant(targetVariantId, {
            dryRun: opts.dryRun,
            skipBuild: opts.skipBuild,
            autoRestart: opts.autoRestart,
          })

          if (!result.success) {
            logger.error('\n✗ Deployment failed!')
            if (result.errors && result.errors.length > 0) {
              console.error('\nErrors:')
              for (const error of result.errors) {
                console.error(`  - ${error}`)
              }
            }
            process.exitCode = 1
            return
          }

          // Success!
          if (!opts.dryRun) {
            console.log('\n✓ Deployment completed successfully!')
            console.log('\nNext steps:')
            if (result.serverRestarted) {
              console.log('  1. ✓ Server has been restarted')
              console.log('  2. Verify the deployment in your AI assistant')
            } else {
              console.log('  1. Restart the MCP server if running')
              console.log('  2. Verify the deployment in your AI assistant')
            }
            console.log('  3. Monitor performance and user feedback')
            console.log('\nRollback:')
            console.log('  If needed: crewchief optimization rollback')
          }
        } catch (error) {
          logger.error(`Failed to deploy variant: ${error instanceof Error ? error.message : String(error)}`)
          process.exitCode = 1
        }
      },
    )

  program.addCommand(optimization)
}
