#!/usr/bin/env node

/**
 * Maproom MCP CLI Wrapper
 *
 * Orchestrates Docker Compose stack (postgres, ollama, maproom-mcp) and proxies
 * stdio between the user and the containerized MCP server.
 *
 * Enables zero-configuration deployment via: npx -y @crewchief/maproom-mcp
 */

const { spawn, spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { needsConfigUpdate, updateConfigs } = require('../dist/config-manager.js');

// Diagnostic Mode: Log environment variables for troubleshooting
const DIAGNOSTIC_MODE = process.env.MAPROOM_MCP_DEBUG === 'true';

// Sensitive environment variables to redact in logs
const SENSITIVE_ENV_VARS = [
  'GOOGLE_APPLICATION_CREDENTIALS',
  'OPENAI_API_KEY',
  'DATABASE_URL',
  'POSTGRES_PASSWORD'
];

// Patterns that indicate sensitive data
const SENSITIVE_PATTERNS = ['KEY', 'SECRET', 'PASSWORD', 'TOKEN'];

/**
 * Redact sensitive values from data object
 * @param {any} data - The data object to redact
 * @returns {any} - A new object with sensitive values replaced by "(redacted)"
 */
function redactSensitive(data) {
  if (!data || typeof data !== 'object') return data;

  const redacted = { ...data };

  Object.keys(redacted).forEach(key => {
    const upperKey = key.toUpperCase();

    // Check explicit list
    const isExplicitlySensitive = SENSITIVE_ENV_VARS.some(
      sensitive => upperKey.includes(sensitive)
    );

    // Check patterns
    const matchesPattern = SENSITIVE_PATTERNS.some(
      pattern => upperKey.includes(pattern)
    );

    if (isExplicitlySensitive || matchesPattern) {
      redacted[key] = '(redacted)';
    } else if (typeof redacted[key] === 'object') {
      // Recursively redact nested objects
      redacted[key] = redactSensitive(redacted[key]);
    }
  });

  return redacted;
}

/**
 * Sanitize DATABASE_URL for logging by masking password
 * @param {string} url - The database URL to sanitize
 * @returns {string} - URL with password replaced by asterisks
 */
function sanitizeDatabaseUrl(url) {
  if (!url) return '(not set)';

  try {
    // Use URL API to properly parse and replace password
    const parsed = new URL(url);
    if (parsed.password) {
      parsed.password = '***';
    }
    return parsed.toString();
  } catch (error) {
    // Fallback to regex for non-standard URLs
    try {
      return url.replace(/:([^@:]+)@/, ':***@');
    } catch (e) {
      return '(invalid URL format)';
    }
  }
}

/**
 * Log diagnostic information to stderr
 * Logs always appear when EMBEDDING_PROVIDER is not set OR when MAPROOM_MCP_DEBUG=true
 */
function diagnosticLog(message, data) {
  if (DIAGNOSTIC_MODE || !process.env.MAPROOM_EMBEDDING_PROVIDER) {
    console.error('🔍 [DIAGNOSTIC]', message);
    if (data) {
      const redactedData = redactSensitive(data);
      console.error('   ', JSON.stringify(redactedData, null, 2));
    }
  }
}

// Log environment variables immediately on startup
diagnosticLog('CLI Started', {
  EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || '(not set)',
  GOOGLE_PROJECT_ID: process.env.GOOGLE_PROJECT_ID || '(not set)',
  GOOGLE_APPLICATION_CREDENTIALS: process.env.GOOGLE_APPLICATION_CREDENTIALS || '(not set)',
  OPENAI_API_KEY: process.env.OPENAI_API_KEY || '(not set)',
  OLLAMA_HOST: process.env.OLLAMA_HOST || '(not set)',
  NODE_ENV: process.env.NODE_ENV || '(not set)',
  cwd: process.cwd(),
  nodeVersion: process.version
});

// Configuration
const CONFIG_DIR = path.join(os.homedir(), '.maproom-mcp');
const COMPOSE_FILE = path.join(CONFIG_DIR, 'docker-compose.yml');
const INIT_SQL_FILE = path.join(CONFIG_DIR, 'init.sql');
const DOCKERFILE_FILE = path.join(CONFIG_DIR, 'Dockerfile.mcp-server');
const MAX_HEALTH_WAIT_MS = 120000; // 2 minutes
const HEALTH_CHECK_INTERVAL_MS = 2000; // 2 seconds

/**
 * Get the correct PostgreSQL connection string based on environment
 * @returns {string} The appropriate connection string
 */
function getDatabaseConnectionString() {
  // Check if MAPROOM_DB_HOST environment variable is set (allows override)
  if (process.env.MAPROOM_DB_HOST) {
    return `postgresql://maproom:maproom@${process.env.MAPROOM_DB_HOST}:${process.env.MAPROOM_DB_PORT || 5432}/maproom`;
  }

  // Try to detect if we're in a Docker/devcontainer environment
  // by checking if we can resolve the maproom-postgres hostname
  try {
    const { execSync } = require('child_process');
    // Quick DNS check for maproom-postgres hostname
    execSync('getent hosts maproom-postgres 2>/dev/null || ping -c 1 -W 1 maproom-postgres 2>/dev/null', {
      stdio: 'pipe',
      timeout: 1000
    });
    // If we got here, maproom-postgres hostname resolves
    diagnosticLog('Using container hostname for database connection');
    return 'postgresql://maproom:maproom@maproom-postgres:5432/maproom';
  } catch (error) {
    // Hostname doesn't resolve, use localhost
    diagnosticLog('Using localhost for database connection');
    return 'postgresql://maproom:maproom@127.0.0.1:5433/maproom';
  }
}

/**
 * Parse command-line arguments
 * @returns {{command: string|null, flags: object}} Parsed command and flags
 */
function parseArgs() {
  const args = process.argv.slice(2);

  // Find the first non-flag argument as the command
  const command = args.find(arg => !arg.startsWith('--'));

  // Parse all --flag=value or --flag arguments
  const flags = {};
  args.forEach(arg => {
    if (arg.startsWith('--')) {
      const flagStr = arg.substring(2);
      const [key, value] = flagStr.split('=');
      flags[key] = value !== undefined ? value : true;
    }
  });

  return { command, flags };
}

// Parse arguments at startup
const { command, flags } = parseArgs();

diagnosticLog('Command Line Arguments', { command, flags });

/**
 * Check if Docker daemon is running
 */
function checkDockerDaemon() {
  console.error('🔍 Checking Docker availability...');

  // Explicitly pass environment variables to docker command
  const env = {
    ...process.env,  // CRITICAL: Include all parent env vars FIRST
    // Ensure key vars are present with defaults
    EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
  };

  diagnosticLog('Docker Command: Checking Docker daemon status', {
    command: 'docker',
    args: ['info'],
    cwd: process.cwd(),
    env: redactSensitive({
      EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
      EMBEDDING_MODEL: env.EMBEDDING_MODEL,
      EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
    })
  });

  const result = spawnSync('docker', ['info'], {
    env: env,
    stdio: 'pipe',
    encoding: 'utf-8'
  });

  if (result.error) {
    console.error('❌ ERROR: Docker is not running or not installed.\n');
    console.error('Please start Docker Desktop or install Docker:');
    console.error('  • macOS: https://docs.docker.com/desktop/install/mac-install/');
    console.error('  • Linux: https://docs.docker.com/engine/install/');
    console.error('  • Windows: https://docs.docker.com/desktop/install/windows-install/');
    process.exit(1);
  }

  if (result.status !== 0) {
    console.error('❌ ERROR: Docker daemon is not running.\n');
    console.error('Please start Docker Desktop and try again.');
    process.exit(1);
  }

  console.error('✓ Docker daemon is running');
}

/**
 * Check if Docker Compose v2 is available
 */
function checkDockerCompose() {
  // Explicitly pass environment variables to docker command
  const env = {
    ...process.env,  // CRITICAL: Include all parent env vars FIRST
    // Ensure key vars are present with defaults
    EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
  };

  diagnosticLog('Docker Command: Checking Docker Compose version', {
    command: 'docker',
    args: ['compose', 'version'],
    cwd: process.cwd(),
    env: redactSensitive({
      EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
      EMBEDDING_MODEL: env.EMBEDDING_MODEL,
      EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
    })
  });

  const result = spawnSync('docker', ['compose', 'version'], {
    env: env,
    stdio: 'pipe',
    encoding: 'utf-8'
  });

  if (result.error || result.status !== 0) {
    console.error('❌ ERROR: Docker Compose v2 is not available.\n');
    console.error('Please install Docker Desktop or Docker Compose v2:');
    console.error('  • Docker Desktop includes Compose v2');
    console.error('  • Or install manually: https://docs.docker.com/compose/install/');
    process.exit(1);
  }

  console.error('✓ Docker Compose v2 detected');
}

/**
 * Setup configuration directory and copy required files
 */
function setupConfigDirectory() {
  // Create config directory if it doesn't exist
  if (!fs.existsSync(CONFIG_DIR)) {
    fs.mkdirSync(CONFIG_DIR, { recursive: true });
    console.error('✓ Created configuration directory:', CONFIG_DIR);
  }

  // Copy docker-compose.yml from package (or update if outdated)
  const srcCompose = path.join(__dirname, '..', 'config', 'docker-compose.yml');

  if (!fs.existsSync(srcCompose)) {
    console.error('❌ ERROR: docker-compose.yml not found in package.');
    console.error('   Expected at:', srcCompose);
    process.exit(1);
  }

  // Check if existing docker-compose.yml needs updating
  let needsUpdate = !fs.existsSync(COMPOSE_FILE);

  if (!needsUpdate && fs.existsSync(COMPOSE_FILE)) {
    const existingContent = fs.readFileSync(COMPOSE_FILE, 'utf-8');
    // Check if file has old hardcoded EMBEDDING_PROVIDER (MCP-008 fix)
    const hasHardcodedProvider = existingContent.includes('EMBEDDING_PROVIDER: ollama');
    const hasEnvironmentVariable = existingContent.includes('${EMBEDDING_PROVIDER');

    if (hasHardcodedProvider && !hasEnvironmentVariable) {
      console.error('⚡ Detected outdated docker-compose.yml (pre-MCP-008)');
      console.error('   Updating to support EMBEDDING_PROVIDER configuration...');
      needsUpdate = true;
    }
  }

  if (needsUpdate) {
    try {
      fs.copyFileSync(srcCompose, COMPOSE_FILE);
      console.error('✓ Updated docker-compose.yml to', CONFIG_DIR);
    } catch (error) {
      console.error('❌ ERROR: Failed to copy docker-compose.yml');
      console.error('   Error:', error.message);
      process.exit(1);
    }
  }

  // Copy init.sql from package
  if (!fs.existsSync(INIT_SQL_FILE)) {
    const srcInitSql = path.join(__dirname, '..', 'config', 'init.sql');

    if (fs.existsSync(srcInitSql)) {
      try {
        fs.copyFileSync(srcInitSql, INIT_SQL_FILE);
        console.error('✓ Copied init.sql to', CONFIG_DIR);
      } catch (error) {
        console.error('⚠️  Warning: Failed to copy init.sql:', error.message);
      }
    }
  }

  // Copy Dockerfile.mcp-server from package
  if (!fs.existsSync(DOCKERFILE_FILE)) {
    const srcDockerfile = path.join(__dirname, '..', 'config', 'Dockerfile.mcp-server');

    if (fs.existsSync(srcDockerfile)) {
      try {
        fs.copyFileSync(srcDockerfile, DOCKERFILE_FILE);
        console.error('✓ Copied Dockerfile.mcp-server to', CONFIG_DIR);
      } catch (error) {
        console.error('⚠️  Warning: Failed to copy Dockerfile.mcp-server:', error.message);
      }
    }
  }

  // Copy TypeScript source files for Docker build
  const srcDir = path.join(__dirname, '..', 'src');
  const destSrcDir = path.join(CONFIG_DIR, 'src');

  if (!fs.existsSync(destSrcDir) && fs.existsSync(srcDir)) {
    try {
      copyRecursive(srcDir, destSrcDir);
      console.error('✓ Copied TypeScript source to', CONFIG_DIR);
    } catch (error) {
      console.error('⚠️  Warning: Failed to copy TypeScript source:', error.message);
    }
  }

  // Copy package.json for Docker build
  const srcPackageJson = path.join(__dirname, '..', 'package.json');
  const destPackageJson = path.join(CONFIG_DIR, 'package.json');

  if (!fs.existsSync(destPackageJson) && fs.existsSync(srcPackageJson)) {
    try {
      fs.copyFileSync(srcPackageJson, destPackageJson);
      console.error('✓ Copied package.json to', CONFIG_DIR);
    } catch (error) {
      console.error('⚠️  Warning: Failed to copy package.json:', error.message);
    }
  }

  // Copy tsconfig.json for Docker build
  const srcTsConfig = path.join(__dirname, '..', 'tsconfig.json');
  const destTsConfig = path.join(CONFIG_DIR, 'tsconfig.json');

  if (!fs.existsSync(destTsConfig) && fs.existsSync(srcTsConfig)) {
    try {
      fs.copyFileSync(srcTsConfig, destTsConfig);
      console.error('✓ Copied tsconfig.json to', CONFIG_DIR);
    } catch (error) {
      console.error('⚠️  Warning: Failed to copy tsconfig.json:', error.message);
    }
  }

  console.error('✓ Configuration ready:', CONFIG_DIR);
}

/**
 * Recursively copy directory
 */
function copyRecursive(src, dest) {
  if (!fs.existsSync(dest)) {
    fs.mkdirSync(dest, { recursive: true });
  }

  const entries = fs.readdirSync(src, { withFileTypes: true });

  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);

    if (entry.isDirectory()) {
      copyRecursive(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

/**
 * Verify docker-compose.yml uses environment variable syntax
 * Prevents silent failures where hardcoded values override user configuration
 *
 * This checks for outdated configs from before MCP-008 and MCP-011 that had:
 *   EMBEDDING_PROVIDER: ollama
 * Instead of the correct environment variable syntax:
 *   EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}
 */
function verifyDockerComposeConfig() {
  if (!fs.existsSync(COMPOSE_FILE)) {
    console.error('⚠️  Warning: docker-compose.yml not found at', COMPOSE_FILE);
    return;
  }

  const content = fs.readFileSync(COMPOSE_FILE, 'utf-8');

  // Check for environment variable syntax (correct pattern)
  const hasEnvVarSyntax = /\$\{EMBEDDING_PROVIDER[:\-]/.test(content);

  // Check for hardcoded provider (incorrect pattern)
  const hasHardcodedProvider = /EMBEDDING_PROVIDER:\s*['"]?ollama['"]?\s*$/m.test(content);

  if (hasHardcodedProvider && !hasEnvVarSyntax) {
    console.error('');
    console.error('❌ ERROR: docker-compose.yml contains hardcoded EMBEDDING_PROVIDER');
    console.error('   File:', COMPOSE_FILE);
    console.error('');
    console.error('   Your config file has:');
    console.error('     EMBEDDING_PROVIDER: ollama');
    console.error('');
    console.error('   It should be:');
    console.error('     EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}');
    console.error('');
    console.error('   This was fixed in MCP-011. Please update your config file or run:');
    console.error('     npx @crewchief/maproom-mcp setup');
    console.error('');
    process.exit(1);
  }

  diagnosticLog('Docker Compose Config Verified', {
    hasEnvVarSyntax,
    hasHardcodedProvider: false,
    configFile: COMPOSE_FILE
  });
}

/**
 * Log current Docker container state for verification
 * Queries `docker compose ps` and logs the actual running state of all containers
 */
function logDockerState() {
  // Explicitly pass environment variables to docker command
  const env = {
    ...process.env,  // CRITICAL: Include all parent env vars FIRST
    // Ensure key vars are present with defaults
    EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
  };

  diagnosticLog('Docker Command: Querying container state', {
    command: 'docker',
    args: ['compose', 'ps', '--format', 'json'],
    cwd: CONFIG_DIR,
    env: redactSensitive({
      EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
      EMBEDDING_MODEL: env.EMBEDDING_MODEL,
      EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
    })
  });

  const result = spawnSync('docker', ['compose', 'ps', '--format', 'json'], {
    cwd: CONFIG_DIR,
    env: env,
    encoding: 'utf-8',
    stdio: 'pipe'
  });

  if (result.status !== 0) {
    diagnosticLog('Container State: Query failed', {
      exitCode: result.status,
      error: result.stderr || 'Unknown error'
    });
    return;
  }

  // Check for empty output (no containers running)
  if (!result.stdout || !result.stdout.trim()) {
    diagnosticLog('Container State', []);
    return;
  }

  try {
    // Parse JSON output (one object per line)
    const containers = result.stdout.trim().split('\n')
      .filter(line => line.trim())
      .map(line => JSON.parse(line));

    // Extract service name, state, and status for each container
    const containerStates = containers.map(c => ({
      service: c.Service,
      state: c.State,
      status: c.Status
    }));

    diagnosticLog('Container State', containerStates);
  } catch (error) {
    diagnosticLog('Container State: Parse error', {
      error: error.message,
      stdout: result.stdout.substring(0, 200) // Log first 200 chars for debugging
    });
  }
}

/**
 * Ensure clean container state before starting services
 * Stops any existing containers to prevent stale configuration
 */
async function ensureCleanState() {
  console.error('\n=== Pre-Flight: Checking for Existing Containers ===');

  // Check if any containers exist
  const psResult = spawnSync('docker', ['compose', 'ps', '-q'], {
    cwd: CONFIG_DIR,
    encoding: 'utf-8',
    stdio: 'pipe'
  });

  const containerIds = psResult.stdout.trim();

  if (containerIds) {
    console.error('Found existing containers, stopping all services...');

    // Log current state before stopping
    logDockerState();

    // Stop all services
    const stopResult = spawnSync('docker', ['compose', 'stop'], {
      cwd: CONFIG_DIR,
      encoding: 'utf-8',
      stdio: 'inherit'
    });

    if (stopResult.status !== 0) {
      console.error('Failed to stop existing containers');
      throw new Error('Container cleanup failed');
    }

    // Wait for complete shutdown
    console.error('Waiting for containers to fully stop...');
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Verify cleanup
    logDockerState();
    console.error('Container cleanup complete\n');
  } else {
    console.error('No existing containers found, clean state confirmed\n');
  }
}

/**
 * Determine which services to start based on EMBEDDING_PROVIDER
 */
function getRequiredServices() {
  const provider = process.env.MAPROOM_EMBEDDING_PROVIDER?.toLowerCase();

  const services = {
    // postgres: NOT included here - automatically started by docker-compose.yml via depends_on
    ollama: false,      // Only if using Ollama provider
    'maproom-mcp': true // Always required - the MCP server itself
  };

  // Determine if Ollama is needed
  if (!provider || provider === 'ollama') {
    // No provider specified (zero-config) or explicitly ollama
    services.ollama = true;
    console.error('🚀 Starting with Ollama (local embeddings)...');
  } else if (provider === 'google') {
    console.error('🚀 Starting with Google Vertex AI...');
    console.error('   (Skipping Ollama - not needed)');
  } else if (provider === 'openai') {
    console.error('🚀 Starting with OpenAI...');
    console.error('   (Skipping Ollama - not needed)');
  } else {
    console.error(`⚠️  Warning: Unknown provider '${provider}', defaulting to Ollama`);
    services.ollama = true;
  }

  return Object.entries(services)
    .filter(([_, needed]) => needed)
    .map(([service, _]) => service);
}

/**
 * Remove unnecessary services that should not be running
 * Stops and removes containers for services not required by current provider configuration
 *
 * @param {string[]} requiredServices - List of services that should be running
 */
function removeUnnecessaryServices(requiredServices) {
  // Note: postgres is NOT included here - it's always needed and auto-started via depends_on
  // We only conditionally start/stop ollama based on the provider
  const CONDITIONAL_SERVICES = ['ollama'];
  const unnecessaryServices = CONDITIONAL_SERVICES.filter(s => !requiredServices.includes(s));

  if (unnecessaryServices.length === 0) {
    console.error('No unnecessary services to remove\n');
    return;
  }

  console.error('\n=== Removing Unnecessary Services ===');
  console.error(`Required services: ${requiredServices.join(', ')} (+ postgres via depends_on)`);
  console.error(`Removing: ${unnecessaryServices.join(', ')}\n`);

  // Explicitly pass environment variables to docker command
  const env = {
    ...process.env,  // CRITICAL: Include all parent env vars FIRST
    // Ensure key vars are present with defaults
    EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
  };

  for (const service of unnecessaryServices) {
    console.error(`Stopping ${service}...`);

    diagnosticLog('Docker Compose Command: Stopping unnecessary service', {
      command: 'docker',
      args: ['compose', 'stop', service],
      cwd: CONFIG_DIR,
      env: redactSensitive({
        EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
        EMBEDDING_MODEL: env.EMBEDDING_MODEL,
        EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
      })
    });

    const stopResult = spawnSync('docker', ['compose', 'stop', service], {
      cwd: CONFIG_DIR,
      env: env,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    // Stopping a non-existent service is not an error
    if (stopResult.status !== 0 && !stopResult.stderr.includes('no such service')) {
      console.error(`Warning: Failed to stop ${service}`);
    }

    console.error(`Removing ${service}...`);

    diagnosticLog('Docker Compose Command: Removing unnecessary service', {
      command: 'docker',
      args: ['compose', 'rm', '-f', service],
      cwd: CONFIG_DIR,
      env: redactSensitive({
        EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
        EMBEDDING_MODEL: env.EMBEDDING_MODEL,
        EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
      })
    });

    const rmResult = spawnSync('docker', ['compose', 'rm', '-f', service], {
      cwd: CONFIG_DIR,
      env: env,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    if (rmResult.status !== 0 && !rmResult.stderr.includes('no such service')) {
      console.error(`Warning: Failed to remove ${service}`);
    }
  }

  // Verify removal
  console.error('\nVerifying removal:');
  logDockerState();
  console.error('Service removal complete\n');
}

/**
 * Verify final container state matches expected services
 * Compares running services against expected services and logs any discrepancies
 *
 * @param {string[]} expectedServices - List of services that should be running
 * @returns {boolean} - True if state matches expectations, false otherwise
 */
function verifyFinalState(expectedServices) {
  console.error('\n=== Verifying Final Container State ===');

  // postgres is always expected (auto-started via depends_on)
  const allExpectedServices = [...expectedServices, 'postgres'];
  console.error(`Expected services: ${allExpectedServices.join(', ')}`);

  // Log current state first
  logDockerState();

  // Explicitly pass environment variables to docker command
  const env = {
    ...process.env,  // CRITICAL: Include all parent env vars FIRST
    // Ensure key vars are present with defaults
    EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
  };

  // Get running services
  const result = spawnSync('docker', ['compose', 'ps', '--format', 'json'], {
    cwd: CONFIG_DIR,
    env: env,
    encoding: 'utf-8',
    stdio: 'pipe'
  });

  if (result.status !== 0) {
    console.error('❌ ERROR: Failed to verify container state');
    console.error(result.stderr);
    return false;
  }

  // Parse JSON output
  const containers = result.stdout.trim().split('\n')
    .filter(line => line.trim())
    .map(line => {
      try {
        return JSON.parse(line);
      } catch (e) {
        console.error(`Warning: Failed to parse container JSON: ${line}`);
        return null;
      }
    })
    .filter(c => c !== null && c.State === 'running');

  const runningServices = containers.map(c => c.Service);

  console.error(`Running services: ${runningServices.join(', ') || '(none)'}`);

  // Check for unexpected services (using allExpectedServices which includes postgres)
  const unexpected = runningServices.filter(s => !allExpectedServices.includes(s));
  if (unexpected.length > 0) {
    console.error(`⚠️  WARNING: Unexpected services running: ${unexpected.join(', ')}`);
    console.error('These services should have been removed. Manual cleanup may be needed.');
  }

  // Check for missing services (using allExpectedServices)
  const missing = allExpectedServices.filter(s => !runningServices.includes(s));
  if (missing.length > 0) {
    console.error(`❌ ERROR: Expected services not running: ${missing.join(', ')}`);
    console.error('Startup may have failed. Check logs above for errors.');
    return false;
  }

  // Success
  if (unexpected.length === 0) {
    console.error(`✅ All expected services running: ${runningServices.join(', ')}`);
    console.error('Final state verification: PASS\n');
    return true;
  } else {
    console.error('Final state verification: PASS (with warnings)\n');
    return true;
  }
}

/**
 * Start Docker Compose stack with selective services
 */
async function startDockerCompose() {
  // Pre-flight: Ensure clean container state
  await ensureCleanState();

  return new Promise((resolve, reject) => {
    const requiredServices = getRequiredServices();

    console.error('📦 Required services:', requiredServices.join(', '));

    // Remove services that shouldn't be running
    removeUnnecessaryServices(requiredServices);

    console.error('⬇️  Starting required services (downloading images if needed)...');

    // Build docker compose command with service selection
    const args = ['compose', 'up', '-d'];

    // Only start required services
    args.push(...requiredServices);

    // Explicitly pass environment variables to docker command
    const env = {
      ...process.env,  // CRITICAL: Include all parent env vars FIRST
      // Ensure key vars are present with defaults
      EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
      EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
      EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
    };

    diagnosticLog('Docker Compose Command: Starting services', {
      command: 'docker',
      args: args,
      cwd: CONFIG_DIR,
      env: redactSensitive({
        EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
        EMBEDDING_MODEL: env.EMBEDDING_MODEL,
        EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION,
        GOOGLE_PROJECT_ID: env.GOOGLE_PROJECT_ID,
        GOOGLE_APPLICATION_CREDENTIALS: env.GOOGLE_APPLICATION_CREDENTIALS,
        OPENAI_API_KEY: env.OPENAI_API_KEY,
        DATABASE_URL: env.DATABASE_URL
      })
    });

    const compose = spawn('docker', args, {
      cwd: CONFIG_DIR,
      env: env,
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf-8'
    });

    let stdout = '';
    let stderr = '';

    compose.stdout.on('data', (data) => {
      stdout += data.toString();
      // Show progress for image pulls
      const lines = data.toString().split('\n');
      for (const line of lines) {
        if (line.includes('Pulling') || line.includes('Downloading') || line.includes('Extracting')) {
          console.error('  ', line.trim());
        }
      }
    });

    compose.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    compose.on('error', (error) => {
      reject(new Error(`Failed to start Docker Compose: ${error.message}`));
    });

    compose.on('exit', (code) => {
      if (code !== 0) {
        // Check for common errors
        const output = stdout + stderr;

        if (output.includes('port is already allocated') || output.includes('address already in use')) {
          console.error('❌ ERROR: Port conflict detected.\n');
          console.error('PostgreSQL requires port 5432, Ollama requires port 11434.');
          console.error('\nTo find conflicting processes:');
          console.error('  lsof -i :5432');
          console.error('  lsof -i :11434');
          console.error('\nOr edit the docker-compose.yml to use different ports:');
          console.error(' ', COMPOSE_FILE);
        } else {
          console.error('❌ ERROR: Docker Compose failed with exit code', code);
          console.error('\nOutput:', output);
        }

        reject(new Error(`Docker Compose exited with code ${code}`));
      } else {
        console.error('✓ Services started');

        // Verify final state
        const stateOk = verifyFinalState(requiredServices);
        if (!stateOk) {
          console.error('\n⚠️  Container state verification failed. Check errors above.');
          console.error('You may need to manually stop containers: docker compose stop');
          reject(new Error('Container state verification failed'));
        } else {
          resolve();
        }
      }
    });
  });
}

/**
 * Wait for all services to become healthy
 */
async function waitForServicesHealthy() {
  const requiredServices = getRequiredServices();

  console.error('⏳ Waiting for services:', requiredServices.join(', '));

  // Only show Ollama model download message if Ollama is being started
  if (requiredServices.includes('ollama')) {
    console.error('   This may take 1-2 minutes on first run (downloading Ollama model)...');
  }

  const startTime = Date.now();
  const serviceStatus = {};

  while (Date.now() - startTime < MAX_HEALTH_WAIT_MS) {
    // Explicitly pass environment variables to docker command
    const env = {
      ...process.env,  // CRITICAL: Include all parent env vars FIRST
      // Ensure key vars are present with defaults
      EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
      EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
      EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
    };

    diagnosticLog('Docker Compose Command: Checking container status', {
      command: 'docker',
      args: ['compose', 'ps', '--format', 'json'],
      cwd: CONFIG_DIR,
      env: redactSensitive({
        EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
        EMBEDDING_MODEL: env.EMBEDDING_MODEL,
        EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
      })
    });

    const result = spawnSync('docker', ['compose', 'ps', '--format', 'json'], {
      cwd: CONFIG_DIR,
      env: env,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    if (result.status === 0 && result.stdout) {
      try {
        // Parse JSON output (one JSON object per line)
        const lines = result.stdout.trim().split('\n').filter(line => line.trim());
        const containers = lines.map(line => JSON.parse(line));

        let allHealthy = true;
        let anyFailed = false;

        for (const serviceName of requiredServices) {
          const container = containers.find(c => c.Service === serviceName);

          if (!container) {
            allHealthy = false;
            if (serviceStatus[serviceName] !== 'missing') {
              console.error(`  ⏳ ${serviceName}: starting...`);
              serviceStatus[serviceName] = 'missing';
            }
            continue;
          }

          const status = container.State;
          const health = container.Health;

          // Check for failure states
          if (status === 'exited' || status === 'dead') {
            anyFailed = true;
            console.error(`  ❌ ${serviceName}: failed (${status})`);
            break;
          }

          // Check health status
          if (health === 'healthy') {
            if (serviceStatus[serviceName] !== 'healthy') {
              console.error(`  ✓ ${serviceName}: healthy`);
              serviceStatus[serviceName] = 'healthy';
            }
          } else if (health === 'unhealthy') {
            allHealthy = false;
            if (serviceStatus[serviceName] !== 'unhealthy') {
              console.error(`  ⏳ ${serviceName}: unhealthy, retrying...`);
              serviceStatus[serviceName] = 'unhealthy';
            }
          } else if (status === 'running' && !health) {
            // No health check defined, consider running as healthy
            if (serviceStatus[serviceName] !== 'running') {
              console.error(`  ✓ ${serviceName}: running`);
              serviceStatus[serviceName] = 'running';
            }
          } else {
            // Starting or in progress
            allHealthy = false;
            if (serviceStatus[serviceName] !== 'starting') {
              console.error(`  ⏳ ${serviceName}: ${health || status}...`);
              serviceStatus[serviceName] = 'starting';
            }
          }
        }

        if (anyFailed) {
          console.error('\n❌ ERROR: One or more services failed to start.\n');
          console.error('Check logs with:');
          console.error(`  cd ${CONFIG_DIR}`);
          console.error('  docker compose logs postgres');
          console.error('  docker compose logs ollama');
          console.error('  docker compose logs maproom-mcp');
          process.exit(1);
        }

        if (allHealthy) {
          console.error('✓ All services are healthy');
          return true;
        }
      } catch (error) {
        // JSON parse error, continue waiting
        console.error('  ⏳ Waiting for container status...');
      }
    }

    // Wait before next check
    await sleep(HEALTH_CHECK_INTERVAL_MS);
  }

  // Timeout reached
  console.error('\n❌ ERROR: Services did not become healthy within 2 minutes.\n');
  console.error('Check logs for errors:');
  console.error(`  cd ${CONFIG_DIR}`);
  console.error('  docker compose logs postgres');
  console.error('  docker compose logs ollama');
  console.error('  docker compose logs maproom-mcp');
  console.error('\nTry restarting:');
  console.error('  docker compose down && docker compose up -d');
  process.exit(1);
}

/**
 * Establish stdio proxy to containerized MCP server
 */
function establishStdioProxy() {
  console.error('🔗 Connected to MCP server (stdio mode)');
  console.error('📝 Logs available: docker compose logs -f (in', CONFIG_DIR + ')');
  console.error('');

  // Explicitly pass environment variables to docker command
  const env = {
    ...process.env,  // CRITICAL: Include all parent env vars FIRST
    // Ensure key vars are present with defaults
    EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
  };

  diagnosticLog('Docker Command: Establishing stdio proxy to MCP server', {
    command: 'docker',
    args: ['exec', '-i', 'maproom-mcp', 'node', '/app/dist/index.js'],
    cwd: process.cwd(),
    env: redactSensitive({
      EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
      EMBEDDING_MODEL: env.EMBEDDING_MODEL,
      EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
    })
  });

  // Proxy stdin/stdout to the maproom-mcp container
  // Use 'exec -T' for non-TTY mode (required for stdio piping)
  const proxy = spawn('docker', [
    'exec',
    '-i',              // Keep stdin open
    'maproom-mcp',     // Container name from docker-compose.yml
    'node',            // Run node
    '/app/dist/index.js'  // MCP server entrypoint
  ], {
    env: env,
    stdio: ['pipe', 'pipe', 'inherit'] // stdin: pipe, stdout: pipe, stderr: inherit (for logs)
  });

  // Forward user stdin to container stdin
  process.stdin.pipe(proxy.stdin);

  // Forward container stdout to user stdout
  proxy.stdout.pipe(process.stdout);

  // Handle proxy errors
  proxy.on('error', (error) => {
    console.error('❌ ERROR: Failed to connect to MCP server:', error.message);
    process.exit(1);
  });

  // Handle proxy exit
  proxy.on('exit', (code, signal) => {
    if (code !== 0 && signal !== 'SIGTERM' && signal !== 'SIGINT') {
      console.error(`❌ ERROR: MCP server exited unexpectedly (code: ${code}, signal: ${signal})`);
      process.exit(code || 1);
    }
    process.exit(0);
  });

  // Handle user interrupts (Ctrl+C) - graceful shutdown
  process.on('SIGINT', () => {
    console.error('\n⏸️  Disconnecting from MCP server (services still running)...');
    console.error('To stop services: cd', CONFIG_DIR, '&& docker compose down');
    proxy.kill('SIGTERM');
    setTimeout(() => process.exit(0), 100);
  });

  process.on('SIGTERM', () => {
    proxy.kill('SIGTERM');
    setTimeout(() => process.exit(0), 100);
  });

  return proxy;
}

/**
 * Validate provider configuration has required environment variables
 * Prevents silent failures by checking before Docker Compose starts
 *
 * @param {string} provider - The embedding provider (google, openai, ollama, etc.)
 */
function validateProviderConfig(provider) {
  diagnosticLog(`Validating provider configuration for: ${provider}`);

  if (provider === 'google') {
    if (!process.env.GOOGLE_PROJECT_ID) {
      console.error('❌ ERROR: EMBEDDING_PROVIDER=google requires GOOGLE_PROJECT_ID');
      console.error('   Check your .mcp.json configuration or set environment variable:');
      console.error('   export GOOGLE_PROJECT_ID=your-project-id');
      process.exit(1);
    }
    diagnosticLog('✓ GOOGLE_PROJECT_ID found');

    if (!process.env.GOOGLE_APPLICATION_CREDENTIALS) {
      console.error('⚠️  WARNING: GOOGLE_APPLICATION_CREDENTIALS not set');
      console.error('   Google Vertex AI may not work without credentials');
    } else {
      diagnosticLog('✓ GOOGLE_APPLICATION_CREDENTIALS found');
    }
  } else if (provider === 'openai') {
    if (!process.env.OPENAI_API_KEY) {
      console.error('❌ ERROR: EMBEDDING_PROVIDER=openai requires OPENAI_API_KEY');
      console.error('   Check your .mcp.json configuration or set environment variable:');
      console.error('   export OPENAI_API_KEY=your-api-key');
      process.exit(1);
    }
    diagnosticLog('✓ OPENAI_API_KEY found');
  } else if (provider === 'ollama' || !provider) {
    diagnosticLog('Using ollama provider (zero-config)');
  } else {
    console.error(`⚠️  WARNING: Unknown provider: ${provider}`);
    console.error('   Supported: ollama, google, openai');
  }
}

/**
 * Validate provider requirements during setup
 * (More lenient than validateProviderConfig - shows helpful messages)
 */
function validateProviderRequirements(provider) {
  console.error(`\n🔍 Validating ${provider.toUpperCase()} configuration...\n`);

  if (provider === 'openai') {
    if (!process.env.OPENAI_API_KEY) {
      console.error('❌ OpenAI API key required!\n');
      console.error('Set OPENAI_API_KEY environment variable:');
      console.error('  export OPENAI_API_KEY=sk-...\n');
      console.error('Get your API key: https://platform.openai.com/api-keys\n');
      process.exit(1);
    }
    console.error('✓ OPENAI_API_KEY found\n');
  } else if (provider === 'google') {
    if (!process.env.GOOGLE_PROJECT_ID) {
      console.error('❌ Google Cloud project required!\n');
      console.error('Set GOOGLE_PROJECT_ID environment variable:');
      console.error('  export GOOGLE_PROJECT_ID=my-project\n');
      process.exit(1);
    }
    console.error('✓ GOOGLE_PROJECT_ID found\n');

    if (!process.env.GOOGLE_APPLICATION_CREDENTIALS) {
      console.error('⚠️  Warning: GOOGLE_APPLICATION_CREDENTIALS not set');
      console.error('   Authentication will use Application Default Credentials\n');
    } else {
      console.error('✓ GOOGLE_APPLICATION_CREDENTIALS found\n');
    }
  } else if (provider === 'ollama') {
    console.error('✓ Ollama requires no API keys (100% local)\n');
    console.error('⚠️  Note: Ollama is slower without GPU hardware\n');
  }
}

/**
 * Save the chosen provider to config directory
 */
function saveProviderChoice(provider) {
  const providerPath = path.join(CONFIG_DIR, '.provider');
  fs.writeFileSync(providerPath, provider);

  const markerPath = path.join(CONFIG_DIR, '.setup-complete');
  fs.writeFileSync(markerPath, new Date().toISOString());

  diagnosticLog('Provider choice saved', { provider, path: providerPath });
}

/**
 * Get the configured provider (from saved config)
 */
function getConfiguredProvider() {
  const providerPath = path.join(CONFIG_DIR, '.provider');

  if (fs.existsSync(providerPath)) {
    return fs.readFileSync(providerPath, 'utf-8').trim();
  }

  // Default to ollama if not configured
  return 'ollama';
}

/**
 * Sleep helper
 */
function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Initialize database schema using maproom binary migrations
 */
async function initializeDatabaseSchema() {
  console.error('🗄️  Initializing database schema...\n');

  try {
    const binPath = await getMaproomBinaryPath();
    const connectionString = getDatabaseConnectionString();

    const result = spawnSync(binPath, ['db', 'migrate'], {
      stdio: 'pipe',
      encoding: 'utf-8',
      env: {
        ...process.env,
        DATABASE_URL: connectionString
      }
    });

    if (result.status !== 0) {
      console.error('❌ Database migration failed:', result.stderr);
      throw new Error('Database migration failed');
    }

    diagnosticLog('Database migrations applied', { stdout: result.stdout });
    console.error('✓ Database schema created\n');
  } catch (error) {
    console.error('❌ Database initialization failed:', error.message);
    throw error;
  }
}

/**
 * Validate database schema exists
 */
async function validateDatabaseSchema() {
  diagnosticLog('Validating database schema');

  try {
    const { Client } = require('pg');
    const connectionString = getDatabaseConnectionString();
    const client = new Client({ connectionString });

    await client.connect();

    // Check if maproom schema exists
    const schemaResult = await client.query(`
      SELECT schema_name FROM information_schema.schemata
      WHERE schema_name = 'maproom'
    `);

    if (schemaResult.rows.length === 0) {
      await client.end();
      throw new Error('Database schema not initialized. Run setup: npx @crewchief/maproom-mcp setup');
    }

    // Check if key tables exist
    const tablesResult = await client.query(`
      SELECT table_name FROM information_schema.tables
      WHERE table_schema = 'maproom'
      AND table_name IN ('repos', 'chunks', 'files')
    `);

    if (tablesResult.rows.length < 3) {
      await client.end();
      throw new Error('Database schema incomplete. Run setup: npx @crewchief/maproom-mcp setup');
    }

    await client.end();

    diagnosticLog('Database schema validated successfully');
  } catch (error) {
    if (error.code === 'ECONNREFUSED') {
      throw new Error('Database not running. Services may not be started.');
    }
    throw error;
  }
}

/**
 * Show provider-specific completion message
 */
function showCompletionMessage(provider) {
  console.error('\n✅ Setup complete!\n');

  if (provider === 'openai') {
    console.error('📝 Add this to your MCP configuration:\n');
    console.error('Claude Code (.claude/mcp.json):');
    console.error(JSON.stringify({
      mcpServers: {
        maproom: {
          command: "npx",
          args: ["-y", "@crewchief/maproom-mcp"],
          env: {
            EMBEDDING_PROVIDER: "openai",
            OPENAI_API_KEY: "${OPENAI_API_KEY}"
          }
        }
      }
    }, null, 2));
    console.error('\nCursor (.cursor/mcp.json):');
    console.error(JSON.stringify({
      mcpServers: {
        maproom: {
          command: "npx",
          args: ["-y", "@crewchief/maproom-mcp"],
          env: {
            EMBEDDING_PROVIDER: "openai",
            OPENAI_API_KEY: "${OPENAI_API_KEY}"
          }
        }
      }
    }, null, 2));
    console.error('\n💡 Set OPENAI_API_KEY in your shell profile (e.g. ~/.bashrc)\n');
  } else if (provider === 'google') {
    console.error('📝 Add this to your MCP configuration:\n');
    console.error('Claude Code (.claude/mcp.json):');
    console.error(JSON.stringify({
      mcpServers: {
        maproom: {
          command: "npx",
          args: ["-y", "@crewchief/maproom-mcp"],
          env: {
            EMBEDDING_PROVIDER: "google",
            GOOGLE_PROJECT_ID: "${GOOGLE_PROJECT_ID}",
            GOOGLE_APPLICATION_CREDENTIALS: "${GOOGLE_APPLICATION_CREDENTIALS}"
          }
        }
      }
    }, null, 2));
    console.error('\nCursor (.cursor/mcp.json):');
    console.error(JSON.stringify({
      mcpServers: {
        maproom: {
          command: "npx",
          args: ["-y", "@crewchief/maproom-mcp"],
          env: {
            EMBEDDING_PROVIDER: "google",
            GOOGLE_PROJECT_ID: "${GOOGLE_PROJECT_ID}",
            GOOGLE_APPLICATION_CREDENTIALS: "${GOOGLE_APPLICATION_CREDENTIALS}"
          }
        }
      }
    }, null, 2));
    console.error('\n💡 Set env vars in your shell profile (e.g. ~/.bashrc)\n');
  } else {  // ollama
    console.error('📝 Add this to your MCP configuration:\n');
    console.error('Claude Code (.claude/mcp.json):');
    console.error(JSON.stringify({
      mcpServers: {
        maproom: {
          command: "npx",
          args: ["-y", "@crewchief/maproom-mcp"]
        }
      }
    }, null, 2));
    console.error('\nCursor (.cursor/mcp.json):');
    console.error(JSON.stringify({
      mcpServers: {
        maproom: {
          command: "npx",
          args: ["-y", "@crewchief/maproom-mcp"]
        }
      }
    }, null, 2));
    console.error('\n💡 Ollama embeddings are 100% local (no API keys needed)\n');
    console.error('⚠️  Note: Ollama is slower without GPU. Consider OpenAI/Google for better performance.\n');
  }

  console.error('📊 Next steps:');
  console.error('  1. Add config to your MCP file');
  console.error('  2. Restart your MCP client (Claude Code, Cursor)');
  console.error('  3. Index your codebase:');
  console.error(`       EMBEDDING_PROVIDER=${provider} npx @crewchief/maproom-mcp scan /path/to/repo`);
  console.error('  4. Keep index updated:');
  console.error(`       EMBEDDING_PROVIDER=${provider} npx @crewchief/maproom-mcp watch /path/to/repo\n`);
}

/**
 * Detect repository information from git
 */
async function detectRepoInfo(scanPath) {
  try {
    const repoResult = spawnSync('git', ['remote', 'get-url', 'origin'], {
      cwd: scanPath,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    const worktreeResult = spawnSync('git', ['branch', '--show-current'], {
      cwd: scanPath,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    const commitResult = spawnSync('git', ['rev-parse', 'HEAD'], {
      cwd: scanPath,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    if (repoResult.status !== 0 || worktreeResult.status !== 0 || commitResult.status !== 0) {
      throw new Error('Not a git repository');
    }

    const repo = repoResult.stdout.trim().split('/').pop().replace('.git', '');
    const worktree = worktreeResult.stdout.trim();
    const commit = commitResult.stdout.trim();

    return { repo, worktree, commit };
  } catch (error) {
    console.error('❌ Not a git repository:', scanPath);
    console.error('   Scan command requires a git repository\n');
    process.exit(1);
  }
}

/**
 * Get path to Maproom binary
 */
async function getMaproomBinaryPath() {
  const platform = process.platform;
  const arch = process.arch;

  let binaryName = 'crewchief-maproom';
  if (platform === 'win32') binaryName += '.exe';

  // Look for binary in package
  const packageBinPath = path.join(__dirname, '..', 'bin', `${platform}-${arch}`, binaryName);

  if (fs.existsSync(packageBinPath)) {
    return packageBinPath;
  }

  // Fall back to cargo build (development)
  const devBinPath = path.join(__dirname, '..', '..', '..', 'target', 'release', binaryName);

  if (fs.existsSync(devBinPath)) {
    return devBinPath;
  }

  console.error('❌ Maproom binary not found');
  console.error('Expected:', packageBinPath);
  console.error('\nFor development, build with: cargo build --release --bin crewchief-maproom\n');
  process.exit(1);
}

/**
 * Ensure Docker Compose services are running
 */
async function ensureServicesRunning() {
  console.error('🔍 Checking Docker services...\n');

  const result = spawnSync('docker', ['compose', 'ps', '--format', 'json'], {
    cwd: CONFIG_DIR,
    stdio: 'pipe',
    encoding: 'utf-8',
    env: process.env
  });

  if (result.status !== 0 || !result.stdout || result.stdout.trim() === '') {
    console.error('⚠️  Services not running, starting them...\n');
    await startDockerCompose();
    await waitForServicesHealthy();
  } else {
    const services = result.stdout.trim().split('\n').map(line => JSON.parse(line));
    const allRunning = services.every(s => s.State === 'running');

    if (!allRunning) {
      console.error('⚠️  Some services stopped, restarting...\n');
      await startDockerCompose();
      await waitForServicesHealthy();
    } else {
      console.error('✓ Services running\n');
    }
  }
}

/**
 * Run scan command
 */
async function runScan() {
  const scanPath = flags.path || process.argv[3] || process.cwd();

  console.error(`\n📊 Scanning repository: ${scanPath}\n`);

  // Ensure containers are running
  await ensureServicesRunning();

  // Detect repo info
  const repoInfo = await detectRepoInfo(scanPath);

  console.error(`Repository: ${repoInfo.repo}`);
  console.error(`Worktree: ${repoInfo.worktree}`);
  console.error(`Commit: ${repoInfo.commit}\n`);

  // Get provider from environment variable or saved config
  const provider = process.env.MAPROOM_EMBEDDING_PROVIDER?.toLowerCase() || getConfiguredProvider();
  console.error(`Using provider: ${provider.toUpperCase()}\n`);

  // Set provider-specific environment variables
  const providerEnv = {
    EMBEDDING_PROVIDER: provider
  };

  if (provider === 'openai') {
    providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
    providerEnv.EMBEDDING_DIMENSION = '1536';
  } else if (provider === 'google') {
    providerEnv.EMBEDDING_MODEL = 'text-embedding-004';
    providerEnv.EMBEDDING_DIMENSION = '768';
  } else if (provider === 'ollama') {
    providerEnv.EMBEDDING_MODEL = 'nomic-embed-text';
    providerEnv.EMBEDDING_DIMENSION = '768';
    providerEnv.EMBEDDING_API_ENDPOINT = 'http://localhost:11434';
  }

  // Get binary path
  const binPath = await getMaproomBinaryPath();

  const args = [
    'scan',
    '--path', scanPath,
    '--repo', repoInfo.repo,
    '--worktree', repoInfo.worktree,
    '--commit', repoInfo.commit,
    '--provider', provider
  ];

  console.error('⚙️  Running indexer...\n');

  // Build environment with provider-specific settings
  const env = {
    ...process.env,
    ...providerEnv
  };

  // Only set DATABASE_URL if not already set
  if (!env.DATABASE_URL) {
    env.DATABASE_URL = getDatabaseConnectionString();
    console.error('🔗 Auto-detected database connection');
  } else {
    console.error('🔗 Using explicit DATABASE_URL from environment');
  }

  // Note: EMBEDDING_API_ENDPOINT is now explicitly set for all providers in providerEnv

  // Debug: Show embedding environment variables
  console.error('🔍 [DEBUG] Embedding environment variables being passed:');
  console.error(`   EMBEDDING_PROVIDER: ${env.EMBEDDING_PROVIDER}`);
  console.error(`   EMBEDDING_MODEL: ${env.EMBEDDING_MODEL}`);
  console.error(`   EMBEDDING_DIMENSION: ${env.EMBEDDING_DIMENSION}`);
  console.error(`   EMBEDDING_API_ENDPOINT: ${env.EMBEDDING_API_ENDPOINT || '(not set)'}`);
  console.error(`   DATABASE_URL: ${sanitizeDatabaseUrl(env.DATABASE_URL)}`);
  console.error(`   OPENAI_API_KEY: ${env.OPENAI_API_KEY ? '(set)' : '(NOT SET)'}\n`);

  const result = spawnSync(binPath, args, {
    stdio: 'inherit',
    env
  });

  if (result.status !== 0) {
    console.error('\n❌ Scan failed');
    process.exit(1);
  }

  console.error('\n✅ Scan complete!\n');
  console.error('Use Maproom MCP tools to search your codebase.\n');
}

/**
 * Run watch command
 */
async function runWatch() {
  const watchPath = flags.path || process.argv[3] || process.cwd();
  const debounceMs = parseInt(flags.debounce) || 3000;  // 3 second default

  console.error(`\n👁️  Watching repository: ${watchPath}\n`);
  console.error(`Debounce: ${debounceMs}ms\n`);

  // Ensure containers are running
  await ensureServicesRunning();

  // Get repo info
  const repoInfo = await detectRepoInfo(watchPath);

  console.error(`Repository: ${repoInfo.repo}`);
  console.error(`Worktree: ${repoInfo.worktree}\n`);

  // Get provider from environment variable or saved config
  const provider = process.env.MAPROOM_EMBEDDING_PROVIDER?.toLowerCase() || getConfiguredProvider();
  console.error(`Using provider: ${provider.toUpperCase()}\n`);

  console.error('Watching for changes (Ctrl+C to stop)...\n');

  // Check if chokidar is available
  let chokidar;
  try {
    chokidar = require('chokidar');
  } catch (error) {
    console.error('❌ chokidar module not found');
    console.error('Install it with: npm install chokidar\n');
    process.exit(1);
  }

  let debounceTimer = null;
  let changedFiles = new Set();

  const watcher = chokidar.watch(watchPath, {
    ignored: /(^|[\/\\])\.|node_modules|\.git|dist|target|build/,
    persistent: true,
    ignoreInitial: true
  });

  watcher.on('change', (filePath) => {
    const relPath = path.relative(watchPath, filePath);

    // Only watch relevant files
    if (relPath.match(/\.(ts|js|tsx|jsx|rs|go|py|java|md|json|yaml|toml|c|cpp|h|hpp|cs|rb|php|swift|kt)$/)) {
      changedFiles.add(relPath);

      // Debounce: wait for changes to settle
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(async () => {
        const filesToUpdate = Array.from(changedFiles);
        changedFiles.clear();

        console.error(`\n📝 Detected changes in ${filesToUpdate.length} file(s)`);
        console.error('   Re-indexing...\n');

        try {
          await upsertFiles(watchPath, repoInfo, provider, filesToUpdate);
          console.error('✓ Index updated\n');
        } catch (error) {
          console.error('⚠️  Re-index failed:', error.message, '\n');
        }

        console.error('Watching for changes...\n');
      }, debounceMs);
    }
  });

  // Keep process alive
  process.on('SIGINT', () => {
    console.error('\n\n👋 Stopping watch...\n');
    watcher.close();
    process.exit(0);
  });
}

/**
 * Upsert files to index
 */
async function upsertFiles(rootPath, repoInfo, provider, files) {
  const binPath = await getMaproomBinaryPath();

  // Update commit hash to latest
  const commitResult = spawnSync('git', ['rev-parse', 'HEAD'], {
    cwd: rootPath,
    encoding: 'utf-8',
    stdio: 'pipe'
  });
  const commit = commitResult.stdout.trim();

  // Set provider-specific environment variables
  const providerEnv = {
    EMBEDDING_PROVIDER: provider
  };

  if (provider === 'openai') {
    providerEnv.EMBEDDING_MODEL = 'text-embedding-3-small';
    providerEnv.EMBEDDING_DIMENSION = '1536';
  } else if (provider === 'google') {
    providerEnv.EMBEDDING_MODEL = 'text-embedding-004';
    providerEnv.EMBEDDING_DIMENSION = '768';
  } else if (provider === 'ollama') {
    providerEnv.EMBEDDING_MODEL = 'nomic-embed-text';
    providerEnv.EMBEDDING_DIMENSION = '768';
    providerEnv.EMBEDDING_API_ENDPOINT = 'http://localhost:11434';
  }

  const args = [
    'upsert',
    '--root', rootPath,
    '--repo', repoInfo.repo,
    '--worktree', repoInfo.worktree,
    '--commit', commit,
    '--provider', provider,
    '--paths', files.join(',')
  ];

  // Build environment with provider-specific settings
  const env = {
    ...process.env,
    ...providerEnv
  };

  // Only set DATABASE_URL if not already set
  if (!env.DATABASE_URL) {
    env.DATABASE_URL = getDatabaseConnectionString();
    console.error('🔗 Auto-detected database connection');
  } else {
    console.error('🔗 Using explicit DATABASE_URL from environment');
  }

  // Debug: Show database connection
  console.error('🔍 [DEBUG] Database connection:');
  console.error(`   DATABASE_URL: ${sanitizeDatabaseUrl(env.DATABASE_URL)}\n`);

  // Note: EMBEDDING_API_ENDPOINT is now explicitly set for all providers in providerEnv

  const result = spawnSync(binPath, args, {
    stdio: 'pipe',
    encoding: 'utf-8',
    env
  });

  if (result.status !== 0) {
    throw new Error(result.stderr || 'Upsert failed');
  }
}

/**
 * Run setup command
 */
async function runSetup() {
  const provider = flags.provider;

  if (!provider) {
    console.error('\n🎯 Provider selection required!\n');
    console.error('Choose an embedding provider:\n');
    console.error('  1. OpenAI (Recommended - fast, low cost)');
    console.error('     npx @crewchief/maproom-mcp setup --provider=openai\n');
    console.error('  2. Google Vertex AI (Recommended - fast, low cost)');
    console.error('     npx @crewchief/maproom-mcp setup --provider=google\n');
    console.error('  3. Ollama (Local, no API key - slower without GPU)');
    console.error('     npx @crewchief/maproom-mcp setup --provider=ollama\n');
    process.exit(1);
  }

  if (!['openai', 'google', 'ollama'].includes(provider)) {
    console.error('❌ Invalid provider. Choose: openai, google, or ollama');
    process.exit(1);
  }

  console.error(`\n🚀 Setting up Maproom with ${provider.toUpperCase()} embeddings...\n`);

  // Set environment variables for this session
  process.env.MAPROOM_EMBEDDING_PROVIDER = provider;

  // Set provider-specific model and dimension
  if (provider === 'openai') {
    process.env.MAPROOM_EMBEDDING_MODEL = 'text-embedding-3-small';
    process.env.MAPROOM_EMBEDDING_DIMENSION = '1536';
  } else if (provider === 'google') {
    process.env.MAPROOM_EMBEDDING_MODEL = 'text-embedding-004';
    process.env.MAPROOM_EMBEDDING_DIMENSION = '768';
  } else if (provider === 'ollama') {
    process.env.MAPROOM_EMBEDDING_MODEL = 'nomic-embed-text';
    process.env.MAPROOM_EMBEDDING_DIMENSION = '768';
    process.env.MAPROOM_EMBEDDING_API_ENDPOINT = 'http://localhost:11434';
  }

  // Validate provider-specific requirements
  validateProviderRequirements(provider);

  // Standard checks
  checkDockerDaemon();
  checkDockerCompose();

  // Copy configs
  setupConfigDirectory();

  // Start Docker Compose (respects EMBEDDING_PROVIDER)
  await startDockerCompose();

  // Wait for services
  await waitForServicesHealthy();

  // Initialize database schema
  await initializeDatabaseSchema();

  // Validate schema
  await validateDatabaseSchema();

  // Save provider choice and create setup marker
  saveProviderChoice(provider);

  // Show completion message with provider-specific config
  showCompletionMessage(provider);
}

/**
 * Run MCP server mode
 */
async function runMCPServer() {
  // Check if setup has been completed
  const setupMarkerPath = path.join(CONFIG_DIR, '.setup-complete');
  if (!fs.existsSync(setupMarkerPath)) {
    console.error('❌ Setup required!\n');
    console.error('Run setup first:\n');
    console.error('  npx @crewchief/maproom-mcp setup --provider=openai\n');
    console.error('Or choose a provider:\n');
    console.error('  --provider=openai   (Recommended, fast)');
    console.error('  --provider=google   (Recommended, fast)');
    console.error('  --provider=ollama   (Local, slower)\n');
    process.exit(1);
  }

  // Normal MCP server startup
  try {
    // Check for config updates
    if (needsConfigUpdate()) {
      console.log('\n📦 Maproom MCP configs need updating...\n');

      try {
        updateConfigs();
        console.log('\n✅ Configs updated successfully!\n');
      } catch (updateError) {
        console.error('\n❌ Failed to update configs:', updateError.message);
        console.error('\n💡 Recovery: Delete ~/.maproom-mcp/ and re-run this command\n');
        process.exit(1);
      }
    }

    // Pre-flight checks
    checkDockerDaemon();
    checkDockerCompose();

    // Setup configuration
    setupConfigDirectory();

    // Verify docker-compose.yml uses environment variables (not hardcoded values)
    verifyDockerComposeConfig();

    // Validate provider configuration (after config verification, before Docker Compose starts)
    const embeddingProvider = process.env.MAPROOM_EMBEDDING_PROVIDER || getConfiguredProvider();
    validateProviderConfig(embeddingProvider);

    // Start Docker Compose stack
    await startDockerCompose();

    // Wait for services to become healthy
    await waitForServicesHealthy();

    // Validate database schema
    await validateDatabaseSchema();

    // Establish stdio proxy
    establishStdioProxy();

  } catch (error) {
    console.error('❌ ERROR:', error.message);
    console.error('\n💡 Try running setup again:');
    console.error('  npx @crewchief/maproom-mcp setup --provider=<your-provider>\n');
    process.exit(1);
  }
}

/**
 * Main entry point - routes to appropriate command
 */
async function main() {
  try {
    // Route to appropriate command
    if (command === 'setup' || flags.setup) {
      await runSetup();
      process.exit(0);
    }

    if (command === 'scan') {
      await runScan();
      process.exit(0);
    }

    if (command === 'watch') {
      await runWatch();
      // Doesn't exit - runs until Ctrl+C
      return;
    }

    // Default: MCP server mode
    await runMCPServer();

  } catch (error) {
    console.error('❌ ERROR:', error.message);
    if (DIAGNOSTIC_MODE) {
      console.error('\nStack trace:');
      console.error(error.stack);
    }
    process.exit(1);
  }
}

// Run main
main();
