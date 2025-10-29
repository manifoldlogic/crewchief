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
 * Log diagnostic information to stderr
 * Logs always appear when EMBEDDING_PROVIDER is not set OR when MAPROOM_MCP_DEBUG=true
 */
function diagnosticLog(message, data) {
  if (DIAGNOSTIC_MODE || !process.env.EMBEDDING_PROVIDER) {
    console.error('🔍 [DIAGNOSTIC]', message);
    if (data) {
      const redactedData = redactSensitive(data);
      console.error('   ', JSON.stringify(redactedData, null, 2));
    }
  }
}

// Log environment variables immediately on startup
diagnosticLog('CLI Started', {
  EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || '(not set)',
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
 * Check if Docker daemon is running
 */
function checkDockerDaemon() {
  console.error('🔍 Checking Docker availability...');

  // Explicitly pass environment variables to docker command
  const env = {
    ...process.env,  // CRITICAL: Include all parent env vars FIRST
    // Ensure key vars are present with defaults
    EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
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
    EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
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
    EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
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
 * Determine which services to start based on EMBEDDING_PROVIDER
 */
function getRequiredServices() {
  const provider = process.env.EMBEDDING_PROVIDER?.toLowerCase();

  const services = {
    postgres: true,     // Always required for database
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
 * Start Docker Compose stack with selective services
 */
function startDockerCompose() {
  return new Promise((resolve, reject) => {
    const requiredServices = getRequiredServices();

    console.error('📦 Required services:', requiredServices.join(', '));

    // Stop any services that are running but not required
    const allServices = ['postgres', 'ollama', 'maproom-mcp'];
    const unnecessaryServices = allServices.filter(s => !requiredServices.includes(s));

    if (unnecessaryServices.length > 0) {
      console.error('🛑 Stopping unnecessary services:', unnecessaryServices.join(', '));

      // Explicitly pass environment variables to docker command
      const env = {
        ...process.env,  // CRITICAL: Include all parent env vars FIRST
        // Ensure key vars are present with defaults
        EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
        EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
        EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
      };

      diagnosticLog('Docker Compose Command: Stopping unnecessary services', {
        command: 'docker',
        args: ['compose', 'stop', ...unnecessaryServices],
        cwd: CONFIG_DIR,
        env: redactSensitive({
          EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
          EMBEDDING_MODEL: env.EMBEDDING_MODEL,
          EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
        })
      });

      const stopResult = spawnSync('docker', ['compose', 'stop', ...unnecessaryServices], {
        cwd: CONFIG_DIR,
        env: env,
        stdio: 'pipe'
      });

      if (stopResult.status === 0) {
        console.error('   ✓ Stopped:', unnecessaryServices.join(', '));
        // Log container state after stopping services
        logDockerState();
      }
    }

    console.error('⬇️  Starting required services (downloading images if needed)...');

    // Build docker compose command with service selection
    const args = ['compose', 'up', '-d'];

    // Only start required services
    args.push(...requiredServices);

    // Explicitly pass environment variables to docker command
    const env = {
      ...process.env,  // CRITICAL: Include all parent env vars FIRST
      // Ensure key vars are present with defaults
      EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
      EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
      EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
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
        // Log container state after starting services
        logDockerState();
        resolve();
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
      EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
      EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
      EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
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
    EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
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
 * Sleep helper
 */
function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Main entry point
 */
async function main() {
  try {
    // Pre-flight checks
    checkDockerDaemon();
    checkDockerCompose();

    // Setup configuration
    setupConfigDirectory();

    // Verify docker-compose.yml uses environment variables (not hardcoded values)
    verifyDockerComposeConfig();

    // Start Docker Compose stack
    await startDockerCompose();

    // Wait for services to become healthy
    await waitForServicesHealthy();

    // Establish stdio proxy
    establishStdioProxy();

  } catch (error) {
    console.error('❌ ERROR:', error.message);
    process.exit(1);
  }
}

// Run main
main();
