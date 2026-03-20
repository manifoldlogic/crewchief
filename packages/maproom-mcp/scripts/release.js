#!/usr/bin/env node
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Get version type from command line (patch, minor, or major)
const type = process.argv[2];
const validTypes = ['patch', 'minor', 'major'];

// Validate input
if (!type || !validTypes.includes(type)) {
  console.error(`Error: Invalid version type "${type}"`);
  console.error('Usage: node scripts/release.js <patch|minor|major>');
  process.exit(1);
}

// Package root directory
const packageRoot = path.join(__dirname, '..');
const packageJsonPath = path.join(packageRoot, 'package.json');

console.log(`\n🚀 Starting ${type} release...\n`);

try {
  // Step 1: Run bump-version.js to update package.json
  console.log('📝 Bumping version...');
  execSync(`node scripts/bump-version.js ${type}`, {
    cwd: packageRoot,
    stdio: 'inherit'
  });

  // Step 2: Read the new version from package.json
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  const version = packageJson.version;

  if (!version) {
    throw new Error('Failed to read version from package.json');
  }

  console.log(`\n📦 New version: ${version}\n`);

  // Step 3: Git add package.json
  console.log('➕ Staging package.json...');
  execSync('git add package.json', {
    cwd: packageRoot,
    stdio: 'inherit'
  });

  // Step 4: Git commit
  console.log(`💾 Creating commit...`);
  execSync(`git commit -m "chore(release): bump version to ${version}"`, {
    cwd: packageRoot,
    stdio: 'inherit'
  });

  // Step 5: Git tag (annotated) with package-scoped name
  const tag = `${packageJson.name}@v${version}`;
  console.log(`🏷️  Creating tag ${tag}...`);
  execSync(`git tag -a ${tag} -m "Release version ${version}"`, {
    cwd: packageRoot,
    stdio: 'inherit'
  });

  // Step 6: Two-step push to avoid race condition
  console.log(`\n⬆️  Pushing commit...`);
  execSync('git push', {
    cwd: packageRoot,
    stdio: 'inherit'
  });

  console.log(`\n⬆️  Pushing tag ${tag}...`);
  execSync(`git push origin ${tag}`, {
    cwd: packageRoot,
    stdio: 'inherit'
  });

  console.log(`\n✅ Successfully released version ${version}!`);
  console.log(`\nGitHub Actions workflows will now:`);
  console.log(`  - Build Rust binaries for 4 platforms`);
  console.log(`  - Publish to npm: @crewchief/maproom-mcp@${version}`);
  console.log(`  - Build and publish Docker images`);
  console.log(`\nMonitor at: https://github.com/manifoldlogic/crewchief/actions\n`);

} catch (error) {
  console.error(`\n❌ Release failed: ${error.message}\n`);
  process.exit(1);
}
