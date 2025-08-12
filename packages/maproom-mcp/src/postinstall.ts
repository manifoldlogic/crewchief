import { spawnSync } from 'node:child_process'
import path from 'node:path'
import fs from 'node:fs'

function ensureExecutable(p: string) {
  try {
    fs.chmodSync(p, 0o755)
  } catch {}
}

function main() {
  // If packaged binary already exists, do nothing
  const execName = process.platform === 'win32' ? 'crewchief-maproom.exe' : 'crewchief-maproom'
  const outDir = path.join(__dirname, '..', 'bin', `${process.platform}-${process.arch}`)
  const outPath = path.join(outDir, execName)
  if (fs.existsSync(outPath)) {
    ensureExecutable(outPath)
    return
  }

  // Try to build locally if Rust is available
  try {
    const res = spawnSync('cargo', ['build', '--release', '-p', 'crewchief-maproom'], {
      cwd: path.join(__dirname, '..', '..', '..'),
      stdio: 'inherit'
    })
    if (res.status === 0) {
      const built = path.join(__dirname, '..', '..', '..', 'target', 'release', execName)
      if (fs.existsSync(built)) {
        fs.mkdirSync(outDir, { recursive: true })
        fs.copyFileSync(built, outPath)
        ensureExecutable(outPath)
      }
    }
  } catch {}
}

main()


