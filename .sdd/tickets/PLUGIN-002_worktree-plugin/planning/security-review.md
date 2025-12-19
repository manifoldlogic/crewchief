# Security Review: Worktree Plugin

## Security Assessment

**Overall Risk Level:** Low

This plugin consists entirely of documentation (markdown and JSON files). It does not execute code, handle sensitive data, or interact with external systems directly. Security considerations focus on the CLI commands that users will execute based on the plugin's instructions.

### Authentication & Authorization

**Not Applicable**

The plugin is documentation only. It does not:
- Handle user credentials
- Perform authentication
- Make authorization decisions
- Access protected resources

The underlying crewchief CLI inherits the user's git credentials and filesystem permissions.

### Data Protection

**Not Applicable**

The plugin does not:
- Store sensitive data
- Transmit data externally
- Handle secrets or tokens

**Documentation Consideration:**
- The SKILL.md should mention that `copy-ignored` handles `.env` files
- Users should be aware that ignored files may contain secrets
- Worktrees created from the same repository share git credentials

### Input Validation

**Not Directly Applicable**

The plugin provides documentation, not executable code. Input validation is handled by the crewchief CLI.

**Documentation Consideration:**
- CLI examples should use safe placeholder values (e.g., `<name>`, not specific paths)
- Worktree names should follow git branch naming conventions
- Path arguments should be relative or clearly documented

### Known Gaps

| Gap | Risk Level | Mitigation | Status |
|-----|------------|------------|--------|
| Ignored files contain secrets | Low | Document that `copy-ignored` copies .env files | Accepted |
| Branch deletion without merge | Low | Document safety warning about data loss | Accepted |
| Worktree in sensitive location | Low | CLI uses configured base path, document default | Accepted |

### Security-Relevant CLI Behaviors

The crewchief CLI implements several security guardrails that the plugin documentation should reference:

1. **Current Worktree Protection**
   - CLI prevents deletion of the worktree containing the current working directory
   - Uses `path.relative()` to detect if cwd is inside target worktree
   - Source: `/packages/cli/src/cli/worktree.ts` lines 302-316

2. **Merge Safety**
   - CLI prevents merging while inside the worktree being merged
   - Checks for uncommitted changes before merge
   - Confirmation prompts for destructive operations
   - Source: `/packages/cli/src/cli/worktree.ts` lines 705-730

3. **Branch Deletion Safety**
   - CLI uses `git branch -d` (safe delete) by default
   - Unmerged branches require manual `git branch -D`
   - Branch deletion errors are handled gracefully with guidance
   - Source: `/packages/cli/src/cli/worktree.ts` lines 20-62

4. **Path Resolution**
   - Worktree paths are expanded and validated
   - Symlinks are resolved with `fs.realpathSync` for accurate protection
   - Source: `/packages/cli/src/git/worktrees.ts` lines 281-287

## Initial Release Security Scope

**In Scope:**
- Document all safety warnings from CLI implementation
- Explain why certain operations are prevented
- Provide guidance for handling unmerged branches
- Note that ignored files may contain sensitive data

**Out of Scope:**
- Access control for worktrees (handled by filesystem permissions)
- Git credential management (handled by git)
- Secret scanning in copied files (user responsibility)

## Security Checklist

Documentation plugin security checklist:

- [x] No hardcoded secrets (documentation only)
- [x] No external API calls (documentation only)
- [x] No user data storage (documentation only)
- [x] CLI examples use safe placeholders
- [ ] Safety warnings documented for:
  - Current worktree deletion prevention
  - Merge inside worktree prevention
  - Unmerged branch handling
  - Ignored file secrets awareness
- [ ] Dependencies are up to date (none - documentation only)
- [x] No SQL injection vulnerabilities (not applicable)
- [x] No XSS vulnerabilities (not applicable)

## Recommendations

### For Plugin Documentation

1. **Include Safety Section**
   - Document CLI-enforced protections
   - Explain what happens when protections trigger
   - Provide recovery guidance

2. **Warn About Sensitive Data**
   - Note that `copy-ignored` copies .env files
   - Remind users that ignored files may contain secrets
   - Suggest reviewing copied files in new worktrees

3. **Document Error Messages**
   - Explain common error scenarios
   - Provide actionable next steps for each

### For Future Versions

1. **Consider Secret Detection**
   - Future CLI enhancement could warn about copying files with secret patterns
   - Not required for initial release

2. **Audit Logging**
   - CLI could log worktree operations for audit trails
   - Not required for documentation plugin

## Conclusion

This plugin presents minimal security risk as it consists entirely of documentation. The primary security considerations involve accurately documenting the safety features already implemented in the crewchief CLI, and making users aware that ignored files may contain sensitive data.

**Approval:** This plugin is approved for implementation from a security perspective.
