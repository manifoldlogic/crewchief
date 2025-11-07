/**
 * SDK configuration management
 */

import type { SDKConfig } from './types.js'

let globalConfig: SDKConfig = {
  defaultPermissionMode: 'acceptEdits',
  verbose: false,
}

/**
 * Get current SDK configuration
 */
export function getSDKConfig(): SDKConfig {
  return { ...globalConfig }
}

/**
 * Update SDK configuration
 */
export function updateSDKConfig(updates: Partial<SDKConfig>): void {
  globalConfig = {
    ...globalConfig,
    ...updates,
  }
}

/**
 * Reset SDK configuration to defaults
 */
export function resetSDKConfig(): void {
  globalConfig = {
    defaultPermissionMode: 'acceptEdits',
    verbose: false,
  }
}
