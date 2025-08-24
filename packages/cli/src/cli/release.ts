import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import chalk from 'chalk'
import { Command } from 'commander'
import inquirer from 'inquirer'
import simpleGit, { SimpleGit } from 'simple-git'
import { logger } from '../utils/logger'

interface ReleaseOptions {
  dryRun?: boolean
  type?: 'patch' | 'minor' | 'major'
  yes?: boolean
  push?: boolean
  publish?: boolean
}

interface PackageInfo {
  name: string
  path: string
  version: string
  hasChanges: boolean
  lastReleaseTag?: string
  changesSinceLastRelease?: string[]
  private?: boolean
}

/**
 * Get the project root directory
 */
function getProjectRoot(): string {
  const __dirname = path.dirname(fileURLToPath(import.meta.url))
  // Go up from packages/cli/src/cli to root
  return path.resolve(__dirname, '..', '..', '..', '..')
}

/**
 * Bump version based on type
 */
function bumpVersion(version: string, type: 'patch' | 'minor' | 'major'): string {
  const [major, minor, patch] = version.split('.').map(v => parseInt(v, 10))

  switch (type) {
    case 'major':
      return `${major + 1}.0.0`
    case 'minor':
      return `${major}.${minor + 1}.0`
    case 'patch':
    default:
      return `${major}.${minor}.${patch + 1}`
  }
}

/**
 * Get all packages in the monorepo
 */
async function getPackages(projectRoot: string): Promise<PackageInfo[]> {
  const packages: PackageInfo[] = []
  const packagesDir = path.join(projectRoot, 'packages')

  if (!fs.existsSync(packagesDir)) {
    return packages
  }

  const dirs = fs.readdirSync(packagesDir).filter(dir => {
    const dirPath = path.join(packagesDir, dir)
    return fs.statSync(dirPath).isDirectory() &&
           fs.existsSync(path.join(dirPath, 'package.json'))
  })

  for (const dir of dirs) {
    const packagePath = path.join(packagesDir, dir)
    const packageJsonPath = path.join(packagePath, 'package.json')
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'))

    packages.push({
      name: packageJson.name,
      path: packagePath,
      version: packageJson.version,
      hasChanges: false,
      private: packageJson.private,
    })
  }

  return packages
}

/**
 * Detect changes since last release for a package
 */
async function detectChanges(pkg: PackageInfo, git: SimpleGit): Promise<boolean> {
  try {
    // Get all tags for this package
    const tags = await git.tags()
    const packageTags = tags.all.filter(tag => tag.startsWith(`${pkg.name}@v`))

    if (packageTags.length === 0) {
      // No previous releases, everything is new
      pkg.hasChanges = true
      return true
    }

    // Get the most recent tag
    const lastTag = packageTags[packageTags.length - 1]
    pkg.lastReleaseTag = lastTag

    // Get the relative path from project root
    const projectRoot = getProjectRoot()
    const relativePath = path.relative(projectRoot, pkg.path)

    // Check for changes in this package directory since the last tag
    const diff = await git.diff([lastTag, 'HEAD', '--', relativePath])

    if (diff) {
      pkg.hasChanges = true

      // Get list of changed files
      const changedFiles = await git.diff([lastTag, 'HEAD', '--name-only', '--', relativePath])
      pkg.changesSinceLastRelease = changedFiles.split('\n').filter(f => f.trim())

      return true
    }

    return false
  } catch {
    // If there's an error (e.g., tag doesn't exist), assume there are changes
    pkg.hasChanges = true
    return true
  }
}

/**
 * Execute a command and return promise
 */
function executeCommand(
  command: string,
  args: string[],
  cwd: string,
  dryRun: boolean = false,
): Promise<void> {
  return new Promise((resolve, reject) => {
    if (dryRun) {
      console.log(chalk.gray(`[DRY RUN] Would execute: ${command} ${args.join(' ')}`))
      resolve()
      return
    }

    const child = spawn(command, args, {
      cwd,
      stdio: 'inherit',
      shell: process.platform === 'win32',
    })

    child.on('error', (error) => {
      reject(error)
    })

    child.on('exit', (code) => {
      if (code === 0) {
        resolve()
      } else {
        reject(new Error(`Command failed with exit code ${code}`))
      }
    })
  })
}

/**
 * Update package.json version
 */
function updatePackageVersion(packagePath: string, newVersion: string, dryRun: boolean): void {
  const packageJsonPath = path.join(packagePath, 'package.json')
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'))

  if (dryRun) {
    console.log(chalk.gray(`[DRY RUN] Would update ${packageJson.name} from ${packageJson.version} to ${newVersion}`))
    return
  }

  packageJson.version = newVersion
  fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n')
}

/**
 * Release a single package
 */
