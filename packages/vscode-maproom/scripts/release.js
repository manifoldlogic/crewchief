#!/usr/bin/env node

/**
 * Release script for vscode-maproom extension
 *
 * Usage:
 *   pnpm release:patch  - Bumps patch version (0.2.0 -> 0.2.1)
 *   pnpm release:minor  - Bumps minor version (0.2.0 -> 0.3.0)
 *   pnpm release:major  - Bumps major version (0.2.0 -> 1.0.0)
 *
 * This script:
 * 1. Reads current version from package.json
 * 2. Calculates next version based on bump type
 * 3. Triggers GitHub Actions workflow with new version
 */

import { execSync } from 'child_process';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const bumpType = process.argv[2];

if (!['patch', 'minor', 'major'].includes(bumpType)) {
  console.error('Usage: node release.js <patch|minor|major>');
  process.exit(1);
}

// Read current version
const packageJsonPath = join(__dirname, '..', 'package.json');
const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8'));
const currentVersion = packageJson.version;

// Calculate next version
const [major, minor, patch] = currentVersion.split('.').map(Number);
let nextVersion;

switch (bumpType) {
  case 'patch':
    nextVersion = `${major}.${minor}.${patch + 1}`;
    break;
  case 'minor':
    nextVersion = `${major}.${minor + 1}.0`;
    break;
  case 'major':
    nextVersion = `${major + 1}.0.0`;
    break;
}

console.log(`📦 VSCode Extension Release`);
console.log(`   Current version: ${currentVersion}`);
console.log(`   Next version:    ${nextVersion}`);
console.log(`   Bump type:       ${bumpType}`);
console.log();

// Trigger GitHub Actions workflow
try {
  console.log('🚀 Triggering GitHub Actions workflow...');

  execSync(
    `gh workflow run release-vscode-maproom.yml --field version=${nextVersion} --field pre_release=false --field dry_run=false`,
    { stdio: 'inherit' }
  );

  console.log();
  console.log('✅ Workflow triggered successfully!');
  console.log();
  console.log('The workflow will:');
  console.log(`  1. Auto-bump package.json to ${nextVersion}`);
  console.log('  2. Build the extension');
  console.log('  3. Publish to VS Code Marketplace');
  console.log('  4. Create GitHub release');
  console.log();
  console.log('Monitor progress:');
  console.log('  gh run list --workflow=release-vscode-maproom.yml --limit 1');
  console.log('  gh run watch <run-id>');
} catch (error) {
  console.error('❌ Failed to trigger workflow');
  console.error(error.message);
  process.exit(1);
}
