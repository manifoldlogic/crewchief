/**
 * Task: Data Flow Tracing - Worktree Creation
 *
 * Trace how a worktree creation request flows from CLI input to git execution.
 * This task requires understanding multiple architectural layers and their interactions.
 *
 * Why grep fails:
 * - Grep can find individual components (CLI parser, WorktreeService, git calls)
 * - Cannot connect the flow across layers (CLI → validation → business logic → execution)
 * - Misses implicit flow through dependency injection and indirection
 * - Cannot identify the sequence and data transformations at each layer
 *
 * Why search succeeds:
 * - Semantic understanding of "data flow" and "request handling"
 * - Can identify entry points, validation, and execution in context
 * - Understands architectural patterns (command pattern, service layer)
 * - Can trace conceptual flow even when implementation details vary
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_DATA_FLOW_WORKTREE_CREATION: SearchTask = {
  id: 'architecture-data-flow-worktree',
  name: 'Trace Worktree Creation Data Flow',
  description:
    'Trace how a worktree creation request flows from CLI input to git execution. ' +
    'Identify the complete flow: 1) CLI command entry point, 2) Input validation layer, ' +
    '3) Worktree manager/business logic, and 4) Git command execution. ' +
    'Explain how data transforms at each layer and what each component is responsible for.',

  category: 'architectural-understanding',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    // Looking for files in the data flow path
    pattern: /worktree\.ts|WorktreeService|createWorktree|git.*worktree|Commander|registerWorktreeCommands/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the complete data flow for worktree creation from CLI to git: ' +
      '1) Where does the CLI command enter the system? ' +
      '2) How is input validated (config loading, branch names)? ' +
      '3) What business logic manages worktree creation? ' +
      '4) How is the git worktree command executed? ' +
      'Include specific files and functions at each layer.',
    validator: {
      type: 'explanation',
      // Must mention key files in the flow
      mentionsFiles: ['worktree.ts', 'worktrees.ts'],
      // Must discuss flow concepts
      mentionsPattern:
        /(CLI|command|entry|input).*(?:validation|config|load).*(?:service|manager|business).*(?:git|execution|spawn)|flow|layer|transform|data.*flow/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  internalNotes:
    'Demonstrates semantic search understanding of multi-layer architecture. ' +
    'Grep would find individual components but miss the connection between CLI → Service → Git. ' +
    'Requires understanding request flow through different architectural layers.',

  expectedGrepSuccess: 0.2, // 20% - grep can find files but not trace flow
  expectedSearchSuccess: 0.8, // 80% - search understands data flow patterns

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /worktree\.ts|WorktreeService|createWorktree|git.*worktree|Commander|registerWorktreeCommands/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['worktree.ts', 'worktrees.ts'],
        mentionsPattern:
          /(CLI|command|entry|input).*(?:validation|config|load).*(?:service|manager|business).*(?:git|execution|spawn)|flow|layer|transform|data.*flow/i,
      },
    },
  }),
}