async function releasePackage(
  pkg: PackageInfo,
  newVersion: string,
  options: ReleaseOptions,
  git: SimpleGit,
): Promise<void> {
  const { dryRun, push, publish } = options

  console.log(chalk.cyan(`\n📦 Releasing ${pkg.name}...`))
  console.log(chalk.gray(`  Current version: ${pkg.version}`))
  console.log(chalk.gray(`  New version: ${newVersion}`))

  // Update package.json
  updatePackageVersion(pkg.path, newVersion, dryRun || false)

  // Commit changes
  if (!dryRun) {
    await git.add(path.join(pkg.path, 'package.json'))
    await git.commit(`chore(release): ${pkg.name}@${newVersion}`)

    // Create tag
    const tag = `${pkg.name}@v${newVersion}`
    await git.addTag(tag)
    console.log(chalk.green(`  ✓ Created tag: ${tag}`))
  } else {
    console.log(chalk.gray(`[DRY RUN] Would commit and tag: ${pkg.name}@v${newVersion}`))
  }

  // Push to remote
  if (push && !dryRun) {
    console.log(chalk.yellow('  Pushing to remote...'))
    await git.push()
    await git.pushTags()
    console.log(chalk.green('  ✓ Pushed to remote'))
  }

  // Publish to npm
  if (publish && !pkg.private && !dryRun) {
    console.log(chalk.yellow('  Publishing to npm...'))
    await executeCommand('pnpm', ['publish', '--access', 'public'], pkg.path, dryRun || false)
    console.log(chalk.green('  ✓ Published to npm'))
  }
}

export function registerReleaseCommand(program: Command): void {
  program
    .command('release')
    .description('Release packages with changes since last release')
    .option('--dry-run', 'Show what would be released without making changes')
    .option('-t, --type <type>', 'Version bump type: patch, minor, or major', 'patch')
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--no-push', 'Do not push to remote')
    .option('--no-publish', 'Do not publish to npm')
    .action(async (options: ReleaseOptions) => {
      try {
        const projectRoot = getProjectRoot()
        const git = simpleGit(projectRoot)

        console.log(chalk.bold.cyan('\n🚀 CrewChief Release System\n'))
        console.log(chalk.gray(`Project root: ${projectRoot}`))

        // Check for uncommitted changes
        const status = await git.status()
        if (!status.isClean() && !options.dryRun) {
          logger.error('Working tree has uncommitted changes. Commit or stash them before releasing.')
          process.exitCode = 1
          return
        }

        // Get all packages
        const packages = await getPackages(projectRoot)
        console.log(chalk.cyan(`\n📋 Found ${packages.length} package(s)\n`))

        // Detect changes for each package
        console.log(chalk.yellow('🔍 Detecting changes...\n'))
        const changedPackages: PackageInfo[] = []

        for (const pkg of packages) {
          const hasChanges = await detectChanges(pkg, git)
          if (hasChanges) {
            changedPackages.push(pkg)
            console.log(chalk.green(`  ✓ ${pkg.name} has changes`))
            if (pkg.changesSinceLastRelease && pkg.changesSinceLastRelease.length > 0) {
              console.log(chalk.gray(`    Files changed: ${pkg.changesSinceLastRelease.length}`))
            }
          } else {
            console.log(chalk.gray(`  - ${pkg.name} (no changes)`))
          }
        }

        if (changedPackages.length === 0) {
          console.log(chalk.yellow('\n📭 No packages have changes to release\n'))
          return
        }

        // Calculate new versions
        const releaseType = options.type || 'patch'
        const releasePlan = changedPackages.map(pkg => ({
          ...pkg,
          newVersion: bumpVersion(pkg.version, releaseType),
        }))

        // Show release plan
        console.log(chalk.cyan('\n📦 Release Plan:\n'))
        for (const pkg of releasePlan) {
          console.log(`  ${pkg.name}:`)
          console.log(`    Current: ${pkg.version}`)
          console.log(`    New:     ${chalk.green(pkg.newVersion)}`)
          if (pkg.private) {
            console.log(chalk.gray('    (private - will not publish)'))
          }
        }

        if (options.dryRun) {
          console.log(chalk.yellow('\n🔍 DRY RUN - No changes will be made\n'))
        }

        // Confirm release
        if (!options.yes && !options.dryRun) {
          const { proceed } = await inquirer.prompt([{
            type: 'confirm',
            name: 'proceed',
            message: `Release ${changedPackages.length} package(s) with ${releaseType} version bump?`,
            default: true,
          }])

          if (!proceed) {
            logger.info('Release cancelled')
            return
          }
        }

        // Release each package
        for (const pkg of releasePlan) {
          await releasePackage(
            pkg,
            pkg.newVersion,
            options,
            git,
          )
        }

        if (options.dryRun) {
          console.log(chalk.yellow('\n✅ DRY RUN completed - no actual changes made\n'))
        } else {
          console.log(chalk.bold.green('\n✨ Release completed successfully!\n'))

          // Show summary
          console.log(chalk.cyan('Released packages:'))
          for (const pkg of releasePlan) {
            console.log(chalk.gray(`  • ${pkg.name}@${pkg.newVersion}`))
          }
        }

      } catch (error) {
        logger.error('Release failed:', error)
        process.exitCode = 1
      }
    })
}
