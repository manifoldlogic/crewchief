#!/usr/bin/env node

import { copyFile, mkdir, chmod, access } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Platform configurations
const PLATFORMS = [
  { name: 'darwin-arm64', binary: 'crewchief-maproom' },
  { name: 'darwin-x64', binary: 'crewchief-maproom' },
  { name: 'linux-arm64', binary: 'crewchief-maproom' },
  { name: 'linux-x64', binary: 'crewchief-maproom' },
  { name: 'win32-x64', binary: 'crewchief-maproom.exe' },
];

const SOURCE_DIR = join(__dirname, '../../cli/bin');
const TARGET_DIR = join(__dirname, '../bin');

async function fileExists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function copyBinary(platform, binary) {
  const sourcePath = join(SOURCE_DIR, platform, binary);
  const targetPath = join(TARGET_DIR, platform, binary);

  // Check if source exists
  if (!(await fileExists(sourcePath))) {
    return { success: false, platform, reason: 'not found' };
  }

  // Create target directory
  await mkdir(dirname(targetPath), { recursive: true });

  // Copy binary
  await copyFile(sourcePath, targetPath);

  // Set executable permissions on Unix binaries
  if (!platform.startsWith('win32')) {
    await chmod(targetPath, 0o755);
  }

  return { success: true, platform };
}

async function main() {
  console.log('Preparing Maproom binaries for packaging...\n');

  // Check if binaries already exist (e.g., in CI where they're pre-downloaded)
  const targetBinaries = await Promise.all(
    PLATFORMS.map(async ({ name, binary }) => {
      const targetPath = join(TARGET_DIR, name, binary);
      return { platform: name, exists: await fileExists(targetPath), path: targetPath };
    })
  );

  const existingCount = targetBinaries.filter((b) => b.exists).length;

  if (existingCount > 0) {
    console.log(`Found ${existingCount} existing binaries in target directory:`);
    targetBinaries.filter((b) => b.exists).forEach((b) => console.log(`  - ${b.platform}`));
    console.log('\nSkipping copy (binaries already in place from CI or previous build)');
    console.log('Binaries prepared successfully!');
    return;
  }

  console.log('No existing binaries found, copying from CLI package...\n');

  // Ensure target directory exists
  await mkdir(TARGET_DIR, { recursive: true });

  const results = await Promise.all(
    PLATFORMS.map(({ name, binary }) => copyBinary(name, binary))
  );

  // Report results
  const successful = results.filter((r) => r.success);
  const failed = results.filter((r) => !r.success);

  console.log(`✓ Successfully copied ${successful.length} platform binaries:`);
  successful.forEach((r) => console.log(`  - ${r.platform}`));

  if (failed.length > 0) {
    console.log(`\n⚠ Missing ${failed.length} platform binaries:`);
    failed.forEach((r) => console.log(`  - ${r.platform} (${r.reason})`));
    console.log('\nNote: Extension will only work on platforms with available binaries.');
  }

  console.log('\nBinaries prepared successfully!');

  // Exit with error if no binaries were found at all
  // EXCEPTION: In CI, allow TypeScript build to proceed without binaries
  // (they'll be downloaded later in the package step after Rust build completes)
  if (successful.length === 0) {
    if (process.env.GITHUB_ACTIONS === 'true' || process.env.CI === 'true') {
      console.log('\n⚠ Running in CI: No binaries found, but continuing (binaries will be downloaded in package step)');
      return;
    }
    console.error('\n✗ ERROR: No binaries found! Cannot package extension.');
    process.exit(1);
  }
}

main().catch((error) => {
  console.error('Error preparing binaries:', error);
  process.exit(1);
});
