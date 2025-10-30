/**
 * Utility functions for the Maproom MCP server
 */

export { Cache, explainCache } from './cache'
export {
  execGit,
  isCommitCheckedOut,
  getFileFromGit,
  getRepoRoot,
} from './git'
export {
  spawnProcess,
  trySpawnWithCandidates,
  findMaproomBinary,
  getBinarycandidates,
  parseIndexingStats,
  type ProcessResult,
  type SpawnOptions,
  type IndexingStats,
  ProcessError,
} from './process'
export {
  validatePath,
  validateWithinRepo,
  validateFileSize,
  validateRange,
  extractRange,
  ValidationError,
} from './validation'
export {
  detectProvider,
  isOllamaAvailable,
  validateExplicitProvider,
  getProviderConfig,
  clearProviderCache,
  type ProviderConfig,
} from './provider-detection'
