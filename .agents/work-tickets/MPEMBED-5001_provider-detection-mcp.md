# Ticket: MPEMBED-5001: Provider detection in MCP TypeScript wrapper

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- typescript-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement auto-detection of Ollama availability in the MCP TypeScript wrapper. Check EMBEDDING_PROVIDER environment variable for explicit configuration, then fall back to detecting Ollama. Pass --provider flag to Rust binary.

## Background
This ticket implements Phase 5 (MCP Integration and Documentation) from the MPEMBED multi-provider embeddings plan. The MCP wrapper needs intelligent provider selection to provide a zero-config experience: if Ollama is running locally, use it automatically; otherwise fall back to OpenAI or Google based on environment variables.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-5-mcp-documentation.md

## Acceptance Criteria
- [ ] `detectProvider()` async function created
- [ ] Checks EMBEDDING_PROVIDER env var first (explicit override)
- [ ] Auto-detects Ollama by attempting connection to localhost:11434
- [ ] 2-second timeout for Ollama detection
- [ ] Falls back to OpenAI if OPENAI_API_KEY present
- [ ] Falls back to Google if GOOGLE_PROJECT_ID present
- [ ] Returns provider name string ("ollama", "openai", "google")
- [ ] Returns helpful error if no provider available
- [ ] Unit tests for detection logic
- [ ] Integration test with mock Ollama server

