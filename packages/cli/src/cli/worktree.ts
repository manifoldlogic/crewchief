import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import chalk from 'chalk'
import { Command } from 'commander'
import inquirer from 'inquirer'
import { loadConfig } from '../config/loader'
import { copyIgnoredFiles, copyIgnoredFilesBack } from '../git/copy-ignored-files'
import { GitMergeService, MergeStrategyType } from '../git/merge'
import { WorktreeService } from '../git/worktrees'
import { RunManager } from '../orchestrator/runManager'
import { removeDirSync } from '../utils/fs'
import { logger } from '../utils/logger'
import { displaySubshellMessage } from '../utils/subshell-message'
import { WorktreeMetadataService } from '../utils/worktree-metadata'

export function registerWorktreeCommands(program: Command): void {
  const worktree = new Command('worktree').description('Worktree operations')

  worktree
    .command('create')
    .argument('<name>')
    .option('--branch <base>', 'Base branch to create the worktree from')
    .option('--base-path <dir>', 'Base directory for storing worktrees')
    .option('--no-cd', 'Do not start a subshell in the created worktree')
    .option('--no-copy-ignored', 'Do not copy ignored files (override config)')
    .description('Create a git worktree from a base branch (general-purpose)')
    .action(async (name: string, opts: { branch?: string; basePath?: string; cd?: boolean; copyIgnored?: boolean }) => {
      try {
        const config = await loadConfig()
        const baseBranch = opts.branch ?? config.repository.mainBranch
        const basePath = opts.basePath ?? config.repository.worktreeBasePath
        const wt = new WorktreeService()
        await wt.initRepository(basePath)
        const skipCopyIgnored = opts.copyIgnored === false
        const createdPath = await wt.createWorktree(name, baseBranch, basePath, skipCopyIgnored)
        logger.success(`Created worktree at ${createdPath} [${baseBranch}]`)

        // Default behavior: start a subshell in the created worktree unless --no-cd is passed
        // Commander sets opts.cd to false when --no-cd is provided; otherwise it's true/undefined
        const shouldCd = opts.cd !== false
        if (shouldCd) {
          const shell = process.env.SHELL || '/bin/bash'
          const currentBranch = await wt.getCurrentBranch()
          const currentDir = process.cwd()

          displaySubshellMessage({
            targetBranch: name,
            targetDirectory: path.relative(currentDir, createdPath) || path.basename(createdPath),
            sourceBranch: currentBranch,
            sourceDirectory: path.basename(currentDir),
            shell: path.basename(shell),
          })

          const child = spawn(shell, { stdio: 'inherit', cwd: createdPath, env: process.env })
          child.on('exit', (code) => process.exit(code ?? 0))
        }
      } catch (err) {
        logger.error('Failed to create worktree:', err)
        process.exitCode = 1
      }
    })

  worktree
    .command('list')
    .description('List active worktrees')
    .action(async () => {
      try {
        const wt = new WorktreeService()
        const list = await wt.listWorktrees()
        const rm = new RunManager()
        const runs = rm.listRuns()
        if (list.length === 0) {
          logger.info('No worktrees found')
          return
        }
        for (const item of list) {
          const run = runs.find((r) => r.worktreePath === item.path && r.status === 'running')
          const suffix = [
            item.branch ? `[${item.branch}]` : undefined,
            run ? `agent=${run.agentTypeId}` : undefined,
            run ? `status=${run.status}` : undefined,
          ]
            .filter(Boolean)
            .join(' ')
          logger.info(`${item.path}${suffix ? ' ' + suffix : ''}`)
        }
      } catch (err) {
        logger.error('Failed to list worktrees:', err)
        process.exitCode = 1
      }
    })

  worktree
    .command('clean')
    .argument('[selector]')
    .option('--stale', 'Remove stale worktrees only')
    .option('--all', 'Remove all non-current worktrees')
    .option('--keep-dir', 'Keep worktree directories (only remove git metadata)')
    .description('Prune worktree metadata, remove a specific worktree, or remove all with --all')
    .action(async (selector: string | undefined, opts: { stale?: boolean; all?: boolean; keepDir?: boolean }) => {
      try {
        const wt = new WorktreeService()
        // --all always takes precedence
        if (opts.all) {
          await wt.pruneWorktrees({ mode: 'all', keepDir: opts.keepDir })
          logger.success(
            `Removed all non-current worktrees${opts.keepDir ? ' (kept directories)' : ' and their directories'}`,
          )
          return
        }

        // If a selector is provided, remove only the matching worktree
        if (selector && !opts.stale) {
          const list = await wt.listWorktrees()
          const matches = list.filter((item) => {
            const sel = selector.trim()
            const byBranch = item.branch && item.branch === sel
            const byBaseName = path.basename(item.path) === sel
            let byPath = false
            try {
              const resolvedSel = path.resolve(sel)
              byPath = path.resolve(item.path) === resolvedSel || path.resolve(item.path).includes(resolvedSel)
            } catch {
              // ignore errors
            }
            return Boolean(byBranch || byBaseName || byPath)
          })
          if (matches.length === 0) {
            logger.error(`No matching worktree for '${selector}'. Try using branch name or worktree directory name.`)
            process.exitCode = 1
            return
          }
          if (matches.length > 1) {
            logger.error(`Ambiguous selector '${selector}'. Candidates:`)
            for (const m of matches) logger.info(`${m.path}${m.branch ? ` [${m.branch}]` : ''}`)
            process.exitCode = 1
            return
          }
          const targetPath = path.resolve(matches[0].path)
          // Resolve real paths to handle symlinks and detect if cwd is inside the target worktree
          let targetReal = targetPath
          let cwdReal = process.cwd()
          try {
            targetReal = fs.realpathSync(targetPath)
            cwdReal = fs.realpathSync(process.cwd())
          } catch {}
          const rel = path.relative(targetReal, cwdReal)
          const isCwdInsideTarget = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel))
          if (isCwdInsideTarget) {
            logger.error('Refusing to remove the current working tree. Switch to another directory and try again.')
            process.exitCode = 1
            return
          }
          await wt.removeWorktree(targetPath)
          if (!opts.keepDir) {
            removeDirSync(targetPath)
          }
          logger.success(`Removed worktree ${targetPath}${opts.keepDir ? ' (kept directory)' : ''}`)
          return
        }

        // Otherwise, default to pruning stale metadata
        await wt.pruneWorktrees({ mode: 'stale', keepDir: opts.keepDir })
        logger.success('Pruned stale worktree metadata')
      } catch (err) {
        logger.error('Failed to prune worktrees:', err)
        process.exitCode = 1
      }
    })

  worktree
    .command('cd')
    .argument('<selector>')
    .option('-p, --print', 'Print the absolute path instead of starting a subshell')
    .description(
      'Resolve a worktree by branch/name/path and start a subshell there by default. Use --print to output the absolute path for command substitution.',
    )
    .action(async (selector: string, opts: { print?: boolean }) => {
      try {
        const wt = new WorktreeService()
        const list = await wt.listWorktrees()
        const matches = list.filter((item) => {
          const sel = selector.trim()
          const byBranch = item.branch && item.branch === sel
          const byBaseName = path.basename(item.path) === sel
          let byPath = false
          try {
            const resolvedSel = path.resolve(sel)
            byPath = path.resolve(item.path) === resolvedSel || path.resolve(item.path).includes(resolvedSel)
          } catch {}
          return Boolean(byBranch || byBaseName || byPath)
        })
        if (matches.length === 0) {
          logger.error(`No matching worktree for '${selector}'. Try using branch name or worktree directory name.`)
          process.exitCode = 1
          return
        }
        if (matches.length > 1) {
          logger.error(`Ambiguous selector '${selector}'. Candidates:`)
          for (const m of matches) logger.info(`${m.path}${m.branch ? ` [${m.branch}]` : ''}`)
          process.exitCode = 1
          return
        }
        const targetPath = path.resolve(matches[0].path)
        if (opts.print) {
          process.stdout.write(targetPath + '\n')
          return
        }
        const shell = process.env.SHELL || '/bin/bash'
        const currentBranch = await wt.getCurrentBranch()
        const currentDir = process.cwd()
        const targetBranch = matches[0].branch

        if (targetBranch) {
          displaySubshellMessage({
            targetBranch: targetBranch,
            targetDirectory: path.relative(currentDir, targetPath) || path.basename(targetPath),
            sourceBranch: currentBranch,
            sourceDirectory: path.basename(currentDir),
            shell: path.basename(shell),
          })
        } else {
          // Fallback for cases where branch is not detected
          console.log(chalk.yellow('\nEntering worktree subshell...'))
          console.log(chalk.gray('Type "exit" to return to your original directory\n'))
        }

        const child = spawn(shell, { stdio: 'inherit', cwd: targetPath, env: process.env })
        child.on('exit', (code) => process.exit(code ?? 0))
      } catch (err) {
        logger.error('Failed to resolve worktree:', err)
        process.exitCode = 1
      }
    })

  worktree
    .command('copy-ignored')
    .argument('<selector>')
    .option('--dry-run', 'Show what would be copied without actually copying')
    .option('--no-copy-ignored', 'Override config and skip copying (for testing)')
    .description('Copy ignored files to an existing worktree based on config')
    .action(async (selector: string, opts: { dryRun?: boolean; copyIgnored?: boolean }) => {
      try {
        // Check if copying is disabled
        if (opts.copyIgnored === false) {
          logger.info('Skipping copy (--no-copy-ignored flag)')
          return
        }

        const config = await loadConfig()
        if (!config.worktree?.copyIgnoredFiles?.length) {
          logger.warn(
            'No ignored files configured to copy. Add patterns to worktree.copyIgnoredFiles in crewchief.config.ts',
          )
          return
        }

        // Find the worktree
        const wt = new WorktreeService()
        const list = await wt.listWorktrees()
        const matches = list.filter((item) => {
          const sel = selector.trim()
          const byBranch = item.branch && item.branch === sel
          const byBaseName = path.basename(item.path) === sel
          let byPath = false
          try {
            const resolvedSel = path.resolve(sel)
            byPath = path.resolve(item.path) === resolvedSel || path.resolve(item.path).includes(resolvedSel)
          } catch {}
          return Boolean(byBranch || byBaseName || byPath)
        })

        if (matches.length === 0) {
          logger.error(`No matching worktree for '${selector}'. Try using branch name or worktree directory name.`)
          process.exitCode = 1
          return
        }
        if (matches.length > 1) {
          logger.error(`Ambiguous selector '${selector}'. Candidates:`)
          for (const m of matches) logger.info(`${m.path}${m.branch ? ` [${m.branch}]` : ''}`)
          process.exitCode = 1
          return
        }

        const targetPath = path.resolve(matches[0].path)
        logger.info(`Copying ignored files to ${targetPath}...`)

        const result = await copyIgnoredFiles({
          sourceRoot: process.cwd(),
          worktreeRoot: targetPath,
          config,
          dryRun: opts.dryRun,
        })

        if (result.errors.length > 0) {
          logger.warn(`Completed with ${result.errors.length} error(s)`)
          process.exitCode = 1
        } else {
          logger.success('Ignored files copied successfully')
        }
      } catch (err) {
        logger.error('Failed to copy ignored files:', err)
        process.exitCode = 1
      }
    })

  worktree
    .command('merge')
    .argument('<name>')
    .option('--no-copy-ignored', 'Skip copying ignored files back to source')
    .option('--dry-run', 'Show what would be done without making changes')
    .option('--strategy <type>', 'Merge strategy (ff, squash, cherry-pick)', 'ff')
    .option('--message <msg>', 'Custom commit message')
    .option('--no-delete', 'Keep the worktree after merging')
    .option('-y, --yes', 'Skip confirmation prompts')
    .description('Merge changes from a worktree back to its source branch and clean up')
    .action(async (name: string, opts: {
      copyIgnored?: boolean
      dryRun?: boolean
      strategy?: string
      message?: string
      delete?: boolean
      yes?: boolean
    }) => {
      try {
        const config = await loadConfig()
        const wt = new WorktreeService()
        const metadataService = new WorktreeMetadataService()

        // Resolve worktree
        const list = await wt.listWorktrees()
        const matches = list.filter((item) => {
          const sel = name.trim()
          const byBranch = item.branch && item.branch === sel
          const byBaseName = path.basename(item.path) === sel
          let byPath = false
          try {
            const resolvedSel = path.resolve(sel)
            byPath = path.resolve(item.path) === resolvedSel || path.resolve(item.path).includes(resolvedSel)
          } catch {}
          return Boolean(byBranch || byBaseName || byPath)
        })

        if (matches.length === 0) {
          logger.error(`No matching worktree for '${name}'. Try using branch name or worktree directory name.`)
          process.exitCode = 1
          return
        }
        if (matches.length > 1) {
          logger.error(`Ambiguous selector '${name}'. Candidates:`)
          for (const m of matches) logger.info(`${m.path}${m.branch ? ` [${m.branch}]` : ''}`)
          process.exitCode = 1
          return
        }

        const worktree = matches[0]
        const worktreePath = path.resolve(worktree.path)
        const worktreeBranch = worktree.branch

        if (!worktreeBranch) {
          logger.error('Cannot determine worktree branch')
          process.exitCode = 1
          return
        }

        // Check if we're inside the worktree being merged
        const cwdReal = fs.realpathSync(process.cwd())
        const worktreeReal = fs.realpathSync(worktreePath)
        const rel = path.relative(worktreeReal, cwdReal)
        const isInsideWorktree = rel === '' || (!rel.startsWith('..') && !path.isAbsolute(rel))

        if (isInsideWorktree) {
          logger.error('Cannot merge a worktree while inside it. Please switch to the main repository first.')
          process.exitCode = 1
          return
        }

        // Read metadata to find source branch
        const metadata = await metadataService.read(worktreePath)
        const sourceBranch = metadata?.sourceBranch || config.repository.mainBranch

        // Initialize merge service
        const mergeService = new GitMergeService()

        // Check for uncommitted changes
        try {
          await mergeService.ensureClean()
        } catch {
          logger.error('Working tree has uncommitted changes. Commit or stash them before merging.')
          process.exitCode = 1
          return
        }

        // Check if there are commits to merge
        const hasCommits = await mergeService.hasCommitsToMerge(worktreeBranch, sourceBranch)
        if (!hasCommits) {
          logger.warn('No commits to merge from this worktree')

          // Still offer to clean up the worktree
          if (!opts.yes) {
            const { cleanup } = await inquirer.prompt([{
              type: 'confirm',
              name: 'cleanup',
              message: 'Do you want to remove this worktree anyway?',
              default: false,
            }])

            if (cleanup && opts.delete !== false) {
              await wt.removeWorktree(worktreePath)
              removeDirSync(worktreePath)
              logger.success(`Removed worktree ${worktreePath}`)
            }
          }
          return
        }

        // Get changes statistics
        const stats = await mergeService.getChangesStats(worktreeBranch, sourceBranch)

        // Display what will be done
        console.log(chalk.cyan('\n📊 Merge Summary:'))
        console.log(`   Source branch: ${chalk.green(sourceBranch)}`)
        console.log(`   Worktree branch: ${chalk.yellow(worktreeBranch)}`)
        console.log(`   Strategy: ${chalk.blue(opts.strategy || 'ff')}`)
        console.log(`   Commits: ${stats.commitCount}`)
        console.log(`   Files changed: ${stats.filesChanged}`)
        console.log(`   Insertions: ${chalk.green(`+${stats.insertions}`)}`)
        console.log(`   Deletions: ${chalk.red(`-${stats.deletions}`)}`)

        if (opts.dryRun) {
          console.log(chalk.yellow('\n🔍 DRY RUN - No changes will be made'))
        }

        // Confirm merge
        if (!opts.yes && !opts.dryRun) {
          const { proceed } = await inquirer.prompt([{
            type: 'confirm',
            name: 'proceed',
            message: `Merge ${worktreeBranch} into ${sourceBranch}?`,
            default: true,
          }])

          if (!proceed) {
            logger.info('Merge cancelled')
            return
          }
        }

        // Copy ignored files back (if configured)
        let ignoredFilesCopied: string[] = []
        if (opts.copyIgnored !== false && config.worktree?.copyIgnoredFiles?.length) {
          console.log(chalk.cyan('\n📁 Copying ignored files back...'))
          const copyResult = await copyIgnoredFilesBack({
            worktreeRoot: worktreePath,
            sourceRoot: process.cwd(),
            config,
            dryRun: opts.dryRun,
          })
          ignoredFilesCopied = copyResult.copied

          if (copyResult.errors.length > 0) {
            logger.warn(`Some files could not be copied: ${copyResult.errors.length} error(s)`)
          }
        }

        if (opts.dryRun) {
          console.log(chalk.yellow('\n✅ DRY RUN completed - no actual changes made'))
          return
        }

        // Perform the merge
        console.log(chalk.cyan('\n🔀 Performing merge...'))

        const strategy = (opts.strategy as MergeStrategyType) || 'ff'
        const commitMessage = opts.message || await mergeService.generateMergeCommitMessage({
          worktreePath,
          sourceBranch: worktreeBranch,
          targetBranch: sourceBranch,
          strategy,
          ignoredFilesCopied,
        })

        const mergeResult = await mergeService.merge({
          sourceBranch: worktreeBranch,
          targetBranch: sourceBranch,
          strategy,
          commitMessage,
        })

        if (!mergeResult.success) {
          logger.error(`Merge failed: ${mergeResult.message}`)
          process.exitCode = 1
          return
        }

        logger.success(`Successfully merged ${worktreeBranch} into ${sourceBranch}`)

        // Clean up worktree (unless --no-delete)
        if (opts.delete !== false) {
          console.log(chalk.cyan('\n🧹 Cleaning up worktree...'))

          // Delete the worktree branch
          try {
            await mergeService.deleteBranch(worktreeBranch)
            logger.success(`Deleted branch ${worktreeBranch}`)
          } catch (error) {
            logger.warn(`Could not delete branch ${worktreeBranch}: ${error}`)
          }

          // Remove the worktree
          await wt.removeWorktree(worktreePath)
          removeDirSync(worktreePath)
          logger.success(`Removed worktree at ${worktreePath}`)

          // Run git worktree prune
          await wt.pruneWorktrees({ mode: 'stale' })
        } else {
          logger.info(`Worktree kept at ${worktreePath} (use 'worktree clean' to remove later)`)
        }

        console.log(chalk.green('\n✅ Merge completed successfully!'))

      } catch (err) {
        logger.error('Failed to merge worktree:', err)
        process.exitCode = 1
      }
    })

  program.addCommand(worktree)
}
