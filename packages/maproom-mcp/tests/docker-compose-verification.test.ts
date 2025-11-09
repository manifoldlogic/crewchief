/**
 * Tests for docker-compose.yml verification
 *
 * These tests verify:
 * - Detection of hardcoded MAPROOM_EMBEDDING_PROVIDER values
 * - Detection of environment variable syntax
 * - Proper error handling for outdated configs
 * - Validation passes for correct configs
 */

import { describe, it, expect } from 'vitest'

describe('Docker Compose Verification', () => {
  // These are the regex patterns from cli.cjs verifyDockerComposeConfig()
  const envVarRegex = /\$\{MAPROOM_EMBEDDING_PROVIDER[:\-]/
  const hardcodedRegex = /MAPROOM_EMBEDDING_PROVIDER:\s*['"]?ollama['"]?\s*$/m

  function wouldFailVerification(content: string): boolean {
    const hasEnvVarSyntax = envVarRegex.test(content)
    const hasHardcodedProvider = hardcodedRegex.test(content)
    return hasHardcodedProvider && !hasEnvVarSyntax
  }

  describe('Hardcoded MAPROOM_EMBEDDING_PROVIDER Detection', () => {
    it('should detect hardcoded value without quotes', () => {
      const content = 'MAPROOM_EMBEDDING_PROVIDER: ollama'
      expect(hardcodedRegex.test(content)).toBe(true)
      expect(wouldFailVerification(content)).toBe(true)
    })

    it('should detect hardcoded value with single quotes', () => {
      const content = "MAPROOM_EMBEDDING_PROVIDER: 'ollama'"
      expect(hardcodedRegex.test(content)).toBe(true)
      expect(wouldFailVerification(content)).toBe(true)
    })

    it('should detect hardcoded value with double quotes', () => {
      const content = 'MAPROOM_EMBEDDING_PROVIDER: "ollama"'
      expect(hardcodedRegex.test(content)).toBe(true)
      expect(wouldFailVerification(content)).toBe(true)
    })

    it('should detect hardcoded value with extra whitespace', () => {
      const content = 'MAPROOM_EMBEDDING_PROVIDER:   ollama  '
      expect(hardcodedRegex.test(content)).toBe(true)
    })
  })

  describe('Environment Variable Syntax Detection', () => {
    it('should detect env var with default value', () => {
      const content = 'MAPROOM_EMBEDDING_PROVIDER: ${MAPROOM_EMBEDDING_PROVIDER:-ollama}'
      expect(envVarRegex.test(content)).toBe(true)
      expect(wouldFailVerification(content)).toBe(false)
    })

    it('should detect env var without default value (colon syntax)', () => {
      const content = 'MAPROOM_EMBEDDING_PROVIDER: ${MAPROOM_EMBEDDING_PROVIDER:ollama}'
      expect(envVarRegex.test(content)).toBe(true)
      expect(wouldFailVerification(content)).toBe(false)
    })

    it('should detect env var with dash syntax', () => {
      const content = 'MAPROOM_EMBEDDING_PROVIDER: ${MAPROOM_EMBEDDING_PROVIDER-ollama}'
      expect(envVarRegex.test(content)).toBe(true)
      expect(wouldFailVerification(content)).toBe(false)
    })
  })

  describe('Mixed Content Scenarios', () => {
    it('should pass if both hardcoded and env var syntax exist', () => {
      // This could happen if there are comments or multiple services
      const content = `
        # Old config (commented out):
        # MAPROOM_EMBEDDING_PROVIDER: ollama

        services:
          maproom-mcp:
            environment:
              MAPROOM_EMBEDDING_PROVIDER: \${MAPROOM_EMBEDDING_PROVIDER:-ollama}
      `
      expect(wouldFailVerification(content)).toBe(false)
    })

    it('should pass for actual maproom-mcp docker-compose.yml format', () => {
      const content = `
services:
  maproom-mcp:
    environment:
      MAPROOM_DATABASE_URL: postgresql://maproom:maproom@maproom-postgres:5432/maproom
      MAPROOM_EMBEDDING_PROVIDER: \${MAPROOM_EMBEDDING_PROVIDER:-ollama}
      MAPROOM_EMBEDDING_MODEL: \${MAPROOM_EMBEDDING_MODEL:-nomic-embed-text}
      EMBEDDING_DIMENSION: \${EMBEDDING_DIMENSION:-768}
      `
      expect(wouldFailVerification(content)).toBe(false)
    })

    it('should fail for pre-MCP-008 config format', () => {
      const content = `
services:
  maproom-mcp:
    environment:
      MAPROOM_DATABASE_URL: postgresql://maproom:maproom@maproom-postgres:5432/maproom
      MAPROOM_EMBEDDING_PROVIDER: ollama
      MAPROOM_EMBEDDING_MODEL: nomic-embed-text
      EMBEDDING_DIMENSION: 768
      `
      expect(wouldFailVerification(content)).toBe(true)
    })
  })

  describe('Edge Cases', () => {
    it('should not detect provider in other context', () => {
      const content = 'SOME_OTHER_PROVIDER: ollama'
      expect(hardcodedRegex.test(content)).toBe(false)
    })

    it('should handle multiline content', () => {
      const content = `
        services:
          postgres:
            image: postgres:16
          maproom-mcp:
            environment:
              MAPROOM_EMBEDDING_PROVIDER: ollama
      `
      expect(hardcodedRegex.test(content)).toBe(true)
      expect(wouldFailVerification(content)).toBe(true)
    })

    it('should pass for empty content', () => {
      const content = ''
      expect(wouldFailVerification(content)).toBe(false)
    })

    it('should pass if MAPROOM_EMBEDDING_PROVIDER is not mentioned', () => {
      const content = `
        services:
          postgres:
            image: postgres:16
            environment:
              POSTGRES_PASSWORD: postgres
      `
      expect(wouldFailVerification(content)).toBe(false)
    })
  })

  describe('Regression Tests', () => {
    it('should handle MCP-008 migration scenario', () => {
      // Before MCP-008: hardcoded
      const beforeContent = `
        environment:
          MAPROOM_EMBEDDING_PROVIDER: ollama
      `
      expect(wouldFailVerification(beforeContent)).toBe(true)

      // After MCP-008: env var syntax
      const afterContent = `
        environment:
          MAPROOM_EMBEDDING_PROVIDER: \${MAPROOM_EMBEDDING_PROVIDER:-ollama}
      `
      expect(wouldFailVerification(afterContent)).toBe(false)
    })

    it('should handle MCP-011 update detection', () => {
      // This tests the auto-update logic that checks for outdated configs
      const outdatedConfig = `
        services:
          maproom-mcp:
            environment:
              MAPROOM_EMBEDDING_PROVIDER: ollama
      `
      const hasHardcoded = /MAPROOM_EMBEDDING_PROVIDER: ollama/.test(outdatedConfig)
      const hasEnvVar = /\$\{MAPROOM_EMBEDDING_PROVIDER/.test(outdatedConfig)

      expect(hasHardcoded).toBe(true)
      expect(hasEnvVar).toBe(false)
      expect(wouldFailVerification(outdatedConfig)).toBe(true)
    })
  })
})
