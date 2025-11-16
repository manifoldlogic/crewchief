/**
 * UI module exports
 *
 * Provides status bar management and setup wizard for the Maproom extension.
 */

export { StatusBarManager } from './statusBar'
export {
  runSetupWizard,
  getConfiguredProvider,
  registerSetupCommand,
  detectOllama,
  type EmbeddingProvider,
} from './setupWizard'
