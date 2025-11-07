/**
 * Type 2: Understanding Architecture/Flow tasks
 */

import type { SearchTask } from '../types.js'
import { createTaskValidator } from '../validators.js'

/**
 * Understand competition flow
 */
export const TASK_UNDERSTAND_COMPETITION: SearchTask = {
  id: 'arch-competition-001',
  name: 'Understand Competition Flow',
  description: 'Understand how agent competitions work from start to winner selection',

  searchTarget: {
    type: 'pattern',
    pattern: /CompetitionManager|competition\.ts|runCompetition/,
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain the competition workflow from creation to winner selection',
    validator: {
      type: 'explanation',
      mentionsFiles: ['competition.ts', 'search-competition.ts'],
      mentionsPattern: /spawn.*agent|evaluate|score|winner/i,
    },
  },

  difficulty: 'medium',
  category: 'understanding-architecture',
  maxSearchAttempts: 8,
  maxTimeSeconds: 300,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /CompetitionManager|competition\.ts|runCompetition/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['competition.ts', 'search-competition.ts'],
        mentionsPattern: /spawn.*agent|evaluate|score|winner/i,
      },
    },
  }),
}

/**
 * Understand SDK integration
 */
export const TASK_UNDERSTAND_SDK_INTEGRATION: SearchTask = {
  id: 'arch-sdk-001',
  name: 'Understand SDK Integration',
  description: 'Understand how the Claude Code Agents SDK is integrated and used',

  searchTarget: {
    type: 'pattern',
    pattern: /@anthropic-ai\/claude-agent-sdk|spawnAgent|SDK/,
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain how the SDK is integrated and what it provides',
    validator: {
      type: 'explanation',
      mentionsFiles: ['sdk', 'spawner'],
      mentionsPattern: /SDK|query|agent.*spawn|hooks/i,
    },
  },

  difficulty: 'medium',
  category: 'understanding-architecture',
  maxSearchAttempts: 8,
  maxTimeSeconds: 300,
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /@anthropic-ai\/claude-agent-sdk|spawnAgent|SDK/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['sdk', 'spawner'],
        mentionsPattern: /SDK|query|agent.*spawn|hooks/i,
      },
    },
  }),
}
