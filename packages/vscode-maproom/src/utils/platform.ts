/**
 * Platform detection utilities for binary selection
 *
 * Provides utilities to detect the current platform (OS + architecture)
 * and map it to the binary directory naming convention.
 *
 * Supported platforms:
 * - darwin-x64 (macOS Intel)
 * - darwin-arm64 (macOS Apple Silicon)
 * - linux-x64 (Linux x86_64)
 * - linux-arm64 (Linux ARM64)
 * - win32-x64 (Windows x86_64)
 */

/**
 * Supported platform identifiers
 */
export type PlatformId =
  | 'darwin-x64'
  | 'darwin-arm64'
  | 'linux-x64'
  | 'linux-arm64'
  | 'win32-x64'

/**
 * Platform detection error
 */
export class PlatformError extends Error {
  constructor(
    message: string,
    public readonly platform: string,
    public readonly arch: string
  ) {
    super(message)
    this.name = 'PlatformError'
  }
}

/**
 * Detect the current platform and architecture
 *
 * @returns Platform identifier (e.g., "darwin-arm64", "linux-x64")
 * @throws PlatformError if platform is unsupported
 */
export function detectPlatform(): PlatformId {
  const platform = process.platform
  const arch = process.arch

  // Normalize architecture names
  const normalizedArch = normalizeArchitecture(arch)

  // Validate platform
  if (platform !== 'darwin' && platform !== 'linux' && platform !== 'win32') {
    throw new PlatformError(
      `Unsupported platform: ${platform}. Supported platforms: darwin, linux, win32`,
      platform,
      arch
    )
  }

  // Validate architecture
  if (normalizedArch !== 'x64' && normalizedArch !== 'arm64') {
    throw new PlatformError(
      `Unsupported architecture: ${arch}. Supported architectures: x64, arm64`,
      platform,
      arch
    )
  }

  // Construct platform ID
  const platformId = `${platform}-${normalizedArch}` as PlatformId

  return platformId
}

/**
 * Normalize architecture names to standard format
 *
 * Maps various Node.js architecture names to our standard naming:
 * - x64, amd64 -> x64
 * - arm64, aarch64 -> arm64
 *
 * @param arch - Node.js architecture string
 * @returns Normalized architecture name
 */
function normalizeArchitecture(arch: string): string {
  const normalized = arch.toLowerCase()

  // Map x86_64 variants to x64
  if (normalized === 'x64' || normalized === 'amd64') {
    return 'x64'
  }

  // Map ARM64 variants to arm64
  if (normalized === 'arm64' || normalized === 'aarch64') {
    return 'arm64'
  }

  // Return as-is for validation to catch unsupported architectures
  return normalized
}

/**
 * Get the binary file extension for the current platform
 *
 * @returns ".exe" for Windows, empty string for Unix-like systems
 */
export function getBinaryExtension(): string {
  return process.platform === 'win32' ? '.exe' : ''
}

/**
 * Check if the current platform is Windows
 *
 * @returns true if running on Windows
 */
export function isWindows(): boolean {
  return process.platform === 'win32'
}

/**
 * Check if the current platform is macOS
 *
 * @returns true if running on macOS
 */
export function isMacOS(): boolean {
  return process.platform === 'darwin'
}

/**
 * Check if the current platform is Linux
 *
 * @returns true if running on Linux
 */
export function isLinux(): boolean {
  return process.platform === 'linux'
}

/**
 * Get platform-specific path separator
 *
 * @returns "\\" for Windows, "/" for Unix-like systems
 */
export function getPathSeparator(): string {
  return process.platform === 'win32' ? '\\' : '/'
}
