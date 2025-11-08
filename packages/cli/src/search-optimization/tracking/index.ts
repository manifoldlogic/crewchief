/**
 * Tracking System
 *
 * Exports leaderboard, production variant, and run registry functionality
 * for genetic optimization tracking and management.
 */

// Leaderboard
export {
  type Leaderboard,
  type LeaderboardEntry,
  LEADERBOARD_SCHEMA_VERSION,
  loadLeaderboard,
  saveLeaderboard,
  updateLeaderboard,
  saveToLeaderboard,
  getLeaderboardEntry,
  getLeaderboardEntryByVariantId,
  generateLeaderboardReport,
  getLeaderboardPath,
} from './leaderboard.js'

// Production
export {
  type ProductionPointer,
  type DeploymentLogEntry,
  getCurrentProduction,
  promoteToProduction,
  rollbackProduction,
  loadProductionVariant,
  getProductionHistory,
  generateProductionReport,
  getProductionDir,
  getProductionPointerPath,
  getProductionVariantsDir,
  getDeploymentLogPath,
} from './production.js'

// Run Registry
export {
  type RunStatus,
  type RunLearnings,
  type OptimizationRun,
  type RunRegistry,
  loadRunRegistry,
  saveRunRegistry,
  registerRun,
  updateRunStatus,
  extractLearnings,
  compareRunResults,
  exportLearnings,
  getRun,
  listRuns,
  generateRunRegistryReport,
  getRunRegistryDir,
  getRunRegistryPath,
} from './run-registry.js'

// Deployment
export {
  type DeploymentResult,
  type DeploymentOptions,
  deployVariant,
  backupCurrentDescription,
  patchToolDescription,
  buildMCPServer,
  detectRunningServer,
  pruneOldBackups,
  readCurrentDescription,
  getBackupsDir,
} from './deployment.js'
