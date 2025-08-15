import fs from 'node:fs'
import { Command } from 'commander'
import simpleGit from 'simple-git'
import { GitMergeService } from '../git/merge'
import { WorktreeService } from '../git/worktrees'
import { CompetitionManager } from '../orchestrator/competition'
import { generateText } from '../utils/llm'
import { logger } from '../utils/logger'

export function registerCompetitionCommands(program: Command): void {
  const comp = new Command('competition').description('Run competition mode across agents')

  comp
    .command('compare')
    .description('Compare N worktrees by diff vs main, summarize and evaluate with an LLM')
    .argument('<worktrees...>')
    .option('--criteria <file>', 'Path to a markdown/text file describing evaluation criteria')
    .option('--base <branch>', 'Base branch to diff against', 'main')
    .option('--json', 'Output JSON instead of Markdown')
    .option('--auto-merge', 'Auto-merge the winner into target branch (not default)')
    .option('--target <branch>', 'Target branch for auto-merge', 'main')
    .option('--strategy <type>', 'Merge strategy: squash|ff|cherry-pick', 'squash')
    .option('--force', 'Proceed even if worktrees have uncommitted changes')
    .action(
      async (
        worktrees: string[],
        opts: {
          criteria?: string
          base: string
          json?: boolean
          autoMerge?: boolean
          target: string
          strategy: 'squash' | 'ff' | 'cherry-pick'
          force?: boolean
        },
      ) => {
        try {
          if (!worktrees || worktrees.length < 2) {
            throw new Error('Provide two or more worktrees to compare')
          }

          const wt = new WorktreeService()
          const listed = await wt.listWorktrees()
          const selected = listed.filter((w) => worktrees.includes(w.branch ?? ''))
          if (selected.length !== worktrees.length) {
            const known = listed.map((w) => w.branch ?? w.path).join(', ')
            throw new Error(`One or more worktrees not found. Known: ${known}`)
          }

          const criteriaText = opts.criteria ? fs.readFileSync(opts.criteria, 'utf8') : undefined

          // Safety: ensure clean worktrees so users don't think uncommitted changes were evaluated
          const dirty: { branch: string; path: string }[] = []
          for (const w of selected) {
            const git = simpleGit({ baseDir: w.path })
            const st = await git.status()
            if (!st.isClean()) dirty.push({ branch: w.branch ?? w.path, path: w.path })
          }
          if (dirty.length > 0 && !opts.force) {
            const list = dirty.map((d) => `- ${d.branch} (${d.path})`).join('\n')
            logger.error(
              `Aborting: uncommitted changes detected in worktrees. Commit or stash first, or re-run with --force.\n${list}`,
            )
            process.exitCode = 1
            return
          }

          // Build a structured prompt: for each worktree, get `git diff --stat` and full patch
          const diffs: { branch: string; stat: string; patch: string }[] = []
          for (const w of selected) {
            const branch = w.branch ?? w.path
            const stat = await runGit(['-C', w.path, 'diff', `${opts.base}...${branch}`, '--stat'])
            const patch = await runGit(['-C', w.path, 'diff', `${opts.base}...${branch}`])
            diffs.push({ branch, stat, patch })
          }

          let winnerBranch: string | undefined

          if (opts.json) {
            const prompt = buildEvalPromptJson(diffs, opts.base, criteriaText)
            const response = await generateText(prompt, {})
            const json = parseJsonFromText(response)
            if (!json) throw new Error('Failed to parse JSON response from LLM')
            process.stdout.write(JSON.stringify(json, null, 2) + '\n')
            winnerBranch = json?.winner?.branch
          } else {
            const prompt = buildEvalPrompt(diffs, opts.base, criteriaText)
            const response = await generateText(prompt, {})
            process.stdout.write(response + '\n')
            winnerBranch = parseWinnerFromMarkdown(response)
          }

          if (opts.autoMerge) {
            if (!winnerBranch) throw new Error('Winner branch not found in evaluation output')
            const merge = new GitMergeService()
            const res = await merge.merge({
              sourceBranch: winnerBranch,
              targetBranch: opts.target,
              strategy: opts.strategy,
            })
            if (res.success) {
              logger.success(`Auto-merged winner '${winnerBranch}' into '${opts.target}' (${res.message ?? 'ok'})`)
            } else {
              logger.warn(`Winner '${winnerBranch}' not merged: ${res.message ?? 'unknown error'}`)
            }
          }
        } catch (err: any) {
          logger.error(err?.message || String(err))
          process.exitCode = 1
        }
      },
    )

  comp
    .command('start')
    .argument('<description>')
    .argument('<agentIds...>')
    .description('Create a new competition')
    .action(async (description: string, agentIds: string[]) => {
      const cm = new CompetitionManager()
      const c = cm.start(description, agentIds)
      logger.success(`Competition ${c.id} created with agents: ${agentIds.join(', ')}`)
    })

  comp
    .command('assign')
    .argument('<competitionId>')
    .description('Assign task to all competition agents')
    .action(async (competitionId: string) => {
      const cm = new CompetitionManager()
      const c = await cm.assign(competitionId)
      logger.success(`Competition ${c.id} assigned: ${c.participants.map((p) => p.runId).join(', ')}`)
    })

  comp
    .command('evaluate')
    .argument('<competitionId>')
    .description('Evaluate competition runs and pick winner')
    .action(async (competitionId: string) => {
      const cm = new CompetitionManager()
      const c = await cm.evaluate(competitionId)
      logger.success(`Competition ${c.id} winner: ${c.winner ?? 'n/a'}`)
    })

  comp
    .command('finalize')
    .argument('<competitionId>')
    .description('Evaluate and attempt auto-merge winner based on score threshold')
    .action(async (competitionId: string) => {
      const cm = new CompetitionManager()
      const result = await cm.finalize(competitionId)
      if (result.merged) {
        logger.success(`Competition ${result.competition.id} winner merged (score=${result.score?.toFixed(2)})`)
      } else {
        logger.warn(`Competition ${result.competition.id} not merged: ${result.reason ?? ''}`)
      }
    })

  program.addCommand(comp)
}

