# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **watch**: Fixed file change detection misclassifying modified files as new files
  - **Root cause**: Path format mismatch between file watcher (absolute paths) and database (relative paths)
  - **Impact**: Watch command now correctly re-indexes modified files with updated timestamps
  - **Security**: Added file size limits (10MB) to prevent DoS attacks
  - **Security**: Added path traversal protection in normalization utility
  - **Related**: See `.agents/projects/WATCHFIX_watch-change-detection-fix/` for detailed analysis
