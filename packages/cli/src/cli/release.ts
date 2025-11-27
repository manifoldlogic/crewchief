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
  packages?: string[]
  all?: boolean
  skipTests?: boolean
}

interface PackageInfo {
  name: string
  path: string
  version: string
  hasChanges: boolean
  lastReleaseTag?: string
  changesSinceLastRelease?: string[]
  private?: boolean
  publishTo?: string[]
  dependencies?: string[]
  requiredCliVersion?: string
}

interface ReleaseConfig {
  releaseOrder: string[]
  packages: Record<
    string,
    {
      path: string
      publishTo: string[]
      dependencies: string[]
      requiredCliVersion?: string
      schemaVersion: number
      notes?: string
    }
  >
  validation: {
    requireCleanWorkingTree: boolean
    requirePassingTests: boolean
    requireVersionBump: boolean
    blockOnUnreleasedDependencies: boolean
  }
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
 * Load release configuration
 */
function loadReleaseConfig(projectRoot: string): ReleaseConfig {
  const configPath = path.join(projectRoot, 'release-config.json')
  if (!fs.existsSync(configPath)) {
    throw new Error('release-config.json not found. Please create it in the project root.')
  }
  return JSON.parse(fs.readFileSync(configPath, 'utf-8'))
}

/**
 * Bump version based on type
 */
function bumpVersion(version: string, type: 'patch' | 'minor' | 'major'): string {
  const [major, minor, patch] = version.split('.').map((v) => parseInt(v, 10))

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
 * Parse semver version string
 */
function parseVersion(version: string): { major: number; minor: number; patch: number } {
  const match = version.replace(/^[>=<^~]+/, '').match(/^(\d+)\.(\d+)\.(\d+)/)
  if (!match) {
    throw new Error(`Invalid version: ${version}`)
  }
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
  }
}

/**
 * Check if version satisfies requirement (simple >= check)
 */
function versionSatisfies(version: string, requirement: string): boolean {
  const req = parseVersion(requirement)
  const ver = parseVersion(version)

  if (ver.major > req.major) return true
  if (ver.major < req.major) return false
  if (ver.minor > req.minor) return true
  if (ver.minor < req.minor) return false
  return ver.patch >= req.patch
}

/**
 * Get all packages in the monorepo based on release config
 */
async function getPackages(projectRoot: string, config: ReleaseConfig): Promise<PackageInfo[]> {
  const packages: PackageInfo[] = []

  for (const pkgName of config.releaseOrder) {
    const pkgConfig = config.packages[pkgName]
    if (!pkgConfig) continue

    const packagePath = path.join(projectRoot, pkgConfig.path)
    const packageJsonPath = path.join(packagePath, 'package.json')

    if (!fs.existsSync(packageJsonPath)) {
      console.log(chalk.yellow(`  Warning: ${pkgName} not found at ${pkgConfig.path}`))
      continue
    }

    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'))

    packages.push({
      name: packageJson.name,
      path: packagePath,
      version: packageJson.version,
      hasChanges: false,
      private: packageJson.private,
      publishTo: pkgConfig.publishTo,
      dependencies: pkgConfig.dependencies,
      requiredCliVersion: pkgConfig.requiredCliVersion,
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
    const packageTags = tags.all.filter((tag) => tag.startsWith(`${pkg.name}@v`))

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
      pkg.changesSinceLastRelease = changedFiles.split('\n').filter((f) => f.trim())

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
 * Check if dependencies have unreleased changes
 */
async function checkDependencyReleaseStatus(
  pkg: PackageInfo,
  allPackages: PackageInfo[],
  git: SimpleGit,
): Promise<{ ok: boolean; blockers: string[] }> {
  const blockers: string[] = []

  if (!pkg.dependencies || pkg.dependencies.length === 0) {
    return { ok: true, blockers: [] }
  }

  for (const depName of pkg.dependencies) {
    const depPkg = allPackages.find((p) => p.name === depName)
    if (!depPkg) continue

    // Check if dependency has unreleased changes
    const hasChanges = await detectChanges(depPkg, git)
    if (hasChanges && depPkg.hasChanges) {
      blockers.push(`${depName} has unreleased changes`)
    }
  }

  return { ok: blockers.length === 0, blockers }
}

/**
 * Check CLI version requirement for MCP package
 */
function checkCliVersionRequirement(pkg: PackageInfo, allPackages: PackageInfo[]): { ok: boolean; message?: string } {
  if (!pkg.requiredCliVersion) {
    return { ok: true }
  }

  const cliPkg = allPackages.find((p) => p.name === '@crewchief/cli')
  if (!cliPkg) {
    return { ok: false, message: '@crewchief/cli not found in packages' }
  }

  if (!versionSatisfies(cliPkg.version, pkg.requiredCliVersion)) {
    return {
      ok: false,
      message: `Requires @crewchief/cli ${pkg.requiredCliVersion}, but found ${cliPkg.version}`,
    }
  }

  return { ok: true }
}

/**
 * Execute a command and return promise
 */
function executeCommand(command: string, args: string[], cwd: string, dryRun: boolean = false): Promise<void> {
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
  if (pkg.publishTo) {
    console.log(chalk.gray(`  Publish to: ${pkg.publishTo.join(', ')}`))
  }

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

  // Publish to npm (if npm is in publishTo)
  if (publish && !pkg.private && pkg.publishTo?.includes('npm') && !dryRun) {
    console.log(chalk.yellow('  Publishing to npm...'))
    await executeCommand('pnpm', ['publish', '--access', 'public', '--no-git-checks'], pkg.path, dryRun || false)
    console.log(chalk.green('  ✓ Published to npm'))
  }

  // Note about other publish targets
  if (publish && pkg.publishTo) {
    const nonNpmTargets = pkg.publishTo.filter((t) => t !== 'npm')
    if (nonNpmTargets.length > 0) {
      console.log(
        chalk.yellow(`  Note: ${nonNpmTargets.join(', ')} publishing requires manual workflow or separate CI`),
      )
    }
  }
}

export function registerReleaseCommand(program: Command): void {
  program
    .command('release')
    .description('Release packages with coordinated versioning and dependency order')
    .option('--dry-run', 'Show what would be released without making changes')
    .option('-t, --type <type>', 'Version bump type: patch, minor, or major', 'patch')
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--no-push', 'Do not push to remote')
    .option('--no-publish', 'Do not publish to npm')
    .option('-p, --packages <packages...>', 'Release specific packages (in release order)')
    .option('--all', 'Release all packages regardless of changes')
    .option('--skip-tests', 'Skip running tests before release (not recommended)')
    .action(async (options: ReleaseOptions) => {
      try {
        const projectRoot = getProjectRoot()
        const git = simpleGit(projectRoot)

        console.log(chalk.bold.cyan('\n🚀 CrewChief Coordinated Release System\n'))
        console.log(chalk.gray(`Project root: ${projectRoot}`))

        // Load release config
        let config: ReleaseConfig
        try {
          config = loadReleaseConfig(projectRoot)
          console.log(chalk.green('✓ Loaded release-config.json'))
          console.log(chalk.gray(`  Release order: ${config.releaseOrder.join(' → ')}`))
        } catch (error) {
          logger.error('Failed to load release config:', error)
          process.exitCode = 1
          return
        }

        // Check for uncommitted changes
        const status = await git.status()
        if (!status.isClean() && !options.dryRun && config.validation.requireCleanWorkingTree) {
          logger.error('Working tree has uncommitted changes. Commit or stash them before releasing.')
          process.exitCode = 1
          return
        }

        // Run tests if required
        if (config.validation.requirePassingTests && !options.skipTests && !options.dryRun) {
          console.log(chalk.yellow('\n🧪 Running tests...\n'))
          try {
            await executeCommand('pnpm', ['test'], projectRoot, false)
            console.log(chalk.green('✓ All tests passed\n'))
          } catch {
            logger.error('Tests failed. Fix test failures before releasing.')
            logger.info('Use --skip-tests to bypass (not recommended)')
            process.exitCode = 1
            return
          }
        } else if (options.skipTests) {
          console.log(chalk.yellow('⚠️  Skipping tests (--skip-tests flag)\n'))
        } else if (options.dryRun) {
          console.log(chalk.gray('[DRY RUN] Would run tests before release\n'))
        }

        // Get all packages in release order
        const packages = await getPackages(projectRoot, config)
        console.log(chalk.cyan(`\n📋 Found ${packages.length} package(s) in release order\n`))

        // Filter to requested packages if specified
        let packagesToProcess = packages
        if (options.packages && options.packages.length > 0) {
          packagesToProcess = packages.filter((p) => options.packages!.includes(p.name))

          // Validate that requested packages maintain release order
          const actualOrder = packagesToProcess.map((p) => config.releaseOrder.indexOf(p.name))

          for (let i = 0; i < actualOrder.length - 1; i++) {
            if (actualOrder[i] > actualOrder[i + 1]) {
              logger.error(
                `Invalid release order. ${packagesToProcess[i + 1].name} must be released before ${packagesToProcess[i].name}`,
              )
              logger.error(`Required order: ${config.releaseOrder.join(' → ')}`)
              process.exitCode = 1
              return
            }
          }
        }

        // Detect changes for each package
        console.log(chalk.yellow('🔍 Detecting changes...\n'))
        const changedPackages: PackageInfo[] = []

        for (const pkg of packagesToProcess) {
          const hasChanges = await detectChanges(pkg, git)

          if (hasChanges || options.all) {
            // Check dependency release status (skip if --all since deps will be released in this run)
            if (config.validation.blockOnUnreleasedDependencies && !options.all) {
              const depStatus = await checkDependencyReleaseStatus(pkg, packages, git)
              if (!depStatus.ok) {
                console.log(chalk.red(`  ✗ ${pkg.name} blocked:`))
                for (const blocker of depStatus.blockers) {
                  console.log(chalk.red(`      - ${blocker}`))
                }
                console.log(chalk.yellow('      Release dependencies first, then retry.'))
                continue
              }
            }

            // Check CLI version requirement
            const cliCheck = checkCliVersionRequirement(pkg, packages)
            if (!cliCheck.ok) {
              console.log(chalk.red(`  ✗ ${pkg.name} blocked: ${cliCheck.message}`))
              continue
            }

            changedPackages.push(pkg)
            console.log(chalk.green(`  ✓ ${pkg.name} ${options.all ? '(forced)' : 'has changes'}`))
            if (pkg.changesSinceLastRelease && pkg.changesSinceLastRelease.length > 0) {
              console.log(chalk.gray(`    Files changed: ${pkg.changesSinceLastRelease.length}`))
            }
          } else {
            console.log(chalk.gray(`  - ${pkg.name} (no changes)`))
          }
        }

        if (changedPackages.length === 0) {
          console.log(chalk.yellow('\n📭 No packages to release\n'))
          return
        }

        // Verify release order
        console.log(chalk.cyan('\n🔒 Verifying release order...\n'))
        const releaseIndices = changedPackages.map((p) => ({
          name: p.name,
          index: config.releaseOrder.indexOf(p.name),
        }))

        let orderValid = true
        for (let i = 0; i < releaseIndices.length - 1; i++) {
          if (releaseIndices[i].index > releaseIndices[i + 1].index) {
            console.log(
              chalk.red(
                `  ✗ Order violation: ${releaseIndices[i + 1].name} must come before ${releaseIndices[i].name}`,
              ),
            )
            orderValid = false
          }
        }

        if (!orderValid) {
          logger.error('Release order validation failed')
          process.exitCode = 1
          return
        }
        console.log(chalk.green('  ✓ Release order verified'))

        // Calculate new versions
        const releaseType = options.type || 'patch'
        const releasePlan = changedPackages.map((pkg) => ({
          ...pkg,
          newVersion: bumpVersion(pkg.version, releaseType),
        }))

        // Show release plan
        console.log(chalk.cyan('\n📦 Release Plan:\n'))
        for (let i = 0; i < releasePlan.length; i++) {
          const pkg = releasePlan[i]
          console.log(`  ${i + 1}. ${pkg.name}:`)
          console.log(`     Current: ${pkg.version}`)
          console.log(`     New:     ${chalk.green(pkg.newVersion)}`)
          if (pkg.publishTo) {
            console.log(`     Targets: ${pkg.publishTo.join(', ')}`)
          }
          if (pkg.private) {
            console.log(chalk.gray('     (private - will not publish)'))
          }
        }

        if (options.dryRun) {
          console.log(chalk.yellow('\n🔍 DRY RUN - No changes will be made\n'))
        }

        // Confirm release
        if (!options.yes && !options.dryRun) {
          const { proceed } = await inquirer.prompt([
            {
              type: 'confirm',
              name: 'proceed',
              message: `Release ${changedPackages.length} package(s) with ${releaseType} version bump in order?`,
              default: true,
            },
          ])

          if (!proceed) {
            logger.info('Release cancelled')
            return
          }
        }

        // Release each package IN ORDER
        console.log(chalk.cyan('\n🚀 Releasing packages in order...\n'))
        for (const pkg of releasePlan) {
          await releasePackage(pkg, pkg.newVersion, options, git)
        }

        if (options.dryRun) {
          console.log(chalk.yellow('\n✅ DRY RUN completed - no actual changes made\n'))
        } else {
          console.log(chalk.bold.green('\n✨ Coordinated release completed successfully!\n'))

          // Show summary
          console.log(chalk.cyan('Released packages (in order):'))
          for (let i = 0; i < releasePlan.length; i++) {
            const pkg = releasePlan[i]
            console.log(chalk.gray(`  ${i + 1}. ${pkg.name}@${pkg.newVersion}`))
          }

          console.log(chalk.cyan('\nNext steps:'))
          console.log(chalk.gray('  • GitHub Actions will build and publish packages'))
          console.log(chalk.gray('  • For vscode-maproom: Manually trigger the release workflow'))
        }
      } catch (error) {
        logger.error('Release failed:', error)
        process.exitCode = 1
      }
    })
}
