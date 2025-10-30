import fs from 'fs';
import path from 'path';
import os from 'os';
import { fileURLToPath } from 'url';

const CACHE_DIR = path.join(os.homedir(), '.maproom-mcp');
const VERSION_FILE = path.join(CACHE_DIR, '.version');

export function readVersion(): string | null {
  if (!fs.existsSync(VERSION_FILE)) {
    return null;
  }
  return fs.readFileSync(VERSION_FILE, 'utf-8').trim();
}

export function writeVersion(version: string): void {
  // Ensure cache directory exists
  if (!fs.existsSync(CACHE_DIR)) {
    fs.mkdirSync(CACHE_DIR, { recursive: true, mode: 0o700 });
  }

  // Write version file
  fs.writeFileSync(VERSION_FILE, version, { mode: 0o600 });
}

export function needsConfigUpdate(): boolean {
  // Read current package version
  const currentDir = path.dirname(fileURLToPath(import.meta.url));
  const packageJsonPath = path.join(currentDir, '../package.json');
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
  const currentVersion = packageJson.version;

  // Read cached version
  const cachedVersion = readVersion();

  // First run - no version file
  if (!cachedVersion) {
    return true;
  }

  // Version mismatch
  return cachedVersion !== currentVersion;
}

export function updateConfigs(): void {
  const currentDir = path.dirname(fileURLToPath(import.meta.url));
  const PACKAGE_CONFIGS = path.join(currentDir, '../config');
  const userEnvPath = path.join(CACHE_DIR, '.env');

  // Step 1: Backup user .env if exists
  let userEnvContent: string | null = null;
  if (fs.existsSync(userEnvPath)) {
    userEnvContent = fs.readFileSync(userEnvPath, 'utf-8');
    console.log('  💾 Preserving user .env file...');
  }

  // Step 2: Delete old cache directory
  if (fs.existsSync(CACHE_DIR)) {
    fs.rmSync(CACHE_DIR, { recursive: true, force: true });
  }

  // Step 3: Copy fresh configs from package
  fs.mkdirSync(CACHE_DIR, { recursive: true, mode: 0o700 });
  fs.cpSync(PACKAGE_CONFIGS, CACHE_DIR, { recursive: true });
  console.log('  📋 Copied fresh configs from package...');

  // Step 4: Restore user .env if it existed
  if (userEnvContent !== null) {
    fs.writeFileSync(userEnvPath, userEnvContent, { mode: 0o600 });
    console.log('  ✅ Restored user .env file');
  }

  // Step 5: Write current version
  const packageJsonPath = path.join(currentDir, '../package.json');
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
  writeVersion(packageJson.version);
}
