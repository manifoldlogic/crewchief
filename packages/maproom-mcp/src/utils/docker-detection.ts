/**
 * Docker container detection utilities
 *
 * Provides functions to detect if running inside a Docker container
 * and discover workspace paths for devcontainer environments.
 */

import * as fs from 'fs'
import { execFileSync } from 'child_process'

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
