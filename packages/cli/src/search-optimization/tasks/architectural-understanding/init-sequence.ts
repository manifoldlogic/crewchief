/**
 * Task: Initialization Sequence - Agent Orchestrator
 *
 * Understand the initialization sequence when the agent orchestrator starts up.
 * This task requires piecing together startup logic across multiple components.
 *
 * Why grep fails:
 * - Grep can find initialization code but cannot determine order
 * - Cannot identify dependencies between initialization steps
 * - Misses implicit initialization through module imports
 * - Cannot distinguish startup code from runtime code
 *
 * Why search succeeds:
 * - Semantic understanding of "initialization", "startup", and "bootstrap"
 * - Can identify configuration loading, registry setup, communication initialization
 * - Understands temporal relationships ("before", "after", "depends on")
 * - Recognizes initialization patterns across different components
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_INIT_SEQUENCE_ORCHESTRATOR: SearchTask = {
  id: 'architecture-init-sequence-orchestrator',
  name: 'Trace Agent Orchestrator Initialization',
  description:
    'Trace the initialization sequence when the agent orchestrator starts up. ' +
    'Identify the startup steps in order: 1) Configuration loading, 2) Agent registry setup, ' +
    '3) Message bus/communication initialization, and 4) First agent spawn preparation. ' +
    'Explain what happens at each step and why the order matters.',

  category: 'architectural-understanding',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    // Looking for initialization-related components
    pattern: /loadConfig|AgentRegistry|registry\.ts|MessageBus|RunManager|Scheduler|assignSingleAgent|spawn.*agent/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the initialization sequence for the agent orchestrator: ' +
      '1) How is configuration loaded and validated? ' +
      '2) How is the agent registry set up and populated? ' +
      '3) How is the message bus or communication layer initialized? ' +
      '4) What preparation happens before spawning the first agent? ' +
      'Include the order of operations and why it matters.',
    validator: {
      type: 'explanation',
      // Must mention key initialization components
      mentionsFiles: ['loader', 'registry', 'scheduler'],
      // Must discuss initialization sequence
      mentionsPattern:
        /(initialize|startup|bootstrap|setup|load|config).*(?:registry|agent|bus|message|communication)|sequence|order|before|after|first/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  internalNotes:
    'Tests understanding of temporal relationships and initialization ordering. ' +
    'Grep cannot determine the sequence of operations or dependencies. ' +
    'Search can identify initialization patterns and understand startup flow.',

  expectedGrepSuccess: 0.25, // 25% - grep can find init code but not sequence
  expectedSearchSuccess: 0.75, // 75% - search understands initialization patterns

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /loadConfig|AgentRegistry|registry\.ts|MessageBus|RunManager|Scheduler|assignSingleAgent|spawn.*agent/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['loader', 'registry', 'scheduler'],
        mentionsPattern:
          /(initialize|startup|bootstrap|setup|load|config).*(?:registry|agent|bus|message|communication)|sequence|order|before|after|first/i,
      },
    },
  }),
}
