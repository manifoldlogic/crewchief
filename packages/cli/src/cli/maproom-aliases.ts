import { Command } from 'commander'
import { runMaproomForward, runMaproomSearchWithAutoIndex } from './maproom.js'

/**
 * Register top-level command aliases that forward to maproom subcommands.
 *
 * These hide the crewchief/maproom seam from users who should not need
 * to know the tool is two binaries.
 *
 * - `crewchief search <query>`   -> `crewchief maproom search <query>`
 * - `crewchief index`            -> `crewchief maproom scan`
 * - `crewchief context`          -> `crewchief maproom context`
 *
 * The `crewchief maproom` prefix is preserved for power users.
 */
export function registerMaproomAliases(program: Command) {
  program
    .command('search')
    .description('Search your codebase by concept')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText(
      'after',
      '\nExamples:\n  $ crewchief search "authentication flow"\n  $ crewchief search "database queries" --limit 10\n  $ crewchief search "error handling" --format agent',
    )
    .action(async (args) => await runMaproomSearchWithAutoIndex(args || []))

  program
    .command('index')
    .description('Index the current repository for code search')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText(
      'after',
      '\nIndexes repository files into SQLite for fast code search.\nAuto-detects: repo name, worktree, file path, and commit from git context.\n\nExamples:\n  $ crewchief index                        # FTS-only index (default)\n  $ crewchief index --generate-embeddings   # Include vector embeddings',
    )
    .action(async (args) => await runMaproomForward(['scan', ...(args || [])]))

  program
    .command('context')
    .description('Retrieve context bundle for a code chunk')
    .allowUnknownOption(true)
    .argument('[args...]')
    .addHelpText(
      'after',
      '\nAssembles a context bundle containing the primary chunk and related code\n(callers, callees, tests, docs, config) within a token budget.\n\nExamples:\n  $ crewchief context --chunk-id 12345\n  $ crewchief context --chunk-id 12345 --callers --budget 4000\n  $ crewchief context --chunk-id 12345 --format agent',
    )
    .action(async (args) => await runMaproomForward(['context', ...(args || [])]))
}
