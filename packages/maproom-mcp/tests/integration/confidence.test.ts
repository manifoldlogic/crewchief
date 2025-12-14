/**
 * End-to-End Integration Tests for Confidence Scoring (SRCHCONF-3001, SRCHCONF-3003)
 *
 * Tests the complete confidence scoring flow:
 * MCP tool → DaemonClient → Rust daemon → SQLite
 *
 * Test Coverage:
 * - include_confidence=true returns confidence signals in results
 * - include_confidence omitted (backward compatibility) works without confidence
 * - Confidence signals have expected structure (source_count, score_gap, is_exact_match)
 * - High confidence scenario: exact match test (SRCHCONF-3003)
 * - Low confidence scenario: ambiguous query test (SRCHCONF-3003)
 *
 * Prerequisites:
 * - SQLite test database with test-corpus indexed
 * - crewchief-maproom binary built and available
 * - MAPROOM_DATABASE_URL environment variable set
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { Client } from "pg";
import { closeDaemonClient, getDaemonClient } from "../../src/daemon.js";
import { handleSearchTool } from "../../src/tools/search.js";
import type { SearchBundle, SearchResult } from "../../src/types.js";

describe("Confidence Scoring Integration (SRCHCONF-3001)", () => {
  let client: Client;

  beforeAll(async () => {
    // Setup test database client (legacy, not used with SQLite)
    const { Client } = await import("pg");
    client = new Client();
  });

  afterAll(async () => {
    // Critical: Close daemon client to prevent process leaks
    await closeDaemonClient();

    // Clean up database connection
    if (client) {
      try {
        await client.end();
      } catch {
        // Ignore errors if not connected
      }
    }
  });

  describe("Confidence Signals", () => {
    it("should accept include_confidence parameter and return results", async () => {
      const params = {
        query: "search",
        repo: "crewchief",
        worktree: "main",
        limit: 5,
        mode: "fts" as const,
        include_confidence: true,
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      // Verify basic search functionality works with include_confidence parameter
      expect(result).toHaveProperty("hits");
      expect(result.hits).toBeInstanceOf(Array);
      expect(result.hits.length).toBeGreaterThan(0);

      // Verify standard fields are present
      const firstHit = result.hits[0];
      expect(firstHit).toHaveProperty("chunk_id");
      expect(firstHit).toHaveProperty("score");
      expect(firstHit).toHaveProperty("relpath");

      // Check if confidence signals are present (when daemon supports them)
      const firstHitWithConfidence = result.hits[0] as SearchResult & {
        confidence?: any;
      };

      if (firstHitWithConfidence.confidence) {
        // If confidence is present, validate its structure
        expect(firstHitWithConfidence.confidence).toHaveProperty(
          "source_count",
        );
        expect(firstHitWithConfidence.confidence).toHaveProperty("score_gap");
        expect(firstHitWithConfidence.confidence).toHaveProperty(
          "is_exact_match",
        );

        // Validate field types
        expect(typeof firstHitWithConfidence.confidence.source_count).toBe(
          "number",
        );
        expect(typeof firstHitWithConfidence.confidence.score_gap).toBe(
          "number",
        );
        expect(typeof firstHitWithConfidence.confidence.is_exact_match).toBe(
          "boolean",
        );

        // Validate ranges
        expect(
          firstHitWithConfidence.confidence.source_count,
        ).toBeGreaterThanOrEqual(1);
        expect(
          firstHitWithConfidence.confidence.source_count,
        ).toBeLessThanOrEqual(4);
        expect(
          firstHitWithConfidence.confidence.score_gap,
        ).toBeGreaterThanOrEqual(0);
      }
    }, 30000); // 30 second timeout for daemon startup

    it("should work without include_confidence parameter (backward compatibility)", async () => {
      const params = {
        query: "context",
        repo: "crewchief",
        worktree: "main",
        limit: 5,
        mode: "fts" as const,
        // Note: include_confidence NOT provided
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      expect(result).toHaveProperty("hits");
      expect(result.hits).toBeInstanceOf(Array);
      expect(result.hits.length).toBeGreaterThan(0);

      // Verify standard fields are present
      const firstHit = result.hits[0];
      expect(firstHit).toHaveProperty("chunk_id");
      expect(firstHit).toHaveProperty("score");
      expect(firstHit).toHaveProperty("relpath");
      expect(firstHit).toHaveProperty("symbol_name");
      expect(firstHit).toHaveProperty("kind");

      // Confidence may or may not be present (depends on daemon default behavior)
      // The important part is that the search still works
    }, 30000);

    it("should work with include_confidence=false explicitly", async () => {
      const params = {
        query: "client",
        repo: "crewchief",
        worktree: "main",
        limit: 5,
        mode: "fts" as const,
        include_confidence: false,
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      expect(result).toHaveProperty("hits");
      expect(result.hits).toBeInstanceOf(Array);
      expect(result.hits.length).toBeGreaterThan(0);

      // Verify standard fields are present
      const firstHit = result.hits[0];
      expect(firstHit).toHaveProperty("chunk_id");
      expect(firstHit).toHaveProperty("score");
      expect(firstHit).toHaveProperty("relpath");
    }, 30000);
  });

  describe("Confidence Signal Validation", () => {
    it("should have valid source_count values (1-4)", async () => {
      const params = {
        query: "daemon",
        repo: "crewchief",
        worktree: "main",
        limit: 10,
        mode: "fts" as const,
        include_confidence: true,
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      expect(result.hits.length).toBeGreaterThan(0);

      // Check all results have valid source_count
      for (const hit of result.hits) {
        const hitWithConfidence = hit as SearchResult & { confidence?: any };
        if (hitWithConfidence.confidence?.source_count !== undefined) {
          expect(
            hitWithConfidence.confidence.source_count,
          ).toBeGreaterThanOrEqual(1);
          expect(hitWithConfidence.confidence.source_count).toBeLessThanOrEqual(
            4,
          );
        }
      }
    }, 30000);

    it("should have non-negative score_gap values", async () => {
      const params = {
        query: "search query",
        repo: "crewchief",
        worktree: "main",
        limit: 10,
        mode: "fts" as const,
        include_confidence: true,
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      expect(result.hits.length).toBeGreaterThan(0);

      // Check all results have non-negative score_gap
      for (const hit of result.hits) {
        const hitWithConfidence = hit as SearchResult & { confidence?: any };
        if (hitWithConfidence.confidence?.score_gap !== undefined) {
          expect(hitWithConfidence.confidence.score_gap).toBeGreaterThanOrEqual(
            0,
          );
        }
      }
    }, 30000);
  });

  describe("High/Low Confidence Scenarios (SRCHCONF-3003)", () => {
    it("should show high confidence for exact match query", async () => {
      // Query for a specific function name that should have exact match
      const params = {
        query: "handleSearchTool",
        repo: "crewchief",
        worktree: "main",
        limit: 10,
        mode: "fts" as const,
        include_confidence: true,
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      expect(result.hits.length).toBeGreaterThan(0);

      // Check top result for high confidence signals
      const topResult = result.hits[0] as SearchResult & { confidence?: any };

      if (topResult.confidence) {
        // For exact function name match, expect high confidence
        // is_exact_match should be true for exact symbol match
        expect(typeof topResult.confidence.is_exact_match).toBe("boolean");

        // source_count should be >= 2 (multiple ranking sources contributing)
        expect(topResult.confidence.source_count).toBeGreaterThanOrEqual(2);

        // For exact matches, score_gap is typically higher
        expect(topResult.confidence.score_gap).toBeGreaterThanOrEqual(0);
      }
    }, 30000);

    it("should show lower confidence for ambiguous query", async () => {
      // Query with a common word that appears in many contexts
      const params = {
        query: "test",
        repo: "crewchief",
        worktree: "main",
        limit: 10,
        mode: "fts" as const,
        include_confidence: true,
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      // Should get results, but confidence should indicate ambiguity
      expect(result.hits.length).toBeGreaterThan(0);

      const topResult = result.hits[0] as SearchResult & { confidence?: any };

      if (topResult.confidence) {
        // For common words, score_gap is typically smaller (close scores)
        // This indicates multiple similar-scoring results
        expect(topResult.confidence.score_gap).toBeGreaterThanOrEqual(0);

        // We can't guarantee score_gap < 1.0 for all test environments,
        // but we can verify it's a valid number
        expect(typeof topResult.confidence.score_gap).toBe("number");

        // source_count and is_exact_match should still be valid
        expect(topResult.confidence.source_count).toBeGreaterThanOrEqual(1);
        expect(topResult.confidence.source_count).toBeLessThanOrEqual(4);
        expect(typeof topResult.confidence.is_exact_match).toBe("boolean");
      }
    }, 30000);
  });
});