## Technical Requirements
- Create new module: packages/maproom-mcp/src/utils/provider-detection.ts
- Use fetch() or http client to check Ollama availability
- Handle network errors gracefully (connection refused, timeout)
- Provide clear error messages guiding user to setup
- Cache detection result per MCP session (don't re-detect per tool call)
- Export detection function for use in scan/upsert tools
- TypeScript with strict type checking

## Implementation Notes
**Module Structure:**
```typescript
// packages/maproom-mcp/src/utils/provider-detection.ts

export interface ProviderConfig {
  provider: string; // "ollama" | "openai" | "google"
  dimension: number; // 768 or 1536
  available: boolean;
}

/**
 * Detect available embedding provider
 *
 * Priority:
 * 1. EMBEDDING_PROVIDER env var (explicit override)
 * 2. Ollama (if running on localhost:11434)
 * 3. OpenAI (if OPENAI_API_KEY set)
 * 4. Google (if GOOGLE_PROJECT_ID and GOOGLE_APPLICATION_CREDENTIALS set)
 *
 * @returns Provider configuration
 * @throws Error if no provider available
 */
export async function detectProvider(): Promise<ProviderConfig> {
  // 1. Check explicit override
  const explicitProvider = process.env.EMBEDDING_PROVIDER?.toLowerCase();
  if (explicitProvider) {
    console.log(`Using explicit provider: ${explicitProvider}`);
    return validateExplicitProvider(explicitProvider);
  }

  // 2. Try Ollama auto-detection
  console.log('Auto-detecting embedding provider...');
  if (await isOllamaAvailable()) {
    console.log('✓ Ollama detected at localhost:11434');
    return {
      provider: 'ollama',
      dimension: 768,
      available: true,
    };
  }

  // 3. Try OpenAI
  if (process.env.OPENAI_API_KEY) {
    console.log('✓ Using OpenAI (OPENAI_API_KEY found)');
    return {
      provider: 'openai',
      dimension: 1536,
      available: true,
    };
  }

  // 4. Try Google
  if (process.env.GOOGLE_PROJECT_ID && process.env.GOOGLE_APPLICATION_CREDENTIALS) {
    console.log('✓ Using Google Vertex AI (GOOGLE_PROJECT_ID found)');
    return {
      provider: 'google',
      dimension: 768,
      available: true,
    };
  }

  // No provider available
  throw new Error(
    'No embedding provider available. Options:\n' +
    '  1. Install Ollama: https://ollama.ai (zero-config)\n' +
    '  2. Set OPENAI_API_KEY environment variable\n' +
    '  3. Configure Google Vertex AI (see docs/providers/google-vertex-ai-setup.md)\n' +
    '  4. Set EMBEDDING_PROVIDER explicitly (ollama|openai|google)'
  );
}

/**
 * Check if Ollama is running locally
 */
async function isOllamaAvailable(): Promise<boolean> {
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000); // 2s timeout

    const response = await fetch('http://localhost:11434/api/tags', {
      method: 'GET',
      signal: controller.signal,
    });

    clearTimeout(timeout);

    if (response.ok) {
      const data = await response.json();
      // Verify nomic-embed-text model is available
      const models = data.models || [];
      const hasEmbedModel = models.some(
        (m: any) => m.name.includes('nomic-embed-text')
      );

      if (!hasEmbedModel) {
        console.warn(
          '⚠ Ollama is running but nomic-embed-text model not found. ' +
          'Run: ollama pull nomic-embed-text'
        );
        return false;
      }

      return true;
    }

    return false;
  } catch (error) {
    // Connection refused, timeout, or network error
    return false;
  }
}

/**
 * Validate and return explicit provider configuration
 */
function validateExplicitProvider(provider: string): ProviderConfig {
  switch (provider) {
    case 'ollama':
      // Note: We don't validate Ollama availability here for explicit config
      // User explicitly requested it, so trust them
      return { provider: 'ollama', dimension: 768, available: true };

    case 'openai':
      if (!process.env.OPENAI_API_KEY) {
        throw new Error(
          'EMBEDDING_PROVIDER set to "openai" but OPENAI_API_KEY not found. ' +
          'Set OPENAI_API_KEY or use a different provider.'
        );
      }
      return { provider: 'openai', dimension: 1536, available: true };

    case 'google':
      if (!process.env.GOOGLE_PROJECT_ID) {
        throw new Error(
          'EMBEDDING_PROVIDER set to "google" but GOOGLE_PROJECT_ID not found. ' +
          'See docs/providers/google-vertex-ai-setup.md for setup instructions.'
        );
      }
      if (!process.env.GOOGLE_APPLICATION_CREDENTIALS) {
        throw new Error(
          'EMBEDDING_PROVIDER set to "google" but GOOGLE_APPLICATION_CREDENTIALS not found. ' +
          'See docs/providers/google-vertex-ai-setup.md for setup instructions.'
        );
      }
      return { provider: 'google', dimension: 768, available: true };

    default:
      throw new Error(
        `Unknown provider: "${provider}". Supported: ollama, openai, google`
      );
  }
}

/**
 * Get provider configuration (cached per session)
 */
let cachedProvider: ProviderConfig | null = null;

export async function getProviderConfig(): Promise<ProviderConfig> {
  if (!cachedProvider) {
    cachedProvider = await detectProvider();
  }
  return cachedProvider;
}

/**
 * Clear provider cache (for testing)
 */
export function clearProviderCache(): void {
  cachedProvider = null;
}
```

**Unit Tests:**
```typescript
// packages/maproom-mcp/tests/provider-detection.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { detectProvider, clearProviderCache } from '../src/utils/provider-detection';

describe('provider detection', () => {
  beforeEach(() => {
    clearProviderCache();
    delete process.env.EMBEDDING_PROVIDER;
    delete process.env.OPENAI_API_KEY;
    delete process.env.GOOGLE_PROJECT_ID;
    delete process.env.GOOGLE_APPLICATION_CREDENTIALS;
  });

  it('should use explicit EMBEDDING_PROVIDER', async () => {
    process.env.EMBEDDING_PROVIDER = 'ollama';
    const config = await detectProvider();
    expect(config.provider).toBe('ollama');
    expect(config.dimension).toBe(768);
  });

  it('should detect Ollama when available', async () => {
    // Mock fetch to simulate Ollama running
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ models: [{ name: 'nomic-embed-text' }] }),
    });

    const config = await detectProvider();
    expect(config.provider).toBe('ollama');
  });

  it('should fall back to OpenAI when Ollama not available', async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));
    process.env.OPENAI_API_KEY = 'sk-test123';

    const config = await detectProvider();
    expect(config.provider).toBe('openai');
    expect(config.dimension).toBe(1536);
  });

  it('should fall back to Google when OpenAI not configured', async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));
    process.env.GOOGLE_PROJECT_ID = 'test-project';
    process.env.GOOGLE_APPLICATION_CREDENTIALS = '/path/to/key.json';

    const config = await detectProvider();
    expect(config.provider).toBe('google');
    expect(config.dimension).toBe(768);
  });

  it('should throw error when no provider available', async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'));

    await expect(detectProvider()).rejects.toThrow(
      'No embedding provider available'
    );
  });

  it('should validate OpenAI API key when explicitly set', async () => {
    process.env.EMBEDDING_PROVIDER = 'openai';
    // No OPENAI_API_KEY set

    await expect(detectProvider()).rejects.toThrow('OPENAI_API_KEY not found');
  });

  it('should cache provider detection', async () => {
    process.env.EMBEDDING_PROVIDER = 'ollama';

    const config1 = await getProviderConfig();
    const config2 = await getProviderConfig();

    expect(config1).toBe(config2); // Same object reference (cached)
  });
});
```

## Dependencies
- None (standalone utility module)

## Risk Assessment
- **Risk**: Ollama detection timeout may be too short/long
  - **Mitigation**: 2s timeout is reasonable for localhost; make configurable via env var if needed
- **Risk**: False positives if other service on port 11434
  - **Mitigation**: Check for specific Ollama API response structure (models list)
- **Risk**: Network proxy may block localhost requests
  - **Mitigation**: Document proxy configuration, provide explicit EMBEDDING_PROVIDER override
- **Risk**: Cached provider may become stale during long-running session
  - **Mitigation**: Acceptable for MCP sessions (typically short-lived); provide cache clear function

## Files/Packages Affected
- packages/maproom-mcp/src/utils/provider-detection.ts (create)
- packages/maproom-mcp/tests/provider-detection.test.ts (create)
- packages/maproom-mcp/src/utils/index.ts (modify - export detection functions)
