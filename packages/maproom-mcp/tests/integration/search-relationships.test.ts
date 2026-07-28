/**
 * Integration Tests for Search Functionality (formerly search-relationships)
 *
 * R2: The phantom "no-op" field was removed from SearchParams — it was never
 * implemented in Rust SearchParams. Cross-repo graph traversal is tracked in
 * Wave 3 (rust-cross-repo-search).
 *
 * This file covers basic search functionality that was tested
 * in the former relationship expansion tests.
 *
 * Prerequisites:
 * - SQLite or PostgreSQL test database with crewchief indexed
 * - maproom binary built and available
 * - MAPROOM_DATABASE_URL environment variable set
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { Client } from "pg";
import { closeDaemonClient } from "../../src/daemon.js";
import { handleSearchTool } from "../../src/tools/search.js";
import type { SearchBundle } from "../../src/types.js";

describe("Search Integration (R2: phantom field removed)", () => {
  let client: Client;

  beforeAll(async () => {
    // Setup test database client (legacy, not used with SQLite)
    const { Client } = await import("pg");
    client = new Client();
  });

  afterAll(async () => {
    await closeDaemonClient();
    if (client) {
      try {
        await client.end();
      } catch {
        // Ignore errors if not connected
      }
    }
  });

  describe("Basic search (R2: no phantom field passed)", () => {
    it("search returns hits with standard fields", async () => {
      const params = {
        query: "handleSearchTool",
        repo: "crewchief",
        worktree: "main",
        limit: 5,
        mode: "fts" as const,
        // R2: no phantom no-op field
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      expect(result).toHaveProperty("hits");
      expect(result.hits).toBeInstanceOf(Array);
      expect(result.hits.length).toBeGreaterThan(0);

      const firstHit = result.hits[0];
      expect(firstHit).toHaveProperty("chunk_id");
      expect(firstHit).toHaveProperty("score");
      expect(firstHit).toHaveProperty("relpath");
    }, 30000);

    it("search with include_confidence works normally", async () => {
      const params = {
        query: "daemon",
        repo: "crewchief",
        worktree: "main",
        limit: 5,
        mode: "fts" as const,
        include_confidence: true,
        // R2: no phantom no-op field
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      expect(result).toHaveProperty("hits");
      expect(result.hits).toBeInstanceOf(Array);
      expect(result.hits.length).toBeGreaterThan(0);

      const firstHit = result.hits[0];
      expect(firstHit).toHaveProperty("chunk_id");
      expect(firstHit).toHaveProperty("score");
      expect(firstHit).toHaveProperty("relpath");
    }, 30000);

    it("search returns valid hit structure", async () => {
      const params = {
        query: "search",
        repo: "crewchief",
        worktree: "main",
        limit: 10,
        mode: "fts" as const,
      };

      const result: SearchBundle = await handleSearchTool(params, client);

      expect(result.hits.length).toBeGreaterThan(0);

      for (const hit of result.hits) {
        expect(typeof hit.chunk_id).toBe("number");
        expect(typeof hit.score).toBe("number");
        expect(typeof hit.relpath).toBe("string");
        expect(hit.relpath.length).toBeGreaterThan(0);
        expect(hit.score).toBeGreaterThan(0);
      }
    }, 30000);
  });
});
