#!/usr/bin/env tsx
/**
 * Simple test script to verify MCP configuration is properly loaded by Claude Agents SDK
 *
 * This script:
 * 1. Spawns an agent using the Claude Agents SDK
 * 2. Tests if maproom MCP tools are accessible
 * 3. Reports success or failure
 *
 * Usage:
 *   tsx scripts/test-mcp-config.ts [worktree-path]
 *
 * If worktree-path is not provided, uses current directory.
 */

import { resolve } from 'path'
import { spawnAgent } from '../src/sdk/spawner.js'

async function main() {
  const worktreePath = process.argv[2] || process.cwd()

  console.log('\n🔍 Testing MCP Configuration')
  console.log(`   Worktree: ${worktreePath}`)
  console.log(`   .mcp.json should be at: ${resolve(worktreePath, '.mcp.json')}`)
  console.log()

  // Task that exercises maproom MCP tools (status + search)
  const task = `Test the maproom MCP integration by doing the following:
1. Use mcp__maproom__status to check if the index is available
2. Use mcp__maproom__search with repo="crewchief" and query="agent spawn" to test search functionality
3. Report back: Did both tools work? What were the results?`

  console.log(`📝 Task: ${task}`)
  console.log()

  try {
    const result = await spawnAgent({
      task,
      worktreePath,
      permissionMode: 'autoApprove',
      maxTurns: 5,
    })

    console.log('\n✅ Agent completed successfully')
    console.log(`   Success: ${result.success}`)
    console.log(`   Turns: ${result.performance?.numTurns}`)
    console.log(`   Duration: ${result.performance?.durationMs}ms`)

    if (result.finalMessage) {
      console.log('\n📄 Final message:')
      console.log(JSON.stringify(result.finalMessage, null, 2))
    }

    // Check if agent used MCP tools and which ones
    const maproomToolsUsed = result.messages
      .filter((msg: any) => msg.type === 'tool_use' && msg.tool_name?.startsWith('mcp__maproom__'))
      .map((msg: any) => msg.tool_name)

    const usedStatus = maproomToolsUsed.includes('mcp__maproom__status')
    const usedSearch = maproomToolsUsed.includes('mcp__maproom__search')

    console.log('\n📊 Maproom MCP Tools Used:')
    console.log(`   mcp__maproom__status: ${usedStatus ? '✅ YES' : '❌ NO'}`)
    console.log(`   mcp__maproom__search: ${usedSearch ? '✅ YES' : '❌ NO'}`)

    if (usedStatus && usedSearch) {
      console.log('\n✅ SUCCESS: Both maproom MCP tools were accessible and used!')
      console.log('   This confirms MCP configuration is working correctly.')
      process.exit(0)
    } else if (maproomToolsUsed.length > 0) {
      console.log('\n⚠️  PARTIAL SUCCESS: Some maproom tools used, but not all')
      console.log(`   Tools used: ${maproomToolsUsed.join(', ')}`)
      console.log('   This might indicate partial MCP functionality')
      process.exit(1)
    } else {
      console.log('\n❌ FAILURE: No maproom MCP tools were used')
      console.log('   This indicates MCP configuration issues')
      process.exit(1)
    }
  } catch (error) {
    console.error('\n❌ ERROR: Agent failed')
    console.error(error)
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('\n❌ FATAL ERROR:')
  console.error(error)
  process.exit(1)
})
