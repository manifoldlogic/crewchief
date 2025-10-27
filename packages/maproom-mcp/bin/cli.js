#!/usr/bin/env node
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

const CONFIG_DIR = path.join(os.homedir(), '.maproom-mcp');
const COMPOSE_FILE = path.join(CONFIG_DIR, 'docker-compose.yml');

// Ensure config directory exists
if (!fs.existsSync(CONFIG_DIR)) {
  fs.mkdirSync(CONFIG_DIR, { recursive: true });
  console.log('✅ Initialized Maproom configuration directory');
}

// Copy docker-compose.yml to config directory
const embeddedCompose = path.join(__dirname, '..', 'config', 'docker-compose.yml');
if (!fs.existsSync(COMPOSE_FILE)) {
  try {
    fs.copyFileSync(embeddedCompose, COMPOSE_FILE);
    console.log('✅ Initialized Maproom configuration');
  } catch (error) {
    console.error('❌ Error: Failed to copy docker-compose.yml to config directory');
    console.error('   Make sure the embedded compose file exists at:', embeddedCompose);
    process.exit(1);
  }
}

// Check if Docker is running and has Compose plugin
function checkDocker() {
  return new Promise((resolve, reject) => {
    const check = spawn('docker', ['compose', 'version']);
    check.on('error', (error) => {
      reject(new Error('Docker not found. Please install Docker Desktop or Docker Engine.'));
    });
    check.on('close', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error('Docker Compose plugin not found. Please install Docker Desktop or Docker Compose v2.'));
      }
    });
  });
}

// Start Docker Compose stack
async function startStack() {
  await checkDocker();

  console.log('🚀 Starting Maproom MCP with local LLM...');

  const compose = spawn('docker', [
    'compose',
    '-f', COMPOSE_FILE,
    'up', '-d'
  ], {
    cwd: CONFIG_DIR,
    stdio: 'inherit'
  });

  return new Promise((resolve, reject) => {
    compose.on('error', (error) => {
      reject(new Error(`Failed to start docker compose: ${error.message}`));
    });
    compose.on('close', (code) => {
      if (code === 0) {
        console.log('✅ Maproom MCP is ready!');
        resolve();
      } else {
        reject(new Error(`docker compose exited with code ${code}`));
      }
    });
  });
}

// Wait for services to be healthy
async function waitForHealth() {
  const maxRetries = 30;
  let retries = 0;

  while (retries < maxRetries) {
    const check = spawn('docker', [
      'compose',
      '-f', COMPOSE_FILE,
      'ps', '--services', '--filter', 'status=running'
    ], { cwd: CONFIG_DIR });

    const output = await new Promise((resolve) => {
      let stdout = '';
      check.stdout.on('data', (data) => stdout += data);
      check.on('close', () => resolve(stdout));
    });

    const running = output.trim().split('\n').filter(s => s.trim().length > 0).length;
    if (running >= 3) {  // All 3 services running
      console.log('✅ All services healthy');
      return true;
    }

    retries++;
    if (retries < maxRetries) {
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
  }

  throw new Error('Services failed to start within timeout (60 seconds). Check logs with: docker compose -f ~/.maproom-mcp/docker-compose.yml logs');
}

// Main entry point
async function main() {
  try {
    await startStack();
    await waitForHealth();

    // Now connect to MCP server and proxy stdio
    console.log('🔌 Connecting to MCP server...');

    const mcp = spawn('docker', [
      'compose',
      '-f', COMPOSE_FILE,
      'exec', '-T', 'maproom',
      '/usr/local/bin/crewchief-maproom', 'serve', '--stdio'
    ], {
      cwd: CONFIG_DIR,
      stdio: ['inherit', 'inherit', 'inherit']
    });

    mcp.on('error', (error) => {
      console.error('❌ Error: Failed to connect to MCP server:', error.message);
      process.exit(1);
    });

    mcp.on('close', (code) => {
      process.exit(code || 0);
    });

    process.on('SIGINT', () => {
      console.log('\n🛑 Shutting down gracefully...');
      mcp.kill('SIGTERM');
    });

    process.on('SIGTERM', () => {
      console.log('\n🛑 Shutting down gracefully...');
      mcp.kill('SIGTERM');
    });

  } catch (error) {
    console.error('❌ Error:', error.message);
    process.exit(1);
  }
}

main();
