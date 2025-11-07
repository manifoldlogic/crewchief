/**
 * Task: System Interactions - MCP Search Request
 *
 * Understand how the CLI communicates with the Maproom MCP server during a search.
 * This task requires understanding cross-process communication and protocol usage.
 *
 * Why grep fails:
 * - Grep can find MCP-related code but cannot connect client and server
 * - Cannot identify protocol boundaries and serialization
 * - Misses the request/response cycle across process boundaries
 * - Cannot trace execution from one process to another
 *
 * Why search succeeds:
 * - Semantic understanding of "client-server", "IPC", and "protocol"
 * - Can identify MCP tools, request handling, and response formatting
 * - Understands communication patterns (request → process → respond)
 * - Recognizes database queries as part of search execution
 */

import type { SearchTask } from '../../types.js'
import { createTaskValidator } from '../../validators.js'

export const TASK_SYSTEM_INTERACTIONS_MCP_SEARCH: SearchTask = {
  id: 'architecture-system-interactions-mcp',
  name: 'Trace MCP Search Request Flow',
  description:
    'Trace how the CLI communicates with the Maproom MCP server during a search request. ' +
    'Identify the complete flow: 1) MCP protocol usage/client setup, 2) Request formation and sending, ' +
    '3) Database query execution on server side, and 4) Result formatting and response. ' +
    'Explain how data crosses process boundaries and what protocols are used.',

  category: 'architectural-understanding',
  difficulty: 'hard',

  searchTarget: {
    type: 'pattern',
    // Looking for MCP communication components
    pattern: /MCP.*server|index\.ts.*maproom-mcp|search.*tool|JSON-RPC|spawn.*maproom|PostgreSQL|database.*query/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe how the CLI communicates with Maproom MCP server during search: ' +
      '1) How is the MCP protocol used? What transport mechanism? ' +
      '2) How are search requests formed and sent to the server? ' +
      '3) How does the server execute database queries? ' +
      '4) How are results formatted and returned to the client? ' +
      'Include details about process boundaries and data serialization.',
    validator: {
      type: 'explanation',
      // Must mention MCP server and communication
      mentionsFiles: ['index.ts', 'search'],
      // Must discuss communication concepts
      mentionsPattern:
        /(MCP|protocol|JSON-RPC|stdio|IPC).*(?:request|query|search).*(?:database|PostgreSQL|pg).*(?:response|result|format)|client.*server|process.*boundary/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  internalNotes:
    'Tests understanding of inter-process communication and system architecture. ' +
    'Grep cannot trace execution across process boundaries. ' +
    'Search can understand client-server patterns and protocol usage.',

  expectedGrepSuccess: 0.2, // 20% - grep can find components but not trace interaction
  expectedSearchSuccess: 0.8, // 80% - search understands system interactions

  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /MCP.*server|index\.ts.*maproom-mcp|search.*tool|JSON-RPC|spawn.*maproom|PostgreSQL|database.*query/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['index.ts', 'search'],
        mentionsPattern:
          /(MCP|protocol|JSON-RPC|stdio|IPC).*(?:request|query|search).*(?:database|PostgreSQL|pg).*(?:response|result|format)|client.*server|process.*boundary/i,
      },
    },
  }),
}