async function runGit(args: string[]): Promise<string> {
  const { spawn } = await import('node:child_process')
  return await new Promise((resolve, reject) => {
    const child = spawn('git', args, { stdio: ['ignore', 'pipe', 'pipe'] })
    let out = ''
    let err = ''
    child.stdout.on('data', (d) => (out += d.toString()))
    child.stderr.on('data', (d) => (err += d.toString()))
    child.on('close', (code) => {
      if (code === 0) resolve(out.trim())
      else reject(new Error(err.trim() || `git ${args.join(' ')} failed with code ${code}`))
    })
  })
}

function buildEvalPrompt(
  diffs: { branch: string; stat: string; patch: string }[],
  base: string,
  criteria?: string,
): string {
  const header = `You are evaluating multiple git worktrees (branches) against base '${base}'. For each branch: summarize the changes, key improvements, potential risks, and technical quality. Identify anything implemented in this branch that is not implemented in the others (unique or net-new capabilities), and call out explicitly what would be sacrificed if this branch is NOT chosen. Then provide a comparative analysis across branches and select a winner.

Output format (Markdown):

## Per-Branch Summaries
- <branch>: Summary (2-3 paragraphs)
- Risks
- Notable files
- Unique implementations (only in this branch)
- Sacrifices if not chosen (what unique items would be lost)

## Comparative Analysis
- Differences vs each other
- Tradeoffs
- Unique coverage matrix: which branches contain unique capabilities not present elsewhere

## Criteria
${criteria ? criteria : '(no custom criteria provided)'}

## Winner
- Branch: <name>
- Rationale: <short>
`
  const parts = diffs
    .map(
      (d) => `### Branch: ${d.branch}
Diff stat:
${d.stat}

Patch (unified):
${d.patch}
`,
    )
    .join('\n\n')
  return `${header}\n\n${parts}`
}

function buildEvalPromptJson(
  diffs: { branch: string; stat: string; patch: string }[],
  base: string,
  criteria?: string,
): string {
  const header = `You are evaluating multiple git branches against base '${base}'. Identify for each branch the items implemented only in that branch (not present in the others) and what would be sacrificed if that branch is NOT chosen. Produce ONLY a single JSON object with this exact schema and no extra commentary:
{
  "base": string,
  "branches": [
    {
      "name": string,
      "summary": string,
      "risks": string[],
      "notableFiles": string[],
      "uniqueImplementations": string[],
      "sacrificesIfNotChosen": string[]
    }
  ],
  "comparison": {
    "differences": string,
    "tradeoffs": string,
    "uniqueCoverage": string
  },
  "winner": {
    "branch": string,
    "rationale": string
  }
}

Criteria (optional): ${criteria ? criteria : '(none)'}
`
  const parts = diffs
    .map(
      (d) => `Branch ${d.branch}:
Diff stat:
${d.stat}

Patch:
${d.patch}
`,
    )
    .join('\n\n')
  return `${header}\n${parts}`
}

function parseJsonFromText(text: string): any | null {
  try {
    return JSON.parse(text)
  } catch {
    // Try to extract a JSON block
    const match = text.match(/\{[\s\S]*\}/)
    if (!match) return null
    try {
      return JSON.parse(match[0])
    } catch {
      return null
    }
  }
}

function parseWinnerFromMarkdown(md: string): string | undefined {
  const re = /Winner[\s\S]*?Branch:\s*([^\s\n]+)/i
  const m = md.match(re)
  return m?.[1]?.trim()
}
