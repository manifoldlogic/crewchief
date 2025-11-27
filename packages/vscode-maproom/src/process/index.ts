/**
 * Process management module
 *
 * Exports orchestration classes for managing long-running Rust processes.
 */

export {
  ProcessOrchestrator,
  ProcessError,
  type OrchestratorConfig,
  type OrchestratorEvents,
} from './orchestrator'

export {
  StdoutParser,
  type ParserEvents,
  type TypedStdoutParser,
} from './parser'

export {
  isWatchEvent,
  validateWatchEvent,
  type WatchEvent,
  type ProgressEvent,
  type ErrorEvent,
  type CompleteEvent,
  type StatusEvent,
} from './events'

export {
  reconcileChanges,
  updateLastIndexedCommit,
  type ReconcileConfig,
  type ReconcileResult,
} from './reconcile'
