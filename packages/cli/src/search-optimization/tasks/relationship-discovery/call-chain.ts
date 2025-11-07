/**
 * Task: Call Chain Tracing
 *
 * Trace how a worktree creation request flows from CLI entry point to git execution.
 * This requires understanding the complete workflow across multiple layers.
 *
 * Why grep fails:
 * - Grep can find individual pieces (CLI command, git operations)
 * - Cannot assemble them into a coherent flow
 * - Easy to miss intermediate steps
 * - Cannot determine order of execution
 *
 * Why search succeeds:
 * - Semantic understanding of "workflow" and "sequence"
 * - Can query for complete flows
 * - Context bundles show related code together
 * - Understands initialization patterns
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_CALL_CHAIN_TRACING: SearchTask = {
  id: 'relationship-call-chain',
  name: 'Trace Worktree Creation Call Chain',
  description:
    'Trace the complete call chain for worktree creation from the CLI command entry point ' +
    'through to the actual git execution. Identify all key steps: ' +
    '1) CLI command definition, ' +
    '2) Command handler/action, ' +
    '3) WorktreeService invocation, ' +
    '4) Git operations. ' +
    'Explain how the flow progresses through each layer.',

  category: 'relationship-discovery',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    // Looking for the complete chain: CLI -> handler -> service -> git
    pattern: /worktree.*create|Command.*worktree|WorktreeService|git\.raw.*worktree/,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe the complete call chain for worktree creation: ' +
      '1) Where is the CLI command defined? ' +
      '2) What handler processes the command? ' +
      '3) How does it invoke WorktreeService? ' +
      '4) How does the service execute git operations? ' +
      'Include key intermediate steps like validation and path resolution.',
    validator: {
      type: 'explanation',
      // Must mention both the CLI entry and the git operations layer
      mentionsFiles: ['worktree.ts', 'worktrees.ts'],
      // Must discuss the flow/chain/sequence
      mentionsPattern: /CLI|command|handler|action|service|git.*raw|flow|chain|sequence|step|invoke|call|execute/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  internalNotes:
    'Demonstrates that semantic search can trace execution flows across architectural layers. ' +
    'Grep can find individual components but cannot connect them into a coherent sequence. ' +
    'Understanding initialization flows is critical for debugging and refactoring.',

  expectedGrepSuccess: 0.25, // 25% - grep can find parts but struggles to connect them
  expectedSearchSuccess: 0.85, // 85% - search understands execution flows

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /worktree.*create|Command.*worktree|WorktreeService|git\.raw.*worktree/,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['worktree.ts', 'worktrees.ts'],
        mentionsPattern: /CLI|command|handler|action|service|git.*raw|flow|chain|sequence|step|invoke|call|execute/i,
      },
    },
  }),
}
