/**
 * UI module exports
 *
 * Provides status bar management and setup wizard for the Maproom extension.
 */

export { StatusBarManager } from './statusBar.js'
export {
  runSetupWizard,
  getConfiguredProvider,
  registerSetupCommand,
  detectOllama,
  type EmbeddingProvider,
} from './setupWizard.js'
