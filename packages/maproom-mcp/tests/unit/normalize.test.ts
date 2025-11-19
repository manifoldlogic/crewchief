/**
 * Unit tests for query normalization function
 *
 * Tests the normalizeForExactMatch() function that handles:
 * - camelCase → snake_case
 * - Acronyms at start (XMLParser → xml_parser)
 * - Acronyms in middle (validateHTTPRequest → validate_http_request)
 * - Consecutive capitals (HTTPSHandler → https_handler)
 * - Numbers with capitals (Base64Encoder → base64_encoder)
 * - kebab-case → snake_case
 */

import { describe, it, expect } from 'vitest'
import { normalizeForExactMatch } from '../../src/tools/search.js'

describe('normalizeForExactMatch', () => {
  describe('camelCase normalization', () => {
    it('should convert simple camelCase to snake_case', () => {
      expect(normalizeForExactMatch('validateProvider')).toBe('validate_provider')
    })

    it('should handle single word', () => {
      expect(normalizeForExactMatch('provider')).toBe('provider')
    })

    it('should handle multiple camelCase transitions', () => {
      expect(normalizeForExactMatch('getUserNameFromDatabase')).toBe(
        'get_user_name_from_database'
      )
    })
  })

  describe('Acronym handling - start of string', () => {
    it('should handle acronym at start: XMLParser', () => {
      expect(normalizeForExactMatch('XMLParser')).toBe('xml_parser')
    })

    it('should handle acronym at start: HTTPClient', () => {
      expect(normalizeForExactMatch('HTTPClient')).toBe('http_client')
    })

    it('should handle acronym at start: FTPUploader', () => {
      expect(normalizeForExactMatch('FTPUploader')).toBe('ftp_uploader')
    })
  })

  describe('Acronym handling - middle of string', () => {
    it('should handle acronym in middle: validateHTTPRequest', () => {
      expect(normalizeForExactMatch('validateHTTPRequest')).toBe('validate_http_request')
    })

    it('should handle acronym in middle: sendSMTPMessage', () => {
      expect(normalizeForExactMatch('sendSMTPMessage')).toBe('send_smtp_message')
    })

    it('should handle acronym in middle: parseJSONData', () => {
      expect(normalizeForExactMatch('parseJSONData')).toBe('parse_json_data')
    })
  })

  describe('Consecutive capitals', () => {
    it('should handle HTTPSHandler', () => {
      expect(normalizeForExactMatch('HTTPSHandler')).toBe('https_handler')
    })

    it('should handle XMLHTTPRequest', () => {
      expect(normalizeForExactMatch('XMLHTTPRequest')).toBe('xmlhttp_request')
    })

    it('should handle SSLContext', () => {
      expect(normalizeForExactMatch('SSLContext')).toBe('ssl_context')
    })
  })

  describe('Numbers with capitals', () => {
    it('should handle Base64Encoder', () => {
      expect(normalizeForExactMatch('Base64Encoder')).toBe('base64_encoder')
    })

    it('should handle MD5Hash', () => {
      expect(normalizeForExactMatch('MD5Hash')).toBe('md5_hash')
    })

    it('should handle SHA256Digest', () => {
      expect(normalizeForExactMatch('SHA256Digest')).toBe('sha256_digest')
    })
  })

  describe('kebab-case normalization', () => {
    it('should convert kebab-case to snake_case', () => {
      expect(normalizeForExactMatch('validate-provider')).toBe('validate_provider')
    })

    it('should handle multiple hyphens', () => {
      expect(normalizeForExactMatch('user-auth-service-factory')).toBe(
        'user_auth_service_factory'
      )
    })
  })

  describe('Space handling', () => {
    it('should convert spaces to underscores', () => {
      expect(normalizeForExactMatch('validate provider')).toBe('validate_provider')
    })

    it('should handle multiple spaces', () => {
      expect(normalizeForExactMatch('user  auth   service')).toBe('user_auth_service')
    })
  })

  describe('Dot handling', () => {
    it('should convert dots to underscores', () => {
      expect(normalizeForExactMatch('user.auth.service')).toBe('user_auth_service')
    })
  })

  describe('Edge cases', () => {
    it('should handle empty string', () => {
      expect(normalizeForExactMatch('')).toBe('')
    })

    it('should handle all uppercase', () => {
      expect(normalizeForExactMatch('HTTP')).toBe('http')
    })

    it('should handle all lowercase', () => {
      expect(normalizeForExactMatch('validate')).toBe('validate')
    })

    it('should clean up multiple consecutive underscores', () => {
      expect(normalizeForExactMatch('user__auth___service')).toBe('user_auth_service')
    })

    it('should remove leading underscores', () => {
      expect(normalizeForExactMatch('_privateMethod')).toBe('private_method')
    })

    it('should remove trailing underscores', () => {
      expect(normalizeForExactMatch('method_')).toBe('method')
    })

    it('should handle mixed separators', () => {
      expect(normalizeForExactMatch('user-auth.service Provider')).toBe(
        'user_auth_service_provider'
      )
    })
  })

  describe('Real-world examples', () => {
    it('should normalize TypeScript class name', () => {
      expect(normalizeForExactMatch('ValidationErrorHandler')).toBe(
        'validation_error_handler'
      )
    })

    it('should normalize React component', () => {
      expect(normalizeForExactMatch('UserAuthFormContainer')).toBe(
        'user_auth_form_container'
      )
    })

    it('should normalize Rust function', () => {
      expect(normalizeForExactMatch('execute_fts_search')).toBe('execute_fts_search')
    })

    it('should normalize kebab-case file', () => {
      expect(normalizeForExactMatch('user-profile-service')).toBe('user_profile_service')
    })
  })
})
