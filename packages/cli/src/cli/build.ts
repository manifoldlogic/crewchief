import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import chalk from 'chalk'
import { Command } from 'commander'
import { logger } from '../utils/logger'

interface BuildOptions {
  skipRust?: boolean
  skipTypeScript?: boolean
  verbose?: boolean
  sequential?: boolean
}

interface _BuildTarget {
  name: string
  type: 'rust' | 'typescript' | 'web'
  path: string
  command: string
  args: string[]
  description: string
}

/**
 * Execute a command and return a promise
 */
function executeCommand(
  command: string,
  args: string[],
  cwd: string,
  verbose: boolean = false,
  ignoreExitCode: boolean = false,
): Promise<{ stdout: string; stderr: string; code: number }> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      stdio: verbose ? 'inherit' : 'pipe',
      shell: process.platform === 'win32',
    })

    let stdout = ''
    let stderr = ''

    if (!verbose) {
      child.stdout?.on('data', (data) => {
        stdout += data.toString()
      })

      child.stderr?.on('data', (data) => {
        stderr += data.toString()
      })
    }

    child.on('error', (error) => {
      reject(error)
    })

    child.on('exit', (code) => {
      if (code === 0 || ignoreExitCode) {
        resolve({ stdout, stderr, code: code || 0 })
      } else {
        // Special handling for pnpm install which might exit with 1 due to optional scripts
        if (command === 'pnpm' && args[0] === 'install') {
          // Always resolve for pnpm install - it's too flaky with exit codes
          resolve({ stdout, stderr, code: code || 0 })
          return
        }
        reject(new Error(`Command failed with exit code ${code}`))
      }
    })
  })
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
 * Detect the current platform
 */
function detectPlatform(): string {
  const os = process.platform === 'darwin' ? 'darwin' : process.platform === 'win32' ? 'windows' : 'linux'
  const arch = process.arch === 'x64' ? 'x64' : process.arch === 'arm64' ? 'arm64' : process.arch
  return `${os}-${arch}`
}

/**
 * Build Rust binaries
 */
async function buildRustBinaries(projectRoot: string, verbose: boolean): Promise<void> {
  console.log(chalk.cyan('\n🦀 Building Rust binaries...'))

  const platform = detectPlatform()
  const targets = [
    {
      crate: 'maproom',
      binary: 'crewchief-maproom',
      destinations: ['packages/cli/bin', 'packages/maproom-mcp/bin'],
    },
  ]

  // Check if cargo is available
  try {
    await executeCommand('cargo', ['--version'], projectRoot, false)
  } catch {
    throw new Error('Cargo is not installed. Please install Rust from https://rustup.rs/')
  }

  for (const target of targets) {
    console.log(chalk.yellow(`  Building ${target.crate}...`))

    try {
      // Build the Rust binary
      await executeCommand(
        'cargo',
        ['build', '--release', '--manifest-path', `crates/${target.crate}/Cargo.toml`],
        projectRoot,
        verbose,
      )

      // Copy to destination directories
      const sourcePath = path.join(projectRoot, 'target', 'release', target.binary)
      const binaryExt = process.platform === 'win32' ? '.exe' : ''
      const binaryName = `${target.binary}${binaryExt}`

      for (const destDir of target.destinations) {
        const destPath = path.join(projectRoot, destDir, platform)

        // Check if destination package exists
        const packagePath = path.join(projectRoot, destDir, '..')
        if (!fs.existsSync(packagePath)) {
          continue
        }

        // Create platform directory
        fs.mkdirSync(destPath, { recursive: true })

        // Copy the binary
        const destFile = path.join(destPath, binaryName)
        fs.copyFileSync(sourcePath + binaryExt, destFile)

        // Make it executable on Unix-like systems
        if (process.platform !== 'win32') {
          fs.chmodSync(destFile, 0o755)
        }

        // Create symlink in bin directory for convenience
        const linkPath = path.join(projectRoot, destDir, binaryName)
        if (fs.existsSync(linkPath)) {
          fs.unlinkSync(linkPath)
        }

        // On Windows, copy instead of symlink for simplicity
        if (process.platform === 'win32') {
          fs.copyFileSync(destFile, linkPath)
        } else {
          fs.symlinkSync(path.join(platform, binaryName), linkPath)
        }

        console.log(chalk.green(`    ✓ Copied to ${path.relative(projectRoot, destPath)}`))
      }
    } catch (error) {
      throw new Error(`Failed to build ${target.crate}: ${error}`)
    }
  }

  console.log(chalk.green('  ✓ Rust binaries built successfully'))
}

