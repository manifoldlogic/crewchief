import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

function ensureExecutable(p: string) {
  try {
    fs.chmodSync(p, 0o755)
  } catch {
    // ignore errors
  }
}

function main() {
  const __filename = fileURLToPath(import.meta.url)
  const __dirname = path.dirname(__filename)
  const execName = process.platform === 'win32' ? 'maproom.exe' : 'maproom'
  const outDir = path.join(__dirname, '..', 'bin', `${process.platform}-${process.arch}`)
  const outPath = path.join(outDir, execName)

  // If already present, just ensure mode and exit
  if (fs.existsSync(outPath)) {
    ensureExecutable(outPath)
    return
  }

  // Try copy from sibling maproom-mcp package bin for monorepo installs
  try {
    const mcpPath = path.join(
      __dirname,
      '..',
      '..',
      'maproom-mcp',
      'bin',
      `${process.platform}-${process.arch}`,
      execName,
    )
    if (fs.existsSync(mcpPath)) {
      fs.mkdirSync(outDir, { recursive: true })
      fs.copyFileSync(mcpPath, outPath)
      ensureExecutable(outPath)
      return
    }
  } catch {
    // ignore errors
  }

  // Try cargo build as a fallback
  try {
    const res = spawnSync('cargo', ['build', '--release', '-p', 'maproom'], {
      cwd: path.join(__dirname, '..', '..', '..'),
      stdio: 'inherit',
    })
    if (res.status === 0) {
      const built = path.join(__dirname, '..', '..', '..', 'target', 'release', execName)
      if (fs.existsSync(built)) {
        fs.mkdirSync(outDir, { recursive: true })
        fs.copyFileSync(built, outPath)
        ensureExecutable(outPath)
        return
      }
    }
  } catch {
    // ignore errors
  }
}

main()
