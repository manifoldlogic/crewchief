/**
 * Custom error types for Ollama operations
 *
 * These errors provide specific failure modes that can be handled
 * with appropriate user feedback (e.g., "Install Ollama" button).
 */

/**
 * Error thrown when Ollama service is not running
 *
 * This typically means Ollama is not installed or the service
 * hasn't been started. Handle by prompting user to install/start Ollama.
 */
export class OllamaNotRunningError extends Error {
  constructor() {
    super('Ollama is not running')
    this.name = 'OllamaNotRunningError'
  }
}

/**
 * Error thrown when a model pull operation fails
 *
 * This can happen due to network issues, invalid model name,
 * or Ollama service problems.
 */
export class ModelPullError extends Error {
  constructor(
    public readonly modelName: string,
    public readonly cause?: Error
  ) {
    super(`Failed to pull model: ${modelName}`)
    this.name = 'ModelPullError'
  }
}

/**
 * Error thrown when model verification fails
 *
 * This happens when we cannot determine if a model exists.
 */
export class ModelCheckError extends Error {
  constructor(
    public readonly modelName: string,
    public readonly cause?: Error
  ) {
    super(`Failed to check model availability: ${modelName}`)
    this.name = 'ModelCheckError'
  }
}
