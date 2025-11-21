/**
 * Docker container detection utilities
 *
 * Provides functions to detect if running inside a Docker container
 * and discover workspace paths for devcontainer environments.
 */

import * as fs from 'fs'
import { execFileSync } from 'child_process'

// Diagnostic logging callback - can be set by cli.cjs to use its diagnosticLog function
type DiagnosticLogFn = (message: string, data?: any) => void
let diagnosticLogFn: DiagnosticLogFn | null = null

/**
 * Set the diagnostic logging function
 * Called by cli.cjs to provide its diagnosticLog implementation
 */
export function setDiagnosticLog(fn: DiagnosticLogFn): void {
  diagnosticLogFn = fn
}

/**
 * Internal diagnostic log helper
 * Uses the injected diagnosticLog function if available, otherwise no-op
 */
function diagnosticLog(message: string, data?: any): void {
  if (diagnosticLogFn) {
    diagnosticLogFn(message, data)
  }
}

/**
 * Check if currently running inside a Docker container
 *
 * Detection strategy (in priority order):
 * 1. Check for /.dockerenv file (most reliable Docker marker)
 * 2. Check for /run/.containerenv file (Podman compatibility)
 * 3. Check /proc/1/cgroup for "docker" or "containerd" patterns (fallback)
 *
 * @returns True if inside Docker, false otherwise
 */
export function isInsideDocker(): boolean {
  // Check for /.dockerenv (most reliable)
  if (fs.existsSync('/.dockerenv')) {
    return true
  }

  // Check for /run/.containerenv (Podman compatibility)
  if (fs.existsSync('/run/.containerenv')) {
    return true
  }

  // Fallback: check cgroup
  try {
    const cgroup = fs.readFileSync('/proc/1/cgroup', 'utf8')
    if (cgroup.includes('docker') || cgroup.includes('containerd')) {
      return true
    }
  } catch (error) {
    // If /proc/1/cgroup doesn't exist, we're probably not in Linux
    return false
  }

  return false
}

/**
 * Discover the host path for /workspace by inspecting the current container
 *
 * In Docker-in-Docker scenarios, /workspace is mounted from the host, but we need
 * to know the actual host path (not the intermediate container path) to properly
 * mount volumes from spawned containers.
 *
 * This function:
 * 1. Gets our container's hostname
 * 2. Inspects our container's mounts using docker CLI
 * 3. Finds the mount with destination /workspace
 * 4. Returns the source (host) path
 *
 * @returns Host path or null if not found/not in container
 */
export function getWorkspaceHostPath(): string | null {
  try {
    // Get our container hostname (using execFileSync for security)
    const hostname = execFileSync('hostname', [], {
      encoding: 'utf8',
      timeout: 5000, // 5 second timeout (DoS prevention)
      maxBuffer: 1024 // 1KB max (hostname is short)
    }).trim()

    if (!hostname) {
      return null
    }

    // Query Docker for mounts of our container (using array args, not shell)
    const hostPath = execFileSync(
      'docker',
      [
        'inspect',
        hostname,
        '--format',
        '{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}'
      ],
      {
        encoding: 'utf8',
        timeout: 10000, // 10 second timeout (DoS prevention)
        maxBuffer: 10240 // 10KB max (docker inspect output can be larger)
      }
    ).trim()

    if (hostPath && hostPath.length > 0) {
      return hostPath // Use first mount with /workspace destination
    }

    return null
  } catch (error) {
    // If docker inspect fails, we might not have docker access
    // or we're not in a devcontainer setup
    return null
  }
}

/**
 * Validate and warn about potentially dangerous workspace paths
 *
 * Provides minimal security monitoring by warning about suspicious patterns:
 * - Path traversal (..)
 * - Relative paths (not starting with /)
 *
 * Does NOT block or throw errors (primary security is read-only mount)
 * Does NOT verify path exists (container can't see host filesystem)
 *
 * @param path - Workspace path to validate
 * @returns Same path (validation warns but doesn't modify)
 */
function validateAndWarnPath(path: string): string {
  // Check for path traversal patterns
  if (path.includes('..')) {
    console.warn(`⚠️  Workspace path contains ".." (path traversal risk): ${path}`)
    console.warn('    Proceeding with caution. Read-only mount limits risk.')
  }

  // Warn if relative path (not absolute)
  if (!path.startsWith('/')) {
    console.warn(`⚠️  Workspace path is not absolute: ${path}`)
    console.warn('    May cause unexpected behavior.')
  }

  // Don't verify path exists - container can't see host filesystem
  return path
}

/**
 * Resolve the appropriate workspace path for the current environment
 *
 * Handles devcontainer (Docker-in-Docker), host, and custom override scenarios
 * using a three-tier priority system:
 *
 * Priority 1: User override via WORKSPACE_HOST_PATH environment variable
 * Priority 2: Auto-detect in Docker-in-Docker using container inspection
 * Priority 3: Use current working directory when running on host
 *
 * This function orchestrates isInsideDocker() and getWorkspaceHostPath()
 * to provide intelligent defaults while respecting user configuration.
 *
 * @returns Workspace path to use for volume mounting
 */
export function resolveWorkspacePath(): string {
  // Priority 1: User override (for custom setups)
  if (process.env.WORKSPACE_HOST_PATH) {
    diagnosticLog('Using user-provided WORKSPACE_HOST_PATH', {
      path: process.env.WORKSPACE_HOST_PATH
    })
    return validateAndWarnPath(process.env.WORKSPACE_HOST_PATH)
  }

  // Priority 2: Docker-in-Docker detection
  if (isInsideDocker()) {
    diagnosticLog('Detected running inside Docker container')

    const hostPath = getWorkspaceHostPath()

    if (hostPath) {
      diagnosticLog('Discovered host workspace path', {
        hostPath,
        source: 'docker inspect'
      })
      return validateAndWarnPath(hostPath)
    }

    // Inside Docker but couldn't find mount - warn and use /workspace
    console.warn(
      '⚠️  Running inside Docker but could not discover workspace host path.'
    )
    console.warn(
      '    Volume mount may fail. Set WORKSPACE_HOST_PATH manually if needed.'
    )
    return validateAndWarnPath('/workspace')
  }

  // Priority 3: Running on host - use current directory
  const hostPath = process.cwd()
  diagnosticLog('Running on host, using current directory', { hostPath })
  return validateAndWarnPath(hostPath)
}
