# Ticket: LANG_PARSE-4003: Production Migration Preparation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- parser-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Prepare production migration infrastructure for multi-language support rollout. This includes database schema validation, rollback procedures, and feature flag implementation to enable safe, controlled deployment of Python, Rust, and Go language parsing capabilities.

## Background
Phase 4 (Production Readiness) requires a safe migration path from the current TypeScript/JavaScript-only system to the multi-language parsing system. This ticket focuses on the critical infrastructure needed to deploy language support to production with minimal risk:

- Database schema must be validated to support all three new languages
- Rollback procedures are essential for quick recovery if issues arise
- Feature flags enable granular control over language enablement
- Graceful degradation ensures the system remains functional even if specific languages are disabled

This work is sourced from Phase 4, Week 7, Task 3 of the LANG_PARSE planning document and is a prerequisite for the final production rollout.

## Acceptance Criteria
- [ ] Database schema validated to support Python, Rust, and Go parsing
- [ ] Rollback migration scripts created and tested in staging environment
- [ ] Feature flags implemented: `enable_python`, `enable_rust`, `enable_go`
- [ ] Graceful degradation tested when individual languages are disabled
- [ ] Migration procedure documented with step-by-step instructions
- [ ] Rollback procedure documented with recovery time estimates
- [ ] Schema changes backwards compatible with existing TypeScript/JavaScript data
- [ ] Feature flag configuration tested across all deployment scenarios

## Technical Requirements
- Database schema validation:
  - Verify all language-specific columns and indexes are present
  - Confirm no conflicts with existing TypeScript/JavaScript data
  - Validate foreign key constraints for multi-language support
  - Test schema performance with mixed language data

- Rollback migration scripts:
  - Must cleanly reverse all Phase 4 schema changes
  - Preserve existing TypeScript/JavaScript indexed data
  - Include verification queries to confirm rollback success
  - Document expected rollback duration and data impact

- Feature flag implementation:
  - Create `language_flags.rs` module for configuration
  - Support environment variable and config file sources
  - Implement per-language enable/disable controls
  - Provide runtime flag checking for parser selection

- Graceful degradation testing:
  - System continues functioning with all languages disabled
  - TypeScript/JavaScript parsing unaffected by flag changes
  - Clear error messages when disabled language requested
  - Fallback behavior documented for each flag combination

## Implementation Notes

### Database Schema Validation
The schema validation should verify:
1. All migration files from Phases 1-3 are applied correctly
2. Language-specific columns (e.g., `language_type`, `parser_version`) exist
3. Indexes optimized for multi-language queries
4. No orphaned data or constraint violations

Validation queries should be automated and included in the migration guide.

### Rollback Procedures
Create rollback migrations that:
1. Drop language-specific tables/columns in reverse order
2. Verify data integrity after each rollback step
3. Include checkpoints for partial rollback scenarios
4. Document recovery procedures for partial rollback failures

Test rollback procedures in staging environment with representative data volumes.

### Feature Flags Architecture
Implement feature flags in `crates/maproom/src/config/language_flags.rs`:
```rust
pub struct LanguageFlags {
    pub enable_python: bool,
    pub enable_rust: bool,
    pub enable_go: bool,
}
```

Load from:
- Environment variables: `MAPROOM_ENABLE_PYTHON=true`
- Config file: `crewchief.config.ts` or `maproom.config.toml`
- Runtime API for dynamic toggling (optional)

Integrate flag checks into parser selection logic to gracefully skip disabled languages.

### Migration Documentation
The migration guide should include:
- Pre-migration checklist (backups, verification queries)
- Step-by-step migration procedure
- Expected downtime and performance impact
- Post-migration verification steps
- Rollback trigger criteria and procedures
- Communication templates for stakeholders

### Testing Requirements
- Test migration on copy of production data
- Verify rollback preserves all existing functionality
- Test each feature flag combination:
  - All enabled (default production state)
  - Individual languages enabled/disabled
  - All disabled (fallback to TypeScript/JavaScript only)
- Measure performance impact of schema changes
- Validate backwards compatibility with existing queries

## Dependencies
- **LANG_PARSE-4002** (Quality Validation) - Must be completed and passing before migration to production
- Database backup and restore procedures must be in place
- Staging environment must mirror production configuration
- Monitoring and alerting systems ready for production deployment

## Risk Assessment
- **Risk**: Schema migration causes downtime or data loss
  - **Mitigation**: Test on production copy, implement rollback scripts, schedule maintenance window, have backups ready

- **Risk**: Feature flags fail to disable languages properly
  - **Mitigation**: Comprehensive testing of all flag combinations, automated integration tests, monitoring for flag-related errors

- **Risk**: Rollback procedure fails or is incomplete
  - **Mitigation**: Test rollback in staging multiple times, document partial rollback procedures, maintain backups at each migration checkpoint

- **Risk**: Backwards compatibility issues with existing TypeScript/JavaScript data
  - **Mitigation**: Explicit backwards compatibility tests, schema changes designed to be additive, validation queries before and after migration

- **Risk**: Migration documentation incomplete or unclear
  - **Mitigation**: Peer review of documentation, dry run with different team members, include troubleshooting section

## Files/Packages Affected
- `crates/maproom/migrations/` - New rollback migration scripts
- `crates/maproom/migrations/README.md` - Migration script documentation
- `crates/maproom/src/config/language_flags.rs` - Feature flag implementation (NEW)
- `crates/maproom/src/config/mod.rs` - Integration of language flags module
- `crates/maproom/src/parser/mod.rs` - Parser selection with flag checks
- `crates/maproom/docs/migration_guide.md` - Step-by-step migration procedure (NEW)
- `crates/maproom/docs/rollback_procedure.md` - Rollback documentation (NEW)
- `crates/maproom/tests/integration/migration_test.rs` - Migration integration tests (NEW)
- `crates/maproom/tests/integration/feature_flags_test.rs` - Feature flag tests (NEW)
- `crewchief.config.ts` - Example configuration with language flags
- `.env.example` - Environment variable examples for feature flags
