#!/usr/bin/env node
import fs from 'node:fs'
import fsp from 'node:fs/promises'
import path from 'node:path'
import { execSync } from 'node:child_process'

function bump(version, level) {
  // Basic semver bump without prerelease handling
  const [core, ...rest] = String(version).split('-')
  const [majS, minS, patS] = core.split('.')
  let major = parseInt(majS || '0', 10)
  let minor = parseInt(minS || '0', 10)
  let patch = parseInt(patS || '0', 10)
  if (level === 'major') {
    major += 1
    minor = 0
    patch = 0
  } else if (level === 'minor') {
    minor += 1
    patch = 0
  } else {
    patch += 1
  }
  const next = `${major}.${minor}.${patch}`
  return rest.length ? `${next}-${rest.join('-')}` : next
}

async function main() {
  const level = process.argv[2] || 'patch'
  if (!['patch', 'minor', 'major'].includes(level)) {
    console.error('usage: release.mjs <patch|minor|major>')
    process.exit(1)
  }
  const pkgPath = path.join(process.cwd(), 'package.json')
  const raw = await fsp.readFile(pkgPath, 'utf8')
  const pkg = JSON.parse(raw)
  const prev = pkg.version || '0.0.0'
  const next = bump(prev, level)
  pkg.version = next
  const updated = JSON.stringify(pkg, null, 2) + '\n'
  await fsp.writeFile(pkgPath, updated, 'utf8')

  // Commit and tag
  execSync('git add package.json', { stdio: 'inherit' })
  execSync(`git commit -m "chore(release): ${next}"`, { stdio: 'inherit' })
  execSync(`git tag ${pkg.name}@v${next}`, { stdio: 'inherit' })
  execSync('git push --follow-tags', { stdio: 'inherit' })

  // Publish (suppress npm warnings about env configs)
  execSync('pnpm publish --access public', { 
    stdio: 'inherit',
    env: {
      ...process.env,
      // Filter out npm env configs that cause warnings
      npm_config_verify_deps_before_run: undefined,
      npm_config__jsr_registry: undefined,
    }
  })
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
