/**
 * Process management module
 *
 * Exports orchestration classes for managing long-running Rust processes.
 */

export {
  ProcessOrchestrator,
  ProcessError,
  type OrchestratorConfig,
  type PostgresConfig,
  type OrchestratorEvents,
} from './orchestrator.js'

export {
  StdoutParser,
  type ParserEvents,
  type TypedStdoutParser,
} from './parser.js'

export {
  isWatchEvent,
  validateWatchEvent,
  type WatchEvent,
  type ProgressEvent,
  type ErrorEvent,
  type CompleteEvent,
  type StatusEvent,
} from './events.js'
