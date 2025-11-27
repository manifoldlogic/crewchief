/**
 * Ollama module exports
 *
 * Provides HTTP client for Ollama API operations including
 * health checks, model verification, and model pulling.
 */

export {
  OllamaClient,
  InvalidModelNameError,
  OllamaApiError,
  type OllamaModel,
  type OllamaTagsResponse,
  type PullProgress,
} from './client'

export {
  OllamaNotRunningError,
  ModelPullError,
  ModelCheckError,
} from './errors'

export {
  ensureOllamaModel,
  showOllamaNotRunningError,
  showModelPullError,
  checkModelAvailability,
  DEFAULT_EMBEDDING_MODEL,
  OLLAMA_INSTALL_URL,
  type ModelCheckResult,
  type EnsureModelOptions,
} from './model-manager'
