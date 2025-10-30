import fs from 'fs';
import path from 'path';
import os from 'os';

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
