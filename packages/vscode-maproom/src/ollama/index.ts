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
