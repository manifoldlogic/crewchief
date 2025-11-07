/**
 * Tests for architectural understanding tasks
 */

import { describe, it, expect } from 'vitest'
import type { AgentOutput } from '../../../types.js'
import {
  TASK_DATA_FLOW_WORKTREE_CREATION,
  TASK_INIT_SEQUENCE_ORCHESTRATOR,
  TASK_SYSTEM_INTERACTIONS_MCP_SEARCH,
} from '../index.js'

describe('Architectural Understanding Tasks', () => {
  describe('Task Structure Validation', () => {
    it('should have valid structure for data-flow task', () => {
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.id).toBe('architecture-data-flow-worktree')
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.category).toBe('architectural-understanding')
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.difficulty).toBe('hard')
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.expectedGrepSuccess).toBe(0.2)
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.expectedSearchSuccess).toBe(0.8)
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.searchTarget.type).toBe('pattern')
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.followUpTask.type).toBe('explanation')
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for init-sequence task', () => {
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.id).toBe('architecture-init-sequence-orchestrator')
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.category).toBe('architectural-understanding')
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.difficulty).toBe('hard')
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.expectedGrepSuccess).toBe(0.25)
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.expectedSearchSuccess).toBe(0.75)
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.searchTarget.type).toBe('pattern')
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.followUpTask.type).toBe('explanation')
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.successValidator).toBeTypeOf('function')
    })

    it('should have valid structure for system-interactions task', () => {
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.id).toBe('architecture-system-interactions-mcp')
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.category).toBe('architectural-understanding')
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.difficulty).toBe('hard')
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.expectedGrepSuccess).toBe(0.2)
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.expectedSearchSuccess).toBe(0.8)
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.searchTarget.type).toBe('pattern')
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.searchTarget.pattern).toBeInstanceOf(RegExp)
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.followUpTask.type).toBe('explanation')
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.successValidator).toBeTypeOf('function')
    })
  })

  describe('Data Flow Task Validation', () => {
    it('should succeed when all components are found', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'worktree creation flow',
            results: [
              { relpath: 'src/cli/worktree.ts', content: 'registerWorktreeCommands' },
              { relpath: 'src/git/worktrees.ts', content: 'createWorktree WorktreeService' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The CLI command enters through registerWorktreeCommands in worktree.ts. ' +
            'Input is validated by loading config and checking branch names. ' +
            'The WorktreeService business logic manages the worktree creation in worktrees.ts. ' +
            'Git execution spawns the git worktree add command. This data flow shows how the ' +
            'request transforms through each layer.',
        },
        searchCount: 3,
        toolCallCount: 8,
        durationSeconds: 45,
      }

      const score = TASK_DATA_FLOW_WORKTREE_CREATION.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.7)
      expect(score.searchQuality).toBeGreaterThan(0.6)
      expect(score.taskCompletion).toBeGreaterThan(0.7)
    })

    it('should fail when key files are missing', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'worktree',
            results: [{ relpath: 'src/some-random-file.ts', content: 'worktree' }],
          },
        ],
        workResult: {
          success: true,
          explanationText: 'Some vague explanation about worktrees.',
        },
        searchCount: 2,
        toolCallCount: 5,
        durationSeconds: 30,
      }

      const score = TASK_DATA_FLOW_WORKTREE_CREATION.successValidator(mockOutput)
      expect(score.total).toBeLessThan(0.7)
      expect(score.taskCompletion).toBeLessThanOrEqual(0.5)
    })

    it('should succeed when files are mentioned but flow is incomplete', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'worktree creation',
            results: [{ relpath: 'src/cli/worktree.ts', content: 'createWorktree command' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The worktree.ts file handles CLI commands and calls WorktreeService. ' +
            'It mentions worktrees.ts but lacks full flow details.',
        },
        searchCount: 4,
        toolCallCount: 10,
        durationSeconds: 60,
      }

      const score = TASK_DATA_FLOW_WORKTREE_CREATION.successValidator(mockOutput)
      // Mentioning both required files gives full task completion credit
      expect(score.total).toBeGreaterThan(0.7)
      expect(score.taskCompletion).toBe(1.0)
    })
  })

  describe('Init Sequence Task Validation', () => {
    it('should succeed when initialization sequence is explained', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'orchestrator initialization',
            results: [
              { relpath: 'src/config/loader.ts', content: 'loadConfig initialization' },
              { relpath: 'src/agents/registry.ts', content: 'AgentRegistry setup' },
              { relpath: 'src/orchestrator/scheduler.ts', content: 'Scheduler assignSingleAgent' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'Initialization starts with loading configuration via loader. ' +
            'Then the agent registry is set up to discover available agents. ' +
            'The message bus initializes for communication between components. ' +
            'Before the first agent spawn, the scheduler prepares the execution environment. ' +
            'This sequence matters because config must load before registry setup, and ' +
            'communication must be ready before spawning agents.',
        },
        searchCount: 4,
        toolCallCount: 12,
        durationSeconds: 50,
      }

      const score = TASK_INIT_SEQUENCE_ORCHESTRATOR.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.7)
      expect(score.taskCompletion).toBeGreaterThan(0.8)
    })

    it('should fail when sequence order is not explained', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'initialization',
            results: [{ relpath: 'src/some-file.ts', content: 'init code' }],
          },
        ],
        workResult: {
          success: true,
          explanationText: 'There is some initialization code in various files.',
        },
        searchCount: 2,
        toolCallCount: 6,
        durationSeconds: 35,
      }

      const score = TASK_INIT_SEQUENCE_ORCHESTRATOR.successValidator(mockOutput)
      expect(score.total).toBeLessThan(0.5)
    })

    it('should succeed when key files are mentioned', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'startup sequence',
            results: [
              { relpath: 'src/config/loader.ts', content: 'loadConfig' },
              { relpath: 'src/agents/registry.ts', content: 'registry setup' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'Configuration is loaded first via loader. Then the registry sets up agents. ' +
            'The scheduler handles agent initialization.',
        },
        searchCount: 3,
        toolCallCount: 9,
        durationSeconds: 45,
      }

      const score = TASK_INIT_SEQUENCE_ORCHESTRATOR.successValidator(mockOutput)
      // Mentioning all required files gives full task completion credit
      expect(score.total).toBeGreaterThan(0.7)
      expect(score.taskCompletion).toBe(1.0)
    })
  })

  describe('System Interactions Task Validation', () => {
    it('should succeed when MCP communication is fully explained', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'MCP search communication',
            results: [
              { relpath: 'packages/maproom-mcp/src/index.ts', content: 'MCP server JSON-RPC' },
              { relpath: 'packages/maproom-mcp/src/tools/search.ts', content: 'search tool handler' },
            ],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The CLI uses MCP protocol via JSON-RPC over stdio to communicate with the server. ' +
            'Search requests are formed as MCP tool calls and sent through the IPC boundary. ' +
            'The server in index.ts receives the request and executes PostgreSQL database queries. ' +
            'Results are formatted and returned as JSON-RPC responses back to the client. ' +
            'This client-server interaction crosses process boundaries using the MCP protocol.',
        },
        searchCount: 5,
        toolCallCount: 14,
        durationSeconds: 55,
      }

      const score = TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.successValidator(mockOutput)
      expect(score.total).toBeGreaterThan(0.7)
      expect(score.searchQuality).toBeGreaterThan(0.6)
      expect(score.taskCompletion).toBeGreaterThan(0.8)
    })

    it('should fail when protocol details are missing', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'search',
            results: [{ relpath: 'src/random.ts', content: 'search function' }],
          },
        ],
        workResult: {
          success: true,
          explanationText: 'There is a search feature that queries some data.',
        },
        searchCount: 2,
        toolCallCount: 5,
        durationSeconds: 30,
      }

      const score = TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.successValidator(mockOutput)
      expect(score.total).toBeLessThan(0.5)
    })

    it('should handle edge case with wrong files but right concepts', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'MCP protocol',
            results: [{ relpath: 'docs/mcp-usage.md', content: 'MCP documentation' }],
          },
        ],
        workResult: {
          success: true,
          explanationText:
            'The system uses MCP protocol with JSON-RPC. The index.ts server handles requests. ' +
            'Database queries execute via PostgreSQL. Responses are formatted and sent back.',
        },
        searchCount: 4,
        toolCallCount: 10,
        durationSeconds: 48,
      }

      const score = TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.successValidator(mockOutput)
      // Should get partial credit for conceptual understanding
      expect(score.taskCompletion).toBeGreaterThan(0.5)
    })
  })

  describe('Edge Cases', () => {
    it('should handle empty search results', () => {
      const mockOutput: AgentOutput = {
        searchResults: [],
        workResult: {
          success: false,
        },
        searchCount: 0,
        toolCallCount: 1,
        durationSeconds: 5,
      }

      const score1 = TASK_DATA_FLOW_WORKTREE_CREATION.successValidator(mockOutput)
      const score2 = TASK_INIT_SEQUENCE_ORCHESTRATOR.successValidator(mockOutput)
      const score3 = TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.successValidator(mockOutput)

      // Efficiency score can still contribute even with no results
      expect(score1.total).toBeLessThan(0.5)
      expect(score2.total).toBeLessThan(0.5)
      expect(score3.total).toBeLessThan(0.5)
      expect(score1.searchQuality).toBe(0)
      expect(score2.searchQuality).toBe(0)
      expect(score3.searchQuality).toBe(0)
    })

    it('should handle work result without explanation', () => {
      const mockOutput: AgentOutput = {
        searchResults: [
          {
            query: 'test',
            results: [{ relpath: 'test.ts', content: 'test' }],
          },
        ],
        workResult: {
          success: true,
          // No explanationText
        },
        searchCount: 1,
        toolCallCount: 3,
        durationSeconds: 20,
      }

      const score1 = TASK_DATA_FLOW_WORKTREE_CREATION.successValidator(mockOutput)
      const score2 = TASK_INIT_SEQUENCE_ORCHESTRATOR.successValidator(mockOutput)
      const score3 = TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.successValidator(mockOutput)

      expect(score1.taskCompletion).toBe(0)
      expect(score2.taskCompletion).toBe(0)
      expect(score3.taskCompletion).toBe(0)
    })
  })

  describe('Expected Success Rates', () => {
    it('should have grep success rates indicating grep-impossible tasks', () => {
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.expectedGrepSuccess).toBeLessThanOrEqual(0.25)
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.expectedGrepSuccess).toBeLessThanOrEqual(0.25)
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.expectedGrepSuccess).toBeLessThanOrEqual(0.25)
    })

    it('should have high search success rates', () => {
      expect(TASK_DATA_FLOW_WORKTREE_CREATION.expectedSearchSuccess).toBeGreaterThanOrEqual(0.75)
      expect(TASK_INIT_SEQUENCE_ORCHESTRATOR.expectedSearchSuccess).toBeGreaterThanOrEqual(0.75)
      expect(TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.expectedSearchSuccess).toBeGreaterThanOrEqual(0.75)
    })

    it('should show clear advantage for semantic search', () => {
      const advantage1 =
        TASK_DATA_FLOW_WORKTREE_CREATION.expectedSearchSuccess - TASK_DATA_FLOW_WORKTREE_CREATION.expectedGrepSuccess
      const advantage2 =
        TASK_INIT_SEQUENCE_ORCHESTRATOR.expectedSearchSuccess - TASK_INIT_SEQUENCE_ORCHESTRATOR.expectedGrepSuccess
      const advantage3 =
        TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.expectedSearchSuccess -
        TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.expectedGrepSuccess

      expect(advantage1).toBeGreaterThanOrEqual(0.5)
      expect(advantage2).toBeGreaterThanOrEqual(0.5)
      expect(advantage3).toBeGreaterThanOrEqual(0.5)
    })
  })

  describe('Pattern Matching', () => {
    it('should match relevant files with search patterns', () => {
      const pattern1 = TASK_DATA_FLOW_WORKTREE_CREATION.searchTarget.pattern!
      expect(pattern1.test('src/cli/worktree.ts')).toBe(true)
      expect(pattern1.test('WorktreeService class')).toBe(true)
      expect(pattern1.test('createWorktree function')).toBe(true)
      expect(pattern1.test('registerWorktreeCommands in CLI')).toBe(true)
    })

    it('should match initialization-related content', () => {
      const pattern2 = TASK_INIT_SEQUENCE_ORCHESTRATOR.searchTarget.pattern!
      expect(pattern2.test('loadConfig from config/loader')).toBe(true)
      expect(pattern2.test('AgentRegistry setup')).toBe(true)
      expect(pattern2.test('MessageBus initialization')).toBe(true)
      expect(pattern2.test('Scheduler.assignSingleAgent')).toBe(true)
    })

    it('should match MCP communication patterns', () => {
      const pattern3 = TASK_SYSTEM_INTERACTIONS_MCP_SEARCH.searchTarget.pattern!
      expect(pattern3.test('MCP server implementation')).toBe(true)
      expect(pattern3.test('index.ts in maproom-mcp')).toBe(true)
      expect(pattern3.test('JSON-RPC protocol')).toBe(true)
      expect(pattern3.test('PostgreSQL database query')).toBe(true)
    })
  })
})
