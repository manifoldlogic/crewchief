/**
 * Docker container detection utilities
 *
 * Provides functions to detect if running inside a Docker container
 * and discover workspace paths for devcontainer environments.
 */

import * as fs from 'fs'

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
