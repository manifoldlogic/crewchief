#!/usr/bin/env node

/**
 * Maproom MCP Server
 *
 * Single-purpose: Run MCP server via stdio.
 * Expects database to exist (use VSCode extension or docker compose for setup).
 */

async function main() {
  // Import resolveDatabase from compiled module
  const { resolveDatabase } = await import('../dist/utils/resolve-database.js')
  process.env.MAPROOM_DATABASE_URL = resolveDatabase()
  await import('../dist/index.js')
}

main().catch(error => {
  console.error('MCP server error:', error.message)
  process.exit(1)
})
