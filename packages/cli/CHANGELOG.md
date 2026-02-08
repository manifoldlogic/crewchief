# Changelog

All notable changes to the CrewChief CLI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking Changes

#### Auto-Scan Now Opt-In

**What Changed**: Worktree creation no longer automatically triggers maproom scanning by default.

**Why**: This change dramatically improves worktree creation speed (from 5-30s to <1s) and gives users control over when indexing happens.

**Migration**: To restore automatic scanning, add one line to your `crewchief.config.js`:

```javascript
export default {
  worktree: {
    autoScanOnWorktreeUse: true, // Restore auto-scan behavior
  },
}
```

**Alternative**: Manually scan when needed: `crewchief maproom scan`

**Impact**: Users relying on automatic indexing must update config or manually scan.

### Security

- Added input validation for platform names to prevent shell injection (`validatePlatformName`)
- Added input validation for agent names to prevent path traversal (`validateAgentName`)
- Implemented defense-in-depth: pattern validation + path containment checks
- Added 25 security-focused unit tests covering shell injection and path traversal vectors
- Commit: 52c2b9c4
