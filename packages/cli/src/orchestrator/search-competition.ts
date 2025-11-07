/**
 * Search-specific competition manager for testing tool description variants
 *
 * Extends the base CompetitionManager to support:
 * - SDK-based agent spawning with variant injection
 * - Search tool metrics capture
 * - Evaluation based on task completion and search efficiency
 */

import { randomUUID } from 'node:crypto'
import { CompetitionManager, SearchCompetition, SearchCompetitionParticipant } from './competition.js'
import type { SearchTask } from './task.types.js'
import { spawnAgentWithVariant } from '../sdk/spawner.js'
import type { Variant, ToolUseEvent, AgentResult } from '../sdk/types.js'

export class SearchCompetitionManager extends CompetitionManager {
  /**
   * Start a search competition with multiple variants
   *
   * @param task - Search task with targets and validation
   * @param variants - Tool description variants to test
   * @returns Created competition
   */
  async startSearchCompetition(task: SearchTask, variants: Variant[]): Promise<SearchCompetition> {
    // Create competition structure
    const competition: SearchCompetition = {
      id: randomUUID(),
      task,
      participants: variants.map((variant) => ({
        agentId: `variant-${variant.id}`,
        variant,
        searchMetrics: {
          searchCount: 0,
          avgResultsPerSearch: 0,
          queriesIssued: [],
          toolCallCount: 0,
          durationSeconds: 0,
        },
      })),
      createdAt: new Date().toISOString(),
    }

    // Save competition state using parent class methods
    // Access private methods via bracket notation
    const saveAll = this['saveAll'].bind(this)
    const competitionPath = this['competitionPath'].bind(this)
    const writeJsonSync = await import('../utils/fs.js').then((m) => m.writeJsonSync)

    saveAll([...this.list(), competition])
    writeJsonSync(competitionPath(competition.id), competition)

    // Note: Actual agent spawning would happen in a separate execution phase
    // This method just sets up the competition structure
    return competition
  }

  /**
   * Execute a search competition by spawning agents with variants
   *
   * @param competitionId - Competition ID to execute
   * @returns Results from all participants
   */
  async executeSearchCompetition(competitionId: string): Promise<AgentResult[]> {
    const competition = this.get(competitionId) as SearchCompetition
    if (!competition) {
      throw new Error(`Competition not found: ${competitionId}`)
    }

    const results: AgentResult[] = []

    // Spawn each participant's agent with their variant
    for (const participant of competition.participants) {
      if (!participant.variant) {
        throw new Error(`Participant ${participant.agentId} missing variant`)
      }

      const startTime = Date.now()

      // Spawn agent with variant and hooks for metrics capture
      const result = await spawnAgentWithVariant(competition.task.description, participant.variant, {
        onToolUse: (event) => this.recordToolUse(participant, event),
        onComplete: (result) => this.recordCompletion(participant, result, startTime),
      })

      results.push(result)
    }

    return results
  }

  /**
   * Record tool use event for metrics tracking
   */
  private recordToolUse(participant: SearchCompetitionParticipant, event: ToolUseEvent): void {
    if (!participant.searchMetrics) {
      participant.searchMetrics = {
        searchCount: 0,
        avgResultsPerSearch: 0,
        queriesIssued: [],
        toolCallCount: 0,
        durationSeconds: 0,
      }
    }

    participant.searchMetrics.toolCallCount++

    // Track search-specific metrics
    if (event.tool_name === 'search' || event.tool_name === 'mcp__maproom__search') {
      participant.searchMetrics.searchCount++

      // Extract query from tool input
      const query = event.tool_input?.query
      if (query && typeof query === 'string') {
        participant.searchMetrics.queriesIssued.push(query)
      }
    }
  }

  /**
   * Record agent completion and calculate final metrics
   */
  private recordCompletion(participant: SearchCompetitionParticipant, result: AgentResult, startTime: number): void {
    if (!participant.searchMetrics) return

    // Calculate duration
    const durationMs = Date.now() - startTime
    participant.searchMetrics.durationSeconds = durationMs / 1000

    // Calculate average results per search
    if (participant.searchMetrics.searchCount > 0) {
      participant.searchMetrics.avgResultsPerSearch =
        participant.searchMetrics.toolCallCount / participant.searchMetrics.searchCount
    }

    // Store result info
    participant.runId = result.sessionId
    participant.worktreePath = result.transcriptPath
  }

  /**
   * Evaluate search competition results
   *
   * @param competitionId - Competition to evaluate
   * @returns Updated competition with scores and winner
   */
  async evaluateSearchCompetition(competitionId: string): Promise<SearchCompetition> {
    const competition = this.get(competitionId) as SearchCompetition
    if (!competition) {
      throw new Error(`Competition not found: ${competitionId}`)
    }

    // Score each participant based on:
    // 1. Task completion (did they find the targets?)
    // 2. Search efficiency (fewer searches is better)
    // 3. Query quality (relevant queries)
    for (const participant of competition.participants) {
      participant.score = this.calculateParticipantScore(competition.task, participant)
    }

    // Determine winner (highest score)
    const sorted = competition.participants
      .filter((p) => typeof p.score === 'number')
      .sort((a, b) => b.score! - a.score!)

    competition.winner = sorted[0]?.agentId
    competition.evaluatedAt = new Date().toISOString()

    return competition
  }

  /**
   * Calculate score for a participant
   *
   * Score components:
   * - Base score from default checks (0-1)
   * - Search efficiency bonus/penalty
   * - Query quality assessment
   */
  private calculateParticipantScore(task: SearchTask, participant: SearchCompetitionParticipant): number {
    let score = 0

    // Base score (0.5 weight)
    // Would integrate with existing evaluation system
    const baseScore = 0.7 // Placeholder - would come from runDefaultChecks

    score += baseScore * 0.5

    // Search efficiency (0.3 weight)
    const metrics = participant.searchMetrics
    if (metrics) {
      // Fewer searches is better (normalize to 0-1, assuming 1-10 searches is reasonable)
      const searchEfficiency = Math.max(0, 1 - (metrics.searchCount - 1) / 9)
      score += searchEfficiency * 0.3
    }

    // Query quality (0.2 weight)
    // Simple heuristic: shorter, more focused queries are better
    if (metrics && metrics.queriesIssued.length > 0) {
      const avgQueryLength = metrics.queriesIssued.reduce((sum, q) => sum + q.length, 0) / metrics.queriesIssued.length
      // Normalize: 10-30 chars is ideal
      const queryQuality = avgQueryLength < 10 || avgQueryLength > 50 ? 0.5 : 1.0
      score += queryQuality * 0.2
    }

    return Math.min(1, Math.max(0, score))
  }
}