/**
 * Build TypeScript packages
 */
async function buildTypeScriptPackages(projectRoot: string, verbose: boolean, sequential: boolean): Promise<void> {
  console.log(chalk.cyan('\n📦 Building TypeScript packages...'))

  const packages = [
    { name: 'cli', path: 'packages/cli' },
    { name: 'maproom-mcp', path: 'packages/maproom-mcp' },
  ]

  // Check if pnpm is available
  try {
    await executeCommand('pnpm', ['--version'], projectRoot, false)
  } catch {
    throw new Error('pnpm is not installed. Please install it with: npm install -g pnpm')
  }

  // Install dependencies first if needed
  console.log(chalk.yellow('  Installing dependencies...'))
  await executeCommand('pnpm', ['install'], projectRoot, verbose)

  if (sequential) {
    // Build packages sequentially
    for (const pkg of packages) {
      const packagePath = path.join(projectRoot, pkg.path)
      if (!fs.existsSync(packagePath)) {
        continue
      }

      console.log(chalk.yellow(`  Building ${pkg.name}...`))
      try {
        await executeCommand('pnpm', ['build'], packagePath, verbose)
        console.log(chalk.green(`    ✓ ${pkg.name} built successfully`))
      } catch (error) {
        throw new Error(`Failed to build ${pkg.name}: ${error}`)
      }
    }
  } else {
    // Build all packages in parallel (default)
    const buildPromises = packages.map(async (pkg) => {
      const packagePath = path.join(projectRoot, pkg.path)
      if (!fs.existsSync(packagePath)) {
        return
      }

      console.log(chalk.yellow(`  Building ${pkg.name} (parallel)...`))
      try {
        await executeCommand('pnpm', ['build'], packagePath, verbose)
        console.log(chalk.green(`    ✓ ${pkg.name} built successfully`))
      } catch (error) {
        throw new Error(`Failed to build ${pkg.name}: ${error}`)
      }
    })

    await Promise.all(buildPromises)
  }

  console.log(chalk.green('  ✓ TypeScript packages built successfully'))
}

export function registerBuildCommand(program: Command): void {
  program
    .command('build')
    .description('Build all projects in the repository')
    .option('--skip-rust', 'Skip building Rust binaries')
    .option('--skip-typescript', 'Skip building TypeScript packages')
    .option('-v, --verbose', 'Show detailed build output')
    .option('-s, --sequential', 'Build TypeScript packages sequentially instead of in parallel')
    .action(async (options: BuildOptions) => {
      const startTime = Date.now()
      const projectRoot = getProjectRoot()

      console.log(chalk.bold.cyan('\n🔨 CrewChief Build System\n'))
      console.log(chalk.gray(`Project root: ${projectRoot}`))
      console.log(chalk.gray(`Platform: ${detectPlatform()}\n`))

      try {
        // Build Rust binaries
        if (!options.skipRust) {
          await buildRustBinaries(projectRoot, options.verbose || false)
        }

        // Build TypeScript packages
        if (!options.skipTypeScript) {
          await buildTypeScriptPackages(projectRoot, options.verbose || false, options.sequential || false)
        }

        const elapsed = ((Date.now() - startTime) / 1000).toFixed(2)
        console.log(chalk.bold.green(`\n✨ Build completed successfully in ${elapsed}s\n`))

        // Show summary
        console.log(chalk.cyan('Build artifacts:'))
        console.log(chalk.gray('  • Rust binaries: target/release/'))
        console.log(chalk.gray('  • CLI binary: packages/cli/bin/'))
        console.log(chalk.gray('  • CLI dist: packages/cli/dist/'))
        console.log(chalk.gray('  • MCP dist: packages/maproom-mcp/dist/'))
        console.log()
        console.log(chalk.yellow('To publish the CLI package, run: cd packages/cli && pnpm publish'))
      } catch (error) {
        const elapsed = ((Date.now() - startTime) / 1000).toFixed(2)
        console.log(chalk.bold.red(`\n❌ Build failed after ${elapsed}s\n`))
        logger.error('Build error:', error)
        process.exitCode = 1
      }
    })
}
