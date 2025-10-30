/**
 * Symbol card template for rendering code chunk explanations in markdown format
 */

/**
 * Symbol card data structure containing all information needed for display
 */
export interface SymbolCard {
  /** Target chunk information */
  chunk: {
    id: number
    symbol_name: string | null
    kind: string
    start_line: number
    end_line: number
  }

  /** File location */
  location: {
    relpath: string
    worktree: string
  }

  /** Metadata about the symbol */
  metadata: {
    language: string | null
    visibility?: string
    parent_context?: string
    [key: string]: any
  }

  /** Code preview/snippet */
  preview: {
    content: string
    line_count: number
  }

  /** Relationships to other chunks */
  relationships: {
    imports: Array<{ symbol_name: string; relpath: string }>
    exports: Array<{ symbol_name: string; relpath: string }>
    calls: Array<{ symbol_name: string; relpath: string }>
    called_by: Array<{ symbol_name: string; relpath: string }>
    tests: Array<{ symbol_name: string; relpath: string }>
  }

  /** Usage examples (if available) */
  examples?: Array<{
    description: string
    code: string
  }>
}

/**
 * Format symbol card as markdown
 * @param card - Symbol card data
 * @returns Formatted markdown string
 */
export function formatSymbolCard(card: SymbolCard): string {
  const lines: string[] = []

  // Header
  const symbolName = card.chunk.symbol_name || 'Unknown Symbol'
  const symbolKind = card.chunk.kind || 'unknown'
  lines.push(`# ${symbolName}`)
  lines.push('')
  lines.push(`**Type:** \`${symbolKind}\``)
  lines.push('')

  // Location
  lines.push('## Location')
  lines.push('')
  lines.push(`- **File:** \`${card.location.relpath}\``)
  lines.push(`- **Lines:** ${card.chunk.start_line}-${card.chunk.end_line}`)
  lines.push(`- **Worktree:** \`${card.location.worktree}\``)
  lines.push('')

  // Metadata
  if (Object.keys(card.metadata).length > 0) {
    lines.push('## Metadata')
    lines.push('')
    if (card.metadata.language) {
      lines.push(`- **Language:** ${card.metadata.language}`)
    }
    if (card.metadata.visibility) {
      lines.push(`- **Visibility:** ${card.metadata.visibility}`)
    }
    if (card.metadata.parent_context) {
      lines.push(`- **Context:** ${card.metadata.parent_context}`)
    }
    // Include any other metadata
    for (const [key, value] of Object.entries(card.metadata)) {
      if (!['language', 'visibility', 'parent_context'].includes(key)) {
        lines.push(`- **${key}:** ${JSON.stringify(value)}`)
      }
    }
    lines.push('')
  }

  // Relationships
  const hasRelationships = Object.values(card.relationships).some((rel) => rel.length > 0)
  if (hasRelationships) {
    lines.push('## Relationships')
    lines.push('')

    if (card.relationships.imports.length > 0) {
      lines.push('### Imports')
      lines.push('')
      for (const imp of card.relationships.imports) {
        lines.push(`- \`${imp.symbol_name}\` from \`${imp.relpath}\``)
      }
      lines.push('')
    }

    if (card.relationships.exports.length > 0) {
      lines.push('### Exports')
      lines.push('')
      for (const exp of card.relationships.exports) {
        lines.push(`- \`${exp.symbol_name}\` to \`${exp.relpath}\``)
      }
      lines.push('')
    }

    if (card.relationships.calls.length > 0) {
      lines.push('### Calls')
      lines.push('')
      for (const call of card.relationships.calls) {
        lines.push(`- \`${call.symbol_name}\` in \`${call.relpath}\``)
      }
      lines.push('')
    }

    if (card.relationships.called_by.length > 0) {
      lines.push('### Called By')
      lines.push('')
      for (const caller of card.relationships.called_by) {
        lines.push(`- \`${caller.symbol_name}\` in \`${caller.relpath}\``)
      }
      lines.push('')
    }

    if (card.relationships.tests.length > 0) {
      lines.push('### Tests')
      lines.push('')
      for (const test of card.relationships.tests) {
        lines.push(`- \`${test.symbol_name}\` in \`${test.relpath}\``)
      }
      lines.push('')
    }
  }

  // Code Preview
  lines.push('## Code Preview')
  lines.push('')
  const language = card.metadata.language || ''
  lines.push('```' + language)
  lines.push(card.preview.content)
  lines.push('```')
  lines.push('')
  lines.push(`*${card.preview.line_count} lines*`)
  lines.push('')

  // Usage Examples
  if (card.examples && card.examples.length > 0) {
    lines.push('## Usage Examples')
    lines.push('')
    for (const example of card.examples) {
      lines.push(`### ${example.description}`)
      lines.push('')
      lines.push('```' + language)
      lines.push(example.code)
      lines.push('```')
      lines.push('')
    }
  }

  return lines.join('\n')
}

/**
 * Create an empty symbol card with default values
 * @param chunkId - Chunk ID
 * @returns Empty symbol card
 */
export function createEmptySymbolCard(chunkId: number): SymbolCard {
  return {
    chunk: {
      id: chunkId,
      symbol_name: null,
      kind: 'unknown',
      start_line: 0,
      end_line: 0,
    },
    location: {
      relpath: '',
      worktree: '',
    },
    metadata: {
      language: null,
    },
    preview: {
      content: '',
      line_count: 0,
    },
    relationships: {
      imports: [],
      exports: [],
      calls: [],
      called_by: [],
      tests: [],
    },
  }
}
